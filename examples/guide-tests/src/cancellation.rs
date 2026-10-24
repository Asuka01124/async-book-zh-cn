use tokio::sync::mpsc;

// ANCHOR: local-pair
pub async fn local_pair(rx: &mut mpsc::Receiver<u8>) -> Option<[u8; 2]> {
    let first = rx.recv().await?;
    let second = rx.recv().await?;
    Some([first, second])
}
// ANCHOR_END: local-pair

// ANCHOR: persistent-pair
#[derive(Default)]
pub struct PairReader {
    first: Option<u8>,
}
impl PairReader {
    pub async fn next(
        &mut self,
        rx: &mut mpsc::Receiver<u8>,
    ) -> Result<Option<[u8; 2]>, u8> {
        if self.first.is_none() {
            self.first = rx.recv().await;
            if self.first.is_none() { return Ok(None); }
        }
        match rx.recv().await {
            Some(second) => Ok(Some([self.first.take().unwrap(), second])),
            None => Err(self.first.take().unwrap()), // 将不完整消息明确交还调用者。
        }
    }
}
// ANCHOR_END: persistent-pair

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cancellation_safe_recv_does_not_make_composition_safe() {
        let (tx, mut rx) = mpsc::channel(4);
        tx.send(1).await.unwrap();
        tokio::select! {
            biased;
            _ = local_pair(&mut rx) => panic!("只有一个值，不能完成"),
            _ = std::future::ready(()) => {},
        }
        tx.send(2).await.unwrap();
        tx.send(3).await.unwrap();
        assert_eq!(local_pair(&mut rx).await, Some([2, 3]));
    }

    #[tokio::test]
    async fn cancellation_external_state_survives_recreated_future_and_reports_eof() {
        let (tx, mut rx) = mpsc::channel(4);
        let mut reader = PairReader::default();
        tx.send(1).await.unwrap();
        tokio::select! {
            biased;
            _ = reader.next(&mut rx) => panic!("只有一个值，不能完成"),
            _ = std::future::ready(()) => {},
        }
        assert_eq!(reader.first, Some(1));
        tx.send(2).await.unwrap();
        assert_eq!(reader.next(&mut rx).await, Ok(Some([1, 2])));
        tx.send(3).await.unwrap();
        drop(tx);
        assert_eq!(reader.next(&mut rx).await, Err(3));
        assert_eq!(reader.next(&mut rx).await, Ok(None));
    }

    #[tokio::test]
    async fn cancellation_borrowed_future_keeps_private_progress() {
        let (tx, mut rx) = mpsc::channel(2);
        tx.send(1).await.unwrap();
        let operation = local_pair(&mut rx);
        tokio::pin!(operation);
        tokio::select! {
            biased;
            _ = &mut operation => panic!("只有一个值，不能完成"),
            _ = std::future::ready(()) => {},
        }
        tx.send(2).await.unwrap();
        assert_eq!(operation.await, Some([1, 2]));
    }
}
