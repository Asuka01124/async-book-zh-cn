<a id="destruction-and-clean-up"></a>
# 析构与清理

资源离开作用域时，Rust 通常通过析构自动释放内存、关闭句柄和解锁。稳定的 `Drop::drop(&mut self)` 是同步方法，不能在其中 `.await`。[析构规则](https://doc.rust-lang.org/reference/destructors.html)还规定了字段、局部变量和展开过程中的销毁顺序。`std::mem::drop(value)` 提前销毁一个值，但不是调用一个异步清理协议。

释放本地资源与完成业务协议是不同工作。关闭本地 socket 不代表对端收到最后一条消息；丢弃事务对象也不代表远程回滚已经确认。程序还可能需要刷新缓冲区、向对端告别、归还远程租约，或等待仍使用资源的任务退出。设计资源类型时要写清楚：哪些动作在析构中同步完成，哪些必须显式等待，失败如何报告。

析构也不是绝对执行保证。进程被强制终止、使用 `panic=abort`，或者值被泄漏时，不能指望析构运行。需要跨崩溃保持正确性，就应依靠事务、幂等操作、租约、持久化恢复或上级进程监督，而不能只靠一段 `Drop` 代码。

<a id="cancellation"></a>
## 取消

取消有几个容易混淆的层次：

- 丢弃一个尚未完成、由当前作用域拥有的 Future，会销毁它保存的状态；不会执行暂停点之后的普通语句。
- 丢弃 Tokio `JoinHandle` 会分离任务，任务仍可能继续运行。要请求强制取消，调用 `abort()`，再等待句柄确认任务退出。
- [`CancellationToken`](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html)发出协作式请求。任务必须主动等待或检查令牌，再决定在哪个安全点结束；请求本身不替任务执行清理，也不会自动中断当前 IO。

一个 `async fn` 的函数体尚未开始轮询时，体内局部守卫还不存在。需要覆盖“连首次轮询都没有”的路径，就必须把守卫作为已经构造的状态移进 Future。对于已开始的任务，同步守卫的析构可记录未完成退出、归还许可和释放锁；它不能等异步确认，也通常无法单凭 `Drop` 区分被取消还是 panic。

`abort()` 也不是任意指令位置的抢占：执行器需要重新获得控制权才能销毁任务。长时间不返回的 `poll` 会延迟它；已开始的 `spawn_blocking` 工作通常不能 abort。[JoinHandle 文档](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html)保证通过等待句柄观察任务完成时，其 Future 的析构已经结束；这仍不代表你在析构中另行生成的后台工作完成。

若只想观察退出，可使用一个同步守卫在 `Drop` 中发一次性消息或记录计数，并在正常完成时标记成功。监控系统将“没有完成标记的退出”与 `JoinError::is_cancelled()`、`is_panic()` 结合解释，比把所有析构都标记为取消准确。

取消会沿拥有关系影响子 Future，却不自动沿所有任务关系传播。丢弃一个 `join!` Future 会丢弃它拥有的分支；分支若只是一个任务句柄，底层任务可能被分离。已写出的数据、已经接受的工作也不会因为调用者取消而撤销。为每个等待点分析中间状态，详见[取消与取消安全](../part-reference/cancellation.md)。

<a id="panicking-and-async"></a>
## Panic 与异步代码

直接 `.await` 的 Future 在同一任务内 panic，通常沿当前调用链展开。`tokio::spawn` 建立单独任务边界；在展开式 panic 配置下，Tokio 捕获任务 panic，等待句柄得到 `JoinError`。父任务不会仅因为子任务 panic 就自动 panic；若需要传播，应检查 `is_panic()` 后显式 `resume_unwind(error.into_panic())`，或把它转成应用错误。`panic=abort` 不提供这种恢复机会。

释放锁不等于恢复不变量。Tokio 互斥锁没有中毒机制，任务持锁修改到一半后 panic，锁会释放，但半成品数据可能留下。把状态变更设计成可提交的整体，或记录可恢复阶段；若无法恢复，就让监督层停止继续使用该资源，而不是静默服务后续请求。

不要在 panic 展开时尝试通过新建运行时、嵌套 `block_on` 或 `spawn` 再等待异步清理：当时可能没有有效运行时，也可能正持锁、运行时正在关闭，或再次 panic。析构中的第二次 panic 在展开期间可能终止进程。日志 subscriber 本身也应避免在这条路径里做会失败的复杂工作。重要异步清理应在仍有正常控制流和有效运行时的时候进行。

<a id="patterns-for-clean-up"></a>
## 清理模式

### 缩小必须异步清理的范围

先问是否能通过协议设计避免它。连接可在断开后由服务器回收，临时租约可超时，工作可用幂等 ID 重试。对于允许丢弃缓存数据的程序，普通析构可能已经足够；对于必须持久化的写入，单纯析构就不够。明确这个区别，往往比设计一个通用“异步析构模拟器”更有价值。

### 显式异步关闭，析构只作兜底

下面的会话将 `flush` 和写端 `shutdown` 放进 `finish()`，用 `Result` 向调用者报告错误。`Drop` 仅记录未显式完成；不发起后台 IO。

```rust
{{#include ../../examples/guide-tests/src/cleanup.rs:explicit-close}}
```

这里 `shutdown` 的含义来自具体 `AsyncWrite` 实现；对于 TCP，它关闭写方向，不保证对端应用已处理数据。如果需要应用层确认，要在协议里另外等待。`finish` 被取消或返回错误时不会设完成标志，但“没有完成”不等于“什么都没发生”；是否允许重试依协议而定。

有时类型使用“析构炸弹”：忘记调用关闭就 panic，以暴露误用。这适合极受控的测试约束，通常不适合作为公共资源类型的恢复机制，因为展开时再次 panic 会更严重。显式状态、`#[must_use]` 的结果、日志和测试常更容易组合；无论怎样都不能把它当成异步关闭已完成的保证。

### 集中清理与优雅关闭

[Tokio 优雅关闭指南](https://tokio.rs/tokio/topics/shutdown)把流程分成三个动作：决定关闭、通知各部分、等待各部分完成。一个管理任务可独占连接或缓冲状态，其他任务通过消息使用它。关闭时先停止接受新请求，再排空已接受工作，执行异步清理，最后等待管理任务退出。

下面用 `watch` 表示保持为真的关闭状态，也可换用取消令牌。收到请求后跳出接收循环，**在正常控制流里**等待清理。不要把 `finish()` 再与同一个已经触发的关闭条件竞争，否则清理很可能立刻被取消。

```rust
{{#include ../../examples/guide-tests/src/cleanup.rs:shutdown}}
```

调用方保存任务句柄，先 `tx.send(true)`，再等待 `JoinHandle<Result<_, _>>` 的两层结果。生产系统还应决定总关闭期限：超时后 abort 并等待退出，意味着明确接受某些异步清理未完成；不能把超时写成“所有资源安全关闭”。也可用 `JoinSet` 或 `tokio_util::task::TaskTracker` 管理多任务的退出，但仍要保证任务实际收到关闭请求。

专用线程或监督进程也能负责清理，前提是它的生命周期长于被清理资源，并且有提交、失败和停止的协议。从 `Drop` 中向无界清理队列同步发消息只是尽力转交：队列可能关闭、内存可能增长、进程可能先退出。需要保证完成时，仍必须在关机前等待清理方的确认。

清理次序应让依赖者先停，再关闭依赖资源，最后销毁运行时。[运行时关闭语义](https://docs.rs/tokio/latest/tokio/runtime/struct.Runtime.html#shutdown)不保证异步任务完成；已绑定的 IO 资源在关闭后无法继续正常使用。`shutdown_timeout` 只限制调用方等待时间，不会强行终止正在运行的阻塞函数。

<a id="why-no-async-drop-yet"></a>
## 为什么稳定 Rust 仍没有异步 Drop

这一节涉及语言实现，不影响使用上面的关闭模式。**截至 2026-10-23，`std::future::AsyncDrop` 仍是 `async_drop` 特性门控的实验 API**，不是稳定版中可以普遍依赖的自动异步析构。核查依据为 [AsyncDrop 标准库文档](https://doc.rust-lang.org/std/future/trait.AsyncDrop.html)、[Unstable Book](https://doc.rust-lang.org/unstable-book/language-features/async-drop.html)和[跟踪问题 #126482](https://github.com/rust-lang/rust/issues/126482)。本书使用稳定 Rust 1.96 的示例，不启用该特性，也不将当前 nightly 的语法作为长期接口。

难点不只是给 `drop` 加一个 `async`：同步调用者离开作用域时由谁轮询？清理 Future 需要保存在哪、它是否为 `Send`？清理过程自己被取消或 panic 时怎么办？在泛型容器、trait 对象、字段递归析构和运行时关闭时，又如何保持一致语义？同步的 `drop(value)` 不能突然隐式阻塞执行器。

显式 `close().await` 暴露等待和错误，但要求调用者遵守协议；后台监督者可继续清理，但引入生命周期和完成确认；语言级异步析构则需要统一处理这些控制流问题。不能把“用 `spawn` 包住清理”视作与语言保证等价。

完成式 IO 进一步放大这个问题：Future 被丢弃时，内核可能仍访问已提交缓冲区，安全实现必须保留其所有权直到操作结束，不能直接释放被借用内存。见 [IO 章的就绪与完成模型](io.md)。这是 IO 抽象和语言设计需要共同处理的生命周期边界，并非用户添加一个析构函数就能修复。

本章测试真实覆盖显式刷新/关闭、未完成缓冲丢弃、abort后同步析构、panic后锁状态及协作关闭；没有运行实验 AsyncDrop，也没有把进程终止后析构当作可保证行为。
