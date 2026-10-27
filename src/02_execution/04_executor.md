# 实践：构建一个执行器 {#applied-build-an-executor}

Rust 的 `async` 函数返回的 Future 在被轮询时才执行其函数体。一般 Future 需要被轮询才能推进其自身的计算，但其构造函数或它等待的独立线程可能已启动外部工作，例如上一节的计时器。驱动 Future 直至完成的一种方式是在 `async` 函数中对它使用 `.await`，但这只是把问题向上推了一层：谁来运行顶层 `async` 函数返回的 Future？答案是，我们需要一个 `Future` 执行器。

`Future` 执行器接收一组顶层 `Future`，并在它们可以继续推进时调用 `poll`，驱动它们直至完成。通常，执行器会先调用一次 `poll` 来启动 Future。当 `Future` 通过调用 `wake()` 表示自己可以继续推进时，执行器会将它们重新放入队列，再次调用 `poll`。如此反复，直到 Future 完成。

本节中，我们将编写自己的简单执行器，使它能够在单个执行线程上并发驱动多个顶层 Future 直至完成。它演示调度协议，不提供生产运行时的资源管理能力。

这个示例依赖 `futures` crate 提供的 `ArcWake` trait，它能让我们方便地构造 `Waker`。编辑 `Cargo.toml`，添加一个新依赖：

```toml
[package]
name = "timer_future"
version = "0.1.0"
authors = ["XYZ Author"]
edition = "2021"

[dependencies]
futures = "0.3"
```

计时器也需要能被导入：本仓库使用独立的 `timer_future` 库依赖（包名 `example_02_03_timer`，路径 `../02_03_timer`）。如果沿用上一节创建的 `timer_future` 项目，可以保留计时器在该包的 `src/lib.rs` 中，再添加 `src/main.rs`，同一个包内即可用 `use timer_future::TimerFuture` 导入。上面的清单只添加 `futures`，不会自动下载本书计时器。

接下来，在 `src/main.rs` 顶部添加以下导入项：

```rust,ignore
{{#include ../../examples/02_04_executor/src/lib.rs:imports}}
```

我们的执行器会通过通道发送待运行的任务。执行器从通道中取出事件并运行它们。当任务可以继续工作（即被唤醒）时，可以把自己重新放回通道，以便再次被轮询。

在这种设计中，执行器本身只需要任务通道的接收端。用户会获得发送端，以便生成新的 Future。任务是 Future 与调度状态的容器，因此我们会将它们存储为 Future 与发送端的组合，任务可以通过发送端将自己重新放入队列。

```rust,ignore
{{#include ../../examples/02_04_executor/src/lib.rs:executor_decl}}
```

我们再给 Spawner 添加一个方法，让生成新 Future 更方便。这个方法会接收一个 Future 类型，将它装箱，并创建一个包含该 Future 的新 `Arc<Task>`，然后把任务放入执行器队列。

```rust,ignore
{{#include ../../examples/02_04_executor/src/lib.rs:spawn_fn}}
```

要轮询 Future，我们需要创建一个 `Waker`。正如[任务唤醒一节]所述，调用 `wake` 后，`Waker` 会负责安排任务再次被轮询。请记住，`Waker` 标识需要再次轮询的任务，但不保证其结果已经就绪；下一次轮询仍可能返回 `Pending`。创建新 `Waker` 最简单的方式是为任务实现 `ArcWake` trait，再使用 `futures::task::waker` 将 `Arc<impl ArcWake>` 转成拥有所有权的 `Waker`，或者使用 `waker_ref(&arc)` 得到临时的 `WakerRef`，通过它借用 `Waker`。当前 `ArcWake` 并不提供 `.into_waker()` 方法。我们来为任务实现 `ArcWake`，这样它们就能转换为 `Waker` 并被唤醒：

```rust,ignore
{{#include ../../examples/02_04_executor/src/lib.rs:arcwake_for_task}}
```

从 `Arc<Task>` 创建 `Waker` 后，调用它的 `wake()` 会将一份 `Arc` 副本发送到任务通道。然后执行器需要从通道中取出任务并轮询它。我们来实现这一点：

```rust,ignore
{{#include ../../examples/02_04_executor/src/lib.rs:executor_run}}
```

这里的 `run` 是唯一轮询任务的消费者。它先短暂加锁取出 Future，释放锁后才调用 `poll`；若返回 `Pending`，再加锁放回。即使 Future 在本次 `poll` 返回前就唤醒自己，消息也会留在队列中，只有放回 Future 后下一次接收才会处理它。完成后的重复消息只看到 `None`，不会再次轮询已完成的 Future。Future 的执行和完成析构均不持有任务槽锁。这个顺序依赖单执行线程，不能直接扩成多消费者算法。

`ArcWake` 要求 `Send + Sync`，因为句柄可能由计时器线程调用；`BoxFuture` 要求内部 Future 为 `Send`，但不要求它本身为 `Sync`，所以 `Mutex` 在这里提供有意义的安全共享边界。生产执行器也可以使用锁；换成 `UnsafeCell` 需要另行证明独占轮询、同步与析构安全。

本例使用无界 `channel`：旧的有界 `sync_channel` 加 `try_send(...).expect(...)` 会在排队超过容量时 panic，不能可靠处理大量或重复唤醒。无界队列消除了这个固定容量错误，但没有背压或唤醒去重，内存仍可能持续增长。接收端关闭后的迟到唤醒被忽略；主动向已关闭执行器生成任务仍会 panic。

我们现在已经有了一个可以工作的教学执行器。我们甚至可以用它来运行 `async/.await` 代码和自定义 Future，例如之前编写的 `TimerFuture`：

```rust,edition2018,ignore
{{#include ../../examples/02_04_executor/src/lib.rs:main}}
```

[任务唤醒一节]: ./03_wakeups.md

`recv` 在队列暂时为空但仍有发送端时阻塞等待；只有所有发送端释放且消息排空后，`run` 才返回。丢弃 `Spawner` 只关闭这个生成入口：任务及外部保留的 Waker 也持有发送端，所以即使 Future 已完成，保留其 Waker 也可能让 `run` 继续等待。示例会实际等待约两秒，再打印 `done!`，不是一遇空队列就退出。

这个实现没有公平性预算、任务取消、panic 隔离或完整关闭协议；不返回的 `poll` 会阻塞整个执行器，丢失唤醒的任务可能无法继续。Pending Future 若保存自己的任务 Waker，还可能形成 `Task → Future → Waker → Task` 的引用环。不要把这一页当成完整运行时；更多状态与关闭设计见[异步运行时](../part-guide/runtimes.md)。

本仓库保留两秒计时器集成测试，并新增确定性回归：Future 在一次 `poll` 中连续自唤醒 10,001 次后返回 `Pending`，再次轮询完成且所有重复消息不再轮询它；另验证接收端关闭后的迟到唤醒不 panic。这些测试不代表吞吐性能或任意多线程调度正确性证明。通道行为见标准库 [`channel`](https://doc.rust-lang.org/std/sync/mpsc/fn.channel.html) 和 [`Receiver::recv`](https://doc.rust-lang.org/std/sync/mpsc/struct.Receiver.html#method.recv)，Waker 构造见 [`ArcWake`](https://docs.rs/futures/latest/futures/task/trait.ArcWake.html)。
