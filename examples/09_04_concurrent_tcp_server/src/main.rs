// ANCHOR: main_func
use async_std::net::TcpListener;
use async_std::net::TcpStream;
use futures::stream::StreamExt;

#[async_std::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").await.unwrap();
    listener
        .incoming()
        .for_each_concurrent(/* 并发限制 */ None, |tcpstream| async move {
            let tcpstream = tcpstream.unwrap();
            handle_connection(tcpstream).await;
        })
        .await;
}
// ANCHOR_END: main_func

const RESPONSE: &str = "hello";
// ANCHOR: handle_connection
use async_std::prelude::*;

async fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0; 1024];
    let n = stream.read(&mut buffer).await.unwrap();
    if n == 0 {
        return;
    }

    // 仅演示异步读写与并发调度；hello 不是 HTTP 响应。
    stream.write_all(RESPONSE.as_bytes()).await.unwrap();
    stream.flush().await.unwrap();
}
// ANCHOR_END: handle_connection
