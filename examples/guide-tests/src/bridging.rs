use tokio::sync::{mpsc, oneshot, Semaphore};
use std::sync::Arc;

// ANCHOR: service
struct Request { value: u32, reply: oneshot::Sender<u32> }
pub struct Service {
    sender: Option<mpsc::Sender<Request>>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Service {
    pub fn start() -> Self {
        let (sender, mut receiver) = mpsc::channel::<Request>(1);
        let thread = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
            runtime.block_on(async move {
                while let Some(request) = receiver.recv().await {
                    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                    let _ = request.reply.send(request.value.saturating_mul(2));
                }
            });
        });
        Self { sender: Some(sender), thread: Some(thread) }
    }
    pub fn request_blocking(&self, value: u32) -> Result<u32, &'static str> {
        let (reply, result) = oneshot::channel();
        self.sender.as_ref().unwrap().blocking_send(Request { value, reply }).map_err(|_| "服务关闭")?;
        result.blocking_recv().map_err(|_| "响应中断")
    }
    pub async fn request_async(&self, value: u32) -> Result<u32, &'static str> {
        let (reply, result) = oneshot::channel();
        self.sender.as_ref().unwrap().send(Request { value, reply }).await.map_err(|_| "服务关闭")?;
        result.await.map_err(|_| "响应中断")
    }
    pub fn shutdown(mut self) -> std::thread::Result<()> {
        drop(self.sender.take()); // 先令接收端最终遇到None，再等待线程。
        self.thread.take().unwrap().join()
    }
}
// ANCHOR_END: service

// ANCHOR: bounded-blocking
pub async fn bounded_blocking<F, T>(limit: Arc<Semaphore>, work: F) -> Result<T, tokio::task::JoinError>
where F: FnOnce() -> T + Send + 'static, T: Send + 'static {
    let permit = limit.acquire_owned().await.expect("工作入口未关闭");
    tokio::task::spawn_blocking(move || {
        let _permit = permit; // 随真实阻塞工作释放，不随外层等待Future释放。
        work()
    }).await
}
// ANCHOR_END: bounded-blocking

// ANCHOR: rayon
pub async fn parallel_sum(pool: &rayon::ThreadPool, input: Vec<u64>) -> Result<u64, &'static str> {
    use rayon::prelude::*;
    let (sender, result) = oneshot::channel();
    pool.spawn(move || {
        let value = std::panic::catch_unwind(|| input.par_iter().copied().sum::<u64>());
        let _ = sender.send(value.map_err(|_| "计算panic"));
    });
    result.await.map_err(|_| "计算响应中断")?
}
// ANCHOR_END: rayon

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bridging_dedicated_runtime_serves_sync_and_async_and_closes() {
        let service = Service::start();
        assert_eq!(service.request_blocking(21), Ok(42));
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        assert_eq!(runtime.block_on(service.request_async(9)), Ok(18));
        service.shutdown().unwrap();
    }
    #[tokio::test]
    async fn bridging_cancelled_wait_keeps_blocking_permit_until_work_finishes() {
        let limit = Arc::new(Semaphore::new(1));
        let (started, ready) = oneshot::channel();
        let (release, gate) = std::sync::mpsc::channel();
        let l = limit.clone();
        let task = tokio::spawn(async move {
            bounded_blocking(l, move || { started.send(()).unwrap(); gate.recv().unwrap(); }).await.unwrap();
        });
        ready.await.unwrap();
        task.abort(); assert!(task.await.unwrap_err().is_cancelled());
        assert_eq!(limit.available_permits(), 0);
        release.send(()).unwrap();
        let permit = limit.acquire().await.unwrap();
        drop(permit);
        assert_eq!(limit.available_permits(), 1);
    }
    #[tokio::test]
    async fn bridging_rayon_reply_without_blocking_executor() {
        let pool = rayon::ThreadPoolBuilder::new().num_threads(2).build().unwrap();
        assert_eq!(parallel_sum(&pool, (1..=100).collect()).await, Ok(5050));
    }
    #[tokio::test]
    async fn bridging_full_channel_backpressure_and_receiver_close_unblocks_thread() {
        let (tx, mut rx) = mpsc::channel(1);
        tx.send(1).await.unwrap();
        assert!(matches!(tx.try_send(2), Err(mpsc::error::TrySendError::Full(2))));
        let (ready, started) = oneshot::channel();
        let thread = std::thread::spawn(move || { ready.send(()).unwrap(); tx.blocking_send(2) });
        started.await.unwrap();
        rx.close();
        let error = tokio::task::spawn_blocking(move || thread.join().unwrap()).await.unwrap().unwrap_err();
        assert_eq!(error.0, 2);
        assert_eq!(rx.recv().await, Some(1));
        assert_eq!(rx.recv().await, None);
    }
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn bridging_block_in_place_reenters_with_handle() {
        let value = tokio::task::block_in_place(|| tokio::runtime::Handle::current().block_on(async {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await; 5
        }));
        assert_eq!(value, 5);
    }
}
