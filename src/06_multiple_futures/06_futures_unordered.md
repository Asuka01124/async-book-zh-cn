<a id="futuresunordered"></a>
# 无序 Futures 集合（`FuturesUnordered`）

上游英文目录只有 `FuturesUnordered` 标题，没有正文。中文说明标题由 GPT-6 Luna 翻译；以下教程和示例由 GPT-6.1 Sol 原创补写，并非原英文正文的翻译。

`join!` 适合数量固定的一组工作。如果要同时推进一批请求，而且希望每得到一个结果就立即处理，不必等整批完成，可以使用 `futures::stream::FuturesUnordered`。它保存多个相同类型的 Future，并实现 `Stream`：每次得到一个已经就绪的结果，就移除对应的 Future，向消费者产出一个元素。

## 从加入工作到获取结果

```rust
{{#include ../../examples/guide-tests/src/legacy_unordered.rs:basic}}
```

两次 `job` 调用返回同一种 Future 类型，所以可以放入同一个集合。调用 async 函数只构造 Future；`push` 也不会轮询它，不会生成任务。只有开始消费集合，例如执行 `pending.next().await`，才会由集合的 `poll_next` 推进内部工作。新加入的工作需要先获得轮询，才能注册等待资源的唤醒通知。

当某个工作返回 Pending 时，集合会利用唤醒通知安排后续轮询，避免每次把所有等待中的工作都重新检查一遍。通知只表示“值得再检查”，不保证工作一定完成。消费者仍需持续驱动这个 Stream；把集合保存在变量中然后去等待不相关的事件，不会自动让这些普通 Future 在后台运行。

这里所有工作都由当前任务推进，没有自动 spawn，也没有自动分配线程。一个分支执行同步阻塞操作，仍会妨碍其他分支。示例用真实定时器接口模拟延迟，不执行真实请求；暂停时间的测试得到 `[2, 1]`。真实调度中，如果消费者很晚才继续轮询，多个工作可能同时已经就绪，不能断言总是观察到这个顺序。

“无序”表示不保留加入顺序，也不保证严格的物理完成时间顺序。需要在输出中带上请求 id、下标等关联信息，才能把结果送回对应业务。如果必须保留输入顺序，可以考虑 `FuturesOrdered` 或 `buffered`；代价是已完成的后续结果可能需要等待前面的工作。

## Future 类型与固定

同一个 async 函数的不同调用返回同一种类型，但两段独立的 `async { ... }` 即使输出相同，也通常是不同类型。需要异构工作时，可以统一为 `Pin<Box<dyn Future<Output = T> + Send + 'a>>`，或使用 `FutureExt::boxed()`；仅在当前线程使用且 Future 不满足 Send 时，可以用 `boxed_local()`。选择的生命周期应允许所有借用存活，不必一律写成 `'static`。也可以用枚举包装有限的几种工作，避免为统一异构类型额外装箱；集合内部仍可能为节点分配内存。

集合会为内部 Future 维护固定位置，因此普通 async 函数返回的非 Unpin Future 也可以直接加入。集合本身是 Unpin，可以用 `StreamExt::next(&mut collection)`；这不表示里面的 Future 也变成了 Unpin，更不表示可以随意移动正在执行的内部状态。

## 暂时为空与永久结束

集合没有任何工作时，`next().await` 返回 None；它不会等待未来有人加入。原始 `FuturesUnordered` 允许在返回 None 后再次 push，然后继续消费。下例使用同一具体 Future 类型展示这个特性：

```rust
# async fn example() {
use futures::{future, stream::FuturesUnordered, StreamExt};
let mut work = FuturesUnordered::new();
assert_eq!(work.next().await, None);
work.push(future::ready(7));
assert_eq!(work.next().await, Some(7));
# }
```

这不等于所有 Stream 都允许结束后重新轮询。特别是调用 `.fuse()` 得到的流适配器，会记住曾观察到的 None；即使通过其底层引用 push 新工作，适配器也会继续返回 None。如果需要在空集合上重新接收工作，保留原始集合并明确管理是否有活动分支，或设计一个同时等待输入通道与现有结果的调度循环。集合为空时应停用它的结果分支，否则立即返回 None 可能造成忙循环。使用前一节的 `futures::select!` 时，还要理解 `FusedStream` 的终止状态以及 push 后重激活的边界。

## 遇到错误该怎么办

`FuturesUnordered` 不会把 Result 当成特殊控制信号。如果元素输出 `Result<T, E>`，普通 `while let Some(result) = work.next().await` 会逐个产出 Ok 和 Err，你可以记录失败后继续处理其余工作。

如果希望一旦错误就退出，可以在循环里用 `result?`，或者消费 `work.try_collect::<Vec<_>>().await`。前者是否销毁集合取决于集合所属作用域；后者按值取得集合，返回错误后丢弃内部剩余 Future。若使用 `(&mut work).try_collect()`，被丢弃的只是借用，集合仍由外部拥有者管理。两种策略都不会回滚已经发生的外部效果。

如果集合装的是 `tokio::task::JoinHandle`，被销毁的是等待句柄，后台任务会分离并继续执行。需要停止它们时，应保留可中止的句柄并明确 abort、join 与清理策略，或选用具有所需任务生命周期语义的任务集合。Result 也可能有两层：任务自身的业务错误与 JoinError 需要分别处理。具体取消边界见[取消与超时](05_cancellation_and_timeouts.md)。

## 给并发数量设置上限

`FuturesUnordered` 的名称不包含容量策略：push 一万项就可能持有一万个未完成工作。它不会自动限制连接数、内存或远程服务压力。对于一个输入流，可用 `buffer_unordered` 限制它同时保存的内部 Future 数量：

```rust
{{#include ../../examples/guide-tests/src/legacy_unordered.rs:limited}}
```

这里最多同时保存两个工作 Future。一个结果被消费后，适配器才有空间继续拉取输入；示例最终仍收集全部六个结果，所以输出 Vec 本身会增长。应使用正数容量，不要把不同组合器的零值语义混用；本节明确选择 2。

限制只覆盖适配器拥有的工作。如果输入早已通过 spawn 启动一万个任务，再对这些 JoinHandle 使用 `buffer_unordered(2)`，只能限制同时等待多少句柄，不能撤回已经启动的任务。输入源本身也可能已有庞大缓冲区。需要真正控制启动量时，应把创建任务放在受限消费过程中，或手动维护集合长度：先填至上限，等待一个结果，再补一项。对服务的持续输入还应配合有界队列和关闭策略，避免只限制了执行数却无限积压请求。

## 运行与进一步阅读

运行 `cargo test --manifest-path examples/guide-tests/Cargo.toml legacy_unordered --locked --offline` 可验证：push 不执行工作、初次消费确实 Pending、按控制信号观察后加入项先完成、空集合恢复与 fuse 保持结束、缓冲容量限制新工作的推进，以及错误短路销毁拥有的剩余工作。定时器测试使用暂停时间，不把测试顺序冒称为真实网络的完成顺序。

接口依据：[FuturesUnordered](https://docs.rs/futures/latest/futures/stream/struct.FuturesUnordered.html)、[StreamExt::buffer_unordered](https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html#method.buffer_unordered)。可继续阅读[并发组合 Future](../part-guide/concurrency-primitives.md)与[迭代与并发](../05_streams/02_iteration_and_concurrency.md)，比较不同消费策略。
