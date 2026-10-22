use std::io::{stdout, Write};
use tokio::time::{sleep, Duration};

async fn say_hello() {
    print!("hello, ");
    // 刷新标准输出，以便立即看到上面 `print` 的效果。
    stdout().flush().unwrap();
}

async fn say_world() {
    println!("world!");
}

#[tokio::main]
async fn main() {
    say_hello().await;
    // 异步 sleep 函数，让当前任务休眠 1 秒。
    sleep(Duration::from_millis(1000)).await;
    say_world().await;
}
