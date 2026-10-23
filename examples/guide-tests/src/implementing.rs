use futures::{stream::FusedStream, Stream};
use std::{future::Future, pin::Pin, sync::{Arc, Mutex}, task::{Context, Poll, Waker}};

// ANCHOR: oneshot
#[derive(Debug, PartialEq, Eq)]
pub struct Closed;
struct State {
    value: Option<u32>,
    sender_alive: bool,
    receiver_alive: bool,
    waker: Option<Waker>,
}
pub struct Sender(Arc<Mutex<State>>);
pub struct Receiver { shared: Arc<Mutex<State>>, done: bool }

pub fn channel() -> (Sender, Receiver) {
    let shared = Arc::new(Mutex::new(State {
        value: None, sender_alive: true, receiver_alive: true, waker: None,
    }));
    (Sender(shared.clone()), Receiver { shared, done: false })
}

impl Sender {
    pub fn send(self, value: u32) -> Result<(), u32> {
        let wake = {
            let mut state = self.0.lock().unwrap();
            if !state.receiver_alive { return Err(value); }
            state.value = Some(value);
            state.waker.take()
        };
        if let Some(waker) = wake { waker.wake(); }
        Ok(())
    }
}
impl Drop for Sender {
    fn drop(&mut self) {
        let wake = {
            let mut state = self.0.lock().unwrap();
            state.sender_alive = false;
            state.waker.take()
        };
        if let Some(waker) = wake { waker.wake(); }
    }
}
impl Future for Receiver {
    type Output = Result<u32, Closed>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut(); // Arc、bool 均 Unpin，无固定字段投影。
        assert!(!this.done, "不能在完成后再次轮询 Receiver");
        let new_waker = cx.waker().clone(); // clone/drop 回调也在锁外运行。
        let mut state = this.shared.lock().unwrap();
        let result = if let Some(value) = state.value.take() { Ok(value) }
            else if !state.sender_alive { Err(Closed) }
            else {
                let old = state.waker.replace(new_waker);
                drop(state);
                drop(old);
                return Poll::Pending;
            };
        let old = state.waker.take();
        drop(state);
        drop(old);
        this.done = true;
        Poll::Ready(result)
    }
}
impl Drop for Receiver {
    fn drop(&mut self) {
        let mut state = self.shared.lock().unwrap();
        state.receiver_alive = false;
        let old = state.waker.take();
        drop(state);
        drop(old);
    }
}
// ANCHOR_END: oneshot

// ANCHOR: once-stream
pub struct OnceStream<F> { future: Option<Pin<Box<F>>> }
impl<F> OnceStream<F> {
    pub fn new(future: F) -> Self { Self { future: Some(Box::pin(future)) } }
}
impl<F: Future> Stream for OnceStream<F> {
    type Item = F::Output;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut(); // 移动 Pin<Box<F>> 句柄不移动 F。
        match this.future.as_mut() {
            None => Poll::Ready(None),
            Some(future) => match future.as_mut().poll(cx) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(value) => { this.future = None; Poll::Ready(Some(value)) }
            }
        }
    }
}
impl<F: Future> FusedStream for OnceStream<F> {
    fn is_terminated(&self) -> bool { self.future.is_none() }
}
// ANCHOR_END: once-stream

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;
    use std::{sync::atomic::{AtomicUsize, Ordering}, task::Wake};
    struct Count(AtomicUsize);
    impl Wake for Count {
        fn wake(self: Arc<Self>) { self.0.fetch_add(1, Ordering::SeqCst); }
        fn wake_by_ref(self: &Arc<Self>) { self.0.fetch_add(1, Ordering::SeqCst); }
    }
    #[test]
    fn implementing_waker_drop_runs_outside_lock() {
        struct Probe { shared: Arc<Mutex<State>>, unlocked: Arc<AtomicUsize> }
        impl Wake for Probe { fn wake(self: Arc<Self>) {} }
        impl Drop for Probe {
            fn drop(&mut self) {
                if self.shared.try_lock().is_ok() {
                    self.unlocked.fetch_add(1, Ordering::SeqCst);
                }
            }
        }
        let (_tx, mut rx) = channel();
        let unlocked = Arc::new(AtomicUsize::new(0));
        for _ in 0..2 {
            let w = Waker::from(Arc::new(Probe { shared: rx.shared.clone(), unlocked: unlocked.clone() }));
            assert!(Pin::new(&mut rx).poll(&mut Context::from_waker(&w)).is_pending());
            drop(w); // 注册中的副本现在是该 Probe 唯一的 Waker。
        }
        assert_eq!(unlocked.load(Ordering::SeqCst), 1); // 替换旧注册。
        drop(rx);
        assert_eq!(unlocked.load(Ordering::SeqCst), 2); // 取消注销。
    }
    #[test]
    fn implementing_latest_waker_is_used_and_real_thread_wakes() {
        let (tx, mut rx) = channel();
        let old = Arc::new(Count(AtomicUsize::new(0)));
        let new = Arc::new(Count(AtomicUsize::new(0)));
        let a = Waker::from(old.clone()); let b = Waker::from(new.clone());
        assert!(Pin::new(&mut rx).poll(&mut Context::from_waker(&a)).is_pending());
        assert!(Pin::new(&mut rx).poll(&mut Context::from_waker(&b)).is_pending());
        std::thread::spawn(move || tx.send(42).unwrap()).join().unwrap();
        assert_eq!(old.0.load(Ordering::SeqCst), 0);
        assert_eq!(new.0.load(Ordering::SeqCst), 1);
        assert_eq!(futures::executor::block_on(rx), Ok(42));
    }
    #[test]
    fn implementing_completion_before_registration_and_sender_close() {
        let (tx, rx) = channel(); tx.send(9).unwrap();
        assert_eq!(futures::executor::block_on(rx), Ok(9));
        let (tx, rx) = channel(); drop(tx);
        assert_eq!(futures::executor::block_on(rx), Err(Closed));
    }
    #[test]
    fn implementing_cancel_unregisters_and_rejects_late_send() {
        let (tx, mut rx) = channel();
        let count = Arc::new(Count(AtomicUsize::new(0)));
        let waker = Waker::from(count.clone());
        assert!(Pin::new(&mut rx).poll(&mut Context::from_waker(&waker)).is_pending());
        drop(rx);
        assert_eq!(tx.send(11), Err(11));
        assert_eq!(count.0.load(Ordering::SeqCst), 0);
    }
    #[test]
    fn implementing_stream_preserves_pending_and_fuses() {
        let (tx, rx) = channel();
        let mut s = OnceStream::new(async move { rx.await.unwrap() });
        let waker = futures::task::noop_waker();
        assert!(Pin::new(&mut s).poll_next(&mut Context::from_waker(&waker)).is_pending());
        assert!(!s.is_terminated());
        tx.send(17).unwrap();
        futures::executor::block_on(async {
            assert_eq!(s.next().await, Some(17));
            assert!(s.is_terminated());
            assert_eq!(s.next().await, None);
            assert_eq!(s.next().await, None);
        });
    }
}
