# 使用 `Waker` 唤醒任务 {#task-wakeups-with-waker}

Future 第一次被调用 `poll` 时通常还无法完成。发生这种情况时，Future 需要确保自己在准备好继续推进后再次被轮询。这是通过 `Waker` 类型实现的。

每次轮询 Future 时，它都是作为某个“任务”的一部分被轮询的。任务通常是执行器调度的单位，持有顶层 Future 和调度状态；其内部组合的子 Future 也在同一任务上下文中被轮询，普通 `.await` 不自动创建任务。

`Waker` 提供了 `wake()` 方法，可以用来通知执行器唤醒与之关联的任务。调用 `wake()` 后，执行器便收到再次轮询关联任务的通知。唤醒不保证计算完成或立即运行；执行器可合并多次唤醒，下一次轮询仍可能返回 `Pending`。

`Waker` 实现了 `Clone`，因此可以克隆并保存句柄；它不是 `Copy`，还实现了 `Send` 和 `Sync`，所以等待线程可以安全持有并调用它。

接下来，我们尝试使用 `Waker` 实现一个简单的计时器 Future。

## 实践：构建计时器 {#applied-build-a-timer}

为了简化示例，我们会在创建计时器时启动一个新线程，让它休眠指定时长，并在时间到达后通知计时器 Future。这是每个计时器一个操作系统线程的教学实现，不适合大量计时器；实际运行时通常集中管理计时器。这里的 `TimerFuture::new` 是同步构造函数，调用时就启动线程，即使随后从未轮询 Future，线程仍会继续运行。因此“Future 是惰性的”并不代表所有构造函数和外部工作都延迟到首次轮询。

首先，使用 `cargo new --lib timer_future` 创建一个新项目，并将所需的导入项添加到 `src/lib.rs`：

```rust
{{#include ../../examples/02_03_timer/src/lib.rs:imports}}
```

先定义 Future 类型本身。我们需要一种方式，让线程通知 Future 计时器已经到期、可以完成了。我们将使用共享的 `Arc<Mutex<..>>` 值在线程和 Future 之间传递信息。

```rust,ignore
{{#include ../../examples/02_03_timer/src/lib.rs:timer_decl}}
```

现在来编写 `Future` 实现！

```rust,ignore
{{#include ../../examples/02_03_timer/src/lib.rs:future_for_timer}}
```

如果线程已将 `shared_state.completed = true`，计时器便返回 `Ready(())`。否则先在锁外克隆当前任务的 `Waker`，再加锁复查完成状态并注册句柄；替换下来的旧句柄和未使用的新句柄都在锁外丢弃。`Waker` 的克隆、析构和唤醒可能执行自定义回调；在共享状态锁内调用它们可能重入并死锁。

每次返回 `Pending` 前，都需要保留最新 `Context` 对应的 `Waker`。同一个 Future 后续可能在另一个任务上下文中被轮询；这不要求把已经固定的 Future 移出其地址，例如可以转移拥有它的固定指针。这里为简单起见每次更新句柄；更精细的实现可以用 `will_wake` 避免不必要的克隆，但不能一直保留旧任务的句柄。

最后，我们还需要提供用于实际创建计时器并启动线程的 API：

```rust,ignore
{{#include ../../examples/02_03_timer/src/lib.rs:timer_new}}
```

完成线程在同一把锁内设置 `completed` 并取出句柄，然后释放锁再唤醒。这样，完成发生在注册前时，第二次检查会直接观察到完成；发生在注册后时，线程就会取到最新句柄。完成状态保存在共享内存中，即使首次轮询前已经到期、没有任何句柄可唤醒，首次轮询仍会返回 `Ready`。唤醒是通知，`completed` 才是判断结果的依据。

这个示例仍有明确限制：丢弃 Future 不会取消睡眠线程，线程会保留共享状态直到结束；没有线程回收、取消、线程创建失败处理或锁中毒恢复。真实线程测试只验证简单到期路径；另外两个确定性测试使用自定义 Waker 检查最新句柄、锁外 clone/drop/wake，以及完成落在注册间隙和首次轮询之前的路径，不把它们当任意线程交错的形式化证明。

协议依据见标准库 [`Waker`](https://doc.rust-lang.org/std/task/struct.Waker.html)、[`RawWakerVTable`](https://doc.rust-lang.org/std/task/struct.RawWakerVTable.html) 和 [`Future`](https://doc.rust-lang.org/std/future/trait.Future.html)。构建一个简单的计时器 Future 所需的内容就这些，接下来还需要[执行器](04_executor.md)来驱动它。
