#![cfg(test)]

use futures::{executor::block_on, join};
use std::thread;

fn download(_url: &str) {
    // 历史教学占位：不发出网络请求，只用于比较调用结构。
}

#[test]
// ANCHOR: get_two_sites
fn get_two_sites() {
    // 创建两个线程来执行工作。
    let thread_one = thread::spawn(|| download("https://www.foo.com"));
    let thread_two = thread::spawn(|| download("https://www.bar.com"));

    // 等待两个线程都完成。
    thread_one.join().expect("第一个线程发生 panic");
    thread_two.join().expect("第二个线程发生 panic");
}
// ANCHOR_END: get_two_sites

async fn download_async(_url: &str) {
    // 历史教学占位：没有真实 I/O 或 Pending，不证明下载并发。
}

// ANCHOR: get_two_sites_async
async fn get_two_sites_async() {
    // 创建两个 Future，代表两项下载工作；本例下载体仍为空占位。
    let future_one = download_async("https://www.foo.com");
    let future_two = download_async("https://www.bar.com");

    // 并发推进两个 Future，直到二者都完成；不创建独立任务。
    join!(future_one, future_two);
}
// ANCHOR_END: get_two_sites_async

#[test]
fn get_two_sites_async_test() {
    block_on(get_two_sites_async());
}
