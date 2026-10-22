// ANCHOR: join-example
pub async fn joined_results() -> (u32, u32) {
    tokio::join!(
        async {
            tokio::task::yield_now().await;
            1
        },
        async { 2 }
    )
}
// ANCHOR_END: join-example

// ANCHOR: select-example
pub async fn first_or_timeout(
    work_delay: std::time::Duration,
    timeout: std::time::Duration,
) -> Option<u32> {
    tokio::select! {
        result = async {
            tokio::time::sleep(work_delay).await;
            42
        } => Some(result),
        _ = tokio::time::sleep(timeout) => None,
    }
}
// ANCHOR_END: select-example

// ANCHOR: idle-timeout
pub async fn collect_with_idle_timeout<S>(
    stream: &mut S,
    timeout: std::time::Duration,
) -> Vec<u32>
where
    S: futures::Stream<Item = u32> + Unpin,
{
    use futures::StreamExt;
    let mut values = Vec::new();
    loop {
        tokio::select! {
            value = stream.next() => match value {
                Some(value) => values.push(value),
                None => break,
            },
            _ = tokio::time::sleep(timeout) => break,
        }
    }
    values
}
// ANCHOR_END: idle-timeout

// ANCHOR: total-timeout
pub async fn collect_with_deadline<S>(
    stream: &mut S,
    duration: std::time::Duration,
) -> Vec<u32>
where
    S: futures::Stream<Item = u32> + Unpin,
{
    use futures::StreamExt;
    let timeout = tokio::time::sleep(duration);
    tokio::pin!(timeout);
    let mut values = Vec::new();
    loop {
        tokio::select! {
            biased; // 同一次轮询中都已就绪时，优先处理总超时。
            _ = &mut timeout => break,
            value = stream.next() => match value {
                Some(value) => values.push(value),
                None => break,
            },
        }
    }
    values
}
// ANCHOR_END: total-timeout

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{FutureExt, StreamExt};
    use std::cell::{Cell, RefCell};
    use std::future::{ready, IntoFuture, Ready};
    use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
    use std::time::Duration;

    struct IntoOnly(u32);
    impl IntoFuture for IntoOnly {
        type Output = u32;
        type IntoFuture = Ready<u32>;
        fn into_future(self) -> Self::IntoFuture {
            ready(self.0)
        }
    }

    #[tokio::test]
    async fn current_tokio_macros_accept_into_future() {
        assert_eq!(IntoOnly(1).await, 1);
        assert_eq!(tokio::join!(IntoOnly(2), IntoOnly(3)), (2, 3));
        let value = tokio::select! { value = IntoOnly(4) => value };
        assert_eq!(value, 4);
    }

    #[tokio::test]
    async fn join_keeps_errors_as_values_and_waits_for_other_branch() {
        let completed = Cell::new(false);
        let (error, value) = tokio::join!(
            async { Err::<(), _>("failure") },
            async {
                tokio::task::yield_now().await;
                completed.set(true);
                42
            }
        );
        assert_eq!(error, Err("failure"));
        assert_eq!(value, 42);
        assert!(completed.get());
        assert_eq!(joined_results().await, (1, 2));
    }

    struct DropFlag(Arc<AtomicBool>);
    impl Drop for DropFlag {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn try_join_drops_owned_losing_future_on_error() {
        let dropped = Arc::new(AtomicBool::new(false));
        let flag = DropFlag(dropped.clone());
        let pending = async move {
            let _flag = flag;
            std::future::pending::<Result<(), &str>>().await
        };
        let result = tokio::try_join!(async { Err::<(), _>("failure") }, pending);
        assert_eq!(result, Err("failure"));
        assert!(dropped.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn task_result_errors_are_nested_inside_join_results() {
        let first = tokio::spawn(async { Err::<u32, _>("inner failure") });
        let second = tokio::spawn(async { Ok::<u32, &str>(42) });
        let (first, second) = tokio::try_join!(first, second).unwrap();
        assert_eq!(first, Err("inner failure"));
        assert_eq!(second, Ok(42));
    }

    #[tokio::test]
    async fn guards_run_before_construction_and_disabled_future_is_not_polled() {
        let events = RefCell::new(Vec::new());
        let value = tokio::select! {
            value = {
                events.borrow_mut().push("construct");
                async {
                    events.borrow_mut().push("poll");
                    1
                }
            }, if {
                events.borrow_mut().push("guard");
                false
            } => value,
            value = async { 2 } => value,
        };
        assert_eq!(value, 2);
        assert_eq!(*events.borrow(), ["guard", "construct"]);
    }

    #[tokio::test]
    async fn select_pattern_mismatch_can_fall_back_to_else() {
        let value = tokio::select! {
            Some(value) = async { None::<u32> } => value,
            else => 42,
        };
        assert_eq!(value, 42);
        let value = tokio::select! {
            result = async { Err::<u32, &str>("failure") } => result.unwrap_or(7),
        };
        assert_eq!(value, 7);
    }

    #[tokio::test(start_paused = true)]
    async fn select_returns_handler_value_and_timeout_result() {
        assert_eq!(first_or_timeout(Duration::from_secs(1), Duration::from_secs(2)).await, Some(42));
        assert_eq!(first_or_timeout(Duration::from_secs(2), Duration::from_secs(1)).await, None);
    }

    #[tokio::test]
    async fn borrowing_a_losing_future_preserves_it_for_later() {
        let executed = Cell::new(false);
        let future = async {
            tokio::task::yield_now().await;
            executed.set(true);
            42
        };
        tokio::pin!(future);
        let value = tokio::select! {
            biased;
            value = &mut future => value,
            _ = async {} => 0,
        };
        assert_eq!(value, 0);
        assert!(!executed.get());
        assert_eq!(future.await, 42);
    }

    fn delayed_stream() -> futures::stream::BoxStream<'static, u32> {
        futures::stream::unfold(0, |index| async move {
            if index == 3 {
                None
            } else {
                tokio::time::sleep(Duration::from_secs(2)).await;
                Some((index, index + 1))
            }
        }).boxed()
    }

    #[tokio::test(start_paused = true)]
    async fn idle_timeout_resets_but_total_deadline_does_not() {
        let start = tokio::time::Instant::now();
        assert_eq!(collect_with_idle_timeout(&mut delayed_stream(), Duration::from_secs(3)).await, [0, 1, 2]);
        assert!(start.elapsed() >= Duration::from_secs(6));
        let start = tokio::time::Instant::now();
        assert_eq!(collect_with_deadline(&mut delayed_stream(), Duration::from_secs(5)).await, [0, 1]);
        assert_eq!(start.elapsed(), Duration::from_secs(5));
    }

    #[tokio::test]
    async fn fused_future_becomes_pending_after_completion() {
        let mut future = ready(42).fuse();
        assert_eq!((&mut future).await, 42);
        assert!((&mut future).now_or_never().is_none());
    }
}
