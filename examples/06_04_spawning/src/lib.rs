#![cfg(test)]
#![allow(dead_code)]

// ANCHOR: example
use async_std::{task, net::TcpListener, net::TcpStream};
use futures::AsyncWriteExt;

async fn process_request(stream: &mut TcpStream) -> Result<(), std::io::Error>{
    stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n").await?;
    stream.write_all(b"Hello World").await?;
    Ok(())
}

async fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
    loop {
        // 接受一个新连接
        let (mut stream, _) = listener.accept().await.unwrap();
        // 独立处理连接；丢弃句柄会分离任务，因而在任务内观察写入错误。
        task::spawn(async move {
            if let Err(error) = process_request(&mut stream).await {
                eprintln!("处理连接失败：{}", error);
            }
        });
    }
}
// ANCHOR_END: example
use std::time::Duration;
async fn my_task(time: Duration) {
    println!("my_task 开始运行，时长 {:?}", time);
    task::sleep(time).await;
    println!("my_task 运行结束，时长 {:?}", time);
}
// ANCHOR: join_all
use futures::future::join_all;
async fn task_spawner(){
    let tasks = vec![
        task::spawn(my_task(Duration::from_secs(1))),
        task::spawn(my_task(Duration::from_secs(2))),
        task::spawn(my_task(Duration::from_secs(3))),
    ];
    // 等待所有已生成任务结束；直接丢弃这些句柄会分离任务，不会自动取消。
    join_all(tasks).await;
}
// ANCHOR_END: join_all

#[test]
fn run_task_spawner() {
    futures::executor::block_on(task_spawner());
}

#[test]
fn dropped_handle_detaches_and_explicit_cancel_releases_resources() {
    use futures::channel::oneshot;

    struct NotifyOnDrop(Option<oneshot::Sender<()>>);
    impl Drop for NotifyOnDrop {
        fn drop(&mut self) {
            if let Some(sender) = self.0.take() { let _ = sender.send(()); }
        }
    }

    task::block_on(async {
        let (started_tx, started_rx) = oneshot::channel();
        let (release_tx, release_rx) = oneshot::channel();
        let (finished_tx, finished_rx) = oneshot::channel();
        let handle = task::spawn(async move {
            started_tx.send(()).unwrap();
            release_rx.await.unwrap();
            finished_tx.send(7).unwrap();
        });
        started_rx.await.unwrap();
        // 先丢句柄，再允许任务完成，以握手验证 detach 而非依赖墙钟。
        drop(handle);
        release_tx.send(()).unwrap();
        assert_eq!(finished_rx.await.unwrap(), 7);

        let (started_tx, started_rx) = oneshot::channel();
        let (dropped_tx, dropped_rx) = oneshot::channel();
        let handle = task::spawn(async move {
            let _guard = NotifyOnDrop(Some(dropped_tx));
            started_tx.send(()).unwrap();
            std::future::pending::<()>().await;
        });
        // 资源已经构造且任务已开始，才请求取消。
        started_rx.await.unwrap();
        assert_eq!(handle.cancel().await, None);
        dropped_rx.await.unwrap();
    });
}
