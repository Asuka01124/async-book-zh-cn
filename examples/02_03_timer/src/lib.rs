// ANCHOR: imports
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
    thread,
    time::Duration,
};
// ANCHOR_END: imports

// ANCHOR: timer_decl
pub struct TimerFuture {
    shared_state: Arc<Mutex<SharedState>>,
}

/// Future 与等待线程之间共享的状态
struct SharedState {
    /// 休眠时间是否已经结束
    completed: bool,

    /// `TimerFuture` 所在任务的 Waker。
    /// 在线程将 `completed = true` 后，可以用它通知 `TimerFuture` 所在的任务唤醒，
    /// 查看 `completed = true`，并继续推进。
    waker: Option<Waker>,
}
// ANCHOR_END: timer_decl

// ANCHOR: future_for_timer
impl Future for TimerFuture {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // 已完成时不再克隆句柄。锁守卫在此检查结束后释放。
        if self.shared_state.lock().unwrap().completed {
            return Poll::Ready(());
        }
        // 自定义 Waker 的 clone/drop/wake 都可能执行任意代码，必须在锁外。
        let mut new_waker = Some(cx.waker().clone());
        let (result, old_waker) = {
            let mut state = self.shared_state.lock().unwrap();
            // 克隆期间也可能完成，因此在注册前重新检查，不能丢失通知。
            if state.completed {
                (Poll::Ready(()), None)
            } else {
                let old = state.waker.replace(new_waker.take().unwrap());
                (Poll::Pending, old)
            }
        };
        drop(old_waker);
        drop(new_waker);
        result
    }
}
// ANCHOR_END: future_for_timer

// ANCHOR: timer_new
impl TimerFuture {
    /// 创建一个新的 `TimerFuture`，它会在指定的等待时长后完成。
    pub fn new(duration: Duration) -> Self {
        let shared_state = Arc::new(Mutex::new(SharedState {
            completed: false,
            waker: None,
        }));

        // 启动新线程
        let thread_shared_state = shared_state.clone();
        thread::spawn(move || {
            thread::sleep(duration);
            Self::complete(&thread_shared_state);
        });

        TimerFuture { shared_state }
    }

    fn complete(shared_state: &Mutex<SharedState>) {
        let waker = {
            let mut state = shared_state.lock().unwrap();
            state.completed = true;
            state.waker.take()
        };
        // 先发布完成状态，再在锁外通知；没有注册者也不会丢失完成状态。
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}
// ANCHOR_END: timer_new

#[test]
fn block_on_timer() {
    futures::executor::block_on(async {
        TimerFuture::new(Duration::from_secs(1)).await
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{mem::ManuallyDrop, sync::{Weak, atomic::{AtomicUsize, Ordering}}, task::{RawWaker, RawWakerVTable}};

    struct Probe {
        state: Weak<Mutex<SharedState>>,
        clones: AtomicUsize,
        drops: AtomicUsize,
        wakes: AtomicUsize,
        complete_on_clone: bool,
    }
    impl Probe {
        fn check_unlocked(&self) {
            let state = self.state.upgrade().unwrap();
            assert!(state.try_lock().is_ok(), "Waker 回调发生在共享状态锁内");
        }
    }
    // SAFETY：每个 data 都来自 Arc::into_raw；clone 增加强计数，wake/drop 各消耗
    // 一个强计数，wake_by_ref 只借用。Probe 的全部字段可跨线程安全访问。
    unsafe fn clone_raw(data: *const ()) -> RawWaker {
        let probe = ManuallyDrop::new(unsafe { Arc::from_raw(data.cast::<Probe>()) });
        probe.check_unlocked();
        probe.clones.fetch_add(1, Ordering::SeqCst);
        if probe.complete_on_clone {
            TimerFuture::complete(&probe.state.upgrade().unwrap());
        }
        raw(Arc::clone(&probe))
    }
    unsafe fn wake_raw(data: *const ()) {
        let probe = unsafe { Arc::from_raw(data.cast::<Probe>()) };
        probe.check_unlocked();
        probe.wakes.fetch_add(1, Ordering::SeqCst);
    }
    unsafe fn wake_ref_raw(data: *const ()) {
        let probe = ManuallyDrop::new(unsafe { Arc::from_raw(data.cast::<Probe>()) });
        probe.check_unlocked();
        probe.wakes.fetch_add(1, Ordering::SeqCst);
    }
    unsafe fn drop_raw(data: *const ()) {
        let probe = unsafe { Arc::from_raw(data.cast::<Probe>()) };
        probe.check_unlocked();
        probe.drops.fetch_add(1, Ordering::SeqCst);
    }
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone_raw, wake_raw, wake_ref_raw, drop_raw);
    fn raw(probe: Arc<Probe>) -> RawWaker {
        RawWaker::new(Arc::into_raw(probe).cast(), &VTABLE)
    }
    fn probe(state: &Arc<Mutex<SharedState>>, complete_on_clone: bool) -> (Arc<Probe>, Waker) {
        let probe = Arc::new(Probe {
            state: Arc::downgrade(state), clones: AtomicUsize::new(0),
            drops: AtomicUsize::new(0), wakes: AtomicUsize::new(0), complete_on_clone,
        });
        // SAFETY：raw 的引用计数和线程安全契约由上方 vtable 维持。
        let waker = unsafe { Waker::from_raw(raw(probe.clone())) };
        (probe, waker)
    }
    fn timer() -> TimerFuture {
        TimerFuture { shared_state: Arc::new(Mutex::new(SharedState { completed: false, waker: None })) }
    }
    #[test]
    fn latest_waker_and_callbacks_outside_lock() {
        let mut timer = timer();
        let state = timer.shared_state.clone();
        let (first, w1) = probe(&state, false);
        let (latest, w2) = probe(&state, false);
        assert!(Pin::new(&mut timer).poll(&mut Context::from_waker(&w1)).is_pending());
        assert!(Pin::new(&mut timer).poll(&mut Context::from_waker(&w2)).is_pending());
        assert_eq!(first.drops.load(Ordering::SeqCst), 1);
        TimerFuture::complete(&state);
        assert_eq!(first.wakes.load(Ordering::SeqCst), 0);
        assert_eq!(latest.wakes.load(Ordering::SeqCst), 1);
        assert!(Pin::new(&mut timer).poll(&mut Context::from_waker(&w2)).is_ready());
        assert_eq!(latest.clones.load(Ordering::SeqCst), 1);
        assert!(state.lock().unwrap().waker.is_none());
        drop((w1, w2));
    }
    #[test]
    fn completion_during_registration_and_before_first_poll() {
        let mut timer = timer();
        let state = timer.shared_state.clone();
        let (probe, waker) = probe(&state, true);
        // clone 回调精确模拟完成落在首次检查与注册之间，没有计时或调度依赖。
        assert!(Pin::new(&mut timer).poll(&mut Context::from_waker(&waker)).is_ready());
        assert_eq!(probe.clones.load(Ordering::SeqCst), 1);
        assert_eq!(probe.drops.load(Ordering::SeqCst), 1);
        assert!(state.lock().unwrap().waker.is_none());
        drop(waker);
        let mut already_done = self::timer();
        TimerFuture::complete(&already_done.shared_state);
        let (probe, waker) = self::probe(&already_done.shared_state, false);
        assert!(Pin::new(&mut already_done).poll(&mut Context::from_waker(&waker)).is_ready());
        assert_eq!(probe.clones.load(Ordering::SeqCst), 0);
        drop(waker);
    }
}
