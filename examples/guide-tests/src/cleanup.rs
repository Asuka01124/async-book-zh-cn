// ANCHOR: explicit-close
use tokio::io::{self, AsyncWrite, AsyncWriteExt};

pub struct Session<W> {
    writer: W,
    finished: bool,
}
impl<W: AsyncWrite + Unpin> Session<W> {
    pub fn new(writer: W) -> Self {
        Self { writer, finished: false }
    }
    pub async fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.writer.write_all(bytes).await
    }
    pub async fn finish(&mut self) -> io::Result<()> {
        self.writer.flush().await?;
        self.writer.shutdown().await?;
        self.finished = true;
        Ok(())
    }
}
impl<W> Drop for Session<W> {
    fn drop(&mut self) {
        if !self.finished {
            // 仅报告遗漏；不能在这里等待网络上的关闭确认。
            tracing::warn!("会话未显式完成关闭");
        }
    }
}
// ANCHOR_END: explicit-close

// ANCHOR: shutdown
pub async fn shutdown_worker(
    mut stop: tokio::sync::watch::Receiver<bool>,
    writer: impl AsyncWrite + Unpin,
) -> io::Result<()> {
    let mut session = Session::new(writer);
    loop {
        if *stop.borrow_and_update() {
            break;
        }
        if stop.changed().await.is_err() {
            break; // 控制端消失也触发关闭。
        }
    }
    // 在收到请求后显式等待清理，不能继续把它和同一关闭请求竞争。
    session.finish().await
}
// ANCHOR_END: shutdown

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{atomic::{AtomicBool, Ordering}, Arc};
    use tokio::io::AsyncReadExt;

    struct ExitGuard(Arc<AtomicBool>);
    impl Drop for ExitGuard {
        fn drop(&mut self) { self.0.store(true, Ordering::SeqCst); }
    }

    #[tokio::test]
    async fn explicit_finish_flushes_buffer_and_shuts_down() {
        let (writer, mut peer) = tokio::io::duplex(16);
        let mut session = Session::new(tokio::io::BufWriter::new(writer));
        session.send(b"bye").await.unwrap();
        session.finish().await.unwrap();
        assert!(session.finished);
        let mut bytes = Vec::new();
        peer.read_to_end(&mut bytes).await.unwrap();
        assert_eq!(bytes, b"bye");
    }

    #[tokio::test]
    async fn abort_observed_via_join_has_run_synchronous_destructors() {
        let dropped = Arc::new(AtomicBool::new(false));
        let flag = dropped.clone();
        let (started, ready) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let _guard = ExitGuard(flag);
            started.send(()).unwrap();
            futures::future::pending::<()>().await;
        });
        ready.await.unwrap();
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(dropped.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn panic_releases_mutex_but_does_not_restore_invariant() {
        let state = Arc::new(tokio::sync::Mutex::new(vec![1, 2]));
        let other = state.clone();
        let task = tokio::spawn(async move {
            let mut guard = other.lock().await;
            guard.clear();
            panic!("更新中途失败");
        });
        assert!(task.await.unwrap_err().is_panic());
        assert!(state.lock().await.is_empty());
    }

    #[tokio::test]
    async fn cooperative_shutdown_completes_before_join() {
        let (tx, rx) = tokio::sync::watch::channel(false);
        let (writer, mut peer) = tokio::io::duplex(8);
        let task = tokio::spawn(shutdown_worker(rx, writer));
        tx.send(true).unwrap();
        task.await.unwrap().unwrap();
        let mut bytes = Vec::new();
        peer.read_to_end(&mut bytes).await.unwrap();
        assert!(bytes.is_empty());
    }

    #[tokio::test]
    async fn dropping_unfinished_buffer_is_not_async_cleanup() {
        let (writer, mut peer) = tokio::io::duplex(16);
        let mut session = Session::new(tokio::io::BufWriter::new(writer));
        session.send(b"lost").await.unwrap();
        drop(session);
        let mut bytes = Vec::new();
        peer.read_to_end(&mut bytes).await.unwrap();
        assert!(bytes.is_empty());
    }
}
