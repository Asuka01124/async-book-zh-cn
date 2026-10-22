<a id="composing-futures-concurrently"></a>
# 并发组合 Future

本章将介绍更多组合 Future 的方式，尤其是一些并发（但不并行）执行 Future 的新方式。从表面上看，本章要介绍的新函数/宏都很简单，但其背后的概念可能相当微妙。我们会先回顾 Future、并发和并行；你也可以重读前面比较[并发与并行](concurrency.md#concurrency-and-parallelism)的章节。

Future 是一种延后执行的计算。使用 `await` 可以推动 Future 向前执行；如果暂时无法就绪，它会使外层 Future 返回 Pending，让执行器重新安排工作；立即就绪的 await 不必让出控制权。如果 `a` 和 `b` 是 Future，可以先 await 一个再 await 另一个来顺序组合它们（也就是组合成一个先完整执行 `a`、再完整执行 `b` 的 Future）：`async { a.await; b.await}`。

我们还见过使用 `spawn` 组合可能并行的 Future：`async { let a = spawn(a); let b = spawn(b); (a.await, b.await)}` 会并发运行这两个 Future，并可能并行。请注意，元组中的 `await` 等待的不是 Future 本身，而是 `JoinHandle`，以便在 Future 完成时取得其结果。

本章介绍两种并发组合 Future、但不进行并行的方法：`join` 和 `select`/`race`。两种情况下，Future 都通过分时方式并发运行：组合中的各个 Future 轮流执行。它不生成独立的运行时任务，而是在同一个任务的 poll 中交错推进各分支，因此这些分支的轮询本身不并行；外层任务仍由执行器驱动，IO 与唤醒也可能依赖运行时。组合结构会在本地交错执行这些 Future。你可以把它们看作小型执行器，在单个异步任务中执行各个组成 Future。

join 和 select/race 的根本区别在于如何处理 Future 完成工作：join 会等所有 Future 都完成后结束；select/race 则会在某个 Future 完成时结束（它拥有的其他分支 Future 被丢弃；借用与任务句柄的情况见下文）。两者也都有用于处理错误的变体。

这些结构（或类似概念）常与流一起使用；下文会略作介绍，更多内容请见[流章节](streams.md)。

如果需要独立调度或计算并行，可以考虑创建任务。它把每个任务直接交给运行时调度，但会增加所有权与 Send/static 约束；同任务组合则便于借用局部数据并明确控制生命周期。两者的性能和易用性取决于工作负载，不能只凭是否可能并行做统一选择。另一方面，创建任务本质上缺少[结构化](../part-reference/structured.md)特性，因此生命周期和资源管理可能更难推理。

值得更深入地考虑一下性能问题。并发组合可能带来的性能问题在于分时是否公平。为说明调度层次，可以设想 100 个相同任务平均分享时间，每个约 1%；若改成两个任务、其中一个内部 join 99 个 Future，再假设两个任务各得 50%，那 99 个分支平均各约 0.5%。这只是一个极度简化的假想模型，**不是运行时提供的 CPU 配额或唤醒概率保证**。真实结果取决于每次 poll 的工作量、就绪情况、调度策略和核心数量。关键是运行时调度的是外层任务，而不是自动为每个内部 Future 提供独立调度配额。

通常任务分布不会如此偏斜，而且我们经常将 join/select 等用于超时场景，此时这种行为实际上正合所需。不过仍值得考虑这一点，以确保程序具备你期望的性能特征。


## Join

Tokio 的 [`join` 宏](https://docs.rs/tokio/latest/tokio/macro.join.html)接收一组 Future，并发运行它们直至全部完成（以元组形式返回所有结果）。所有 Future 都完成后它才会返回。这些分支在同一个任务中被轮询，同一时刻不会并行；若外层任务可迁移到其他工作线程，后续轮询也可能换线程，所以不保证整个生命周期固定在同一线程。

下面是一个简单的例子：

```rust,edition2021
{{#include ../../examples/guide-tests/src/composition.rs:join-example}}
```

这里两个分支并发推进，二者都完成后结果元组 `(1, 2)` 便可用。请注意，我们没有通过 `await` 获取结果。`join!` 会隐式 await 其中的 Future 并产生一个值。从调用方看，整个宏表达式直接产生结果元组，而不是返回一个需要另外 await 的 Future；宏内部仍通过 Future 实现组合等待。不过仍然需要在异步上下文中使用它（例如在异步函数内部）。

虽然上面的例子中看不出来，`join!` 接收的是求值结果为 Future 的表达式[^into]。`join` 不会在其主体中创建异步上下文，而且不应 await 传给 `join` 的 Future（否则它们会在被组合的 Future 之前求值）。

由于这些分支共享一次任务轮询，只要其中一个 Future 阻塞当前线程，它们就都无法取得进展。如果使用互斥锁或其他锁，一个 Future 等待另一个 Future 持有的锁时，很容易造成死锁。

[`join`](https://docs.rs/tokio/latest/tokio/macro.join.html) 不关心 Future 的结果。特别是，某个分支返回 Result::Err 不会让其他分支停止。如果分支是 JoinHandle，任务被中止会表现为这个句柄的错误值。反过来，若外层 join 所在的 Future 被丢弃，它所拥有的所有未完成分支也会被丢弃。如果需要“快速失败”行为，请使用 [`try_join`](https://docs.rs/tokio/latest/tokio/macro.try_join.html)。`try_join` 与 `join` 类似，但只要有一个 Future 返回 `Err`，它拥有的其他未完成分支会被丢弃，`try_join` 也会立即返回该错误。

在前面关于 [async/await](async-await.md) 的章节中，我们用“join”来描述等待创建的任务完成。顾名思义，join Future 和 join 任务彼此相关：join 意味着并发执行多个 Future，并在继续之前等待结果。语法有所不同：等待独立任务使用 `JoinHandle`，同任务组合 Future 使用 `join` 宏，但思路类似。关键区别在于，join 任务时，任务会并发且可能并行执行；而使用 `join!` 时，Future 并发执行但不并行。此外，创建的任务由运行时调度器调度，而 `join!` 中的 Future 则在本地“调度”（位于同一个任务中，且处于宏执行的时间范围内）。另一个区别是，如果创建的任务发生 panic，运行时会捕获该 panic；但如果 `join` 中的 Future 发生 panic，整个任务都会 panic。


<a id="alternatives"></a>
### 替代方案

并发运行 Future 并收集其结果是常见需求。若需要独立调度，可使用 `spawn` 和 `JoinHandle`；需要非 Send 的本地任务时，可考虑 [`spawn_local`](https://docs.rs/tokio/latest/tokio/task/fn.spawn_local.html)，但它仍要求 static。若任务需要借用局部数据，或希望由同一父 Future 明确控制生命周期，join 通常更合适。[`JoinSet`](https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html) 抽象可以管理这些生成的任务，并按完成顺序获取结果；丢弃 JoinSet 会请求中止集合内的任务，它与丢弃单个 JoinHandle 的分离行为不同。

大多数运行时（以及 [futures.rs](https://docs.rs/futures/latest/futures/macro.join.html)）都有与 Tokio `join` 宏等效的功能，行为大致相同。也有 `join` 函数，功能类似宏，但灵活性稍差。例如，futures.rs 提供了用于组合两个 Future 的 [`join`](https://docs.rs/futures/latest/futures/future/fn.join.html)，用于组合相应数量 Future 的 [`join3`](https://docs.rs/futures/latest/futures/future/fn.join3.html)、[`join4`](https://docs.rs/futures/latest/futures/future/fn.join4.html) 和 [`join5`](https://docs.rs/futures/latest/futures/future/fn.join5.html)，以及用于组合 Future 集合的 [join_all](https://docs.rs/futures/latest/futures/future/fn.join_all.html)（这些函数也都有对应的 `try_` 变体）。

[Futures-concurrency](https://docs.rs/futures-concurrency/latest) 也提供 join（和 try_join）功能。在 futures-concurrency 的风格中，这些操作是 Future 组合（例如 tuple、`Vec` 或数组）上的 trait 方法。例如，要 join 两个 Future，可以写成 `(fut1, fut2).join().await`（请注意，这里的 `await` 是显式的）。

如果要 join 的 Future 集合会动态变化（例如随着网络输入到达而创建新的 Future），或者你想在各个 Future 完成时就取得结果，而不是等全部完成后再统一获取，那么就需要使用流以及 [`FuturesUnordered`](https://docs.rs/futures/latest/futures/stream/struct.FuturesUnordered.html) 或 [`FuturesOrdered`](https://docs.rs/futures/latest/futures/stream/struct.FuturesOrdered.html) 功能。我们会在[流](streams.md)一章中介绍它们。


[^into]: 本章锁定验证的 Tokio 1.54.1 支持实现 `IntoFuture` 的表达式；不能据此假定所有历史版本或其他库的 join 都相同。宏会对表达式求值，并将其转换为 Future。也就是说，表达式的求值结果不一定必须是 Future，也可以是能够转换为 Future 的值；不过这只是很细微的区别。表达式本身会按顺序求值，然后才执行由此产生的 Future。


## Race/select

与 join Future 对应的操作是让它们竞争（也称为在它们之间进行选择）。使用 race/select 时，Future 会并发执行；但我们不再等待所有 Future 完成，而是等待最先完成且满足分支模式的结果，然后丢弃其他分支表达式产生的 Future。若这些只是指向外部 Future 的引用，被引用的 Future 仍保留；若是 JoinHandle，后台任务仍可继续。虽然这听起来与 join 类似，但它有趣得多（有时也更容易出错），因为此时必须考虑取消。

下面是使用 Tokio [`select`](https://docs.rs/tokio/latest/tokio/macro.select.html) 宏的示意片段；do_a_thing 和 timeout 省略了定义，随后会给出可测试的实际实现：

```rust,ignore
async fn main() {
  select! {
    result = do_a_thing() => {
      println!("computation completed and returned {result}");
    }
    _ = timeout() => {
      println!("computation timed-out");
    }
  }
}
```

你会发现，它已经比 `join` 宏更有意思，因为我们在 `select` 宏内部处理 Future 的结果。它看起来有点像 `match` 表达式，但在 `select` 中，所有分支会并发运行，最先完成的分支会带着其结果执行分支主体（其他分支不会执行，其 Future 会通过 `drop` 被取消）。在这个例子中，`do_a_thing` 和 `timeout` 并发执行，先完成的那个会执行其代码块（也就是说，只有一条 `println` 会运行），另一个 Future 则会被取消。与 `join` 宏一样，await Future 是隐式进行的。

Tokio 的 `select` 宏支持许多功能：

- 模式匹配：每个分支中 `=` 左侧的语法可以是一个模式；只有 Future 的结果与模式匹配时，代码块才会执行。如果模式不匹配，该 Future 就不会再被轮询（但其他 Future 仍会继续）。这对可能返回某个值、也可能不返回值的 Future 很有用，例如 `Some(x) = do_a_thing() => { ... }`。
- `if` 守卫：先计算所有分支的守卫，再对各分支的异步表达式求值以生成 Future。即使守卫为 false，表达式也仍会求值，只是不轮询得到的 Future。例如，`x = do_a_thing(), if false => { ... }` 会调用 do_a_thing 来创建 Future；若它是 async fn，函数体不会运行。若表达式是有同步副作用的普通函数，那些副作用仍会发生。守卫只在本次宏调用初始化时计算，轮询时不重新计算。
- `else` 分支：`select` 可以包含 `else` 分支 `else => { ... }`；如果所有分支都因守卫为 false 或结果模式不匹配而被禁用，就会执行该分支。如果此时没有 `else` 分支，`select` 就会 panic。

`select!` 宏的值就是已执行分支的值（与 `match` 一样），因此所有分支都必须具有相同类型。例如，如果要在 `select` 外部使用上例的结果，可以这样写：

```rust,edition2021
{{#include ../../examples/guide-tests/src/composition.rs:select-example}}
```

与 `join!` 一样，除了前面提到的模式匹配之外，`select!` 不会以特殊方式处理 `Result`。某个分支返回错误时，是否匹配取决于左侧模式；若匹配，就执行该分支的处理表达式，宏返回的是处理表达式的值，而不一定是那个 Err。比如处理代码可以记录错误后返回 None，或把 Err 转换成其他值。

`select` 默认会丢弃未选中的分支 Future，因此需要设计取消行为；但把长期存在的 Future 固定后以 &mut 传入，可以保留原 Future，在竞争失败后继续使用它。事实上，`select` 往往是异步程序中取消的主要来源。如[其他章节](../part-reference/cancellation.md)所述，取消存在许多微妙的问题，可能导致 bug。特别要注意，`select` 丢弃的是它当前持有的分支值；移动进去的 Future 被销毁，借用值被销毁则只结束本次借用。这不会通知被丢弃的 Future，也不会触发任何取消令牌等机制。

`select!` 常在循环中用于处理流或其他 Future 序列。这会增加一层复杂度，也带来更多出错机会。如果每次循环迭代都创建新的、相互独立的 Future，情况不会复杂太多。不过，我们很少真的需要这样。通常希望在迭代之间保留一些状态。常见做法是将 `select` 与流一起放在循环中，每次迭代处理流中的一个结果。例如：

```rust,edition2021
{{#include ../../examples/guide-tests/src/composition.rs:idle-timeout}}
```

在这个例子中，我们从 `stream` 读取并收集值（调用者也可以逐项打印），直到没有剩余值，或等待结果超时。这里每次迭代都会重新创建超时，所以它限制的是相邻数据之间的空闲时间。futures 的 StreamExt::next 本身取消安全：取消等待不会丢弃仍在 stream 对象里的状态。退出循环后，如果调用方继续保留 stream，就仍可继续使用它；若连流本身也丢弃，底层资源和剩余数据的处理就取决于其实现。不要把 next 的保证推广到任意自定义读取函数。

我们可能希望在多次迭代间重用某个 Future，而不只是流。例如，我们可能希望与一个超时 Future 竞争，并让超时时间覆盖所有迭代，而不是每次迭代都重新计时。可以在循环外创建 Future 并通过引用使用它：

```rust,edition2021
{{#include ../../examples/guide-tests/src/composition.rs:total-timeout}}
```

在循环中使用 `select!`，并将循环外创建的 Future 或流传入其中时，有几个重要细节需要注意。这些是 `select` 工作方式所带来的根本结果，因此我会以最后一个例子中的 `timeout` 为例，逐步介绍 `select` 的工作过程。

- `timeout` 在循环外创建，并初始化为需要倒计时的一段时间。
- 每次循环迭代时，`select` 都会创建一个指向 `timeout` 的引用，但不会改变它的状态。
- `select` 执行时会轮询 `timeout`：剩余时间未耗尽时返回 `Pending`，时间耗尽时返回 `Ready`，此时就会执行它的代码块。

在上面的例子中，`timeout` 就绪后，我们会通过 `break` 跳出循环。但如果没有这样做呢？此时 `select` 会再次轮询 `timeout`，而 [`Future` 文档](https://doc.rust-lang.org/std/future/trait.Future.html#tymethod.poll)指出不应这样做！`select` 无法避免这种情况，因为它没有保存跨迭代的状态来判断是否应该轮询 `timeout`。根据 `timeout` 的实现方式，这可能导致 panic、无限等待或其他错误；即使再次轮询也不得产生未定义行为。某些特定类型允许额外操作，应以其文档为准；不要据此把任意 Future 当作可重复使用。

可以通过几种方式避免这类 bug：

- 使用 [fused Future](futures.md#fusing)（通过 [FutureExt::fuse](https://docs.rs/futures/latest/futures/future/trait.FutureExt.html#method.fuse)）或[流的 fuse](https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html#method.fuse)。Future 的 Fuse 在完成后返回 Pending；流的 Fuse 在结束后返回 None。后者若仍在循环中匹配 None，必须退出或禁用分支，否则可能忙循环。
- 确保代码结构不会重复轮询 Future，例如跳出循环（如前一个例子所示），或使用 `if` 守卫。

现在来考虑 `&mut timeout` 的类型。假设 `timeout()` 返回一个实现了 `Future` 的类型，它可能是异步函数返回的匿名类型，也可能是名为 `Timeout` 这样的具名类型。为方便举例，假设是后者（不过两种情况的原理相同）。既然 `Timeout` 实现了 `Future`，那么 `&mut Timeout` 也实现 `Future` 吗？不一定！有一个[通用 `impl`](https://doc.rust-lang.org/std/future/trait.Future.html#impl-Future-for-%26mut+F) 可以让它成立，但前提是 `Timeout` 实现 `Unpin`。并非所有 Future 都满足这一条件，因此按照上一个例子那样写代码时，经常会遇到类型错误。不过，可以使用 `pin` 宏轻松修复这类错误，例如 `let mut timeout = std::pin::pin!(timeout());`。上面的完整示例使用 tokio::pin!，然后传入 &mut timeout，从而也支持非 Unpin 的 Sleep。

在循环中使用 `select` 取消 Future，是微妙 bug 的常见来源。这类问题通常发生在 Future 保存了与某些数据相关的状态，却没有保存数据本身时。Future 因取消而被丢弃后，状态会丢失，但底层数据并未更新。这可能导致数据丢失或被重复处理。


<a id="alternatives-1"></a>
### 替代方案

Futures.rs 提供了自己的 [`select` 宏](https://docs.rs/futures/latest/futures/macro.select.html)，而 futures-concurrency 提供了 [Race trait](https://docs.rs/futures-concurrency/latest/futures_concurrency/future/trait.Race.html)；两者都可以替代 Tokio 的 `select` 宏。它们的核心语义相同：让多个 Future 并发竞争，处理最先完成的结果，并取消其他 Future；但语法不同，细节也有所差异。

Futures.rs 的 `select` 表面上与 Tokio 的版本相似；概括来说，futures.rs 版本有以下不同：

- Future 必须始终是 fused（由类型检查强制保证）。
- `select` 使用 `default` 和 `complete` 分支，而不是 `else` 分支。
- `select` 不支持 `if` 守卫。

Futures-concurrency 的 `Race` 语法截然不同，类似于它提供的 `join` 版本，例如 `(future_a, future_b).race().await`（它支持 `Vec` 和数组，也支持 tuple）。这种语法不如宏灵活，但很容易融入大多数异步代码。请注意，如果在循环中使用 `race`，仍然可能遇到与 `select` 相同的问题。

与 `join` 一样，需要独立任务调度时，也可以创建任务并让它们可能并行执行；这需要另外管理生命周期。不过，第一个任务完成后，要取消其余任务还需要一些额外工作。使用通道或取消令牌可以实现协作退出，让任务有机会清理。也可以对普通异步任务请求 abort，再 await 句柄确认结束；这通常只保证析构清理。仅丢弃竞争失败的 JoinHandle 不会停止后台任务，try_join 处理 JoinHandle 时还要区分外层 JoinError 与任务返回的内层 Result。

`select` 的一种常见用途（尤其是在循环中）是处理流。有些流组合器方法可以替代部分 `select` 用法。例如，futures-concurrency 中的 [`merge`](https://docs.rs/futures-concurrency/latest/futures_concurrency/stream/trait.Merge.html) 就是合并多个流的良好替代方案。


<a id="final-words"></a>
## 最后说明

本节介绍了并发运行一组 Future 的两种方式。Join Future 意味着等待所有 Future 完成；select（也称为 race）Future 意味着等待第一个 Future 完成。与创建任务不同，这些组合方式不利用并行。

`join` 和 `select` 都作用于预先已知的 Future 集合（通常在编写程序时就已知，而不是等到运行时）。有时要组合的 Future 并非预先可知——执行期间还需要向组合集合中添加 Future。此时可以使用流及其[组合操作](streams.md)，例如 FuturesUnordered；独立生成的动态任务也可使用 JoinSet。函数形式的 join_all 还能接收运行时才构造的集合，不能把宏的固定分支数推广为所有 join 工具的限制。

选择组合还是 spawn，应依据是否需要并行、能否满足所有权约束、取消与资源管理方式以及测量到的调度表现。同任务组合便于借用并保持结构；spawn 便于独立调度。两种方法都需要避免阻塞并明确处理任务结束，不能保证其中一种天然更少出错或资源分配更公平。

> 校订说明：本章保留上游 join/select/race 及替代工具的完整讨论，校正固定线程、运行时参与、公平时间配额、守卫顺序、模式匹配与 Result、借用式竞争和后台任务取消等说法。可运行示例来自 guide-tests；29 个累计行为测试包括本章的 10 个测试，验证当前 Tokio 的 IntoFuture 支持、错误传播层次、分支创建顺序、取消与保留状态、总期限与空闲超时、Fuse 完成行为。依据见 [Tokio join](https://docs.rs/tokio/latest/tokio/macro.join.html)、[select](https://docs.rs/tokio/latest/tokio/macro.select.html)、[try_join](https://docs.rs/tokio/latest/tokio/macro.try_join.html)、[JoinSet](https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html)及 [futures select](https://docs.rs/futures/latest/futures/macro.select.html) 文档。IntoFuture 的版本细节同时核对了 [Tokio maybe_done 源码](https://docs.rs/tokio/latest/src/tokio/future/maybe_done.rs.html)。
