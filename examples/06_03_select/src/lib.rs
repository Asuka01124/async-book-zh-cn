#![cfg(test)]
#![recursion_limit="128"]

mod example {
// ANCHOR: example
use futures::{
    future::FutureExt, // 用于 `.fuse()`
    pin_mut,
    select,
};

async fn task_one() { /* 教学占位 */ }
async fn task_two() { /* 教学占位 */ }

async fn race_tasks() {
    let t1 = task_one().fuse();
    let t2 = task_two().fuse();

    pin_mut!(t1, t2);

    select! {
        () = t1 => println!("任务一被选中"),
        () = t2 => println!("任务二被选中"),
    }
}
// ANCHOR_END: example
}

mod default_and_complete {
// ANCHOR: default_and_complete
use futures::{future, select};

async fn count() {
    let mut a_fut = future::ready(4);
    let mut b_fut = future::ready(6);
    let mut total = 0;

    loop {
        select! {
            a = a_fut => total += a,
            b = b_fut => total += b,
            complete => break,
            default => unreachable!(), // 不会执行（future 已就绪，随后会完成）
        };
    }
    assert_eq!(total, 10);
}
// ANCHOR_END: default_and_complete

#[test]
fn run_count() {
    futures::executor::block_on(count());
}
}

mod fused_stream {
// ANCHOR: fused_stream
use futures::{
    stream::{Stream, StreamExt, FusedStream},
    select,
};

async fn add_two_streams(
    mut s1: impl Stream<Item = u8> + FusedStream + Unpin,
    mut s2: impl Stream<Item = u8> + FusedStream + Unpin,
) -> u8 {
    let mut total = 0;

    loop {
        let item = select! {
            x = s1.next() => x,
            x = s2.next() => x,
            complete => break,
        };
        if let Some(next_num) = item {
            total += next_num;
        }
    }

    total
}
// ANCHOR_END: fused_stream
}

mod fuse_terminated {
// ANCHOR: fuse_terminated
use futures::{
    future::{Fuse, FusedFuture, FutureExt},
    stream::{FusedStream, Stream, StreamExt},
    pin_mut,
    select,
};

async fn get_new_num() -> u8 { /* 教学占位 */ 5 }

async fn run_on_new_num(_: u8) { /* 教学占位 */ }

async fn run_loop(
    mut interval_timer: impl Stream<Item = ()> + FusedStream + Unpin,
    starting_num: u8,
) {
    let run_on_new_num_fut = run_on_new_num(starting_num).fuse();
    let get_new_num_fut = Fuse::terminated();
    pin_mut!(run_on_new_num_fut, get_new_num_fut);
    loop {
        select! {
            () = interval_timer.select_next_some() => {
                // 定时器已到期。如果尚无正在运行的 `get_new_num_fut`，
                // 就启动一个新的。
                if get_new_num_fut.is_terminated() {
                    get_new_num_fut.set(get_new_num().fuse());
                }
            },
            new_num = get_new_num_fut => {
                // 收到一个新数字——启动新的 `run_on_new_num_fut`，
                // 并丢弃旧的 future。
                run_on_new_num_fut.set(run_on_new_num(new_num).fuse());
            },
            // 运行 `run_on_new_num_fut`
            () = run_on_new_num_fut => {},
            // 如果所有任务都已完成，则触发 panic，因为 `interval_timer` 应该
            // 会持续不断地产生值。
            complete => panic!("`interval_timer` 意外完成"),
        }
    }
}
// ANCHOR_END: fuse_terminated
}

mod futures_unordered {
// ANCHOR: futures_unordered
use futures::{
    future::{Fuse, FusedFuture, FutureExt},
    stream::{FusedStream, FuturesUnordered, Stream, StreamExt},
    pin_mut,
    select,
};

async fn get_new_num() -> u8 { /* 教学占位 */ 5 }

async fn run_on_new_num(_: u8) -> u8 { /* 教学占位 */ 5 }

async fn run_loop(
    mut interval_timer: impl Stream<Item = ()> + FusedStream + Unpin,
    starting_num: u8,
) {
    let mut run_on_new_num_futs = FuturesUnordered::new();
    run_on_new_num_futs.push(run_on_new_num(starting_num));
    let get_new_num_fut = Fuse::terminated();
    pin_mut!(get_new_num_fut);
    loop {
        select! {
            () = interval_timer.select_next_some() => {
                // 定时器已到期。如果尚无正在运行的 `get_new_num_fut`，
                // 就启动一个新的。
                if get_new_num_fut.is_terminated() {
                    get_new_num_fut.set(get_new_num().fuse());
                }
            },
            new_num = get_new_num_fut => {
                // 收到一个新数字——启动新的 `run_on_new_num_fut`。
                run_on_new_num_futs.push(run_on_new_num(new_num));
            },
            // 运行 `run_on_new_num_futs`，并检查其中是否有 future 已完成
            res = run_on_new_num_futs.select_next_some() => {
                println!("run_on_new_num_fut 返回了 {:?}", res);
            },
            // 如果所有任务都已完成，则触发 panic，因为 `interval_timer` 应该
            // 会持续不断地产生值。
            complete => panic!("`interval_timer` 意外完成"),
        }
    }
}

// ANCHOR_END: futures_unordered
}

#[test]
fn select_tracks_terminated_and_reactivated_branches() {
    use futures::{future::{self, Fuse, FusedFuture, FutureExt}, stream::{FusedStream, FuturesUnordered, StreamExt}};
    futures::executor::block_on(async {
        // Pending 本身报告 terminated；外层 Fuse 保持尚未完成状态。
        let mut waiting = future::pending::<()>().fuse();
        let took_default = futures::select! {
            () = waiting => false,
            default => true,
        };
        assert!(took_default);

        let slot: Fuse<future::Ready<u8>> = Fuse::terminated();
        futures::pin_mut!(slot);
        assert!(slot.is_terminated());
        let took_complete = futures::select! {
            _ = slot => false,
            complete => true,
            default => false,
        };
        assert!(took_complete);
        slot.set(future::ready(7).fuse());
        let value = futures::select! { value = slot => value };
        assert_eq!(value, 7);
        assert!(slot.is_terminated());

        // 直接表达式由宏固定，async Future 本身不必实现 Unpin。
        let value = futures::select! { value = async { 9 }.fuse() => value };
        assert_eq!(value, 9);

        let mut group = FuturesUnordered::<future::Ready<u8>>::new();
        // 新集合尚未报告终止：首次观察空流返回 Pending 并安排唤醒。
        let took_default = futures::select! {
            _ = group.select_next_some() => false,
            complete => false,
            default => true,
        };
        assert!(took_default);
        assert!(group.is_terminated());
        // 此后终止分支被排除，complete 优先于 default。
        let took_complete = futures::select! {
            _ = group.select_next_some() => false,
            complete => true,
            default => false,
        };
        assert!(took_complete);
        for value in [3, 5] {
            group.push(future::ready(value));
            assert!(!group.is_terminated());
            let item = futures::select! { item = group.select_next_some() => item };
            assert_eq!(item, value);
            // 显式消费到结束，再验证后一次 push 可以重新激活集合。
            assert_eq!(group.next().await, None);
            assert!(group.is_terminated());
        }
    });
}
