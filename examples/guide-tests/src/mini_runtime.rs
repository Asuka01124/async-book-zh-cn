use std::{collections::{HashMap, VecDeque}, future::Future, pin::Pin,
    sync::{Arc, Condvar, Mutex, Weak}, task::{Context, Poll, Wake, Waker},
    time::{Duration, Instant}};

// ANCHOR: scheduler
#[derive(Default)]
struct QueueState { ready: VecDeque<usize>, closed: bool }
#[derive(Default)]
struct Queue { state: Mutex<QueueState>, changed: Condvar }
struct TaskWake { id: usize, queue: Weak<Queue> }
impl Wake for TaskWake {
    fn wake(self: Arc<Self>) { self.wake_by_ref(); }
    fn wake_by_ref(self: &Arc<Self>) {
        if let Some(queue) = self.queue.upgrade() {
            let mut state = queue.state.lock().unwrap();
            if !state.closed {
                state.ready.push_back(self.id);
                queue.changed.notify_one();
            }
        }
    }
}
type Task = Pin<Box<dyn Future<Output = ()>>>;
pub struct MiniRuntime {
    queue: Arc<Queue>, tasks: HashMap<usize, Task>, next_id: usize,
    timers: Vec<(Instant, Weak<Mutex<TimerState>>)>,
}
impl Default for MiniRuntime {
    fn default() -> Self {
        Self { queue: Arc::new(Queue::default()), tasks: HashMap::new(), next_id: 0, timers: Vec::new() }
    }
}
impl MiniRuntime {
    pub fn spawn(&mut self, future: impl Future<Output = ()> + 'static) -> usize {
        assert!(!self.queue.state.lock().unwrap().closed, "运行时已经关闭");
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).expect("任务ID耗尽");
        self.tasks.insert(id, Box::pin(future));
        self.queue.state.lock().unwrap().ready.push_back(id);
        id
    }
    pub fn tick(&mut self) -> bool {
        let id = self.queue.state.lock().unwrap().ready.pop_front();
        let Some(id) = id else { return false; };
        let Some(future) = self.tasks.get_mut(&id) else { return true; };
        let waker = Waker::from(Arc::new(TaskWake { id, queue: Arc::downgrade(&self.queue) }));
        if future.as_mut().poll(&mut Context::from_waker(&waker)).is_ready() {
            self.tasks.remove(&id); // poll已经结束，队列锁没有持有。
        }
        true
    }
    pub fn cancel(&mut self, id: usize) -> bool { self.tasks.remove(&id).is_some() }
    pub fn run(&mut self) {
        while !self.tasks.is_empty() {
            self.drive_timers();
            if self.tick() { continue; }
            let deadline = self.timers.iter().map(|entry| entry.0).min();
            let mut state = self.queue.state.lock().unwrap();
            // wake若在检查之前到达，会留下队列项；之后到达则会通知condvar。
            while state.ready.is_empty() {
                if let Some(deadline) = deadline {
                    let wait = deadline.saturating_duration_since(Instant::now());
                    let (guard, timed) = self.queue.changed.wait_timeout(state, wait).unwrap();
                    state = guard;
                    if timed.timed_out() || Instant::now() >= deadline { break; }
                } else { state = self.queue.changed.wait(state).unwrap(); }
            }
        }
    }
}
// ANCHOR_END: scheduler

// ANCHOR: timer-driver
#[derive(Debug, PartialEq, Eq)]
pub struct TimerClosed;
#[derive(Default)]
struct TimerState { fired: bool, closed: bool, waker: Option<Waker> }
pub struct Sleep(Arc<Mutex<TimerState>>);
impl Future for Sleep {
    type Output = Result<(), TimerClosed>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let new = cx.waker().clone();
        let mut state = self.0.lock().unwrap();
        if state.closed { return Poll::Ready(Err(TimerClosed)); }
        if state.fired { return Poll::Ready(Ok(())); }
        let old = state.waker.replace(new);
        drop(state); drop(old);
        Poll::Pending
    }
}
impl Drop for Sleep {
    fn drop(&mut self) {
        let old = self.0.lock().unwrap().waker.take();
        drop(old);
    }
}
impl MiniRuntime {
    pub fn sleep(&mut self, duration: Duration) -> Sleep {
        assert!(!self.queue.state.lock().unwrap().closed, "运行时已经关闭");
        let state = Arc::new(Mutex::new(TimerState::default()));
        let deadline = Instant::now().checked_add(duration).expect("期限溢出");
        self.timers.push((deadline, Arc::downgrade(&state)));
        Sleep(state)
    }
    fn drive_timers(&mut self) {
        let now = Instant::now();
        let mut wakes = Vec::new();
        self.timers.retain(|(deadline, weak)| {
            let Some(timer) = weak.upgrade() else { return false; };
            if *deadline > now { return true; }
            let mut state = timer.lock().unwrap();
            state.fired = true;
            if let Some(waker) = state.waker.take() { wakes.push(waker); }
            false
        });
        for waker in wakes { waker.wake(); }
    }
    pub fn shutdown(&mut self) {
        {
            let mut state = self.queue.state.lock().unwrap();
            state.closed = true; state.ready.clear();
        }
        let mut wakes = Vec::new();
        for (_, weak) in self.timers.drain(..) {
            if let Some(timer) = weak.upgrade() {
                let mut state = timer.lock().unwrap();
                state.closed = true;
                if let Some(waker) = state.waker.take() { wakes.push(waker); }
            }
        }
        for waker in wakes { waker.wake(); }
        self.tasks.clear(); // 本例的强制取消：只执行同步析构。
    }
}
impl Drop for MiniRuntime { fn drop(&mut self) { self.shutdown(); } }
// ANCHOR_END: timer-driver

