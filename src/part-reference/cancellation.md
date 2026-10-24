<a id="cancellation-and-cancellation-safety"></a>
# 取消与取消安全

取消是停止继续等待或推进一项工作的请求与过程。它可能来自超时、用户离开页面、其他分支先完成、父任务退出，也可能来自工作自身发现不必继续。上游英文提纲由 GPT-6 Luna 完整翻译，本章解释、示例与边界由 GPT-6.1 Sol 补写。

## 内部取消与外部取消

内部取消由操作自己决定，例如检查停止标志后返回 Err，或者收到关闭请求后先保存进度、排空队列再返回。操作仍有执行代码的机会，能明确选择可恢复的停止点。外部取消则由拥有 Future 的调用者停止轮询并销毁它；操作的同步析构会执行，尚未执行的普通语句与 await 清理不会自动执行。

这与 OS 线程不同。Rust 的标准线程没有一个可以在任意机器指令处安全销毁线程栈的通用取消 API；丢弃线程 JoinHandle 也只会分离线程。Future 是一个保存进度的 Rust 值，拥有者可以在未被借用轮询时 drop 它，释放其中保存的缓冲区、guard 和子 Future。取消因此能像资源管理一样组合，但 Future 在释放前已经产生的外部效果不会倒转。

“丢弃 Future 就是取消”是对该 Future 后续执行的简写，不保证底层工作消失。丢弃 Tokio JoinHandle 会分离任务；丢弃发送后等响应的 Future 不会撤回已接收命令；丢弃完成式 I/O 的等待者也不能提前释放内核仍访问的内存。这些边界分别见 [设计模式](design-patterns.md)、[异步 I/O 模型](io-models.md)。

## 取消发生在什么位置

