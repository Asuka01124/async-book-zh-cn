// ANCHOR: manual-future
use std::{future::Future, pin::Pin, task::{Context, Poll}};

pub struct YieldOnce {
    yielded: bool,
}
impl YieldOnce {
    pub fn new() -> Self { Self { yielded: false } }
}
impl Future for YieldOnce {
    type Output = &'static str;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.yielded {
            Poll::Ready("完成")
        } else {
            self.yielded = true;
            // 此例下一次轮询一定能前进，所以主动安排一次唤醒。
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}
// ANCHOR_END: manual-future

// ANCHOR: builder
pub struct AddBuilder {
    value: u32,
    extra: u32,
}
impl AddBuilder {
    pub fn new(value: u32) -> Self { Self { value, extra: 0 } }
    pub fn extra(mut self, extra: u32) -> Self { self.extra = extra; self }
}
impl std::future::IntoFuture for AddBuilder {
    type Output = Option<u32>;
    type IntoFuture = std::future::Ready<Option<u32>>;
    fn into_future(self) -> Self::IntoFuture {
        std::future::ready(self.value.checked_add(self.extra))
    }
}
pub async fn await_builder() -> Option<u32> {
    AddBuilder::new(20).extra(22).await
}
// ANCHOR_END: builder

// ANCHOR: boxed
pub fn borrowed_future<'a>(text: &'a str, uppercase: bool)
    -> futures::future::BoxFuture<'a, String>
{
    use futures::FutureExt;
    if uppercase {
        async move { text.to_uppercase() }.boxed()
    } else {
        async move { text.to_owned() }.boxed()
    }
}
// ANCHOR_END: boxed

// ANCHOR: combinators
pub async fn parsed_double(text: &str) -> Result<u32, std::num::ParseIntError> {
    use futures::TryFutureExt;
    futures::future::ready(text.parse::<u32>())
        .map_ok(|value| value.saturating_mul(2))
        .await
}
// ANCHOR_END: combinators

// ANCHOR: concurrent-library
pub async fn borrowed_join(text: &str) -> (usize, String) {
    use futures_concurrency::prelude::*;
    (async { text.len() }, async { text.to_uppercase() }).join().await
}
// ANCHOR_END: concurrent-library

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{atomic::{AtomicUsize, Ordering}, Arc};

    struct WakeCounter(AtomicUsize);
    impl futures::task::ArcWake for WakeCounter {
        fn wake_by_ref(arc: &Arc<Self>) { arc.0.fetch_add(1, Ordering::SeqCst); }
    }

    #[test]
    fn pending_arranges_wakeup_then_completes() {
        let counter = Arc::new(WakeCounter(AtomicUsize::new(0)));
        let waker = futures::task::waker_ref(&counter);
        let mut cx = Context::from_waker(&waker);
        let mut future = std::pin::pin!(YieldOnce::new());
        assert!(future.as_mut().poll(&mut cx).is_pending());
        assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        assert_eq!(future.as_mut().poll(&mut cx), Poll::Ready("完成"));
    }

    #[tokio::test]
    async fn builders_boxes_and_combinators_preserve_contracts() {
        assert_eq!(await_builder().await, Some(42));
        assert_eq!(AddBuilder::new(u32::MAX).extra(1).await, None);
        let text = String::from("rust");
        assert_eq!(borrowed_future(&text, true).await, "RUST");
        assert_eq!(borrowed_future(&text, false).await, "rust");
        assert_eq!(parsed_double("21").await.unwrap(), 42);
        assert!(parsed_double("invalid").await.is_err());
        assert_eq!(borrowed_join(&text).await, (4, String::from("RUST")));
    }

    struct PendingDrop(Arc<AtomicUsize>);
    impl Future for PendingDrop {
        type Output = u32;
        fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<u32> { Poll::Pending }
    }
    impl Drop for PendingDrop {
        fn drop(&mut self) { self.0.fetch_add(1, Ordering::SeqCst); }
    }

    #[tokio::test]
    async fn race_cancels_owned_loser() {
        use futures_concurrency::prelude::*;
        let drops = Arc::new(AtomicUsize::new(0));
        let loser = PendingDrop(drops.clone());
        assert_eq!((loser, std::future::ready(9)).race().await, 9);
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn generic_executor_needs_no_tokio_for_pure_futures() {
        assert_eq!(futures::executor::block_on(YieldOnce::new()), "完成");
    }
}