// ANCHOR: runtime-demo
pub fn demo() -> Vec<u32> {
    let mut runtime = MiniRuntime::default();
    let out = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    for (delay, n) in [(2, 2), (0, 1)] {
        let sleep = runtime.sleep(Duration::from_millis(delay));
        let out = out.clone();
        runtime.spawn(async move { sleep.await.unwrap(); out.borrow_mut().push(n); });
    }
    runtime.run();
    let values = out.borrow().clone();
    values
}
// ANCHOR_END: runtime-demo

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc, sync::atomic::{AtomicUsize, Ordering}};
    #[test]
    fn mini_runtime_drives_timers_and_non_send_tasks() {
        let mut values = demo(); values.sort_unstable(); assert_eq!(values, [1, 2]);
    }
    #[test]
    fn mini_runtime_wake_during_poll_is_retained() {
        let mut runtime = MiniRuntime::default();
        let polls = Rc::new(Cell::new(0)); let p = polls.clone();
        runtime.spawn(std::future::poll_fn(move |cx| {
            p.set(p.get() + 1);
            if p.get() == 1 { cx.waker().wake_by_ref(); Poll::Pending }
            else { Poll::Ready(()) }
        }));
        runtime.run(); assert_eq!(polls.get(), 2);
    }
    #[test]
    fn mini_runtime_cross_thread_wake_and_no_busy_poll() {
        let mut runtime = MiniRuntime::default();
        let (tx, rx) = crate::implementing::channel();
        let polls = Arc::new(AtomicUsize::new(0)); let p = polls.clone();
        let mut rx = Box::pin(rx);
        runtime.spawn(std::future::poll_fn(move |cx| { p.fetch_add(1, Ordering::SeqCst); rx.as_mut().poll(cx).map(|r| { assert_eq!(r, Ok(33)); }) }));
        assert!(runtime.tick()); // 确定已经Pending并注册，才让生产者运行。
        assert_eq!(polls.load(Ordering::SeqCst), 1);
        let thread = std::thread::spawn(move || tx.send(33).unwrap());
        runtime.run(); thread.join().unwrap();
        assert_eq!(polls.load(Ordering::SeqCst), 2);
    }
    #[test]
    fn mini_runtime_cancel_shutdown_and_late_wake() {
        struct DropFlag(Rc<Cell<bool>>);
        impl Drop for DropFlag { fn drop(&mut self) { self.0.set(true); } }
        let mut runtime = MiniRuntime::default();
        let dropped = Rc::new(Cell::new(false));
        let guard = DropFlag(dropped.clone());
        let last = Arc::new(Mutex::new(None)); let copy = last.clone();
        let id = runtime.spawn(async move {
            let _guard = guard;
            std::future::poll_fn(|cx| { *copy.lock().unwrap() = Some(cx.waker().clone()); Poll::<()>::Pending }).await;
        });
        runtime.tick(); assert!(runtime.cancel(id)); assert!(dropped.get());
        last.lock().unwrap().take().unwrap().wake();
        assert!(runtime.tick()); // 已取消ID只成为一个可忽略的陈旧队列项。
        struct CountWake(AtomicUsize);
        impl Wake for CountWake {
            fn wake(self: Arc<Self>) { self.0.fetch_add(1, Ordering::SeqCst); }
        }
        let mut sleep = runtime.sleep(Duration::from_secs(3600));
        let notified = Arc::new(CountWake(AtomicUsize::new(0)));
        let waker = Waker::from(notified.clone());
        assert!(Pin::new(&mut sleep).poll(&mut Context::from_waker(&waker)).is_pending());
        runtime.shutdown();
        assert_eq!(notified.0.load(Ordering::SeqCst), 1);
        assert_eq!(futures::executor::block_on(sleep), Err(TimerClosed));
        runtime.run();
    }
}
