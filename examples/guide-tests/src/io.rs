// ANCHOR: frames
use bytes::Bytes;
use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const MAX_FRAME: usize = 1024 * 1024;

pub async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<Bytes> {
    let length = reader.read_u32().await? as usize;
    if length > MAX_FRAME {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "消息过大"));
    }
    let mut buffer = vec![0; length];
    reader.read_exact(&mut buffer).await?;
    Ok(Bytes::from(buffer))
}

pub async fn write_frame<W: AsyncWrite + Unpin>(
    writer: &mut W,
    payload: &[u8],
) -> io::Result<()> {
    if payload.len() > MAX_FRAME {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "消息过大"));
    }
    writer.write_u32(payload.len() as u32).await?;
    writer.write_all(payload).await?;
    writer.flush().await
}
// ANCHOR_END: frames

// ANCHOR: shared-buffer
pub fn share_payload() -> (bytes::Bytes, bytes::Bytes) {
    let mut buffer = bytes::BytesMut::with_capacity(64);
    buffer.extend_from_slice(b"hello world");
    let message = buffer.freeze();
    let prefix = message.slice(..5);
    (message, prefix)
}
// ANCHOR_END: shared-buffer

// ANCHOR: buffered-lines
pub async fn buffered_lines() -> std::io::Result<Vec<String>> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};

    let (writer, reader) = tokio::io::duplex(16);
    let send = async move {
        let mut writer = BufWriter::new(writer);
        writer.write_all(b"hello\nworld\n").await?;
        writer.flush().await?; // Drop 不会替我们异步刷新缓冲区。
        writer.shutdown().await
    };
    let receive = async move {
        let mut lines = BufReader::new(reader).lines();
        let mut result = Vec::new();
        while let Some(line) = lines.next_line().await? {
            result.push(line);
        }
        Ok::<_, std::io::Error>(result)
    };
    let ((), result) = tokio::try_join!(send, receive)?;
    Ok(result)
}
// ANCHOR_END: buffered-lines

