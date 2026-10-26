<a id="the-state-of-asynchronous-rust"></a>
# 异步 Rust 的现状

本章由 GPT-6 Luna 完整翻译，GPT-6.1 Sol 按 2026-10-26 的官方文档校订。保留旧版的主题与顺序；稳定特性、实验 API 和第三方项目维护状态分别说明。本书示例使用现有 Rust 1.96.0，在线标准库文档核查时对应 1.99.1；查阅未来版本时仍应重新确认实验标记。

异步 Rust 的部分功能享有与同步 Rust 相同的稳定性保证。其他部分仍在发展成熟，今后也会继续变化。使用异步 Rust 时，你可以预期：

- 对大量等待 I/O 等合适的并发工作负载，有实现高性能的空间，实际收益仍需测量。
- 会更频繁地接触生命周期和 pin 等高级语言特性。
- 存在一些兼容性限制，既涉及同步代码与异步代码之间，也涉及不同异步运行时之间。
- 运行时与语言支持仍在演进，升级、适配和依赖选择可能带来额外维护工作。

异步 Rust 往往需要掌握更多概念，维护成本也可能高于相应的同步程序；它为高并发系统提供了有效工具，但性能收益不是采用 async 后必然得到的回报。语言和库会继续发展，一些使用困难已有改善，另一些仍需要在接口与架构中显式处理，不能只等待生态升级来解决。

<a id="language-and-library-support"></a>
## 语言与库的支持

Rust 本身支持异步编程，但大多数异步应用依赖社区 crate 提供的功能。因此，你需要结合使用语言特性和库的支持：

