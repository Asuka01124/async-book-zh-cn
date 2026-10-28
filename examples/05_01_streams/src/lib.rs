#![cfg(test)]

mod stream_trait {
use futures::stream::Stream as RealStream;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

// ANCHOR: stream_trait
trait Stream {
    /// 流产生的值的类型。
    type Item;

    /// 尝试取得流中的下一个元素。
    /// 尚未就绪时返回 `Poll::Pending`，值就绪时返回 `Poll::Ready(Some(x))`，
    /// 流完成时返回 `Poll::Ready(None)`。
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>)
        -> Poll<Option<Self::Item>>;
}
// ANCHOR_END: stream_trait

// 断言 `Stream` 与 `RealStream` 相符：
impl<I> Stream for dyn RealStream<Item = I> {
    type Item = I;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>)
        -> Poll<Option<Self::Item>>
    {
        RealStream::poll_next(self, cx)
    }
}
}

mod channels {
use futures::{
    channel::mpsc,
    prelude::*,
};

// ANCHOR: channels
async fn send_recv() {
    const BUFFER_SIZE: usize = 10;
    let (mut tx, mut rx) = mpsc::channel::<i32>(BUFFER_SIZE);

    tx.send(1).await.unwrap();
    tx.send(2).await.unwrap();
    drop(tx);

    // `StreamExt::next` 与 `Iterator::next` 类似，但它返回的类型实现了
    // `Future<Output = Option<T>>`。
    assert_eq!(Some(1), rx.next().await);
    assert_eq!(Some(2), rx.next().await);
    assert_eq!(None, rx.next().await);
}
// ANCHOR_END: channels

#[test]
fn run_send_recv() { futures::executor::block_on(send_recv()) }
}
