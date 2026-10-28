#![cfg(test)]

struct Book;
struct Music;
// 教学占位：这里只立即返回值，不执行 I/O 或挂起。
async fn get_book() -> Book { Book }
async fn get_music() -> Music { Music }

mod naiive {
use super::*;
// ANCHOR: naiive
async fn get_book_and_music() -> (Book, Music) {
    let book = get_book().await;
    let music = get_music().await;
    (book, music)
}
// ANCHOR_END: naiive
}

mod other_langs {
use super::*;
// ANCHOR: other_langs
// 错误做法——不能由此实现并发
async fn get_book_and_music() -> (Book, Music) {
    let book_future = get_book();
    let music_future = get_music();
    (book_future.await, music_future.await)
}
// ANCHOR_END: other_langs
}

mod join {
use super::*;
// ANCHOR: join
use futures::join;

async fn get_book_and_music() -> (Book, Music) {
    let book_fut = get_book();
    let music_fut = get_music();
    join!(book_fut, music_fut)
}
// ANCHOR_END: join
}

mod try_join {
use super::{Book, Music};
// ANCHOR: try_join
use futures::try_join;

async fn get_book() -> Result<Book, String> { /* 教学占位 */ Ok(Book) }
async fn get_music() -> Result<Music, String> { /* 教学占位 */ Ok(Music) }

async fn get_book_and_music() -> Result<(Book, Music), String> {
    let book_fut = get_book();
    let music_fut = get_music();
    try_join!(book_fut, music_fut)
}
// ANCHOR_END: try_join
}

mod mismatched_err {
use super::{Book, Music};
// ANCHOR: try_join_map_err
use futures::{
    future::TryFutureExt,
    try_join,
};

async fn get_book() -> Result<Book, ()> { /* 教学占位 */ Ok(Book) }
async fn get_music() -> Result<Music, String> { /* 教学占位 */ Ok(Music) }

async fn get_book_and_music() -> Result<(Book, Music), String> {
    let book_fut = get_book().map_err(|()| "无法获取图书".to_string());
    let music_fut = get_music();
    try_join!(book_fut, music_fut)
}
// ANCHOR_END: try_join_map_err
}

#[test]
fn try_join_drops_owned_but_preserves_borrowed_pending_future() {
    use std::{cell::Cell, future::Future, pin::Pin, rc::Rc, task::{Context, Poll}};

    struct PendingProbe {
        polled: Rc<Cell<bool>>,
        dropped: Rc<Cell<bool>>,
    }
    impl Future for PendingProbe {
        type Output = Result<(), &'static str>;
        fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
            self.polled.set(true);
            Poll::Pending
        }
    }
    impl Drop for PendingProbe {
        fn drop(&mut self) { self.dropped.set(true); }
    }

    for borrowed in [false, true] {
        let polled = Rc::new(Cell::new(false));
        let dropped = Rc::new(Cell::new(false));
        let probe = PendingProbe { polled: polled.clone(), dropped: dropped.clone() };
        let mut probe = Box::pin(probe);
        // 先明确轮询一次，避免依赖 try_join! 内部的分支轮询次序。
        let mut cx = Context::from_waker(futures::task::noop_waker_ref());
        assert!(probe.as_mut().poll(&mut cx).is_pending());
        assert!(polled.get());
        if borrowed {
            let result = futures::executor::block_on(async {
                futures::try_join!(probe.as_mut(), async { Err::<(), _>("失败") })
            });
            assert_eq!(result, Err("失败"));
            assert!(!dropped.get());
            // 宏只释放借用，调用方仍拥有该 Future，且可继续轮询。
            assert!(probe.as_mut().poll(&mut cx).is_pending());
            drop(probe);
        } else {
            let result = futures::executor::block_on(async {
                futures::try_join!(probe, async { Err::<(), _>("失败") })
            });
            assert_eq!(result, Err("失败"));
        }
        assert!(dropped.get());
    }
}
