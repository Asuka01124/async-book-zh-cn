// ANCHOR: primer-pending-song-dance
use crate::rustc_async::YieldOnce;
use std::cell::RefCell;

pub async fn song_and_dance(events: &RefCell<Vec<&'static str>>) {
    let learn_and_sing = async {
        events.borrow_mut().push("开始学歌");
        YieldOnce::new().await;
        events.borrow_mut().push("学会歌曲");
        events.borrow_mut().push("开始唱歌");
        YieldOnce::new().await;
        events.borrow_mut().push("唱歌结束");
    };
    let dance = async {
        events.borrow_mut().push("开始跳舞");
        YieldOnce::new().await;
        events.borrow_mut().push("继续跳舞");
        YieldOnce::new().await;
        events.borrow_mut().push("跳舞结束");
    };
    // 两个分支在同一任务中并发推进，无须独立 spawn。
    futures::join!(learn_and_sing, dance);
}
// ANCHOR_END: primer-pending-song-dance

#[cfg(test)]
mod tests {
    use super::*;
    use std::{future::Future, task::{Context, Poll}};
    #[test]
    fn primer_pending_branches_overlap_but_learning_precedes_singing() {
        let events = RefCell::new(Vec::new());
        let mut future = std::pin::pin!(song_and_dance(&events));
        let mut cx = Context::from_waker(std::task::Waker::noop());
        assert_eq!(future.as_mut().poll(&mut cx), Poll::Pending);
        {
            let first = events.borrow();
            assert_eq!(first.len(), 2);
            assert!(first.contains(&"开始学歌"));
            assert!(first.contains(&"开始跳舞"));
        }
        assert_eq!(future.as_mut().poll(&mut cx), Poll::Pending);
        {
            let middle = events.borrow();
            assert!(middle.contains(&"学会歌曲"));
            assert!(middle.contains(&"开始唱歌"));
            assert!(middle.contains(&"继续跳舞"));
            assert!(!middle.contains(&"唱歌结束"));
            assert!(!middle.contains(&"跳舞结束"));
        }
        assert_eq!(future.as_mut().poll(&mut cx), Poll::Ready(()));
        let log = events.borrow();
        let at = |name| log.iter().position(|event| *event == name).unwrap();
        assert_eq!(log.len(), 7);
        assert!(at("开始学歌") < at("学会歌曲"));
        assert!(at("学会歌曲") < at("开始唱歌"));
        assert!(at("开始唱歌") < at("唱歌结束"));
        assert!(at("开始跳舞") < at("学会歌曲"));
        assert!(at("继续跳舞") < at("唱歌结束"));
        assert!(at("继续跳舞") < at("跳舞结束"));
    }
}
