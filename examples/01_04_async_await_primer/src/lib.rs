#![cfg(test)]

use futures::executor::block_on;

mod first {
// ANCHOR: hello_world
// `block_on` 占用当前线程，直到给定 Future 完成。
// 其他执行器可以提供更复杂的行为，例如在同一线程上调度多个任务。
use futures::executor::block_on;

async fn hello_world() {
    println!("hello, world!");
}

fn main() {
    let future = hello_world(); // 尚未打印任何内容。
    block_on(future); // 驱动 Future，打印 "hello, world!"。
}
// ANCHOR_END: hello_world

#[test]
fn run_main() { main() }
}

struct Song;
// 历史教学桩：下列三个函数立即完成，没有真实等待或交错证据。
async fn learn_song() -> Song { Song }
async fn sing_song(_: Song) {}
async fn dance() {}

mod second {
use super::*;
// ANCHOR: block_on_each
fn main() {
    let song = block_on(learn_song());
    block_on(sing_song(song));
    block_on(dance());
}
// ANCHOR_END: block_on_each

#[test]
fn run_main() { main() }
}

mod third {
use super::*;
// ANCHOR: block_on_main
async fn learn_and_sing() {
    // 等歌曲学会之后再唱。
    // 使用 `.await`，让真正 Pending 的学习阶段可以交还控制权。
    // 不可在外层 block_on 中再嵌套同一执行器的 block_on；它会拒绝并 panic。
    let song = learn_song().await;
    sing_song(song).await;
}

async fn async_main() {
    let f1 = learn_and_sing();
    let f2 = dance();

    // `join!` 在同一任务中并发等待两个 Future，不会独立 spawn。
    // 一个分支 Pending 时，另一个可以继续被轮询；两个都未完成时，
    // 组合 Future 返回 Pending，由执行器在唤醒后继续驱动。
    // 本例的立即完成桩不展示这一挂起过程，正文另有真实 Pending 示例。
    futures::join!(f1, f2);
}

fn main() {
    block_on(async_main());
}
// ANCHOR_END: block_on_main

#[test]
fn run_main() { main() }
}
