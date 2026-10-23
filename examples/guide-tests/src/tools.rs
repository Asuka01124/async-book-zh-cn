// ANCHOR: instrument
use tracing::{info, info_span, Instrument};

pub async fn traced_request(request_id: u64) -> u64 {
    async move {
        info!(phase = "start", "开始请求");
        tokio::task::yield_now().await;
        info!(phase = "complete", "完成请求");
        request_id + 1
    }
    .instrument(info_span!("request", request_id))
    .await
}
// ANCHOR_END: instrument

// ANCHOR: metrics
pub fn runtime_snapshot() -> (usize, usize) {
    let metrics = tokio::runtime::Handle::current().metrics();
    // 是观测快照，不能作为所有任务已退出的同步协议。
    (metrics.num_workers(), metrics.num_alive_tasks())
}
// ANCHOR_END: metrics

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;

    #[test]
    fn instrument_exits_span_while_future_is_pending() {
        tracing::subscriber::with_default(tracing_subscriber::registry(), || {
            let work = async {
                assert_eq!(tracing::Span::current().metadata().unwrap().name(), "request");
                tokio::task::yield_now().await;
                assert_eq!(tracing::Span::current().metadata().unwrap().name(), "request");
                42
            }
            .instrument(info_span!("request"));
            futures::pin_mut!(work);
            let mut cx = std::task::Context::from_waker(futures::task::noop_waker_ref());
            assert!(work.as_mut().poll(&mut cx).is_pending());
            assert!(tracing::Span::current().is_none());
            assert_eq!(work.as_mut().poll(&mut cx), std::task::Poll::Ready(42));
            assert!(tracing::Span::current().is_none());
        });
    }

    // ANCHOR: loom
    #[test]
    fn loom_atomic_increment_has_no_lost_update() {
        loom::model(|| {
            use loom::sync::atomic::{AtomicUsize, Ordering};
            use loom::sync::Arc;
            let value = Arc::new(AtomicUsize::new(0));
            let other = value.clone();
            let worker = loom::thread::spawn(move || {
                other.fetch_add(1, Ordering::Relaxed);
            });
            value.fetch_add(1, Ordering::Relaxed);
            worker.join().unwrap();
            assert_eq!(value.load(Ordering::Relaxed), 2);
        });
    }
    // ANCHOR_END: loom

    #[test]
    #[should_panic(expected = "assertion `left == right` failed")]
    fn loom_detects_split_load_store_lost_update() {
        loom::model(|| {
            use loom::sync::atomic::{AtomicUsize, Ordering};
            use loom::sync::Arc;
            let value = Arc::new(AtomicUsize::new(0));
            let other = value.clone();
            let worker = loom::thread::spawn(move || {
                let n = other.load(Ordering::SeqCst);
                other.store(n + 1, Ordering::SeqCst);
            });
            let n = value.load(Ordering::SeqCst);
            value.store(n + 1, Ordering::SeqCst);
            worker.join().unwrap();
            assert_eq!(value.load(Ordering::SeqCst), 2);
        });
    }

    #[tokio::test]
    async fn observed_worker_count_matches_current_thread_runtime() {
        assert_eq!(runtime_snapshot().0, 1);
        assert_eq!(traced_request(41).await, 42);
    }
}
