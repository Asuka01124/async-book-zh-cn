// 定义一个异步函数。
async fn say_hello() {
    println!("hello, world!");
}

#[tokio::main] // 允许我们编写 `async fn main` 的样板代码，后面会解释。
async fn main() {
    // 调用异步函数并等待其结果。
    say_hello().await;
}
