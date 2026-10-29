#![cfg(test)]
#![allow(dead_code)]

// ANCHOR: example
use futures::future::{BoxFuture, FutureExt};

fn recursive() -> BoxFuture<'static, ()> {
    async move {
        recursive().await;
        recursive().await;
    }.boxed()
}
// ANCHOR_END: example

// ANCHOR: example_pinned
async fn recursive_pinned() {
    Box::pin(recursive_pinned()).await;
    Box::pin(recursive_pinned()).await;
}
// ANCHOR_END: example_pinned

// ANCHOR: bounded
async fn count_levels(remaining: u32) -> u32 {
    if remaining == 0 {
        return 0;
    }
    1 + Box::pin(count_levels(remaining - 1)).await
}
// ANCHOR_END: bounded

#[test]
fn bounded_recursion_reaches_base_case() {
    futures::executor::block_on(async {
        assert_eq!(count_levels(0).await, 0);
        assert_eq!(count_levels(8).await, 8);
    });
}
