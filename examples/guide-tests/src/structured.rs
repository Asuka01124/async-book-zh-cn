use std::{future::Future, pin::Pin};
use tokio::{sync::{oneshot, watch}, task::{JoinError, JoinSet}};

// ANCHOR: structured-supervisor
pub type Job = Pin<Box<dyn Future<Output = Result<usize, &'static str>> + Send>>;

#[derive(Debug)]
pub enum Failure {
    Work(&'static str),
    Task(JoinError),
}

// 兜底只能同步发信号；不能在 Drop 中保证异步清理已经结束。
struct StopOnDrop(watch::Sender<bool>);
impl Drop for StopOnDrop {
    fn drop(&mut self) { self.0.send_replace(true); }
}

pub async fn supervise(
    jobs: Vec<Job>,
    stop: watch::Sender<bool>,
    mut shutdown: oneshot::Receiver<()>,
) -> Result<Vec<usize>, Failure> {
    let guard = StopOnDrop(stop);
    let mut children = JoinSet::new();
    for job in jobs { children.spawn(job); }
    let mut closing = false;
    let mut first_error = None;
    let mut values = Vec::new();
    while !children.is_empty() {
        tokio::select! {
            biased;
            // 发送者被丢弃也表示关闭；进入关闭阶段后不再轮询它。
            _ = &mut shutdown, if !closing => {
                closing = true;
                guard.0.send_replace(true);
            }
            outcome = children.join_next() => {
                match outcome.expect("非空任务集") {
                    Ok(Ok(value)) => values.push(value),
                    Ok(Err(error)) => {
                        if first_error.is_none() { first_error = Some(Failure::Work(error)); }
                    }
                    Err(error) => {
                        if first_error.is_none() { first_error = Some(Failure::Task(error)); }
                    }
                }
                if first_error.is_some() && !closing {
                    closing = true;
                    guard.0.send_replace(true);
                }
                // 发生错误也继续 join，直到每个子任务退出。
            }
        }
    }
    match first_error {
        Some(error) => Err(error),
        None => Ok(values),
    }
}
// ANCHOR_END: structured-supervisor

// ANCHOR: structured-worker
pub async fn stop_requested(stop: &mut watch::Receiver<bool>) {
    loop {
        if *stop.borrow_and_update() { return; }
        if stop.changed().await.is_err() { return; }
    }
}
// ANCHOR_END: structured-worker

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
    struct Release(Arc<AtomicUsize>);
    impl Drop for Release {
        fn drop(&mut self) { self.0.fetch_add(1, Ordering::SeqCst); }
    }

    // 关闭后先确认收到信号，再等待显式许可，模拟真正需要 await 的清理。
    fn worker(
        mut stop: watch::Receiver<bool>,
        seen: oneshot::Sender<()>,
        cleanup: oneshot::Receiver<()>,
        released: Arc<AtomicUsize>,
    ) -> Job {
        Box::pin(async move {
            let _resource = Release(released);
            stop_requested(&mut stop).await;
            let _ = seen.send(());
            cleanup.await.map_err(|_| "清理许可关闭")?;
            Ok(1)
        })
    }

    #[tokio::test]
    async fn structured_shutdown_waits_for_async_cleanup_and_child_exit() {
        let released = Arc::new(AtomicUsize::new(0));
        let (stop, rx) = watch::channel(false);
        let (request, shutdown) = oneshot::channel();
        let (seen, acknowledged) = oneshot::channel();
        let (cleanup, permit) = oneshot::channel();
        let mut parent = tokio::spawn(supervise(
            vec![worker(rx, seen, permit, released.clone())], stop, shutdown));
        request.send(()).unwrap();
        acknowledged.await.unwrap();
        assert!(futures::poll!(&mut parent).is_pending());
        assert_eq!(released.load(Ordering::SeqCst), 0);
        cleanup.send(()).unwrap();
        assert_eq!(parent.await.unwrap().unwrap(), [1]);
        assert_eq!(released.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn structured_business_error_and_panic_stop_sibling_before_return() {
        for panic in [false, true] {
            let released = Arc::new(AtomicUsize::new(0));
            let (stop, rx) = watch::channel(false);
            let (_request, shutdown) = oneshot::channel();
            let (seen, acknowledged) = oneshot::channel();
            let (cleanup, permit) = oneshot::channel();
            let fail: Job = Box::pin(async move {
                if panic { panic!("教学测试的子任务 panic"); }
                Err("工作失败")
            });
            let mut parent = tokio::spawn(supervise(
                vec![worker(rx, seen, permit, released.clone()), fail], stop, shutdown));
            acknowledged.await.unwrap();
            assert!(futures::poll!(&mut parent).is_pending());
            assert_eq!(released.load(Ordering::SeqCst), 0);
            cleanup.send(()).unwrap();
            match parent.await.unwrap().unwrap_err() {
                Failure::Work(message) => { assert!(!panic); assert_eq!(message, "工作失败"); }
                Failure::Task(error) => { assert!(panic); assert!(error.is_panic()); }
            }
            assert_eq!(released.load(Ordering::SeqCst), 1);
        }
    }
}
