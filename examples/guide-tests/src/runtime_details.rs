// ANCHOR: explicit-runtime
pub fn explicit_runtime() -> std::io::Result<u32> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        Ok(42)
    })
}
// ANCHOR_END: explicit-runtime

// ANCHOR: local-runtime
pub async fn local_work() -> usize {
    use std::{cell::Cell, rc::Rc};
    let local = tokio::task::LocalSet::new();
    let value = Rc::new(Cell::new(0));
    let other = value.clone();
    local.run_until(async move {
        tokio::task::spawn_local(async move {
            tokio::task::yield_now().await;
            other.set(7);
        }).await.unwrap();
    }).await;
    value.get()
}
// ANCHOR_END: local-runtime

// ANCHOR: blocking-bridge
pub async fn blocking_bridge() -> u32 {
    // 只能在多线程运行时使用；闭包中的block_on重新进入异步上下文。
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            7
        })
    })
}
// ANCHOR_END: blocking-bridge

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc, sync::{Arc, atomic::{AtomicBool, Ordering}}, time::Duration};

    #[test]
    fn explicit_startup_and_enter_context_are_different() {
        assert_eq!(explicit_runtime().unwrap(), 42);
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let ran = Arc::new(AtomicBool::new(false));
        let other = ran.clone();
        let task = {
            let _context = runtime.enter();
            tokio::spawn(async move { other.store(true, Ordering::SeqCst); })
        };
        assert!(!ran.load(Ordering::SeqCst));
        // Handle::block_on能运行纯Future，但不会让current_thread的任务池自行运行。
        assert_eq!(runtime.handle().block_on(async { 3 }), 3);
        assert!(!ran.load(Ordering::SeqCst));
        runtime.block_on(task).unwrap();
        assert!(ran.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn local_set_can_resume_remaining_tasks_after_run_until() {
        assert_eq!(local_work().await, 7);
        let local = tokio::task::LocalSet::new();
        let state = Rc::new(Cell::new(false));
        let other = state.clone();
        let (tx, rx) = tokio::sync::oneshot::channel();
        local.spawn_local(async move { rx.await.unwrap(); other.set(true); });
        local.run_until(async {}).await;
        assert!(!state.get());
        tx.send(()).unwrap();
        local.await;
        assert!(state.get());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn block_in_place_can_reenter_multithread_runtime() {
        assert_eq!(tokio::spawn(blocking_bridge()).await.unwrap(), 7);
    }

    #[tokio::test]
    #[should_panic(expected = "multi-threaded runtime")]
    async fn block_in_place_rejects_current_thread() {
        tokio::task::block_in_place(|| 1);
    }

    #[tokio::test]
    #[should_panic(expected = "Cannot start a runtime")]
    async fn handle_block_on_rejects_nested_async_context() {
        tokio::runtime::Handle::current().block_on(async { 1 });
    }

    struct DropFlag(Arc<AtomicBool>);
    impl Drop for DropFlag {
        fn drop(&mut self) { self.0.store(true, Ordering::SeqCst); }
    }

    #[test]
    fn handle_does_not_keep_runtime_alive() {
        let runtime = tokio::runtime::Builder::new_current_thread().build().unwrap();
        let handle = runtime.handle().clone();
        let dropped = Arc::new(AtomicBool::new(false));
        let guard = DropFlag(dropped.clone());
        let task = runtime.spawn(async move {
            let _guard = guard;
            futures::future::pending::<()>().await;
        });
        drop(runtime);
        assert!(dropped.load(Ordering::SeqCst));
        assert!(futures::executor::block_on(task).unwrap_err().is_cancelled());
        // 与驱动器无关的计算仍可由Handle运行，不表示IO资源还有效。
        assert_eq!(handle.block_on(async { 5 }), 5);
    }

    #[test]
    fn shutdown_timeout_does_not_terminate_started_blocking_work() {
        let runtime = tokio::runtime::Builder::new_multi_thread().worker_threads(1).build().unwrap();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let task = runtime.spawn_blocking(move || {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            9
        });
        started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        runtime.shutdown_timeout(Duration::ZERO);
        assert!(!task.is_finished());
        release_tx.send(()).unwrap();
        assert_eq!(futures::executor::block_on(task).unwrap(), 9);
    }
}
