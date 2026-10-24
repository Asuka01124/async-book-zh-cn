use std::sync::{Arc, Condvar, Mutex, atomic::{AtomicBool, Ordering}};
use tokio::sync::oneshot;

// ANCHOR: callback-model
// 安全Rust模型：没有链接C库，也没有把Arc布局当作C ABI。
struct Context {
    response: Mutex<Option<oneshot::Sender<Result<u32, i32>>>>,
}
struct State {
    registered: bool,
    in_flight: usize,
    context: Option<Arc<Context>>,
}
pub struct MockForeign {
    state: Mutex<State>,
    idle: Condvar,
    cancel_requested: AtomicBool,
}
pub struct CallbackLease {
    foreign: Arc<MockForeign>,
    context: Option<Arc<Context>>,
}
impl MockForeign {
    pub fn request_cancel(&self) {
        self.cancel_requested.store(true, Ordering::SeqCst);
        // 请求不等于注销；外库仍可能交付已经准备好的回调。
    }
    pub fn try_begin_callback(self: &Arc<Self>) -> Option<CallbackLease> {
        let mut state = self.state.lock().unwrap();
        if !state.registered { return None; }
        state.in_flight += 1;
        let context = state.context.as_ref().unwrap().clone();
        Some(CallbackLease { foreign: self.clone(), context: Some(context) })
    }
    fn unregister(&self) {
        self.state.lock().unwrap().registered = false;
    }
    fn wait_for_callbacks(&self) {
        let mut state = self.state.lock().unwrap();
        assert!(!state.registered);
        while state.in_flight != 0 { state = self.idle.wait(state).unwrap(); }
        let context = state.context.take();
        drop(state);
        drop(context); // 注销且所有在途回调结束之后才能回收注册上下文。
    }
}
impl CallbackLease {
    pub fn complete(&self, status: i32, value: u32) {
        let sender = self.context.as_ref().unwrap().response.lock().unwrap().take();
        if let Some(sender) = sender {
            let result = if status == 0 { Ok(value) } else { Err(status) };
            let _ = sender.send(result); // 等待者已经取消也不能让回调panic。
        }
    }
}
impl Drop for CallbackLease {
    fn drop(&mut self) {
        drop(self.context.take()); // 确认归零后，不再有本回调对上下文的访问或引用。
        let mut state = self.foreign.state.lock().unwrap();
        state.in_flight -= 1;
        let idle = state.in_flight == 0;
        drop(state);
        if idle { self.foreign.idle.notify_all(); }
    }
}
pub struct Registration { foreign: Arc<MockForeign> }
impl Registration {
    pub fn start() -> (Self, oneshot::Receiver<Result<u32, i32>>) {
        let (tx, rx) = oneshot::channel();
        let context = Arc::new(Context { response: Mutex::new(Some(tx)) });
        let foreign = Arc::new(MockForeign {
            state: Mutex::new(State { registered: true, in_flight: 0, context: Some(context) }),
            idle: Condvar::new(), cancel_requested: AtomicBool::new(false),
        });
        (Self { foreign }, rx)
    }
    pub fn mock_foreign(&self) -> Arc<MockForeign> { self.foreign.clone() }
    pub async fn shutdown(self) -> Result<(), tokio::task::JoinError> {
        self.foreign.unregister();
        let foreign = self.foreign.clone();
        tokio::task::spawn_blocking(move || foreign.wait_for_callbacks()).await
    }
}
impl Drop for Registration {
    fn drop(&mut self) {
        self.foreign.unregister();
        // 此安全模型的在途租约拥有Arc，故即使漏掉显式shutdown也不会悬空。
        // 真正裸指针FFI若无此所有权机制，必须由监督者保活并等待确认。
    }
}
// ANCHOR_END: callback-model

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn async_ffi_callback_from_thread_delivers_result() {
        let (registration, response) = Registration::start();
        let foreign = registration.mock_foreign();
        let callback = foreign.try_begin_callback().unwrap();
        let thread = std::thread::spawn(move || callback.complete(0, 42));
        assert_eq!(response.await.unwrap(), Ok(42));
        tokio::task::spawn_blocking(move || thread.join().unwrap()).await.unwrap();
        registration.shutdown().await.unwrap();
        assert!(foreign.try_begin_callback().is_none());
    }

    #[tokio::test]
    async fn async_ffi_cancelled_waiter_retains_context_until_quiescence() {
        let (registration, response) = Registration::start();
        let foreign = registration.mock_foreign();
        let weak = Arc::downgrade(foreign.state.lock().unwrap().context.as_ref().unwrap());
        drop(response);
        foreign.request_cancel();
        assert!(foreign.cancel_requested.load(Ordering::SeqCst));
        let callback = foreign.try_begin_callback().unwrap(); // 请求取消后仍可能迟到。
        let shutdown = registration.shutdown();
        tokio::pin!(shutdown);
        assert!(futures::poll!(&mut shutdown).is_pending());
        assert!(foreign.try_begin_callback().is_none()); // 注销后拒绝新回调。
        assert!(weak.upgrade().is_some()); // 已进入的回调还没有退出。
        let thread = std::thread::spawn(move || {
            callback.complete(0, 7); // 响应已关闭，回调安全地丢弃回复。
            drop(callback);
        });
        shutdown.await.unwrap();
        assert!(weak.upgrade().is_none());
        tokio::task::spawn_blocking(move || thread.join().unwrap()).await.unwrap();
    }

    #[tokio::test]
    async fn async_ffi_status_error_and_duplicate_completion_are_explicit() {
        let (registration, response) = Registration::start();
        let foreign = registration.mock_foreign();
        let callback = foreign.try_begin_callback().unwrap();
        callback.complete(-5, 0);
        callback.complete(0, 9); // 此模型只交付第一个完成结果。
        assert_eq!(response.await.unwrap(), Err(-5));
        drop(callback);
        registration.shutdown().await.unwrap();
    }
}
