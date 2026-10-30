use async_std::fs;

use futures::stream::StreamExt;

use async_std::net::TcpListener;
use async_std::prelude::*;
// ANCHOR: main_func
use async_std::task::spawn;

#[async_std::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").await.unwrap();
    listener
        .incoming()
        .for_each_concurrent(/* 并发限制 */ None, |stream| async move {
            match stream {
                Ok(stream) => {
                    // 丢弃 async-std 句柄会分离任务；错误在任务内部报告。
                    spawn(async move {
                        if let Err(error) = handle_connection(stream).await {
                            eprintln!("处理连接失败：{error}");
                        }
                    });
                }
                Err(error) => eprintln!("接受连接失败：{error}"),
            }
        })
        .await;
}
// ANCHOR_END: main_func

use async_std::io::{Read, Write};

async fn handle_connection(mut stream: impl Read + Write + Unpin) -> std::io::Result<()> {
    let mut buffer = [0; 1024];
    let n = stream.read(&mut buffer).await?;
    if n == 0 {
        return Ok(());
    }
    let get = b"GET / HTTP/1.1\r\n";
    let (status_line, filename) = if buffer[..n].starts_with(get) {
        ("HTTP/1.1 200 OK\r\n\r\n", "hello.html")
    } else {
        ("HTTP/1.1 404 NOT FOUND\r\n\r\n", "404.html")
    };
    let contents = fs::read_to_string(filename).await?;
    let response = format!("{status_line}{contents}");
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    // ANCHOR: mock_read
    use super::*;
    use futures::task::{Context, Poll};
    use std::io::{Error, ErrorKind};
    use std::pin::Pin;

    struct MockTcpStream {
        read_data: Vec<u8>,
        read_pos: usize,
        write_data: Vec<u8>,
        max_write: usize,
        pending_read: bool,
        pending_write: bool,
        read_pending_count: usize,
        write_pending_count: usize,
        read_error: bool,
        write_error: bool,
        flush_error: bool,
    }

    impl MockTcpStream {
        fn new(input: &[u8]) -> Self {
            Self {
                read_data: input.to_vec(),
                read_pos: 0,
                write_data: Vec::new(),
                max_write: usize::MAX,
                pending_read: false,
                pending_write: false,
                read_pending_count: 0,
                write_pending_count: 0,
                read_error: false,
                write_error: false,
                flush_error: false,
            }
        }
    }

    impl Read for MockTcpStream {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<Result<usize, Error>> {
            if buf.is_empty() {
                return Poll::Ready(Ok(0));
            }
            if self.pending_read {
                // 模拟资源下一轮就绪：改变状态并通知执行器，不能只返回 Pending。
                self.pending_read = false;
                self.read_pending_count += 1;
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            if self.read_error {
                return Poll::Ready(Err(Error::from(ErrorKind::ConnectionReset)));
            }
            let size = (self.read_data.len() - self.read_pos).min(buf.len());
            buf[..size].copy_from_slice(&self.read_data[self.read_pos..self.read_pos + size]);
            self.read_pos += size;
            Poll::Ready(Ok(size))
        }
    }
    // ANCHOR_END: mock_read

    // ANCHOR: mock_write
    impl Write for MockTcpStream {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<Result<usize, Error>> {
            if buf.is_empty() {
                return Poll::Ready(Ok(0));
            }
            if self.pending_write {
                self.pending_write = false;
                self.write_pending_count += 1;
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
            if self.write_error {
                return Poll::Ready(Err(Error::from(ErrorKind::BrokenPipe)));
            }
            let size = buf.len().min(self.max_write);
            self.write_data.extend_from_slice(&buf[..size]);
            Poll::Ready(Ok(size))
        }

        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Error>> {
            if self.flush_error {
                Poll::Ready(Err(Error::from(ErrorKind::BrokenPipe)))
            } else {
                Poll::Ready(Ok(()))
            }
        }

        fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Error>> {
            // 内存模拟器没有额外关闭资源；这不模拟真实 socket 的关闭协议。
            Poll::Ready(Ok(()))
        }
    }
    // ANCHOR_END: mock_write

    // ANCHOR: unpin
    // 所有字段自动满足 Unpin，不需要手写 impl Unpin。
    fn assert_unpin<T: Unpin>(_: &T) {}
    // ANCHOR_END: unpin

    // ANCHOR: test
    #[async_std::test]
    async fn test_handle_connection() {
        let mut stream = MockTcpStream::new(b"GET / HTTP/1.1\r\n");
        assert_unpin(&stream);
        handle_connection(&mut stream).await.unwrap();

        let contents = include_str!("../hello.html");
        let expected = format!("HTTP/1.1 200 OK\r\n\r\n{contents}");
        assert_eq!(stream.write_data, expected.as_bytes());
    }
    // ANCHOR_END: test

    #[async_std::test]
    async fn missing_path_returns_complete_404() {
        let mut stream = MockTcpStream::new(b"GET /missing HTTP/1.1\r\n");
        handle_connection(&mut stream).await.unwrap();
        let expected = format!("HTTP/1.1 404 NOT FOUND\r\n\r\n{}", include_str!("../404.html"));
        assert_eq!(stream.write_data, expected.as_bytes());
    }

    #[async_std::test]
    async fn eof_does_not_write() {
        let mut stream = MockTcpStream::new(b"");
        handle_connection(&mut stream).await.unwrap();
        assert!(stream.write_data.is_empty());
    }

    #[test]
    fn mock_consumes_reads_appends_short_writes_and_wakes() {
        use futures::task::{waker, ArcWake};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;

        #[derive(Default)]
        struct WakeCount(AtomicUsize);
        impl ArcWake for WakeCount {
            fn wake_by_ref(this: &Arc<Self>) {
                this.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let count = Arc::new(WakeCount::default());
        let waker = waker(count.clone());
        let mut cx = Context::from_waker(&waker);
        let mut stream = MockTcpStream::new(b"abcd");
        stream.pending_read = true;
        stream.pending_write = true;
        stream.max_write = 2;
        let mut buf = [0; 2];
        assert!(matches!(Pin::new(&mut stream).poll_read(&mut cx, &mut []), Poll::Ready(Ok(0))));
        assert!(Pin::new(&mut stream).poll_read(&mut cx, &mut buf).is_pending());
        assert_eq!(count.0.load(Ordering::SeqCst), 1);
        assert_eq!(stream.read_pos, 0);
        assert!(matches!(Pin::new(&mut stream).poll_read(&mut cx, &mut buf), Poll::Ready(Ok(2))));
        assert_eq!(&buf, b"ab");
        assert!(matches!(Pin::new(&mut stream).poll_read(&mut cx, &mut buf), Poll::Ready(Ok(2))));
        assert_eq!(&buf, b"cd");
        assert!(matches!(Pin::new(&mut stream).poll_read(&mut cx, &mut buf), Poll::Ready(Ok(0))));
        assert!(matches!(Pin::new(&mut stream).poll_write(&mut cx, b""), Poll::Ready(Ok(0))));
        assert!(Pin::new(&mut stream).poll_write(&mut cx, b"abcd").is_pending());
        assert_eq!(count.0.load(Ordering::SeqCst), 2);
        assert!(stream.write_data.is_empty());
        assert!(matches!(Pin::new(&mut stream).poll_write(&mut cx, b"abcd"), Poll::Ready(Ok(2))));
        assert!(matches!(Pin::new(&mut stream).poll_write(&mut cx, b"cd"), Poll::Ready(Ok(2))));
        assert_eq!(stream.write_data, b"abcd");
    }

    #[async_std::test]
    async fn pending_and_short_writes_preserve_complete_response() {
        let mut stream = MockTcpStream::new(b"GET / HTTP/1.1\r\n");
        stream.max_write = 3;
        stream.pending_read = true;
        stream.pending_write = true;
        handle_connection(&mut stream).await.unwrap();
        assert_eq!(stream.read_pending_count, 1);
        assert_eq!(stream.write_pending_count, 1);
        let expected = format!("HTTP/1.1 200 OK\r\n\r\n{}", include_str!("../hello.html"));
        assert_eq!(stream.write_data, expected.as_bytes());
    }

    #[async_std::test]
    async fn io_errors_are_returned() {
        let mut read_failure = MockTcpStream::new(b"GET / HTTP/1.1\r\n");
        read_failure.read_error = true;
        assert_eq!(handle_connection(&mut read_failure).await.unwrap_err().kind(), ErrorKind::ConnectionReset);
        assert!(read_failure.write_data.is_empty());

        let mut write_failure = MockTcpStream::new(b"GET / HTTP/1.1\r\n");
        write_failure.write_error = true;
        assert_eq!(handle_connection(&mut write_failure).await.unwrap_err().kind(), ErrorKind::BrokenPipe);
        assert!(write_failure.write_data.is_empty());

        let mut flush_failure = MockTcpStream::new(b"GET / HTTP/1.1\r\n");
        flush_failure.flush_error = true;
        assert_eq!(handle_connection(&mut flush_failure).await.unwrap_err().kind(), ErrorKind::BrokenPipe);

        let mut zero_write = MockTcpStream::new(b"GET / HTTP/1.1\r\n");
        zero_write.max_write = 0;
        assert_eq!(handle_connection(&mut zero_write).await.unwrap_err().kind(), ErrorKind::WriteZero);
    }
}
