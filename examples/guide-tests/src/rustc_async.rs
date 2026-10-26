//! 用可观察的挂起与析构检查编译器生成的 Future。
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll},
};

// ANCHOR: rustc-yield-once
pub struct YieldOnce(bool);
impl YieldOnce {
    pub fn new() -> Self { Self(false) }
}
impl Default for YieldOnce {
    fn default() -> Self { Self::new() }
}
impl Future for YieldOnce {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0 { return Poll::Ready(()); }
        self.0 = true;
        // 下一次已经能够完成，先请求再次轮询，再真实返回 Pending。
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}
// ANCHOR_END: rustc-yield-once

pub type Events = Arc<Mutex<Vec<&'static str>>>;
pub struct Probe { name: &'static str, events: Events }
impl Probe {
    pub fn new(name: &'static str, events: &Events) -> Self {
        Self { name, events: events.clone() }
    }
}
impl Drop for Probe {
    fn drop(&mut self) { self.events.lock().unwrap().push(self.name); }
}

// ANCHOR: rustc-stages
pub async fn stages(input: Probe, events: Events) -> usize {
    events.lock().unwrap().push("进入函数体");
    let first = Probe::new("释放第一阶段", &events);
    YieldOnce::new().await;
    drop(first);
    let second = Probe::new("释放第二阶段", &events);
    YieldOnce::new().await;
    drop(second);
    drop(input);
    events.lock().unwrap().push("正常完成");
    7
}
// ANCHOR_END: rustc-stages

// ANCHOR: rustc-borrow-and-send
pub async fn borrowed_len(text: String) -> usize {
    let view = &text;
    YieldOnce::new().await;
    view.len()
}

pub async fn scoped_rc() -> usize {
    let n = {
        let local = std::rc::Rc::new(7);
        *local
    }; // Rc 在挂起之前离开作用域，跨挂起保存的是 usize。
    YieldOnce::new().await;
    n
}
// ANCHOR_END: rustc-borrow-and-send

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct CountWake(AtomicUsize);
    impl std::task::Wake for CountWake {
        fn wake(self: Arc<Self>) { self.0.fetch_add(1, Ordering::SeqCst); }
        fn wake_by_ref(self: &Arc<Self>) { self.0.fetch_add(1, Ordering::SeqCst); }
    }
    fn events() -> Events { Arc::new(Mutex::new(Vec::new())) }
    fn snapshot(events: &Events) -> Vec<&'static str> { events.lock().unwrap().clone() }

    #[test]
    fn rustc_async_capture_and_each_suspended_drop_path() {
        for polls in 0..=3 {
            let log = events();
            let parameter = {
                log.lock().unwrap().push("求值参数");
                Probe::new("释放参数", &log)
            };
            let mut future = Box::pin(stages(parameter, log.clone()));
            assert_eq!(snapshot(&log), ["求值参数"]);
            let wake = Arc::new(CountWake(AtomicUsize::new(0)));
            let waker = std::task::Waker::from(wake.clone());
            let mut cx = Context::from_waker(&waker);
            for poll in 0..polls {
                assert_eq!(future.as_mut().poll(&mut cx),
                    if poll < 2 { Poll::Pending } else { Poll::Ready(7) });
            }
            assert_eq!(wake.0.load(Ordering::SeqCst), polls.min(2));
            drop(future);
            let expected: &[&str] = match polls {
                0 => &["求值参数", "释放参数"],
                1 => &["求值参数", "进入函数体", "释放第一阶段", "释放参数"],
                2 => &["求值参数", "进入函数体", "释放第一阶段", "释放第二阶段", "释放参数"],
                _ => &["求值参数", "进入函数体", "释放第一阶段", "释放第二阶段", "释放参数", "正常完成"],
            };
            assert_eq!(snapshot(&log), expected);
        }
    }

    #[test]
    fn rustc_async_borrow_survives_pending_and_scoped_rc_is_send() {
        fn assert_send<T: Send>(_: &T) {}
        let mut future = Box::pin(borrowed_len(String::from("rustc")));
        let mut cx = Context::from_waker(std::task::Waker::noop());
        assert_eq!(future.as_mut().poll(&mut cx), Poll::Pending);
        assert_eq!(future.as_mut().poll(&mut cx), Poll::Ready(5));
        let future = scoped_rc();
        assert_send(&future);
        assert_eq!(futures::executor::block_on(future), 7);
    }
}
