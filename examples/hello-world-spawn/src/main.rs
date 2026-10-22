use tokio::{spawn, time::{sleep, Duration}};

async fn say_hello() {
    // 打印前等待一会儿，让竞争过程更有意思。
    sleep(Duration::from_millis(100)).await;
    println!("hello");
}

async fn say_world() {
    sleep(Duration::from_millis(100)).await;
    println!("world!");
}

#[tokio::main]
async fn main() {
    spawn(say_hello());
    spawn(say_world());
    // 等待一会儿，让任务有时间运行。
    sleep(Duration::from_millis(1000)).await;
}
