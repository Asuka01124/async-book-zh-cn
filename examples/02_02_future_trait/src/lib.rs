// ANCHOR: simple_future
trait SimpleFuture {
    type Output;
    fn poll(&mut self, wake: fn()) -> Poll<Self::Output>;
}

enum Poll<T> {
    Ready(T),
    Pending,
}
// ANCHOR_END: simple_future

// 教学占位：没有真实套接字、就绪检测或回调注册。
struct Socket;
impl Socket {
    fn has_data_to_read(&self) -> bool {
        // 占位检查：恒为真，没有检查实际 I/O 状态。
        true
    }
    fn read_buf(&self) -> Vec<u8> {
        // 占位读取：返回空向量，没有执行 I/O。
        vec![]
    }
    fn set_readable_callback(&self, _wake: fn()) {
        // 此处未实现注册。真实实现需要就绪通知机制及防丢失唤醒协议，
        // 例如基于 `epoll` 的事件循环。
    }
}

// ANCHOR: socket_read
pub struct SocketRead<'a> {
    socket: &'a Socket,
}

impl SimpleFuture for SocketRead<'_> {
    type Output = Vec<u8>;

    fn poll(&mut self, wake: fn()) -> Poll<Self::Output> {
        if self.socket.has_data_to_read() {
            // 套接字中有数据——将其读入缓冲区并返回。
            Poll::Ready(self.socket.read_buf())
        } else {
            // 套接字中暂时还没有数据。
            //
            // 安排在数据就绪时调用 `wake`。
            // 回调应通知使用者再次尝试 `poll`；唤醒不保证读取一定成功。
            self.socket.set_readable_callback(wake);
            Poll::Pending
        }
    }
}
// ANCHOR_END: socket_read

// ANCHOR: join
/// 一个并发运行另外两个 Future 直至完成的 SimpleFuture。
///
/// 并发之所以能够实现，是因为对每个 Future 的 `poll` 调用可以交错进行，
/// 从而让每个 Future 按自己的节奏推进。
pub struct Join<FutureA, FutureB> {
    // 每个字段都可能包含一个需要运行直至完成的 Future。
    // 如果 Future 已经完成，就将对应字段设为 `None`。
    // 这样可以避免在 Future 完成后继续轮询它，否则就会违反 `Future` trait 的约定。
    a: Option<FutureA>,
    b: Option<FutureB>,
}

impl<FutureA, FutureB> SimpleFuture for Join<FutureA, FutureB>
where
    FutureA: SimpleFuture<Output = ()>,
    FutureB: SimpleFuture<Output = ()>,
{
    type Output = ();
    fn poll(&mut self, wake: fn()) -> Poll<Self::Output> {
        // 尝试完成 Future `a`。
        if let Some(a) = &mut self.a {
            if let Poll::Ready(()) = a.poll(wake) {
                self.a.take();
            }
        }

        // 尝试完成 Future `b`。
        if let Some(b) = &mut self.b {
            if let Poll::Ready(()) = b.poll(wake) {
                self.b.take();
            }
        }

        if self.a.is_none() && self.b.is_none() {
            // 两个 Future 都已完成——可以成功返回。
            Poll::Ready(())
        } else {
            // 一个或两个 Future 返回了 `Poll::Pending`，仍有工作要做。
            // 它们会在可以继续推进时调用 `wake()`。
            Poll::Pending
        }
    }
}
// ANCHOR_END: join

// ANCHOR: and_then
/// 一个依次运行两个 Future 直至完成的 SimpleFuture。
//
// 注意：为了简化示例，`AndThenFut` 假定第一个和第二个 Future 在创建时都已可用。
// 实际的 `AndThen` 组合器允许根据第一个 Future 的输出创建第二个 Future，
// 例如 `get_breakfast.and_then(|food| eat(food))`。
pub struct AndThenFut<FutureA, FutureB> {
    first: Option<FutureA>,
    second: FutureB,
}

impl<FutureA, FutureB> SimpleFuture for AndThenFut<FutureA, FutureB>
where
    FutureA: SimpleFuture<Output = ()>,
    FutureB: SimpleFuture<Output = ()>,
{
    type Output = ();
    fn poll(&mut self, wake: fn()) -> Poll<Self::Output> {
        if let Some(first) = &mut self.first {
            match first.poll(wake) {
                // 第一个 Future 已完成——移除它并开始执行第二个！
                Poll::Ready(()) => self.first.take(),
                // 第一个 Future 暂时还无法完成。
                // 注意，这里的 `return` 语句会中断 `poll` 函数的执行流程。
                Poll::Pending => return Poll::Pending,
            };
        }
        // 第一个 Future 现在已经完成，接下来尝试完成第二个。
        self.second.poll(wake)
    }
}
// ANCHOR_END: and_then

mod real_future {
use std::{
    future::Future as RealFuture,
    pin::Pin,
    task::{Context, Poll},
};

// ANCHOR: real_future
trait Future {
    type Output;
    fn poll(
        // 注意，这里从 `&mut self` 改成了 `Pin<&mut Self>`：
        self: Pin<&mut Self>,
        // 同时也从 `wake: fn()` 改成了 `cx: &mut Context<'_>`：
        cx: &mut Context<'_>,
    ) -> Poll<Self::Output>;
}
// ANCHOR_END: real_future

// 确保 `Future` 与 `RealFuture` 相匹配：
impl<O> Future for dyn RealFuture<Output = O> {
    type Output = O;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        RealFuture::poll(self, cx)
    }
}
}
