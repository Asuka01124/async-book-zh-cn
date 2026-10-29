//! 旧章节 FuturesUnordered 的可运行示例。
use futures::{stream, stream::FuturesUnordered, StreamExt};
use std::time::Duration;

// ANCHOR: basic
async fn job(id: u32, delay: Duration) -> u32 {
    tokio::time::sleep(delay).await;
    id
}

pub async fn collect_jobs() -> Vec<u32> {
    let mut pending = FuturesUnordered::new();
    pending.push(job(1, Duration::from_secs(2)));
    pending.push(job(2, Duration::from_secs(1)));
    let mut observed = Vec::new();
    while let Some(id) = pending.next().await {
        observed.push(id);
    }
    observed
}
// ANCHOR_END: basic

// ANCHOR: limited
pub async fn limited_jobs() -> Vec<u32> {
    stream::iter(0..6)
        .map(|id| job(id, Duration::from_secs(1)))
        .buffer_unordered(2)
        .collect()
        .await
}
// ANCHOR_END: limited

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{channel::oneshot, future::FutureExt, TryStreamExt};
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};

    #[test]
    fn push_is_lazy_pending_is_real_and_empty_set_can_restart() {
        let polls = Arc::new(AtomicUsize::new(0));
        let (first_tx, first_rx) = oneshot::channel::<u32>();
        let (second_tx, second_rx) = oneshot::channel::<u32>();
        let mut set = FuturesUnordered::new();
        for rx in [first_rx, second_rx] {
            let polls = polls.clone();
            set.push(async move {
                polls.fetch_add(1, Ordering::SeqCst);
                rx.await.unwrap()
            });
        }
        assert_eq!(polls.load(Ordering::SeqCst), 0);
        assert!(set.next().now_or_never().is_none());
        assert_eq!(polls.load(Ordering::SeqCst), 2);
        second_tx.send(2).unwrap();
        assert_eq!(futures::executor::block_on(set.next()), Some(2));
        first_tx.send(1).unwrap();
        assert_eq!(futures::executor::block_on(set.next()), Some(1));
        assert_eq!(futures::executor::block_on(set.next()), None);
        // 使用同一 async 表达式类型才能继续加入这个集合。
        let mut reusable = FuturesUnordered::new();
        reusable.push(futures::future::ready(3));
        assert_eq!(futures::executor::block_on(reusable.next()), Some(3));
        assert_eq!(futures::executor::block_on(reusable.next()), None);
        reusable.push(futures::future::ready(4));
        assert_eq!(futures::executor::block_on(reusable.next()), Some(4));
        assert_eq!(futures::executor::block_on(reusable.next()), None);

        let mut fused = reusable.fuse();
        assert_eq!(futures::executor::block_on(fused.next()), None);
        fused.get_ref().push(futures::future::ready(5));
        assert_eq!(futures::executor::block_on(fused.next()), None);
        assert_eq!(fused.get_ref().len(), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn timers_observe_shorter_job_first() {
        assert_eq!(collect_jobs().await, vec![2, 1]);
        let mut results = limited_jobs().await;
        results.sort_unstable();
        assert_eq!(results, vec![0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn buffer_limit_admits_more_only_after_completion() {
        let started = Arc::new(AtomicUsize::new(0));
        let channels: Vec<_> = (0..3).map(|_| oneshot::channel::<u32>()).collect();
        let (senders, receivers): (Vec<_>, Vec<_>) = channels.into_iter().unzip();
        let mut senders = senders.into_iter();
        let count = started.clone();
        let mut buffered = stream::iter(receivers).map(move |rx| {
            let count = count.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                rx.await.unwrap()
            }
        }).buffer_unordered(2);
        assert!(buffered.next().now_or_never().is_none());
        assert_eq!(started.load(Ordering::SeqCst), 2);
        senders.next().unwrap().send(1).unwrap();
        assert_eq!(futures::executor::block_on(buffered.next()), Some(1));
        assert!(buffered.next().now_or_never().is_none());
        assert_eq!(started.load(Ordering::SeqCst), 3);
        drop(senders);
    }

    #[test]
    fn try_collect_drops_owned_pending_work_on_error() {
        struct CountDrop(Arc<AtomicUsize>);
        impl Drop for CountDrop {
            fn drop(&mut self) { self.0.fetch_add(1, Ordering::SeqCst); }
        }
        let dropped = Arc::new(AtomicUsize::new(0));
        let guard = CountDrop(dropped.clone());
        let mut set = FuturesUnordered::new();
        set.push(async move {
            let _guard = guard;
            std::future::pending::<Result<u32, &'static str>>().await
        }.boxed());
        // 先明确推进至 Pending，不依赖内部首次轮询的次序。
        assert!(set.next().now_or_never().is_none());
        set.push(async { Err("失败") }.boxed());
        let result = futures::executor::block_on(set.try_collect::<Vec<_>>());
        assert_eq!(result, Err("失败"));
        assert_eq!(dropped.load(Ordering::SeqCst), 1);
    }
}