// ANCHOR: readiness
pub async fn write_ready(
    stream: &tokio::net::TcpStream,
    mut data: &[u8],
) -> std::io::Result<()> {
    while !data.is_empty() {
        stream.writable().await?;
        match stream.try_write(data) {
            Ok(0) => return Err(std::io::ErrorKind::WriteZero.into()),
            Ok(n) => data = &data[n..],
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => continue,
            Err(error) => return Err(error),
        }
    }
    Ok(())
}
// ANCHOR_END: readiness

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{poll, FutureExt};
    use tokio::io::{duplex, BufWriter};

    #[tokio::test]
    async fn frames_survive_partial_transfers_and_backpressure() -> io::Result<()> {
        let (mut writer, mut reader) = duplex(3);
        let message = b"payload larger than the bounded pipe";
        let ((), frame) = tokio::try_join!(
            async {
                write_frame(&mut writer, message).await?;
                writer.shutdown().await
            },
            read_frame(&mut reader)
        )?;
        assert_eq!(frame.as_ref(), message);
        Ok(())
    }

    #[tokio::test]
    async fn truncated_and_oversized_frames_are_rejected() -> io::Result<()> {
        let (mut writer, mut reader) = duplex(16);
        writer.write_u32(4).await?;
        writer.write_all(b"ab").await?;
        writer.shutdown().await?;
        assert_eq!(read_frame(&mut reader).await.unwrap_err().kind(), io::ErrorKind::UnexpectedEof);
        let (mut writer, mut reader) = duplex(16);
        writer.write_u32(MAX_FRAME as u32 + 1).await?;
        assert_eq!(read_frame(&mut reader).await.unwrap_err().kind(), io::ErrorKind::InvalidData);
        Ok(())
    }

    #[test]
    fn bytes_views_share_storage() {
        let (message, prefix) = share_payload();
        assert_eq!(message.as_ref(), b"hello world");
        assert_eq!(prefix.as_ref(), b"hello");
        assert_eq!(message.as_ptr(), prefix.as_ptr());
    }

    #[tokio::test]
    async fn buffered_lines_are_flushed_and_eof_terminates() -> io::Result<()> {
        assert_eq!(buffered_lines().await?, ["hello", "world"]);
        Ok(())
    }

    #[tokio::test]
    async fn read_is_cancel_safe_but_read_exact_can_consume_a_prefix() -> io::Result<()> {
        let (mut writer, mut reader) = duplex(16);
        let mut buffer = [0; 4];
        {
            let read = reader.read(&mut buffer);
            tokio::pin!(read);
            assert!(poll!(&mut read).is_pending());
        }
        writer.write_all(b"ab").await?;
        assert_eq!(reader.read(&mut buffer).await?, 2);
        assert_eq!(&buffer[..2], b"ab");

        writer.write_all(b"cd").await?;
        {
            let read = reader.read_exact(&mut buffer);
            tokio::pin!(read);
            assert!(poll!(&mut read).is_pending());
        }
        // read_exact 已取走 cd；取消之后不能从流中再次读到它。
        assert_eq!(&buffer[..2], b"cd");
        writer.shutdown().await?;
        let mut rest = Vec::new();
        reader.read_to_end(&mut rest).await?;
        assert!(rest.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn write_is_cancel_safe_but_write_all_can_send_a_prefix() -> io::Result<()> {
        let (mut writer, mut reader) = duplex(2);
        writer.write_all(b"ab").await?;
        {
            let write = writer.write(b"cd");
            tokio::pin!(write);
            assert!(poll!(&mut write).is_pending());
        }
        let mut first = [0; 2];
        reader.read_exact(&mut first).await?;
        assert_eq!(&first, b"ab");
        {
            let write = writer.write_all(b"cdef");
            tokio::pin!(write);
            assert!(poll!(&mut write).is_pending());
        }
        writer.shutdown().await?;
        let mut rest = Vec::new();
        reader.read_to_end(&mut rest).await?;
        assert_eq!(rest, b"cd");
        Ok(())
    }

    #[tokio::test]
    async fn dropping_buf_writer_loses_unflushed_data() -> io::Result<()> {
        let (writer, mut reader) = duplex(16);
        let mut writer = BufWriter::with_capacity(16, writer);
        writer.write_all(b"lost").await?;
        assert!(reader.read_u8().now_or_never().is_none());
        drop(writer);
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        assert!(bytes.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn readiness_loop_writes_to_a_real_local_socket() -> io::Result<()> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let send = async {
            let stream = tokio::net::TcpStream::connect(address).await?;
            write_ready(&stream, b"ready").await
        };
        let receive = async {
            let (mut stream, _) = listener.accept().await?;
            let mut bytes = Vec::new();
            stream.read_to_end(&mut bytes).await?;
            Ok::<_, io::Error>(bytes)
        };
        let ((), bytes) = tokio::try_join!(send, receive)?;
        assert_eq!(bytes, b"ready");
        Ok(())
    }

    #[tokio::test]
    async fn copy_bidirectional_handles_half_close() -> io::Result<()> {
        let (mut client, mut proxy_left) = duplex(16);
        let (mut proxy_right, mut server) = duplex(16);
        let proxy = tokio::io::copy_bidirectional(&mut proxy_left, &mut proxy_right);
        let client_work = async {
            client.write_all(b"request").await?;
            client.shutdown().await?;
            let mut response = Vec::new();
            client.read_to_end(&mut response).await?;
            Ok::<_, io::Error>(response)
        };
        let server_work = async {
            let mut request = Vec::new();
            server.read_to_end(&mut request).await?;
            assert_eq!(request, b"request");
            server.write_all(b"reply").await?;
            server.shutdown().await
        };
        let ((sent, received), response, ()) = tokio::try_join!(proxy, client_work, server_work)?;
        assert_eq!((sent, received), (7, 5));
        assert_eq!(response, b"reply");
        Ok(())
    }

    #[tokio::test]
    async fn started_blocking_work_survives_abort() {
        let (started_send, started_receive) = tokio::sync::oneshot::channel();
        let (release_send, release_receive) = std::sync::mpsc::channel();
        let handle = tokio::task::spawn_blocking(move || {
            started_send.send(()).unwrap();
            release_receive.recv().unwrap();
            42
        });
        started_receive.await.unwrap();
        handle.abort();
        release_send.send(()).unwrap();
        assert_eq!(handle.await.unwrap(), 42);
    }
}
