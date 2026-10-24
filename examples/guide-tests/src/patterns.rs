use tokio::sync::{mpsc, oneshot};

// ANCHOR: counter-actor
#[derive(Debug)]
pub struct Add {
    pub amount: u64,
    pub reply: oneshot::Sender<Result<u64, &'static str>>,
}

pub fn add(amount: u64) -> (Add, oneshot::Receiver<Result<u64, &'static str>>) {
    let (reply, response) = oneshot::channel();
    (Add { amount, reply }, response)
}

pub async fn counter_actor(
    mut requests: mpsc::Receiver<Add>,
    mut shutdown: oneshot::Receiver<()>,
    mut total: u64,
) -> u64 {
    let mut closing = false;
    loop {
        tokio::select! {
            biased; // 一次性关闭优先；进入排空阶段后禁用该分支。
            _ = &mut shutdown, if !closing => {
                closing = true;
                requests.close();
            }
            request = requests.recv() => {
                let Some(Add { amount, reply }) = request else { break };
                let result = match total.checked_add(amount) {
                    Some(next) => { total = next; Ok(total) }
                    None => Err("计数溢出"),
                };
                // 调用者放弃响应，并不撤销已经完成的状态变化。
                let _ = reply.send(result);
            }
        }
    }
    total // 真正的服务还应在这里 await 必要的 flush/close。
}
// ANCHOR_END: counter-actor

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn patterns_backpressure_then_shutdown_drains_and_rejects() {
        let (tx, rx) = mpsc::channel(1);
        let (stop, shutdown) = oneshot::channel();
        let (first, response) = add(1);
        tx.send(first).await.unwrap();
        let (second, _) = add(2);
        let blocked = tx.send(second);
        tokio::pin!(blocked);
        assert!(futures::poll!(&mut blocked).is_pending());
        stop.send(()).unwrap();
        // stop 已就绪，明确在读取第一条消息前关闭入口。
        let actor = tokio::spawn(counter_actor(rx, shutdown, 0));
        assert_eq!(blocked.await.unwrap_err().0.amount, 2);
        assert_eq!(response.await.unwrap(), Ok(1));
        // tx 仍活着；不能依靠最后一个Sender被drop来完成关闭。
        assert_eq!(actor.await.unwrap(), 1);
        assert!(tx.is_closed());
    }

    #[tokio::test]
    async fn patterns_abandoned_reply_does_not_rollback_work() {
        let (tx, rx) = mpsc::channel(2);
        let (stop, shutdown) = oneshot::channel();
        let actor = tokio::spawn(counter_actor(rx, shutdown, 0));
        let (first, abandoned) = add(3);
        drop(abandoned);
        tx.send(first).await.unwrap();
        let (second, response) = add(4);
        tx.send(second).await.unwrap();
        assert_eq!(response.await.unwrap(), Ok(7));
        stop.send(()).unwrap();
        assert_eq!(actor.await.unwrap(), 7);
    }

    #[tokio::test]
    async fn patterns_business_error_preserves_state_and_actor_survives() {
        let (tx, rx) = mpsc::channel(1);
        let (stop, shutdown) = oneshot::channel();
        let actor = tokio::spawn(counter_actor(rx, shutdown, u64::MAX - 1));
        let (overflow, response) = add(2);
        tx.send(overflow).await.unwrap();
        assert_eq!(response.await.unwrap(), Err("计数溢出"));
        let (valid, response) = add(1);
        tx.send(valid).await.unwrap();
        assert_eq!(response.await.unwrap(), Ok(u64::MAX));
        stop.send(()).unwrap();
        assert_eq!(actor.await.unwrap(), u64::MAX);
    }
}
