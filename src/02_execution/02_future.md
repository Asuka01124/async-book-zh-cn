# `Future` trait {#the-future-trait}

`Future` trait 是 Rust 异步编程的核心。`Future` 表示一种异步计算，它可以产生一个值（当然，这个值也可能为空，例如 `()`）。`Future` trait 的一个*简化版*大致如下：

```rust
{{#include ../../examples/02_02_future_trait/src/lib.rs:simple_future}}
```

调用 `poll` 函数可以推进 Future；它会尝试推进计算，并应尽快返回，不能为了等待结果而阻塞线程。如果 Future 已完成，就会返回 `Poll::Ready(result)`。如果 Future 暂时还无法完成，就会返回 `Poll::Pending`，并安排在 Future 可以继续推进时调用 `wake()`。调用 `wake()` 后，负责驱动该 Future 的执行器会再次调用 `poll`，让 Future 继续推进。

如果没有 `wake()`，执行器就无从得知某个 Future 何时可以继续推进，只能不断轮询每一个 Future。有了 `wake()`，执行器就能收到再次轮询相关任务的通知。唤醒只是提示应重试，不保证下一次轮询返回 `Ready`，也不保证立即执行；多次唤醒可以合并成一次轮询。组合 Future 通常共享外层任务的唤醒句柄，并非每个子 Future 都是一个独立任务。

例如，假设我们想从一个套接字读取数据，但数据可能已经就绪，也可能还没有。如果有数据，我们就可以读取数据并返回 `Poll::Ready(data)`；如果数据尚未就绪，我们的 Future 就会返回 `Pending`，暂时无法继续推进；执行器线程仍可处理其他工作。没有数据可读时，我们必须注册一个 `wake` 回调，让它在套接字数据就绪时被调用。这样执行器就会知道我们的 Future 可以继续推进。一个简单的 `SocketRead` Future 大致如下：

```rust,ignore
{{#include ../../examples/02_02_future_trait/src/lib.rs:socket_read}}
```

上面的 `SocketRead` 是控制流程示意，并非可用的网络实现：所引文件中的 `Socket` 是占位类型，可读检查恒为真、读取返回空向量、回调注册为空操作。因此它没有演示真实等待、唤醒或套接字错误。真实实现还必须处理“检查后、注册前数据就绪”的竞争，例如协调事件状态与注册，并在注册后重新检查状态，避免丢失唤醒；还须处理非阻塞读取可能暂时无数据、EOF 和错误。不能把这里的两步伪代码直接当成可靠的注册协议。

这种 `Future` 模型可以组合多个异步操作，而不要求组合步骤本身为每个 Future 单独进行堆分配。可以通过不分配内存的状态机，同时运行多个 Future 或将它们串联起来，例如：

```rust,ignore
{{#include ../../examples/02_02_future_trait/src/lib.rs:join}}
```

这里的两个 Future 保存在结构体字段中，在同一次外层 `poll` 中依次尝试推进；跨多次轮询可以交错进行，但这不表示它们在不同线程上并行执行。组合器本身无需单独为每个子 Future 分配堆内存，子操作和执行器仍可能分配内存。同样，也可以依次运行多个 Future，让它们一个接一个地执行，例如：

```rust,ignore
{{#include ../../examples/02_02_future_trait/src/lib.rs:and_then}}
```

`AndThenFut` 预先持有两个 Future，只有第一个完成后才轮询第二个；它不是根据第一个结果创建第二个 Future 的完整 `and_then` 实现。`Join` 在子 Future 完成后清空对应字段，避免再次轮询已经完成的子 Future。标准协议不允许调用方依赖完成后再次轮询的行为；它可能 panic 等，但安全的 `poll` 调用不能导致未定义行为。

这些示例展示了如何使用 `Future` trait 表达异步控制流，减少额外堆分配和层层嵌套的回调。基本控制流介绍到这里，接下来我们来看看真正的 `Future` trait，以及它与简化版本的区别。

```rust,ignore
{{#include ../../examples/02_02_future_trait/src/lib.rs:real_future}}
```

你首先会注意到，`self` 的类型不再是 `&mut Self`，而变成了 `Pin<&mut Self>`。稍后我们会详细讨论固定（pinning）；现在只需知道，`Pin<&mut Self>` 允许地址敏感的 Future 依赖固定后的地址稳定性。对于不实现 `Unpin` 的已固定值，必须维持固定契约直到原址析构；实现 `Unpin` 的类型不受这种移动限制，而 Future 在被固定前仍可移动。地址敏感状态可以包含字段间指针，例如 `struct MyFut { a: i32, ptr_to_a: *const i32 }`，但仅声明裸指针字段并不会自动建立或验证自引用。固定机制使编译器能够安全支持跨挂起点的内部借用；真正实现上述手写自引用类型还需要额外的不变量。详情见[固定](../part-reference/pinning.md)。

其次，`wake: fn()` 变成了 `&mut Context<'_>`。在 `SimpleFuture` 中，我们调用函数指针（`fn()`），通知 Future 执行器应该轮询相应的 Future。但 `fn()` 只是一个函数指针，不能捕获每次注册所需的任务环境；即便函数可以访问全局状态，`fn()` 这个值本身也不携带特定任务的上下文。

在真实应用中，像 Web 服务器这样的复杂程序可能有成千上万个不同的连接，需要分别管理它们的唤醒操作。`Context` 类型通过提供对 `Waker` 值的访问解决了这个问题；`Waker` 可以用来唤醒特定任务。等待中的 Future 必须安排后续通知，并在重新轮询时使用最新 `Context` 对应的唤醒句柄，不能一直通知已失效的旧任务。

上述真实协议以标准库 [`Future`](https://doc.rust-lang.org/std/future/trait.Future.html)、[`Waker`](https://doc.rust-lang.org/std/task/struct.Waker.html) 和 [`pin`](https://doc.rust-lang.org/std/pin/index.html) 文档为准。这里的 `SimpleFuture` 及其组合器仅解释控制流；迁移到标准 `Future` 时还需正确处理 `Pin` 投影，不能直接沿用 `&mut Self` 中移动字段的写法。
