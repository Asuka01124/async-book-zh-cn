use std::{future::Future, pin::Pin, task::{Context, Poll}};

// ANCHOR: budgeted-sum
pub struct BudgetedSum {
    values: std::vec::IntoIter<u64>,
    total: u64,
    budget: usize,
}
impl BudgetedSum {
    pub fn new(values: Vec<u64>, budget: usize) -> Self {
        assert!(budget > 0, "每次poll至少推进一项");
        Self { values: values.into_iter(), total: 0, budget }
    }
}
impl Future for BudgetedSum {
    type Output = Result<u64, &'static str>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        for _ in 0..self.budget {
            let Some(value) = self.values.next() else { return Poll::Ready(Ok(self.total)); };
            let Some(total) = self.total.checked_add(value) else {
                return Poll::Ready(Err("求和溢出"));
            };
            self.total = total;
        }
        if self.values.len() == 0 { return Poll::Ready(Ok(self.total)); }
        // 仍可推进，但先交回控制；没有外部事件会替这项CPU工作唤醒。
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}
// ANCHOR_END: budgeted-sum

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
    struct CountWake(AtomicUsize);
    impl std::task::Wake for CountWake {
        fn wake(self: Arc<Self>) { self.0.fetch_add(1, Ordering::SeqCst); }
        fn wake_by_ref(self: &Arc<Self>) { self.0.fetch_add(1, Ordering::SeqCst); }
    }

    #[test]
    fn starvation_ready_awaits_do_not_end_the_outer_poll() {
        let count = std::cell::Cell::new(0);
        let finite = async {
            for _ in 0..7 {
                std::future::ready(()).await;
                count.set(count.get() + 1);
            }
        };
        let mut future = std::pin::pin!(finite);
        let waker = std::task::Waker::noop();
        assert!(future.as_mut().poll(&mut Context::from_waker(waker)).is_ready());
        assert_eq!(count.get(), 7);
    }

    #[test]
    fn starvation_budget_bounds_each_poll_and_self_wakes() {
        let wake = Arc::new(CountWake(AtomicUsize::new(0)));
        let waker = std::task::Waker::from(wake.clone());
        let mut cx = Context::from_waker(&waker);
        let mut work = BudgetedSum::new(vec![1, 2, 3, 4, 5], 2);
        assert_eq!(Pin::new(&mut work).poll(&mut cx), Poll::Pending);
        assert_eq!(work.values.len(), 3);
        assert_eq!(wake.0.load(Ordering::SeqCst), 1);
        // 控制权已经交回，调用者现在可以轮询另一项工作。
        assert_eq!(Pin::new(&mut std::future::ready(9)).poll(&mut cx), Poll::Ready(9));
        assert_eq!(Pin::new(&mut work).poll(&mut cx), Poll::Pending);
        assert_eq!(work.values.len(), 1);
        assert_eq!(Pin::new(&mut work).poll(&mut cx), Poll::Ready(Ok(15)));
        assert_eq!(wake.0.load(Ordering::SeqCst), 2);
        let mut overflow = BudgetedSum::new(vec![u64::MAX, 1], 2);
        assert_eq!(Pin::new(&mut overflow).poll(&mut cx), Poll::Ready(Err("求和溢出")));
    }

    #[tokio::test]
    async fn starvation_biased_ready_branch_prevents_later_branch_poll() {
        let later_polls = std::cell::Cell::new(0);
        for _ in 0..5 {
            tokio::select! {
                biased;
                _ = std::future::ready(()) => {},
                _ = std::future::poll_fn(|_| {
                    later_polls.set(later_polls.get() + 1);
                    Poll::Ready(())
                }) => panic!("前分支总是Ready"),
            }
        }
        assert_eq!(later_polls.get(), 0);
        let stop = tokio::select! {
            biased;
            _ = std::future::ready(()) => true, // 有限、优先的控制分支。
            _ = std::future::ready(()) => false,
        };
        assert!(stop);
    }
}
