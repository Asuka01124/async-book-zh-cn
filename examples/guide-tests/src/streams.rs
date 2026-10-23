use futures::{stream, SinkExt, Stream, StreamExt};
use std::{pin::Pin, task::{Context, Poll}};

// ANCHOR: unfold
pub fn squares() -> impl Stream<Item = u32> {
    stream::unfold(0, |n| async move {
        if n == 4 { None } else { Some((n * n, n + 1)) }
    })
}

pub async fn consume() -> Vec<u32> {
    let values = squares();
    futures::pin_mut!(values);
    let mut out = Vec::new();
    while let Some(value) = values.next().await {
        out.push(value);
    }
    out
}
// ANCHOR_END: unfold

// ANCHOR: counter
pub struct Counter(pub u32);
impl Stream for Counter {
    type Item = u32;
    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<u32>> {
        let this = self.get_mut(); // 这里只有 u32，因此 Counter: Unpin。
        if this.0 == 0 { Poll::Ready(None) } else {
            let n = this.0;
            this.0 -= 1;
            Poll::Ready(Some(n))
        }
    }
}
// ANCHOR_END: counter

// ANCHOR: sink
pub async fn send_batch() -> Vec<u32> {
    let (mut tx, rx) = futures::channel::mpsc::channel(2);
    let producer = async move {
        tx.feed(10).await.unwrap();
        tx.feed(20).await.unwrap();
        tx.flush().await.unwrap();
        tx.close().await.unwrap();
    };
    let (_, values) = futures::join!(producer, rx.collect::<Vec<_>>());
    values
}
// ANCHOR_END: sink

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};

    #[test]
    fn streams_lazy_pin_and_fuse() {
        futures::executor::block_on(async {
            let count = Rc::new(Cell::new(0));
            let c = count.clone();
            let s = stream::iter([1, 2]).map(move |x| { c.set(c.get() + 1); x });
            assert_eq!(count.get(), 0);
            let mut s = s.fuse();
            assert_eq!(s.next().await, Some(1));
            assert_eq!(count.get(), 1);
            assert_eq!(s.next().await, Some(2));
            assert_eq!(s.next().await, None);
            assert_eq!(s.next().await, None);
            assert_eq!(consume().await, [0, 1, 4, 9]);
            assert_eq!(Counter(3).collect::<Vec<_>>().await, [3, 2, 1]);
        });
    }

    #[tokio::test(start_paused = true)]
    async fn streams_buffer_order_and_concurrency_limit() {
        async fn work(n: u64) -> u64 {
            tokio::time::sleep(std::time::Duration::from_secs(4 - n)).await;
            n
        }
        let ordered = stream::iter(1..=3).map(work).buffered(3).collect::<Vec<_>>().await;
        let unordered = stream::iter(1..=3).map(work).buffer_unordered(3).collect::<Vec<_>>().await;
        assert_eq!(ordered, [1, 2, 3]);
        assert_eq!(unordered, [3, 2, 1]);
        let active = Rc::new(Cell::new(0));
        let peak = Rc::new(Cell::new(0));
        stream::iter(0..8).for_each_concurrent(2, |_| {
            let active = active.clone(); let peak = peak.clone();
            async move {
                active.set(active.get() + 1); peak.set(peak.get().max(active.get()));
                tokio::task::yield_now().await;
                active.set(active.get() - 1);
            }
        }).await;
        assert_eq!(peak.get(), 2);
        assert_eq!(active.get(), 0);
    }

    #[test]
    fn streams_group_remove_and_sink_close() {
        futures::executor::block_on(async {
            let mut group = futures_concurrency::stream::StreamGroup::new();
            group.insert(stream::iter([1, 2]));
            let removed = group.insert(stream::iter([99, 100]));
            assert!(group.remove(removed));
            group.insert(stream::iter([3, 4]));
            let mut values = group.collect::<Vec<_>>().await;
            values.sort_unstable();
            assert_eq!(values, [1, 2, 3, 4]);
            assert_eq!(send_batch().await, [10, 20]);
        });
    }

    #[tokio::test]
    async fn streams_select_keeps_receiver_after_other_branch_wins() {
        let (mut tx, mut rx) = futures::channel::mpsc::channel(1);
        tokio::select! {
            biased;
            _ = std::future::ready(()) => {},
            _ = rx.next() => panic!("接收端应仍在等待"),
        }
        tx.send(7).await.unwrap();
        assert_eq!(rx.next().await, Some(7));
    }
}