拥有者不能在另一个 poll 调用仍通过 &mut Self 使用 Future 时把同一个值销毁。poll 正在执行的同步代码会继续，直到控制权交回调用者。生成的 async 状态机通常在子 Future 返回 Pending 的 await 处把控制权交回执行器，于是执行器能随后销毁暂停的任务；自定义 Future 也可以直接返回 Pending。[Future 协议](https://doc.rust-lang.org/std/future/trait.Future.html) 要求 poll 尽快返回。

因此“仅在 await 点取消”不是精确的语言规则：Future 尚未第一次 poll 就能被丢弃；一个 await 也可能立即 Ready，根本没有交回执行器。一直执行 CPU 循环或同步阻塞调用的 poll，不能靠外部超时、abort 或取消令牌立即打断。即使是多线程 runtime，取消请求也不能安全地抢走正在运行的 Rust 栈。

这个设计让许多外部取消变得实用：race/select 的失败分支、超时和短路返回可以直接释放不再需要的子 Future。但它仍可能突然发生。调用者只看见一个 await，内部却可能已经读走一部分数据、移走一项队列消息、或改变业务状态；理解这些中间状态，是取消安全的核心。

## abort 与合作式取消令牌

Tokio 的 [JoinHandle::abort](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html#method.abort) 请求运行时中止任务。调用 abort 后应继续 await 句柄，以确认任务终止并检查 JoinError；若任务已完成，仍可能得到正常结果。对可中止任务，观察句柄完成时同步析构已结束，不能由此推断异步 flush 已执行。已经开始的 spawn_blocking 工作无法被 abort 强制终止，运行中的不让出 poll 也不会被立即打断。

[futures::future::Abortable](https://docs.rs/futures/latest/futures/future/struct.Abortable.html) 则是普通 Future 包装器，由关联 AbortHandle 令它短路为 Aborted。它仍依赖外层继续 poll 这个包装器，不是运行时线程的终止原语；其资源最终依照普通所有权释放。不要把同名 abort 当成所有库都完全相同的任务生命周期保证。

合作式取消把请求交给任务自己处理。[tokio_util::sync::CancellationToken](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html) 的 cancel 会通知 cancelled 等待者；克隆 token 共享取消，child_token 由父向子传播，但取消子 token 不取消父 token。token 是通知机制，不会替任务运行清理，也不代表任务已经退出。可以先停止接纳，然后让任务 finish().await，最后由宿主 join；清理阶段不要继续与已经触发的同一关闭请求竞争，否则清理 Future 会立即被丢弃。正文没有为介绍 token 而加入新依赖，已有 watch 关闭实例见 [析构与清理](../part-guide/dtors.md)。

<a id="cancellation-safety"></a>
## 取消安全

这里首先讨论逻辑正确性：丢数据、重复请求、状态失配或无法完成协议，并非把安全 Rust 写出的取消直接称为内存未定义行为。它们也不一定需要线程或数据竞争，单线程按确定顺序轮询就能发生。若库包含 unsafe/FFI，驱动必须另外保证取消后的缓冲区、回调与生命周期仍满足内存安全契约；这是库实现责任，不能要求安全 API 的用户猜测何时释放会造成 UB。

“取消安全”在不同文档中的范围不相同，必须先确定要保留什么：

- Tokio 在 [select 的取消安全说明](https://docs.rs/tokio/latest/tokio/macro.select.html#cancellation-safety) 中关注：尚未完成的操作被丢弃、随后重新创建时，不丢失原本应继续处理的进展。这对循环中的 recv/read/锁等待很有用。
- 更广的停止安全性（halt safety）关注整个操作或系统在任意允许的停止点，仍能维持其资源、状态和业务不变量。它可能允许显式报告“不完整消息”或关闭连接，而不保证重建同一个 Future 就能接着读。
- “先假想复制一份 Future，再比较取消后的行为”可以用作思考实验：用同一外部状态重新构建操作，能否等价地继续？这不是要求 Future 实现 Clone，也不是允许真的复制一个具有独占借用的 Future。状态已经消费或业务效果已经提交时，两次运行通常不会等价。

局部 API 的保证必须按文档解释。Tokio mpsc::Receiver::recv 等待取消不会消费消息；但 Mutex::lock 取消会失去队列位置，这种公平性进度也可能重要。read_exact/write_all 可能已推进部分字节，不能像一次 read/write 一样直接假定无损重试；见 [AsyncReadExt](https://docs.rs/tokio/latest/tokio/io/trait.AsyncReadExt.html#method.read_exact)。即便单次 read 是取消安全的，围绕它的整段解析函数也未必安全。

## 两个安全 recv 组成的函数仍会丢进度

这个函数从通道收两个字节，组成一条消息：

```rust
{{#include ../../examples/guide-tests/src/cancellation.rs:local-pair}}
```

如果第一字节已收到，第二次 recv 返回 Pending，first 只保存在 local_pair 的 Future 内。在 select 中别的分支获胜后，该 Future 被销毁；通道里的第一字节已移出，而它保存的那份副本也消失了。下一轮重新创建 local_pair，只能从第二字节开始，将后续消息拼错。每个 recv 本身的契约都没有被违反。

把缓冲区、游标、已收消息计数留在临时 Future 里，同时把“数据已消耗”留在外部通道或连接里，会让状态被拆分到不同生命周期。解决方法不是机械地禁止 select，而是明确未完成进度应由谁拥有。

## 让进度比单次等待活得更久

一种方法是把可恢复状态放到长期存活的上下文里。下面的 PairReader 将第一字节保存到 self；取消 next 的等待后，只要 reader 和 rx 仍被保留，再调用 next 就会继续等待第二字节：

```rust
{{#include ../../examples/guide-tests/src/cancellation.rs:persistent-pair}}
```

在两次 await 之间，已收字节会在同一次 poll 内存入 self；没有额外可取消的暂停点。EOF 时用 Err(first) 显式交还不完整消息，而不是悄悄当成正常结束。真实解析器还需限制帧大小，记录头部/正文偏移、错误与 EOF 状态，并定义损坏连接是否还能复用。

这只在上下文保留时可恢复。若整个任务连同 PairReader 一起被中止，first 同样会丢弃；应用是否接受丢弃一条连接的半帧，取决于整体停止协议。写入数据库的业务进度还可能必须保存到事务或持久记录里，不能仅靠把字段移出 Future 获得崩溃恢复能力。

另一种方法是保留同一个操作 Future。把它创建并固定在 select 循环外，分支使用 &mut operation，别的分支获胜时只丢弃这个临时借用，不丢弃 operation 本身。它内部的 first 因而还在，下次继续 poll 同一个状态机即可。源码第三个测试实际验证了这个路径。

这种写法会在等待期间持续借用它使用的资源；需要并行修改相同 reader 的其他分支可能因此无法通过借用检查。完成后应移出循环或明确禁用分支，不能继续轮询已经返回 Ready 的普通 Future。fuse、Option 状态或合适的分支条件可以管理终态，但不能把“Future 仍存在”误当成可反复重启。

## select 循环还会悄悄重置什么

每次进入 select 都在分支表达式中重建 sleep，会把“总期限”变成“最近一次循环后的超时”；业务若要求总期限，应在循环外固定定时器或保存绝对 deadline。read_exact 的偏移、发送 Future 持有的命令、锁等待的位置也可能在每次竞争后重置。检查每一条失败分支的所有权，往往比只看获胜分支更有价值。

保留一个底层操作不表示应该永远忽略停止信号。应先决定完成当前帧、报告已提交操作、回滚事务、还是关闭连接，然后在可维持不变量的位置停止。同步 RAII guard 可以恢复同步状态；需要 await 的补偿或关闭应由明确的任务协议完成。外部中止仍可能发生时，安全包装还应保证最坏情况下释放内存安全，业务则用日志、幂等和恢复程序处理剩余不确定性。

## 本章验证边界

三个新测试在真实 Tokio mpsc 与 select 上验证：局部组合消费第一字节后被取消，重建会改组后续字节；外部状态在重建后恢复并报告 EOF 半帧；固定同一 Future 的临时借用被取消后仍能恢复。biased 与立即就绪分支让交错确定，不依赖延时或制造挂死。

abort 的同步析构、超时后 join 和关闭后异步清理已在先前章节实测，本章没有重复计数；CancellationToken 仅核查官方协议，没有新增依赖或宣称运行 token 集成。完成式内核 I/O 的取消依赖具体驱动，本章通道例子不能证明 IOCP/io_uring 实现正确。
