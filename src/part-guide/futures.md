<a id="futures"></a>
# Future 详解

前面各章已多次使用 Future；它是 Rust 异步编程的核心。本章进一步解释 Future 是什么、怎样被驱动，以及直接操作 Future 的常用库。大多数应用继续使用 `async` 与 `.await` 即可；理解下面的协议有助于阅读 trait 约束、取消行为和编译错误。

<a id="the-future-and-intofuture-traits"></a>
## Future 与 IntoFuture trait

### Future 表示一次计算

标准库的 [`Future`](https://doc.rust-lang.org/std/future/trait.Future.html) 有一个关联类型 `Output`，表示最终结果。它可以是普通值、`Result<T, E>`，也可以是 `()`；失败并不是 Future 协议的独立状态，而是输出类型的选择。`async fn get() -> T` 返回的就是某个 `Future<Output = T>`。

每个异步块通常有自己的匿名具体类型，即使输出相同，也不代表类型相同。`impl Future<Output = T>` 在不暴露具体类型时保留静态分派；泛型参数 `F: Future` 可以接收不同实现。只有需要异构集合、动态接口或某些递归布局时，才考虑类型擦除和堆分配。

`Future` 本身不要求 `Send`、`'static` 或 `Unpin`；这些通常来自运行它的方式。借用局部字符串的 Future 可以在当前作用域里 `.await`，却未必能交给要求 `Send + 'static` 的 `tokio::spawn`。Future 类型也不强制绑定 Tokio：只是内部调用的计时器、IO 或任务 API 可能要求特定运行时。

### IntoFuture 与可等待构建器

[IntoFuture](https://doc.rust-lang.org/std/future/trait.IntoFuture.html)把一个值转成真正的 Future，`.await` 会先调用这次转换。所有 `Future` 已通过 blanket implementation 实现 `IntoFuture`；普通 Future 不必再手写一份。

仅实现 `IntoFuture` 的类型可先配置参数，再消费自身执行转换。下面用已就绪 Future 演示这个接口；真实网络构建器可以在转换后才开始异步步骤。

```rust
{{#include ../../examples/guide-tests/src/future_details.rs:builder}}
```

这样调用方可以 `AddBuilder::new(20).extra(22).await`，构建阶段与执行阶段分开。代价是值看起来可等待，却不一定满足 `F: Future`；把构建器传给只接受 `Future` 的泛型函数时，须显式 `.into_future()`。接收 `F: IntoFuture` 的库接口更宽，但具体组合器是否支持要查看版本。Tokio 1.54.1 的 `join!`、`select!` 已支持该转换，前面的[并发组合章](concurrency-primitives.md)有对应测试，不应沿用“所有宏只接受 Future”的旧结论。

### 装箱与动态分派

单独的 `Box<dyn Future<Output = T>>` 通常还不够：动态对象没有一般的 `Unpin` 保证，需要固定后才能轮询。常见表示是 `Pin<Box<dyn Future<Output = T> + Send + 'a>>`，futures 提供别名 `BoxFuture<'a, T>`；不需要 `Send` 时可用 `LocalBoxFuture` 和 `.boxed_local()`。

```rust
{{#include ../../examples/guide-tests/src/future_details.rs:boxed}}
```

这个 Future 借用 `text`，因此生命周期是 `'a`，不是自动 `'static`。`Box` 提供间接层，`Pin` 提供不随意移动被固定值的约束，两者职责不同。早期异步 trait 和递归代码常需要装箱；现在原生异步函数、原生异步 trait 方法和适当的 `impl Future` 可避免很多分配。递归异步调用仍需某处引入间接层，例如 `Box::pin`，但不意味着整个接口必须动态分派。更多见[固定](../part-reference/pinning.md)。

<a id="polling"></a>
## 轮询

`Future::poll` 的核心签名是 `fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>`。`Poll::Ready(value)` 表示结果完成；`Poll::Pending` 表示现在不能完成。执行器或上层组合器调用 `poll`，应用通常通过 `.await` 间接使用它。

返回 `Pending` 前，实现必须安排当将来可以继续时唤醒当前任务，通常通过保存或注册 `cx.waker()`。`wake` 请求执行器继续推进任务，并不等于当前调用者直接执行 Future；不应依赖具体轮询线程或时机。唤醒可合并，也可能有额外轮询；实现必须在每次被轮询时重新检查实际条件。后续轮询可能传入新的 Waker，保存注册状态时要避免一直唤醒旧任务。

下面的教学 Future 主动让出一次，下一次轮询就完成。它没有外部 IO，所以自己安排唤醒；真实 socket 等待就绪应登记 IO 通知，而不能无条件自唤醒形成忙循环。

```rust
{{#include ../../examples/guide-tests/src/future_details.rs:manual-future}}
```

`poll` 应快速返回，不进行阻塞等待，也不长时间占住 CPU。`.await` 轮询子 Future，若子 Future 尚未完成，外层任务也返回等待；若它立刻就绪，则继续执行，不必真的让出线程。这解释了为什么大量“立即就绪”的 `.await` 仍可能占用线程，也解释了为什么 `async` 不等于自动并行。

`Ready` 是一次计算的终点。一般 Future 完成后不应再次轮询；具体实现可以 panic、永远等待或表现出其他非有用行为，但安全实现仍不能因此造成未定义行为。不要把某些具体 Future 支持重复轮询当作通用保证，也不要通过重复 `.await` 同一个普通 Future 来重复执行操作。要重试，就重新构造操作，并考虑先前尝试已经产生的副作用。

丢弃未完成 Future 会销毁它拥有的状态，后面的代码不再执行；这通常称为取消。它不会撤销外部副作用，也不一定停止一个已生成任务，详见[取消安全](../part-reference/cancellation.md)和[析构清理](dtors.md)。`Pin` 不阻止正常销毁，也不使计算必然完成。

<a id="fusing"></a>
### 融合（Fusing）

有些循环每次迭代都轮询一组 Future，必须记住哪些已完成。[`FutureExt::fuse`](https://docs.rs/futures/latest/futures/future/trait.FutureExt.html#method.fuse)在外层保存终止状态，第一次完成后不再轮询原 Future，之后返回 `Pending`；`FusedFuture::is_terminated()` 用来识别该状态。它不是缓存结果并反复返回的容器。

futures 的 `select!` 利用终止状态跳过已结束分支，并可通过 `complete` 分支处理全部结束；Tokio `select!` 没有相同的融合要求，可用外部状态和 guard 禁用已完成分支。两套宏的约束和语法不能混用。Future 的融合结束后等待，与融合 Stream 的结束后继续返回 `None` 也不同。[Fuse 文档](https://docs.rs/futures/latest/futures/future/struct.Fuse.html)与并发组合章展示了循环用法。

<a id="futures-rs-crate"></a>
## futures-rs 库

[futures-rs](https://docs.rs/futures/latest/futures/)在标准库 Future 稳定之前就参与 Rust 异步生态建设。今天 0.3 系列基于标准库 `Future`，提供组合器、通道、流、异步 IO trait、固定/装箱辅助和执行器。它不是另一套与标准库不兼容的 Future，也不要求每个 Tokio 应用都依赖它。

### 方便的 Future 与组合器

`std::future::ready(value)` 立即完成，`pending::<T>()` 一直等待。futures 还提供 `future::ok`、`err` 等返回 `Result` 的辅助函数，以及把轮询闭包转成 Future 的 `poll_fn`（标准库也有）。它们适合组合、测试和适配接口；`pending` 不产生唤醒，因为它本来就不会前进。

[`TryFuture`](https://docs.rs/futures/latest/futures/future/trait.TryFuture.html)把 `Future<Output = Result<T, E>>` 的成功值和错误类型命名为 `Ok`、`Error`，通过现有 Future 自动实现，无需自己实现这个 sealed trait。`TryFutureExt` 提供 `map_ok`、`map_err`、`and_then` 等操作，只对相应分支应用转换：

```rust
{{#include ../../examples/guide-tests/src/future_details.rs:combinators}}
```

[`FutureExt`](https://docs.rs/futures/latest/futures/future/trait.FutureExt.html)则为普通 Future 提供 `map`（同步转换输出）、`then`（返回下一步 Future）、`inspect`、`boxed`、`fuse` 等方法。普通 Future 的输出若是 `Result`，`map` 接收整个 `Result`，与只处理 `Ok` 的 `map_ok` 不同。转换通常仍是惰性的，必须等待或交给执行器驱动；方法链不会自动创建任务。

`now_or_never()` 会轮询一次，不能立即完成就消费并丢弃 Future；它不是“查看一下但保留状态”。`catch_unwind()` 可以捕获轮询时展开的 panic，但不能捕获 `panic=abort`，也不会恢复被破坏的业务不变量。复杂控制流里 `async`/`.await` 和 `?` 往往更直观，不需要为使用组合器而改写所有代码。

### 执行器、IO 与 Tokio 的关系

[`futures::executor`](https://docs.rs/futures/latest/futures/executor/index.html)提供 `block_on`、本地任务池以及可选线程池。对于不依赖外部驱动器的 Future，`futures::executor::block_on` 可以独立运行。它不等于一个含 Tokio IO、时间和阻塞池的完整 Tokio 运行时，不能用它自动满足 `tokio::time::sleep` 或 Tokio socket 的上下文要求。

`pin_mut!`、`BoxFuture`、`ArcWake` 等辅助适合实现执行器或底层适配器；普通应用通常不需要手写 Waker。运行时如何将操作系统事件转成唤醒，见旧版[任务唤醒章](../02_execution/03_wakeups.md)和[执行器示例](../02_execution/04_executor.md)。

futures 的 `join!`、`select!`、`FuturesUnordered` 与 Tokio 的组合工具有重叠，但返回类型、固定要求、取消和任务拥有关系可能不同。其 `futures::io::AsyncRead/AsyncWrite` 也不是 Tokio 的同名 trait，接入时须使用对应生态或显式兼容适配器。流相关工具将在[异步迭代器章](streams.md)进一步讨论。

<a id="futures-concurrency-crate"></a>
## futures-concurrency 库

[`futures-concurrency`](https://docs.rs/futures-concurrency/latest/futures_concurrency/)提供以 trait 方法组合元组、数组、向量及流的接口。和上一章的任务管理器不同，这类组合通常在当前任务内驱动子 Future，可以保留对栈上数据的借用：

```rust
{{#include ../../examples/guide-tests/src/future_details.rs:concurrent-library}}
```

这里两个分支输出类型不同，`.join().await` 返回对应元组；没有生成需要 `'static` 的独立任务。数组和向量则要求其中元素具有兼容类型，异构动态集合可结合装箱。加入依赖 `futures-concurrency = "7.7.1"`，再导入 `prelude::*` 即可使用这类扩展方法。

`.join()` 等待所有输出，`.try_join()` 在错误时提前结束，`.race()` 取第一个完成值并丢弃其余拥有的分支；`Result` 作为 `.race()` 的普通输出时，第一个错误也能获胜，不能自动理解成“第一个成功”。选择操作前先确定需要所有结果、任一完成还是成功结果，再查相应方法的契约。

[`Race`](https://docs.rs/futures-concurrency/latest/futures_concurrency/future/trait.Race.html)的空数组/向量没有候选者，当前版本会 panic，动态集合应先处理空集。取消仍遵循 Future 的拥有关系：丢弃输掉的 IO Future 不会撤销已经发出的数据；输家是 `JoinHandle` 时，丢弃它也不等价于取消底层任务。运行时无关是组合层性质，分支内部依赖 Tokio 就仍需要 Tokio。

本章于 2026-10-23核查上述标准库、futures0.3.34和futures-concurrency7.7.1文档。Rust1.96测试真实覆盖手动Pending/wake/Ready协议、IntoFuture构建器、借用装箱与错误传播、保留借用的join、race输家析构，以及不使用Tokio的纯Future执行。
