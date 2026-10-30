<a id="the-async-ecosystem"></a>
# 异步生态系统

Rust 目前只为编写异步代码提供了最基本的要素。
标准库提供 `Future`、`Poll`、`Context` 和 `Waker` 等基础协议，但没有通用的异步任务调度器、I/O 与时间驱动器，也没有这里介绍的完整组合器和异步 I/O trait 集合。
与此同时，社区提供的异步生态系统填补了这些空缺。

原版《Async Book》曾邀请社区扩展示例，以涵盖多种运行时。原文留下的讨论入口是
[Zulip](https://rust-lang.zulipchat.com/#narrow/stream/201246-wg-async-foundations.2Fbook)；参与贡献时请同时核对项目当前的贡献说明。

<a id="async-runtimes"></a>
## 异步运行时

异步运行时是用于执行异步应用程序的库。
运行时通常会将一个 *反应器* 与一个或多个 *执行器* 组合在一起。
反应器提供对外部事件的订阅机制，例如异步 I/O、进程间通信和计时器。
在异步运行时中，订阅者通常是代表底层 I/O 操作的 Future。
执行器负责任务的调度和执行。
它们跟踪任务，并轮询可运行任务中的 Future；反应器或其他事件源通常通过 Waker 通知执行器重新调度任务，而非执行器不断轮询等待中的任务。
“执行器”一词经常与“运行时”互换使用。
在这里，我们用“生态系统”来描述与兼容的 trait 和功能打包在一起的运行时。

<a id="community-provided-async-crates"></a>
## 社区提供的异步 crate

<a id="the-futures-crate"></a>
### Futures crate

[`futures` crate](https://docs.rs/futures/) 包含编写异步代码时有用的 trait 和函数。
其中包括 `Stream`、`Sink`、`AsyncRead` 和 `AsyncWrite` trait，以及组合器等实用工具。
这些能力是否进入标准库应以相应稳定公告为准，不应把可能的未来演进当作已有保证。

`futures` 有自己的执行器，但没有内置的操作系统 I/O 和时间驱动器，因此通常不被视为完整运行时。
这不代表它不能轮询任何 I/O 或计时器 Future：如果 Future 所需的驱动器独立运行并能正确唤醒任务，就可能配合使用。例如 [`async-io`](https://docs.rs/async-io/latest/async_io/) 提供独立的 I/O 与计时支持；这并不自动满足 Tokio 资源的上下文和驱动要求。
一种常见做法是使用 `futures` 中的实用工具，并搭配其他 crate 提供的执行器。

<a id="popular-async-runtimes"></a>
### 流行的异步运行时

标准库中没有异步运行时，也没有任何运行时获得官方推荐。
以下保留原文列举的运行时，并标明已核实的维护状态（2026-10-30）。

- [Tokio](https://docs.rs/tokio/)：一个广泛使用的异步运行时，其生态包含 HTTP、gRPC 和 tracing 等相关库；这些不是全部内置于 Tokio crate 的功能。
- [async-std](https://docs.rs/async-std/)：曾为标准库组件提供异步对应实现；[官方文档](https://docs.rs/async-std/latest/async_std/)现已明确宣布停止维护，并建议用户及依赖它的库迁移至 smol。后续旧版示例使用它时应结合这一历史背景阅读。
- [smol](https://docs.rs/smol/)：一个小巧、简化的异步运行时。
  它重新导出 [`Async<T>` 结构体](https://docs.rs/smol/latest/smol/struct.Async.html)，可包装受支持的 `TcpListener`、`UnixStream` 等 I/O 类型；它不是 trait，也不能任意包装所有标准库 I/O 类型，且 Unix 类型有平台限制。
- [fuchsia-async](https://fuchsia.googlesource.com/fuchsia/+/master/src/lib/fuchsia-async/)：
  一个供 Fuchsia OS 使用的执行器。

<a id="determining-ecosystem-compatibility"></a>
## 判断生态系统兼容性

并非所有异步应用程序、框架和库彼此兼容，也并非都能兼容每一种操作系统或平台。
大多数异步代码可以用于任意生态系统，但有些框架和库要求使用特定的生态系统。
生态系统限制不一定总会写入文档，不过有几条经验法则可以用来判断某个库、trait 或函数是否依赖特定生态系统。

与异步 I/O、计时器、进程间通信或任务生成交互的代码，需要核对具体驱动器、上下文及调度接口；这种依赖不一定要求由同一个执行器轮询。
`async` 表达式和只负责轮询子 Future 的组合器通常不绑定运行时，但前提是其内部操作及嵌套 Future 也不绑定运行时。流和同步类型也要逐项检查：有些通道或锁只依赖 Waker，有些同步原语依赖特定运行时的任务上下文、协作调度或计时设施，不能仅凭类别断言兼容。
开始一个项目之前，建议先研究相关的异步框架和库，确保它们与你选择的运行时以及彼此之间兼容。

Tokio 的网络事件驱动使用 `mio`，并定义了 `tokio::io::AsyncRead` 和 `AsyncWrite`；它们与 `futures::io` 的同名 trait 不同。smol 生态通常采用 futures I/O trait，并组合 [`async-executor` crate](https://docs.rs/async-executor) 等组件；不应据此假定 async-std 的所有历史版本具有完全相同的内部实现。
[`tokio-util::compat`](https://docs.rs/tokio-util/latest/tokio_util/compat/) 可以双向适配两套 I/O trait，但转换接口不会自动建立运行时或持续驱动已绑定的资源。Tokio 资源所需的上下文、已启用的驱动器及其生命周期，仍必须满足。

有时可以通过兼容层解决相互冲突的运行时要求，让你能够在一个运行时中调用为另一个运行时编写的代码。
例如，[`async_compat` crate](https://docs.rs/async_compat)（包名 `async-compat`）既能适配 I/O trait，也能在轮询包装的 Future 时进入 Tokio 上下文；这不等于把外层 Future 移交 Tokio 执行。适配不是任意运行时 API 的通用保证，仍应核对版本、资源创建时机、任务和阻塞操作等限制。

暴露异步 API 的库应尽可能避免不必要的运行时绑定；需要生成任务、使用运行时特定资源或保证协作调度行为时，应清楚记录依赖，也可考虑由调用方传入资源或生成任务的接口。
通常应由应用入口负责选择运行时、管理生命周期并关闭后台任务，而不是每个库都隐式启动自己的运行时。更多细节见[运行时及其使用边界](../part-guide/runtimes.md)与[替代运行时](../part-reference/alternate-runtimes.md)。

<a id="single-threaded-vs-multi-threaded-executors"></a>
## 单线程与多线程执行器

异步执行器可以是单线程或多线程的。
例如，`async-executor` 提供只能在创建它的线程驱动的 [`LocalExecutor`](https://docs.rs/async-executor/latest/async_executor/struct.LocalExecutor.html)，以及可从多个线程驱动的 [`Executor`](https://docs.rs/async-executor/latest/async_executor/struct.Executor.html)。后者不会仅因被创建就自动启动工作线程；也可以仅由一个线程驱动。

多线程执行器在有多个工作线程及硬件资源时，可以并行推进不同任务；单线程也能在等待期间并发推进多个任务。
多线程可能改善吞吐量，也会引入同步、调度和缓存成本；任务多或等待 I/O 多，并不保证多线程更快。
在单线程和多线程运行时之间作选择时，建议测量应用程序的性能。

任务可以在创建它们的线程上运行，也可以在单独的线程上运行。
运行时通常将普通异步任务调度到当前线程或工作线程池，生成一个任务并不意味着为它创建专用线程。
即使任务在不同线程上执行，它们仍应保持非阻塞。
会把任务在线程之间移动的生成任务接口通常要求 Future 为 `Send`，还可能要求输出为 `Send` 及满足生命周期约束；这是具体接口的契约。单线程运行时的普通 spawn 接口也可能要求 `Send`，而直接等待的根 Future 不一定受同样限制。
有些运行时提供本地任务接口，让非 `Send` Future 固定在一个线程运行；应核对本地执行环境的创建与驱动规则，而不能笼统认为它总是在任意调用 spawn 的线程上运行。
它们也可能提供将阻塞任务派生到专用线程上的函数，这对于从其他库运行阻塞式同步代码很有用。