- 最基础的 trait、类型和函数（例如 [`Future`](https://doc.rust-lang.org/std/future/trait.Future.html) trait）由标准库提供。
- `async/await` 语法由 Rust 编译器直接支持。
- 许多实用类型、宏和函数由 [`futures`](https://docs.rs/futures/) crate 提供；其组合器通常可跨执行器使用，但要检查启用的功能、具体 API 与子 Future 的运行环境。例如组合器可以组织 Tokio 的计时器，却不能替计时器创建 Tokio 时间驱动。
- 执行器、I/O 驱动和任务启动通常由运行时库提供，例如 [Tokio](https://docs.rs/tokio/latest/tokio/runtime/index.html) 与 [smol](https://docs.rs/smol/latest/smol/)。旧版也曾在这里列出 async-std，但其[官方 README](https://github.com/async-rs/async-std#readme) 已宣布停止维护，并建议现有用户和库迁移到 smol；这里保留其历史位置，不把它当成仍在维护的并列推荐。许多应用和一些库确实依赖具体运行时。旧版[“异步生态”](../08_ecosystem/00_chapter.md)提供背景，当前选择与平台差异也可参阅新版[运行时](../part-guide/runtimes.md)和[替代运行时](../part-reference/alternate-runtimes.md)。

某些在同步接口中常见的写法，换成异步形式后仍有额外限制，但不能再笼统地说“trait 不支持 async”。[Rust 1.75 已稳定 trait 中的 async fn 和返回位置 impl Trait](https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits/)，可用于静态分发。当前 [Reference 的 dyn 兼容规则](https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility) 仍不允许直接通过 trait 对象分发这种带匿名 Future 返回类型的 async 方法；若方法以 `Self: Sized` 排除在动态分发之外，也不能通过 `dyn Trait` 调用它。需要动态分发时，可以设计返回固定类型的接口，例如装箱的 Future，或采用相应宏，但应了解其分配和约束。

另一个边界是 `Send`：trait 对象本身或实现类型是 `Send`，并不自动保证某个异步方法返回的 Future 也是 `Send`。若接口要让调用者把工作交给要求 `Send` 的执行器，应在 API 设计时表达相应返回 Future 约束，例如合适的 `fn ... -> impl Future<Output = T> + Send`，并处理捕获的借用。不要把 native async trait 的稳定等同于所有动态分发和跨线程接口问题都已解决。

截至上述核查日期，标准库 [`AsyncIterator`](https://doc.rust-lang.org/std/async_iter/trait.AsyncIterator.html)（`async_iterator`）与 [`AsyncDrop`](https://doc.rust-lang.org/std/future/trait.AsyncDrop.html)（`async_drop`）仍标为 nightly-only 实验 API。本书稳定示例使用生态中的 `Stream`，需要等待的关闭则用显式异步方法；没有开启这些实验特性或安装 nightly。

<a id="compiling-and-debugging"></a>
## 编译与调试

总体而言，异步 Rust 中的编译器错误和运行时错误，与 Rust 一贯的情况相同。不过有一些值得注意的差异：

<a id="compilation-errors"></a>
### 编译错误

异步 Rust 的编译错误和同步 Rust 一样遵循高标准；但由于异步 Rust 经常依赖生命周期和 pin 等更复杂的语言特性，你可能会更频繁地遇到这类错误。

<a id="runtime-errors"></a>
### 运行时错误

编译器遇到异步函数时，会生成可挂起和恢复的状态机。异步 Rust 的堆栈跟踪通常包含状态机和运行时调用，因此解读时可能比同步 Rust 更复杂。挂起后，原来那次 `poll` 的调用栈已经退出；普通线程 backtrace 不一定能复原“谁在逻辑上等待谁”。任务名称、请求身份和 tracing span 可以补充运行上下文，见[异步编程工具](../part-guide/tools.md)；状态机实现见 [rustc 中 async/await 的实现](../part-reference/rustc-async-await.md)。

<a id="new-failure-modes"></a>
### 新的故障模式

异步 Rust 可能出现一些新型故障模式，例如在异步上下文中调用阻塞函数，或错误地实现 `Future` trait。这类错误可能悄无声息地通过编译器检查，有时甚至能通过单元测试。理解底层概念有助于避免这些陷阱，而这正是本书希望帮助你做到的。

例如返回 `Pending` 却未安排唤醒，程序可能一直等待；在同一任务内持有锁再等待另一个需要该锁的分支，可能形成死锁；无意丢弃半完成的操作，也可能破坏业务状态。这些大多属于活性和协议问题，类型检查并不保证它们不会发生。应验证真实 `Pending`、关闭、错误和取消路径，而不是只测试所有 Future 立即完成的顺利路径。

<a id="compatibility-considerations"></a>
## 兼容性注意事项

异步代码和同步代码并不总能自由组合，但同步函数可以直接调用 async 函数来构造 Future。它不能在普通同步函数体中直接 `.await`，也不能仅靠构造 Future 就把函数体运行完；需要在适当边界交给执行器。反过来，async 函数也可以调用同步函数，只是阻塞调用会阻塞当前执行线程。同步和异步代码往往采用不同设计模式，因此桥接应检查线程、阻塞和运行时生命周期；不要在已运行的异步任务里随意嵌套 `block_on`。[Tokio 桥接文档](https://tokio.rs/tokio/topics/bridging)与本书[同步异步桥接](../part-reference/sync-async.md)说明了这些取舍。

异步代码之间也并非总能自由组合。共享标准库 `Future` 协议，只保证它们能按共同的 `poll` 接口交互，不保证计时器、网络驱动、任务局部状态和 spawn API 能在任意运行时使用。依赖列表是线索，还应看具体 API 文档、功能开关和目标平台；依赖 Tokio 并不表示每个功能都需要 Tokio 运行时。例如 [Tokio sync 模块](https://docs.rs/tokio/latest/tokio/sync/index.html#runtime-compatibility) 的同步原语可用于其他执行器，但需要 Tokio 定时器的 `_timeout` 方法是例外。

也要核对相似名称背后的 trait：futures 与 Tokio 的 `AsyncRead`/`AsyncWrite` 并非同一个接口，通常需要明确适配；历史 futures 0.1 的 Future 也不同于当前的标准库 Future。[futures 的 compat 模块](https://docs.rs/futures/latest/futures/compat/index.html) 提供部分历史协议适配，但协议转换不等于自动补齐底层运行时。

这些兼容性问题可能限制选择，因此应尽早了解所需运行时和依赖。选定一套相容组件能减少后续适配工作，但升级、插件接入、平台变更或引入新库时仍应重新检查。

<a id="performance-characteristics"></a>
## 性能特征

异步 Rust 的性能取决于运行时实现，也取决于应用的任务粒度、并发限制、I/O 方式、内存分配、锁与背压。已有成熟运行时可用于实际系统，但“使用 async”本身不保证吞吐、延迟或资源占用优于同步方案；应使用目标平台上的代表性负载测量。

一些任务 API 要求 `Send`，便于多线程执行器迁移任务；例如 [`tokio::spawn`](https://docs.rs/tokio/latest/tokio/task/fn.spawn.html) 即使在当前线程运行时中使用也保留这一约束。但生态并非统一假定多线程：Tokio 有当前线程运行时和 [`LocalSet`](https://docs.rs/tokio/latest/tokio/task/struct.LocalSet.html)，smol 有 [`LocalExecutor`](https://docs.rs/smol/latest/smol/struct.LocalExecutor.html)，可以按适用条件执行非 `Send` 工作。单线程设计可以减少某些跨线程同步，但库仍可能内部使用锁或原子操作，因此不保证自动获得更低成本；CPU 密集工作还可能阻塞同线程的所有任务。

另一个需要单独评估的场景是对延迟敏感的任务，例如驱动程序或 GUI 工作。这依赖运行时与操作系统的调度支持、线程亲和要求和工作预算，不能把高吞吐自动等同于低尾延迟，也不能把普通异步运行时当成实时系统。明确线程边界、把长时间阻塞或计算移出关键执行路径，通常比等待未来某个库版本更直接。本节没有运行性能或实时性基准；相关单线程、桥接与调度行为的已测实例见新版[运行时](../part-guide/runtimes.md)、[替代运行时](../part-reference/alternate-runtimes.md)和[饥饿](../part-reference/starvation.md)。
