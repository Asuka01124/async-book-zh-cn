//! 旧版 I/O 导读的有界换行帧示例，仅使用 Tokio I/O trait。
// ANCHOR: bounded-frame
use std::io;
use tokio::io::{AsyncBufRead, AsyncBufReadExt};

/// 读取以 LF 结束的一帧，返回不含 LF 的载荷；上限以字节计。
/// 干净 EOF 返回 None；缺少 LF 的末帧是错误。
pub async fn read_line_frame<R>(reader: &mut R, max_payload: usize) -> io::Result<Option<Vec<u8>>>
where
    R: AsyncBufRead + Unpin,
{
    let mut frame = Vec::new();
    loop {
        let available = reader.fill_buf().await?;
        if available.is_empty() {
            return if frame.is_empty() {
                Ok(None)
            } else {
                Err(io::Error::new(io::ErrorKind::UnexpectedEof, "帧未以 LF 结束"))
            };
        }
        let delimiter = available.iter().position(|&byte| byte == b'\n');
        let count = delimiter.unwrap_or(available.len());
        // 先检查再追加，保持 frame.len() <= max_payload，避免加法溢出。
        if count > max_payload - frame.len() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "帧载荷超过上限"));
        }
        frame.extend_from_slice(&available[..count]);
        reader.consume(count + usize::from(delimiter.is_some()));
        if delimiter.is_some() {
            return Ok(Some(frame));
        }
    }
}
// ANCHOR_END: bounded-frame

// ANCHOR: frame-demo
pub async fn in_memory_demo() -> io::Result<()> {
    use tokio::io::{duplex, AsyncWriteExt, BufReader};

    // 每个方向最多缓冲 2 字节，长于它的帧必须经多次传输。
    let (input, mut output) = duplex(2);
    let producer = async {
        output.write_all(b"abcd\nx\n").await?;
        output.shutdown().await
    };
    let consumer = async {
        let mut reader = BufReader::with_capacity(2, input);
        assert_eq!(read_line_frame(&mut reader, 4).await?, Some(b"abcd".to_vec()));
        assert_eq!(read_line_frame(&mut reader, 4).await?, Some(b"x".to_vec()));
        assert_eq!(read_line_frame(&mut reader, 4).await?, None);
        Ok::<_, io::Error>(())
    };
    tokio::try_join!(producer, consumer)?;
    Ok(())
}
// ANCHOR_END: frame-demo

#[cfg(test)]
mod tests {
    use super::*;
    use futures::task::{waker, ArcWake};
    use std::future::Future;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::task::{Context, Poll};
    use tokio::io::{duplex, AsyncReadExt, AsyncWriteExt, BufReader};

    #[tokio::test]
    async fn fragmented_frames_and_clean_eof() {
        in_memory_demo().await.unwrap();
    }

    #[tokio::test]
    async fn payload_limit_empty_frame_and_truncated_eof() {
        let mut oversized = BufReader::with_capacity(2, &b"abc\n"[..]);
        assert_eq!(read_line_frame(&mut oversized, 2).await.unwrap_err().kind(), io::ErrorKind::InvalidData);
        let mut truncated = BufReader::with_capacity(2, &b"ab"[..]);
        assert_eq!(read_line_frame(&mut truncated, 2).await.unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
        let mut empty = BufReader::new(&b"\n"[..]);
        assert_eq!(read_line_frame(&mut empty, 0).await.unwrap(), Some(Vec::new()));
        assert_eq!(read_line_frame(&mut empty, 0).await.unwrap(), None);
    }

    #[tokio::test]
    async fn duplex_read_is_pending_then_woken_and_reaches_eof() {
        #[derive(Default)]
        struct WakeCount(AtomicUsize);
        impl ArcWake for WakeCount {
            fn wake_by_ref(this: &Arc<Self>) {
                this.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let (mut reader, mut writer) = duplex(4);
        let mut buf = [0; 4];
        let count = Arc::new(WakeCount::default());
        let waker = waker(count.clone());
        let mut cx = Context::from_waker(&waker);
        let mut read = Box::pin(reader.read(&mut buf));
        assert!(matches!(read.as_mut().poll(&mut cx), Poll::Pending));
        writer.write_all(b"xy").await.unwrap();
        assert!(count.0.load(Ordering::SeqCst) > 0);
        assert_eq!(read.await.unwrap(), 2);
        assert_eq!(&buf[..2], b"xy");
        writer.shutdown().await.unwrap();
        assert_eq!(reader.read(&mut buf).await.unwrap(), 0);
    }
}
