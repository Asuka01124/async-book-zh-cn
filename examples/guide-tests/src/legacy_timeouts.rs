//! 旧章节“取消与超时”的独立可运行示例。
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;

// ANCHOR: borrowed
pub async fn wait_briefly_then_resume() -> (bool, u32) {
    let work = async {
        tokio::time::sleep(Duration::from_secs(2)).await;
        42
    };
    tokio::pin!(work);
    match timeout(Duration::from_secs(1), &mut work).await {
        Ok(value) => (false, value),
        // 超时只销毁了可变借用；原 Future 仍保存剩余进度。
        Err(_) => (true, work.await),
    }
}
// ANCHOR_END: borrowed

// ANCHOR: shutdown
pub async fn worker(
    mut jobs: mpsc::Receiver<u32>,
    mut stop: oneshot::Receiver<()>,
) -> Vec<u32> {
    let mut completed = Vec::new();
    loop {
        tokio::select! {
            // 优先观察已就绪的停止请求；发送端释放也表示关闭。
            biased;
            _ = &mut stop => break,
            job = jobs.recv() => match job {
                Some(job) => completed.push(job),
                None => break,
            },
        }
    }
    jobs.close(); // 拒绝后续发送，保留已排队的工作。
    while let Some(job) = jobs.recv().await {
        completed.push(job);
    }
    completed // 真实程序可在这里 await flush/持久化，再退出。
}
// ANCHOR_END: shutdown

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

    struct MarkDrop(Arc<AtomicBool>);
    impl Drop for MarkDrop {
        fn drop(&mut self) { self.0.store(true, Ordering::SeqCst); }
    }

    #[tokio::test(start_paused = true)]
    async fn owned_timeout_drops_work_but_keeps_effects() {
        let dropped = Arc::new(AtomicBool::new(false));
        let effect = Arc::new(AtomicBool::new(false));
        let guard = MarkDrop(dropped.clone());
        let effect_in_work = effect.clone();
        let work = async move {
            let _guard = guard;
            effect_in_work.store(true, Ordering::SeqCst);
            std::future::pending::<()>().await;
        };
        assert!(timeout(Duration::from_secs(1), work).await.is_err());
        assert!(dropped.load(Ordering::SeqCst));
        assert!(effect.load(Ordering::SeqCst));
    }

    #[tokio::test(start_paused = true)]
    async fn borrowed_timeout_preserves_work() {
        assert_eq!(wait_briefly_then_resume().await, (true, 42));
    }

    #[tokio::test(start_paused = true)]
    async fn timed_out_handle_detaches_but_retained_handle_can_abort() {
        let (release, wait) = oneshot::channel::<()>();
        let (done, finished) = oneshot::channel();
        let handle = tokio::spawn(async move {
            wait.await.unwrap();
            done.send(7).unwrap();
        });
        assert!(timeout(Duration::from_secs(1), handle).await.is_err());
        release.send(()).unwrap();
        assert_eq!(finished.await.unwrap(), 7);

        let dropped = Arc::new(AtomicBool::new(false));
        let guard = MarkDrop(dropped.clone());
        let (started, ready) = oneshot::channel();
        let mut handle = tokio::spawn(async move {
            let _guard = guard;
            started.send(()).unwrap();
            std::future::pending::<()>().await;
        });
        ready.await.unwrap();
        assert!(timeout(Duration::from_secs(1), &mut handle).await.is_err());
        handle.abort();
        assert!(handle.await.unwrap_err().is_cancelled());
        assert!(dropped.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn cooperative_shutdown_drains_buffer_and_joins() {
        let (tx, rx) = mpsc::channel(4);
        tx.send(1).await.unwrap();
        tx.send(2).await.unwrap();
        let (stop, request) = oneshot::channel();
        stop.send(()).unwrap();
        let handle = tokio::spawn(worker(rx, request));
        assert_eq!(handle.await.unwrap(), vec![1, 2]);
        assert!(tx.send(3).await.is_err());
    }
}
