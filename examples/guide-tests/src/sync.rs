use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Semaphore};

// ANCHOR: actor
pub async fn actor_total() -> u32 {
    let (tx, mut rx) = mpsc::channel::<(u32, oneshot::Sender<u32>)>(2);
    let worker = tokio::spawn(async move {
        let mut total = 0;
        while let Some((delta, reply)) = rx.recv().await {
            total += delta;
            // 答复被取消不回滚已经接受的更新。
            let _ = reply.send(total);
        }
        total
    });
    for delta in [2, 3] {
        let (reply, answer) = oneshot::channel();
        tx.send((delta, reply)).await.unwrap();
        let _current = answer.await.unwrap();
    }
    drop(tx);
    worker.await.unwrap()
}
// ANCHOR_END: actor

// ANCHOR: short-lock
#[derive(Default)]
pub struct Counter(std::sync::Mutex<u32>);
impl Counter {
    pub fn increment(&self) -> u32 {
        let mut value = self.0.lock().unwrap();
        *value += 1;
        *value
    } // 返回前释放守卫。
}
pub async fn count_and_send(counter: &Counter, tx: &mpsc::Sender<u32>) {
    let value = counter.increment();
    let _ = tx.send(value).await;
}
// ANCHOR_END: short-lock

// ANCHOR: limited
pub async fn limited_jobs() -> Vec<u32> {
    let slots = Arc::new(Semaphore::new(2));
    let mut jobs = tokio::task::JoinSet::new();
    for value in 0..5 {
        let permit = slots.clone().acquire_owned().await.unwrap();
        jobs.spawn(async move {
            let _permit = permit;
            tokio::task::yield_now().await;
            value * 2
        });
    }
    let mut values = Vec::new();
    while let Some(result) = jobs.join_next().await {
        values.push(result.unwrap());
    }
    values.sort_unstable();
    values
}
// ANCHOR_END: limited

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{poll, FutureExt};
    use tokio::sync::{broadcast, watch, Mutex, Notify, OnceCell};

    #[tokio::test]
    async fn actor_drains_and_terminates() {
        assert_eq!(actor_total().await, 5);
        let counter = Counter::default();
        let (tx, mut rx) = mpsc::channel(1);
        count_and_send(&counter, &tx).await;
        assert_eq!(rx.recv().await, Some(1));
        assert_eq!(limited_jobs().await, [0, 2, 4, 6, 8]);
    }

    #[tokio::test]
    async fn bounded_reservation_retains_message_on_cancellation() {
        let message = String::from("second");
        let (tx, mut rx) = mpsc::channel(1);
        tx.send("first").await.unwrap();
        {
            let reservation = tx.reserve();
            tokio::pin!(reservation);
            assert!(poll!(&mut reservation).is_pending());
        }
        assert_eq!(message, "second");
        assert_eq!(rx.recv().await, Some("first"));
        tx.reserve().await.unwrap().send(&message);
        rx.close();
        assert_eq!(rx.recv().await, Some("second"));
        assert_eq!(rx.recv().await, None);
        assert!(tx.send("third").await.is_err());
    }

    #[tokio::test]
    async fn broadcast_lag_differs_from_watch_coalescing() {
        let (tx, mut rx) = broadcast::channel(1);
        tx.send(1).unwrap();
        tx.send(2).unwrap();
        assert!(matches!(rx.recv().await, Err(broadcast::error::RecvError::Lagged(1))));
        assert_eq!(rx.recv().await.unwrap(), 2);
        let (tx, mut rx) = watch::channel(0);
        tx.send(1).unwrap();
        tx.send(2).unwrap();
        rx.changed().await.unwrap();
        assert_eq!(*rx.borrow_and_update(), 2);
        assert!(!rx.has_changed().unwrap());
    }

    #[tokio::test]
    async fn guard_stays_locked_and_abort_returns_permit() {
        let lock = Mutex::new(0);
        let mut guard = lock.lock().await;
        tokio::task::yield_now().await;
        assert!(lock.try_lock().is_err());
        *guard = 3;
        drop(guard);
        assert_eq!(*lock.lock().await, 3);
        let semaphore = Arc::new(Semaphore::new(1));
        let permit = semaphore.clone().acquire_owned().await.unwrap();
        let task = tokio::spawn(async move {
            let _permit = permit;
            futures::future::pending::<()>().await;
        });
        assert!(semaphore.try_acquire().is_err());
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(semaphore.try_acquire().is_ok());
    }

    #[tokio::test]
    async fn notification_is_not_an_event_counter() {
        let notify = Notify::new();
        notify.notify_one();
        notify.notify_one();
        assert!(notify.notified().now_or_never().is_some());
        assert!(notify.notified().now_or_never().is_none());
        notify.notify_waiters();
        assert!(notify.notified().now_or_never().is_none());
    }

    #[tokio::test]
    async fn cancelled_initialization_can_retry() {
        let cell = OnceCell::new();
        {
            let first = cell.get_or_init(|| futures::future::pending::<u32>());
            tokio::pin!(first);
            assert!(poll!(&mut first).is_pending());
        }
        assert!(cell.get().is_none());
        let (a, b) = tokio::join!(cell.get_or_init(|| async { 7 }), cell.get_or_init(|| async { 8 }));
        assert_eq!(*a, 7);
        assert!(std::ptr::eq(a, b));
    }
}
