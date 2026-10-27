#![cfg(test)]

// ANCHOR: imports
use futures::{
    future::{BoxFuture, FutureExt},
    task::{waker_ref, ArcWake},
};
use std::{
    future::Future,
    sync::mpsc::{channel, Receiver, Sender},
    sync::{Arc, Mutex},
    task::Context,
    time::Duration,
};
// 上一节中编写的计时器：
use timer_future::TimerFuture;
// ANCHOR_END: imports

// ANCHOR: executor_decl
/// 从通道接收任务并运行它们的任务执行器。
struct Executor {
    ready_queue: Receiver<Arc<Task>>,
}

/// `Spawner` 将新的 Future 放入任务通道。
#[derive(Clone)]
struct Spawner {
    task_sender: Sender<Arc<Task>>,
}

/// 一个能够重新调度自身、以便由 `Executor` 轮询的 Future。
struct Task {
    /// 尚在进行中、需要继续驱动直至完成的 Future。
    ///
    /// Task 的 Waker 可以跨线程使用；Mutex 为非 Sync 的 BoxFuture 提供安全共享边界。
    /// 本例只允许一个执行线程轮询，且在调用 poll/析构 Future 前释放锁。
    /// 生产实现也可以使用 Mutex；UnsafeCell 需要另行证明同步与独占访问不变量。
    future: Mutex<Option<BoxFuture<'static, ()>>>,

    /// 用于将任务本身重新放回任务队列的句柄。
    task_sender: Sender<Arc<Task>>,
}

fn new_executor_and_spawner() -> (Executor, Spawner) {
    // 无界队列避免唤醒时因固定容量耗尽而 panic，但没有背压或唤醒去重。
    let (task_sender, ready_queue) = channel();
    (Executor { ready_queue }, Spawner { task_sender })
}
// ANCHOR_END: executor_decl

// ANCHOR: spawn_fn
impl Spawner {
    fn spawn(&self, future: impl Future<Output = ()> + 'static + Send) {
        let future = future.boxed();
        let task = Arc::new(Task {
            future: Mutex::new(Some(future)),
            task_sender: self.task_sender.clone(),
        });
        self.task_sender.send(task).expect("执行器已关闭");
    }
}
// ANCHOR_END: spawn_fn

// ANCHOR: arcwake_for_task
impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        // 通过将此任务重新发送到任务通道来实现 `wake`，
        // 这样执行器就会再次轮询它。
        let cloned = arc_self.clone();
        // 接收端已关闭时不再有执行器可通知，迟到的唤醒直接丢弃。
        let _ = arc_self.task_sender.send(cloned);
    }
}
// ANCHOR_END: arcwake_for_task

// ANCHOR: executor_run
impl Executor {
    fn run(&self) {
        while let Ok(task) = self.ready_queue.recv() {
            // 取出 Future；如果它尚未完成（仍为 Some），
            // 就轮询它，尝试将其驱动至完成。
            let future = { task.future.lock().unwrap().take() };
            if let Some(mut future) = future {
                // 根据任务本身创建一个 `Waker`
                let waker = waker_ref(&task);
                let context = &mut Context::from_waker(&waker);
                // `BoxFuture<T>` 是 `Pin<Box<dyn Future<Output = T> + Send + 'static>>` 的类型别名。
                // 调用 `Pin::as_mut` 方法，可以从中获得 `Pin<&mut dyn Future + Send + 'static>`。
                if future.as_mut().poll(context).is_pending() {
                    // Future 尚未处理完毕，因此将它放回任务中，
                    // 以便之后再次运行。
                    *task.future.lock().unwrap() = Some(future);
                }
            }
        }
    }
}
// ANCHOR_END: executor_run

// ANCHOR: main
fn main() {
    let (executor, spawner) = new_executor_and_spawner();

    // 生成一个任务，在等待计时器之前和之后分别打印内容。
    spawner.spawn(async {
        println!("howdy!");
        // 等待计时器 Future 在两秒后完成。
        TimerFuture::new(Duration::new(2, 0)).await;
        println!("done!");
    });

    // 丢弃这个生成入口；任务自身以及被保存的 Waker 仍可能持有发送端。
    drop(spawner);

    // 运行到所有发送端释放且队列排空；暂时无任务时 recv 会阻塞等待。
    // 执行器会打印 "howdy!"，暂停，然后打印 "done!"。
    executor.run();
}
// ANCHOR_END: main

#[test]
fn run_main() {
    main()
}

#[test]
fn self_wakes_survive_pending_and_completed_duplicates_are_ignored() {
    use std::{pin::Pin, sync::atomic::{AtomicUsize, Ordering}, task::Poll};
    struct SelfWake(Arc<AtomicUsize>);
    impl Future for SelfWake {
        type Output = ();
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
            match self.0.fetch_add(1, Ordering::SeqCst) {
                0 => {
                    // 超过旧队列上限，且全部唤醒发生在本次 poll 返回之前。
                    for _ in 0..10_001 { cx.waker().wake_by_ref(); }
                    Poll::Pending
                }
                1 => {
                    cx.waker().wake_by_ref();
                    Poll::Ready(())
                }
                _ => panic!("已完成 Future 不应再次轮询"),
            }
        }
    }
    let (executor, spawner) = new_executor_and_spawner();
    let polls = Arc::new(AtomicUsize::new(0));
    spawner.spawn(SelfWake(polls.clone()));
    drop(spawner);
    executor.run();
    assert_eq!(polls.load(Ordering::SeqCst), 2);
}

#[test]
fn late_wake_after_receiver_closed_is_harmless() {
    let (executor, spawner) = new_executor_and_spawner();
    spawner.spawn(async {});
    let task = executor.ready_queue.recv().unwrap();
    let waker = futures::task::waker(task.clone());
    drop(task);
    drop(spawner);
    drop(executor);
    waker.wake_by_ref();
    waker.wake();
}
