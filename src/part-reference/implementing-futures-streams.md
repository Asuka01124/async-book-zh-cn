<a id="implementing-futures-and-streams"></a>
# 实现 Future 与 Stream

上游此处仅有英文目录标题，没有英文正文；标题由 GPT-6 Luna 翻译，以下中文正文由 GPT-6.1 Sol 补写。应用代码通常优先用 async/await 与现成通道；手写 poll 主要用于包装底层事件源、实现组合器和连接运行时。接口小并不表示协议简单：状态、唤醒、固定和取消必须一起设计。

## 先画状态，再写 poll

[`Future::poll`](https://doc.rust-lang.org/std/future/trait.Future.html) 是一次非阻塞的推进请求。下面实现一个单生产者、单消费者、只发送一次 u32 的教学通道：初始是“等待”，生产者可转到“有值”或“关闭”；接收者取走结果后转到“已完成”。取消接收者则使后续发送失败，并把待发送值还给发送方。单次发送通过 `send(self, value)` 在类型层面保证。

这不是 Tokio 通道的替代品：没有容量配置、超时、泛型值和高性能无锁队列，Mutex 中毒也采用教学性的 unwrap 处理。它只展示完整的等待/发布/取消协议，所有代码都是安全 Rust：

```rust
{{#include ../../examples/guide-tests/src/implementing.rs:oneshot}}
```

poll 内的顺序很重要：先在锁内查看值和关闭状态；若仍不可用，在同一把锁内记录当前 waker，再返回 Pending。生产者也必须拿同一把锁更新值并取走 waker，因此不会插入到“检查完尚未注册”的空隙。若生产者早已发送，第一次 poll 直接读取值，不需要过去那次通知仍然存在。

锁只保护很短的内存操作，绝不跨 await。实现没有在锁内调用 wake：Waker 的执行行为由执行器决定，可能间接触发复杂逻辑，持锁唤醒会扩大重入或死锁风险。我们先取走通知句柄、释放锁，再调用它。生产者 Drop 会发布关闭，确保接收者不会因发送端消失而永久挂起。

## 为什么“检查一次，再注册”会丢唤醒

设消费端观察到“没有值”，此时生产者写入值并发现没有 waker；接着消费端注册并返回 Pending。值已经存在，却再没有人负责叫醒它。这种程序在生产者先完成或很慢时可能通过测试，只在某个交错顺序中挂住。

本例用同一互斥区覆盖检查与注册解决这个竞态。无锁实现常用 [`AtomicWaker`](https://docs.rs/futures/latest/futures/task/struct.AtomicWaker.html)：先注册，再检查条件，或先快速检查、注册、再次检查。条件仍须由合适的原子操作或其他同步来发布，AtomicWaker 不是存储业务结果的队列。一个 AtomicWaker 只保留一个当前等待者，不能直接当广播列表。

每次 poll 都可能传来不同 waker，不能永远保留第一次的副本。本例每次等待都更新它；优化时可用 `will_wake` 减少 clone，但不能省略更新协议。通知只是“值得再次 poll”，不是附带结果的消息，也不是对恰好一次执行的承诺；重复或虚假唤醒都应由状态检查吸收。

Pending 不能靠消费者忙循环补救。事件源有进展可能时必须通知任务；消费者也不能把一次 wake 等同于 Ready，仍需重新检查状态。一个总能立即工作但无限循环的 poll 同样错误，它会长时间占住执行线程。

## Pin、Unpin 与字段投影

[`Pin`](https://doc.rust-lang.org/std/pin/index.html) 约束被固定对象在生命周期中的移动方式，并不让它永生，也不取消 Drop。本例 Receiver 只有 Arc 和 bool，没有地址敏感字段，自动满足 Unpin，因而可以安全 `get_mut()`。

若包装任意 Future，不能给任意 F 随意加上 Unpin 假设。一个简单策略是字段持有 `Pin<Box<F>>`，外层可以移动盒子句柄，内部 F 留在堆上。下面把任意 Future 转成单项流，支持编译器生成的 `!Unpin` async 状态机：

```rust
{{#include ../../examples/guide-tests/src/implementing.rs:once-stream}}
```

这需要一次堆分配。如果需要把 F 内嵌在包装器里，可以使用 [`pin-project-lite`](https://docs.rs/pin-project-lite/latest/pin_project_lite/) 或 pin-project 为 `#[pin]` 字段生成 `Pin<&mut F>` 投影。未固定字段可取得普通可变引用。结构固定会影响 Unpin 和 Drop 的实现，不能用 `mem::replace` 把固定的 `!Unpin` 字段偷偷取走。本章没有为投影新增依赖；装箱实现实际编译并测试，宏仅解释官方使用方式。

写组合器时，输入输出的约束也要区分：`Future::Output` 可以是不含 Send 的类型；只有跨线程执行等使用场景才需要额外的 Send。手写 Future 不必自动要求 `'static`，借用的生命周期也可以由外层 API 约束。

## Stream 要保存每一项之间的等待状态

[`Stream::poll_next`](https://docs.rs/futures/latest/futures/stream/trait.Stream.html) 把终止编码为 `Ready(None)`，并允许 `Ready(Some(item))` 后继续推进。上面的 OnceStream 在 Pending 时保留原 F；获得输出后才丢掉已经完成的 F；以后永远返回 None。不能每次 poll 都重新调用工厂创建 Future，否则每次都会从起点开始，已注册的等待也可能反复被取消。

[`FusedStream`](https://docs.rs/futures/latest/futures/stream/trait.FusedStream.html) 的 `is_terminated` 允许消费者知道再轮询是否有意义。本例交出唯一结果后就为 true，因为已经没有下一项。普通流不必实现 FusedStream；调用方若需终止后安全重复轮询，可以用 `StreamExt::fuse()`。不要把 Future 的 Fuse（完成后 Pending）与 Stream 的 Fuse（结束后 None）混为一谈。

多步协议通常需要 enum 状态，例如 Idle、Reading、Decoding、Finished；每次转移都须决定谁拥有缓冲区、当前等待对象是否保留、错误后是否还能重试。对于连续立即就绪的输入，还应限制单次 poll 的工作量，必要时 self-wake 后返回 Pending，为别的任务让出执行机会。这里的 self-wake 是明确安排再次轮询，不能成为无条件自唤醒的空转。

## 取消与完成是协议的一部分

Future 可能从未 poll 就被丢弃，也可能在任意 Pending 之后被丢弃。本例 Receiver 的 Drop 清掉注册、标记接收者已消失；之后发送返回 Err(value)。若发送和取消竞争，发送可能已经成功但值没有被业务读取。通道“成功接收发送值”不等于远端业务“成功提交”，应用需要另外定义确认语义。

锁内清理 waker 保证本实现不保留已取消等待者的长期引用；它不保证任意真实事件源绝无已在途的唤醒。执行器必须能处理已结束任务的迟到通知，底层资源也必须在 OS 仍可能访问时保活。对于完成式 I/O，Drop Future 通常还需要更复杂的操作登记和内核完成回收，不能照搬这个内存通道的析构方式。

完成后的 Receiver 再 poll 会明确 panic。这符合 Future 契约，因为完成后重复 poll 没有通用保证，但任何状态下仍不能造成未定义行为。该 panic 也提醒实现者：如果希望对外提供可重复读取的结果，那应当是另一个明确承诺缓存输出的 API。

## 验证哪些性质

本章四个测试实际运行：用两个计数 Waker 验证只通知最新等待者，真实辅助线程发送并唤醒；生产者先发送以及发送者先关闭；取消注销后拒绝迟到发送并保留值；单项 Stream 在 Pending 后继续并最终融合。它们验证行为协议，不只检查函数返回类型。

这些确定性测试不是所有线程交错的穷举证明。更复杂的无锁代码应把共享状态抽成小模型，用 Loom 等工具探索交错，并对 unsafe 的固定/资源生存期另作审计。本章没有声称测试已经证明任意 I/O 或任意内存序实现正确。所有新增共享源码可在 Windows 用现有 Rust 1.96.0 运行，不依赖网络或实验编译器。
