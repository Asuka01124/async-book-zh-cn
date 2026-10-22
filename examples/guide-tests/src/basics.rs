// ANCHOR: add
use std::time::Duration;
use tokio::time::sleep;

// 异步函数也可以不等待任何外部事件。
pub async fn add(a: u32, b: u32) -> u32 {
    a + b
}

pub async fn wait_to_add(a: u32, b: u32) -> u32 {
    sleep(Duration::from_millis(1000)).await;
    a + b
}
// ANCHOR_END: add

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[tokio::test(start_paused = true)]
    async fn async_function_is_lazy_and_waiting_advances_time() {
        let executed = Cell::new(false);
        let future = async {
            executed.set(true);
            add(15, 3).await
        };
        assert!(!executed.get());
        assert_eq!(future.await, 18);
        assert!(executed.get());
        let start = tokio::time::Instant::now();
        assert_eq!(wait_to_add(15, 3).await, 18);
        assert!(start.elapsed() >= Duration::from_secs(1));
    }

    #[tokio::test]
    async fn dropping_join_handle_does_not_cancel_task() {
        let (send, receive) = tokio::sync::oneshot::channel();
        let handle = tokio::spawn(async move {
            send.send(42).expect("接收端仍存活");
        });
        drop(handle);
        assert_eq!(receive.await.expect("任务应发送结果"), 42);
    }

    #[tokio::test]
    async fn abort_is_observed_as_join_error() {
        let handle = tokio::spawn(std::future::pending::<()>());
        handle.abort();
        assert!(handle.await.expect_err("任务应被取消").is_cancelled());
    }
}
