<a id="async-iterators-fka-streams"></a>
# 异步迭代器（Stream）

一次 HTTP 请求可以建模为一个 Future；持续到来的消息则需要反复等待下一项。`Stream<Item = T>` 是这种异步序列的接口。可以把它理解成异步的 `Iterator`，也可以把每次 `next()` 理解成一次产生 `Option<T>` 的 Future。Stream 本身并不是一个存放了许多 Future 的容器：它可以按需读取文件、等待通道，甚至永不结束。

本章上游只有提纲，提纲由 GPT-6 Luna 翻译，正文由 GPT-6.1 Sol 补写。可运行代码来自独立验证 crate，使用 Rust 1.96.0、futures 0.3.34；涉及计时的测试使用 Tokio 1.54.1。

## 接口、惰性与固定

[`futures::Stream`](https://docs.rs/futures/latest/futures/stream/trait.Stream.html) 的核心接口如下（接口摘录，不是另一个需要实现的 trait）：

```rust,ignore
trait Stream {
    type Item;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>)
        -> Poll<Option<Self::Item>>;
}
```

`Pending` 表示暂时没有下一项，实现必须安排在有进展可能时唤醒当前任务；`Ready(Some(item))` 交出一项，以后还可以继续轮询；`Ready(None)` 表示结束。普通 Stream 在结束后再次轮询没有统一保证，调用方不能假定永远继续得到 None。`StreamExt::fuse()` 包装后，首次结束之后会一直返回 None。[`FusedStream`](https://docs.rs/futures/latest/futures/stream/trait.FusedStream.html) 的 `is_terminated()` 则让组合器查询终止状态；它本身不会替任意实现修复逻辑。

Stream 组合器通常惰性执行：构造 `map` 不会运行闭包，只有下游轮询才向上游索取数据。已经启动的后台生产任务仍可能独立运行，惰性不等于整个系统都停止。消费暂停时，有界队列最终可以把压力传到生产端；无界队列会继续积累内存。

`next(&mut self)` 要求接收者 `Unpin`。`unfold` 或包含 async 状态的组合器常常不满足这个条件；可用 `pin_mut!`、`std::pin::pin!` 或 `Box::pin` 固定。移动 `Pin<Box<S>>` 句柄不会移动堆上的 S。固定不阻止销毁，结束消费时仍可丢弃整个 Stream；结构体内部字段的投影问题见[固定章节](../part-reference/pinning.md)。

`tokio_stream::Stream` 与 futures 的基础 Stream 是同一个生态 trait，扩展 trait 则不同。[Tokio StreamExt](https://docs.rs/tokio-stream/latest/tokio_stream/trait.StreamExt.html) 提供 `merge`、`throttle`、带时间限制的分块等；futures 提供下面使用的缓冲与并发消费。两者同名方法容易歧义，应只导入一个，另一套用 `tokio_stream::StreamExt::merge(a, b)` 这样的完整路径调用。Tokio 的时间组合器仍需要 Tokio 时间驱动器；通用 trait 不消除运行时依赖。

<a id="consuming-an-async-iterator"></a>
## 消费异步迭代器

常见写法是在 async 函数里执行 `while let Some(item) = stream.next().await`。下例同时展示构造与固定，整个函数已经编译并测试：

```rust
{{#include ../../examples/guide-tests/src/streams.rs:unfold}}
```

循环体可以使用 `break`、`?` 并借用外部变量。`for_each(|item| async move { ... }).await` 适合每项执行一个异步动作，但它会等当前动作完成再取下一项。`collect::<Vec<_>>().await` 则等到序列结束后返回集合；对无限序列没有终点，对超大序列可能耗尽内存。`Stream<Item = Result<T, E>>` 可配合 `TryStreamExt::try_collect` 在首个错误处返回，已经发生的副作用不会回滚。

[`StreamExt`](https://docs.rs/futures/0.3.34/futures/stream/trait.StreamExt.html) 的 `into_future()` 消费 Stream，输出 `(Option<Item>, 剩余的Stream)`；它只等下一项，不会一次收齐所有项，也不同于标准库 `IntoFuture` trait。`next()` 是借用式包装器，不拥有整个流。

### 顺序、并发度与输出

假设 `work(n)` 返回 Future，以下表达式有不同语义（展示 API 用法的片段）：

```rust,ignore
source.then(work)                            // 每次只等待一个 work
source.map(work).buffered(8)                 // 至多 8 个在途，按输入顺序交付
source.map(work).buffer_unordered(8)         // 至多 8 个在途，按可获得结果交付
source.for_each_concurrent(8, work).await    // 并发处理，最终输出 ()
```

`buffered` 可能因最早那项很慢而暂缓交付后面已完成的结果；`buffer_unordered` 更早交付可用项，调用者须自行恢复业务顺序。它们处理的是“项为 Future 的 Stream”，不会自动 spawn 独立任务。处理 Future 被父任务轮询时才推进，所以 CPU 密集工作仍可能阻塞整个消费任务。`for_each_concurrent` 的限制针对正在处理的异步动作；futures 的此方法传 `None` 或零表示不限制，不宜拿零表示暂停。缓冲方法应传明确的正数，不依赖零容量的版本差异。

并发度限制不等于整个流水线有严格内存上界：每个结果大小、上游队列、重试和排序缓存都要计入。丢弃组合器会销毁其拥有的在途 Future；若这些 Future 只是 JoinHandle，底层已 spawn 的任务仍可能继续。

<a id="stream-combinators"></a>
## Stream 组合器

`map` 接收同步闭包，`then` 接收返回 Future 的闭包；futures 的 `filter`、`filter_map` 也接受异步判断。`take_until(stop)` 则直接接受一个 Future：它代表整个序列共用的停止条件，不是每项重新创建一次的闭包。`take(n)` 限制项数，`chain` 先耗尽前一个序列，`zip` 按位置配对，并可能被较慢的一侧限制。

同时监听多个来源可以用 `futures::stream::select(a, b)`、Tokio 的 `merge` 或 futures-concurrency 的 `Merge`。合并使下游在任一来源有值时就能处理，不承诺跨来源的业务顺序。`zip` 适合成对关系，不能代替事件合并。

[`StreamGroup`](https://docs.rs/futures-concurrency/latest/futures_concurrency/stream/struct.StreamGroup.html) 适合运行中增删来源：`insert(stream)` 返回键，`remove(key)` 移除来源，`keyed()` 可把来源键一起交给消费者。组中的具体流类型须一致；不同类型可装箱成同一 `BoxStream`。移除会销毁组所拥有的流，是否还须关闭远程订阅取决于协议。空组的结束与外部生产者将来可能添加来源是两个问题，动态订阅服务应另有控制消息与关闭状态，不能把“当前没有来源”直接当作系统永久结束。

<a id="joinselectrace-with-streams"></a>
### 使用 Stream 进行 join、select 与 race

`join!(a.next(), b.next())` 等待两边各一项；`select!` 或 race 等待其中一项。和只产生一个输出的 Future 不同，Stream 的一个 `Some` 并不结束 Stream。消息循环通常把两个流保留在循环外，每轮只新建短命的 next Future，收到 None 后禁用相应分支，或使用理解终止状态的融合组合器。

循环里重新创建拥有整个状态的 Stream 会丢掉未处理数据；一直轮询已结束的普通 Stream 可能 panic；反复让永远 ready 的停止分支参赛又会形成忙循环。futures 的 `select!` 通常要求分支 Future 具备 `FusedFuture`，可使用 `select_next_some` 和已 fuse 的 Stream；Tokio 的 `select!` 不要求这一 trait，应显式记录结束状态。不要把两个宏的要求混在一起。

取消 next 包装器一般保留底层流，因为包装器只是借用；但底层自定义 `poll_next` 仍必须正确保存等待过程中的状态。若每次 poll 都创建并丢弃一个需要多次轮询的内部 Future，即使编译成功，也可能永远无法完成。带中途副作用的自定义操作须另审取消安全性。对于单纯合并消息来源，merge/select 流通常比手写竞争循环更直接。

<a id="implementing-an-async-iterator"></a>
## 实现异步迭代器

下面是始终立即就绪的有限流，不需要注册唤醒：

```rust
{{#include ../../examples/guide-tests/src/streams.rs:counter}}
```

若下一项依赖异步工作，应把进度与等待 Future 保存为字段，向它转发 Context，并在 Pending 时保留状态。`poll_next` 不应阻塞线程或自行等待锁很久。需要固定的内部字段应安全投影，不能因为“示例简单”随意 `get_unchecked_mut`；参考篇的“实现 Future 与 Stream” 将给出完整等待实现。

不必总手写状态机。`stream::iter` 包装同步迭代器，`once` 把一次 Future 转为单项序列；[`unfold`](https://docs.rs/futures/latest/futures/stream/fn.unfold.html) 以状态和返回 Future 的闭包反复生成 `(item, 新状态)`，返回 None 结束。前面的 squares 即为此模式。

上游提纲提到 stream 宏。在稳定 Rust 中可使用第三方 [`async-stream`](https://docs.rs/async-stream/latest/async_stream/) 的 `stream!` / `try_stream!`，在宏内部写 await、yield 和 `?`。宏返回匿名 Stream，仍可能需要固定；宏内部的 yield/for await 语法不是稳定 Rust 在任意代码位置都能使用的语言功能。本章未新增该宏依赖或运行它；共享源码采用已编译的 unfold。

<a id="sinks"></a>
## Sink：反向的数据通道

[`Sink<Item>`](https://docs.rs/futures/0.3.34/futures/sink/trait.Sink.html) 表示异步接收项的目的地，常与 Stream 组成协议的收发两半。发送必须先 `poll_ready`，返回 Ready(Ok) 才能调用一次 `start_send`；Pending 时需要等待唤醒。这让接收方容量形成背压。`start_send` 只是交给 Sink，不保证已经写到连接或被对端业务确认。

`poll_flush` 确认按该 Sink 契约清空已有输出，`poll_close` 刷新并关闭；关闭后不继续发送。`SinkExt::send(item).await` 包含刷新，`feed(item).await` 允许积累后批量 flush。下例生产与消费并发推进，避免有界通道写满后双方互等：

```rust
{{#include ../../examples/guide-tests/src/streams.rs:sink}}
```

流控等待可以取消，但发送参数通常已经移入发送 Future，取消时值可能被销毁。需要保留待发消息时，应先获取容量许可或在业务层保存可重试记录。flush 完成也不等于对端提交事务；可靠交付仍需确认、幂等键和重试协议。丢弃 Sink 不能替代需要 await 的 close。

<a id="future-work"></a>
## 语言方向与当前边界

截至 2026-10-24，官方 [`std::async_iter::AsyncIterator`](https://doc.rust-lang.org/std/async_iter/trait.AsyncIterator.html) 文档仍标为 nightly-only，核心接口仍是 poll_next。稳定代码应使用生态 Stream，不能因为 trait 的名字出现在标准库文档就去掉特性门。

[RFC 2996](https://rust-lang.github.io/rfcs/2996-async-iterator.html) 解释了标准化动机及未来语法设想，是设计资料，不是当前稳定性承诺。async next 在消费侧很方便；poll_next 在实现侧便于直接转发、保存状态和动态分派。trait 中 async fn 的语言支持也不自动让一个新 async trait 与现有 Stream 互通。

普通 `for item in stream` 仍要求同步 IntoIterator。语言级异步迭代及生成器应分别核对特性状态；[`gen_blocks`](https://doc.rust-lang.org/unstable-book/language-features/gen-blocks.html) 仍列在 Unstable Book，不能把库宏的使用说明当作语言已经稳定。同步生成器与异步生成器也不是同一项保证。

最后，普通 Stream 的关联类型 Item 不随本次 `&mut self` 借用变化，因此无法表达“每次交出借用流内部缓冲区的切片，下一次迭代前必须归还”这种一般的 lending 关系。固定生命周期的外部引用可以作为 Item；借用自身的项则往往需要不同的、带泛型关联类型或回调的接口。这影响能否保留多个项、能否并发消费与是否需要复制，不能只把 `next` 改成 async 就解决。

本章四个行为测试覆盖惰性/固定/融合、有序与无序缓冲及并发限制、动态组移除与发送关闭、竞争失败后接收状态保留。实验语法与第三方宏仅核查文档，未计作已运行示例。
