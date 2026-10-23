// ANCHOR: periodic
use tokio::time::{self, Duration, Instant, MissedTickBehavior};

pub async fn scheduled_ticks(count: usize) -> Vec<Instant> {
    let mut interval = time::interval(Duration::from_secs(10));
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
    let mut ticks = Vec::new();
    for _ in 0..count {
        ticks.push(interval.tick().await);
    }
    ticks
}
// ANCHOR_END: periodic

// ANCHOR: task-timeout
pub async fn task_with_limit(
    limit: Duration,
    mut task: tokio::task::JoinHandle<u32>,
) -> Result<u32, &'static str> {
    match time::timeout(limit, &mut task).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(_join_error)) => Err("任务失败"),
        Err(_elapsed) => {
            task.abort();
            let _ = task.await; // 确认任务及其同步析构已经结束。
            Err("超时")
        }
    }
}
// ANCHOR_END: task-timeout

// ANCHOR: unix-signals
#[cfg(unix)]
pub async fn unix_shutdown_signal() -> std::io::Result<()> {
    use tokio::signal::unix::{signal, SignalKind};
    let mut terminate = signal(SignalKind::terminate())?;
    tokio::select! {
        result = tokio::signal::ctrl_c() => result,
        _ = terminate.recv() => Ok(()),
    }
}
// ANCHOR_END: unix-signals

// ANCHOR: windows-signals
#[cfg(windows)]
pub async fn windows_console_signal() -> std::io::Result<()> {
    let mut ctrl_break = tokio::signal::windows::ctrl_break()?;
    tokio::select! {
        result = tokio::signal::ctrl_c() => result,
        _ = ctrl_break.recv() => Ok(()),
    }
}
// ANCHOR_END: windows-signals

// ANCHOR: signal-routing
pub async fn notify_shutdown(
    trigger: impl std::future::Future<Output = std::io::Result<()>>,
    stop: &tokio::sync::watch::Sender<bool>,
) -> std::io::Result<()> {
    trigger.await?;
    stop.send(true).map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::BrokenPipe, "关闭接收者已退出")
    })
}
// ANCHOR_END: signal-routing

#[cfg(test)]
mod tests {
    use super::*;
    use futures::poll;
    use std::sync::{atomic::{AtomicBool, Ordering}, Arc};

    #[tokio::test(start_paused = true)]
    async fn sleep_can_be_reset_before_and_after_completion() {
        let start = Instant::now();
        let timer = time::sleep(Duration::from_secs(1));
        tokio::pin!(timer);
        timer.as_mut().reset(start + Duration::from_secs(3));
        time::advance(Duration::from_secs(2)).await;
        assert!(poll!(&mut timer).is_pending());
        (&mut timer).await;
        assert_eq!(start.elapsed(), Duration::from_secs(3));
        timer.as_mut().reset(Instant::now() + Duration::from_secs(2));
        timer.await;
        assert_eq!(start.elapsed(), Duration::from_secs(5));
    }

    #[tokio::test(start_paused = true)]
    async fn interval_first_tick_and_missed_tick_policies() {
        let ticks = scheduled_ticks(3).await;
        assert_eq!(ticks[1] - ticks[0], Duration::from_secs(10));
        assert_eq!(ticks[2] - ticks[0], Duration::from_secs(20));
        for (behavior, next_seconds) in [
            (MissedTickBehavior::Burst, 20),
            (MissedTickBehavior::Delay, 45),
            (MissedTickBehavior::Skip, 40),
        ] {
            let start = Instant::now();
            let mut timer = time::interval(Duration::from_secs(10));
            timer.set_missed_tick_behavior(behavior);
            assert_eq!(timer.tick().await, start);
            time::advance(Duration::from_secs(35)).await;
            assert_eq!(timer.tick().await - start, Duration::from_secs(10));
            assert_eq!(timer.tick().await - start, Duration::from_secs(next_seconds));
        }
    }

    #[tokio::test(start_paused = true)]
    async fn zero_timeout_still_accepts_immediately_ready_work() {
        assert_eq!(time::timeout(Duration::ZERO, async { 7 }).await.unwrap(), 7);
        assert!(time::timeout(Duration::ZERO, futures::future::pending::<()>()).await.is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn timeout_of_borrowed_handle_does_not_cancel_task() {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut task = tokio::spawn(async move { rx.await.unwrap() });
        assert!(time::timeout(Duration::from_secs(1), &mut task).await.is_err());
        tx.send(42).unwrap();
        assert_eq!(task.await.unwrap(), 42);
    }

    struct Flag(Arc<AtomicBool>);
    impl Drop for Flag {
        fn drop(&mut self) { self.0.store(true, Ordering::SeqCst); }
    }

    #[tokio::test(start_paused = true)]
    async fn deadline_helper_waits_for_task_destruction() {
        let dropped = Arc::new(AtomicBool::new(false));
        let guard = Flag(dropped.clone());
        let task = tokio::spawn(async move {
            let _guard = guard;
            futures::future::pending::<u32>().await
        });
        assert_eq!(task_with_limit(Duration::from_secs(1), task).await, Err("超时"));
        assert!(dropped.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn injected_trigger_routes_shutdown_and_reports_failure() {
        let (tx, mut rx) = tokio::sync::watch::channel(false);
        let error = std::io::Error::other("信号监听失败");
        assert!(notify_shutdown(async { Err(error) }, &tx).await.is_err());
        assert!(!*rx.borrow_and_update());
        notify_shutdown(async { Ok(()) }, &tx).await.unwrap();
        rx.changed().await.unwrap();
        assert!(*rx.borrow_and_update());
    }
}
