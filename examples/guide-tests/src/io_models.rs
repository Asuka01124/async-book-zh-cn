use std::{future::Future, io, pin::Pin, sync::{Arc, Mutex}, task::{Context, Poll, Waker}};
use tokio::net::TcpStream;

// ANCHOR: readiness
pub async fn read_until_eof(stream: &TcpStream, limit: usize) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    loop {
        stream.readable().await?;
        loop {
            let mut buffer = [0u8; 64]; // 没有把这个栈缓冲区交给内核长期持有。
            match stream.try_read(&mut buffer) {
                Ok(0) => return Ok(out),
                Ok(n) => {
                    if n > limit.saturating_sub(out.len()) {
                        return Err(io::Error::new(io::ErrorKind::InvalidData, "消息超过限制"));
                    }
                    out.extend_from_slice(&buffer[..n]);
                }
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
    }
}
// ANCHOR_END: readiness

// ANCHOR: completion-model
// 纯内存生命周期模型：不是IOCP或io_uring封装，没有提交任何内核请求。
struct Operation {
    buffer: Option<Vec<u8>>, result: Option<usize>, interested: bool,
    cancel_requested: bool, waker: Option<Waker>,
}
pub struct ModelWait(Arc<Mutex<Operation>>);
pub struct ModelDriver(Arc<Mutex<Operation>>);
pub fn model_submit(buffer: Vec<u8>) -> (ModelWait, ModelDriver) {
    let state = Arc::new(Mutex::new(Operation {
        buffer: Some(buffer), result: None, interested: true,
        cancel_requested: false, waker: None,
    }));
    (ModelWait(state.clone()), ModelDriver(state))
}
impl Future for ModelWait {
    type Output = usize;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<usize> {
        let new = cx.waker().clone();
        let mut state = self.0.lock().unwrap();
        if let Some(n) = state.result { return Poll::Ready(n); }
        let old = state.waker.replace(new);
        drop(state); drop(old);
        Poll::Pending
    }
}
impl Drop for ModelWait {
    fn drop(&mut self) {
        let mut state = self.0.lock().unwrap();
        state.interested = false;
        state.cancel_requested = state.result.is_none();
        let old = state.waker.take();
        drop(state); drop(old);
    }
}
impl ModelDriver {
    pub fn complete(self, data: &[u8]) -> Vec<u8> {
        let (buffer, wake) = {
            let mut state = self.0.lock().unwrap();
            let mut buffer = state.buffer.take().unwrap();
            // 模拟取消请求输给正常完成：驱动器仍可在这时访问缓冲区。
            buffer.clear(); buffer.extend_from_slice(data);
            state.result = Some(buffer.len());
            (buffer, state.waker.take())
        };
        if let Some(waker) = wake { waker.wake(); }
        buffer // 到模拟完成点后才交还/回收。
    }
}
// ANCHOR_END: completion-model

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpListener};
    async fn pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (client, accepted) = tokio::join!(TcpStream::connect(address), listener.accept());
        (client.unwrap(), accepted.unwrap().0)
    }
    #[tokio::test]
    async fn io_models_real_tcp_readiness_short_reads_and_eof() {
        let (client, mut server) = pair().await;
        let writer = async move {
            server.write_all(b"abc").await.unwrap();
            tokio::task::yield_now().await;
            server.write_all(&[7; 128]).await.unwrap();
            server.shutdown().await.unwrap();
        };
        let (bytes, _) = tokio::join!(read_until_eof(&client, 256), writer);
        let bytes = bytes.unwrap();
        assert_eq!(&bytes[..3], b"abc");
        assert_eq!(&bytes[3..], &[7; 128]);
    }
    #[tokio::test]
    async fn io_models_real_tcp_cancel_pending_borrowed_read_then_read_again() {
        let (mut client, mut server) = pair().await;
        let mut buffer = [0; 8];
        let mut read = Box::pin(client.read(&mut buffer));
        std::future::poll_fn(|cx| {
            assert!(read.as_mut().poll(cx).is_pending()); // 对端此时确实尚未写入。
            Poll::Ready(())
        }).await;
        drop(read);
        server.write_all(b"xyz").await.unwrap();
        client.read_exact(&mut buffer[..3]).await.unwrap();
        assert_eq!(&buffer[..3], b"xyz");
    }
    #[test]
    fn io_models_model_cancel_keeps_buffer_until_simulated_completion() {
        let (mut wait, driver) = model_submit(vec![0; 16]);
        let w = futures::task::noop_waker();
        assert!(Pin::new(&mut wait).poll(&mut Context::from_waker(&w)).is_pending());
        drop(wait);
        {
            let state = driver.0.lock().unwrap();
            assert!(!state.interested); assert!(state.cancel_requested);
            assert_eq!(state.buffer.as_ref().unwrap().len(), 16);
        }
        assert_eq!(driver.complete(b"late result"), b"late result");
    }
    #[test]
    fn io_models_model_completion_notifies_registered_waiter() {
        use std::{sync::atomic::{AtomicUsize, Ordering}, task::Wake};
        struct Count(AtomicUsize);
        impl Wake for Count { fn wake(self: Arc<Self>) { self.0.fetch_add(1, Ordering::SeqCst); } }
        let (mut wait, driver) = model_submit(vec![]);
        let count = Arc::new(Count(AtomicUsize::new(0))); let w = Waker::from(count.clone());
        assert!(Pin::new(&mut wait).poll(&mut Context::from_waker(&w)).is_pending());
        assert_eq!(driver.complete(b"ok"), b"ok");
        assert_eq!(count.0.load(Ordering::SeqCst), 1);
        assert_eq!(futures::executor::block_on(wait), 2);
    }
}
