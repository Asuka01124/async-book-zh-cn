# `select!` {#select}

`futures::select!` 宏会在当前任务中轮询多个 Future，并执行被选中的就绪分支。
它不会自动创建独立任务或线程；多个分支同时就绪时会伪随机选择一个，不能据此判断外部操作
在物理时间上严格的完成先后。这里讨论的是 [futures::select!](https://docs.rs/futures/latest/futures/macro.select.html)，
其他运行时的同名宏可能有不同约束。

```rust,edition2018
{{#include ../../examples/06_03_select/src/lib.rs:example}}
```

上面的函数会并发推进 `t1` 和 `t2`；选中分支调用 `println!` 后，函数结束，局部未完成 Future
随作用域退出而被丢弃。这里两个任务函数是空函数体的教学占位，都会立即就绪，示例不保证谁获选，
因此输出改为描述“本次被选中”。丢弃 Future 不会回滚副作用，也不保证终止它另行启动的后台工作。

`select` 的基本语法是 `<pattern> = <expression> => <code>,`，根据要在其中进行
`select` 的 future 数量重复书写即可。

## `default => ...` 与 `complete => ...` {#default---and-complete--}

`select` 还支持 `default` 和 `complete` 分支。

如果本次检查没有可选的就绪分支，且并非所有分支都已终止，就执行 `default`。
这样选择过程无需挂起等待就绪；但被轮询的代码和分支处理体仍可能耗时、阻塞线程或包含 `.await`，
不能承诺整个调用总是立即返回。循环里不断进入 `default` 还可能造成忙等。

当参与 `select` 的所有 future 都已完成、不会再取得进展时，可以用 `complete` 分支处理这种情况。
在循环中使用 `select!` 时，这通常很方便。所有分支都已终止时，`complete` 优先于 `default`。

```rust,edition2018
{{#include ../../examples/06_03_select/src/lib.rs:default_and_complete}}
```

## 与 `Unpin` 和 `FusedFuture` 的交互 {#interaction-with-unpin-and-fusedfuture}

你可能注意到，上面的第一个示例对两个 `async fn` 返回的 Future 调用了 `.fuse()`，并使用
`pin_mut!` 对它们进行固定。对这里通过变量名传入的 Future，宏要求接收者同时实现
`Unpin` 和 `FusedFuture`；固定后传给宏的 `Pin<&mut _>` 满足 `Unpin`，并不是把内部的 async Future
变成了 `Unpin`。`fuse()` 则提供完成状态跟踪。

通过变量名（路径）传入时，宏借用该 Future，因此尚未完成的 Future 可以在选择之后继续使用；
是否随后被丢弃取决于外层作用域。还有一个重要例外：直接传入产生 Future 的表达式，例如
`task_one().fuse()`，宏会把表达式结果固定在栈上，不要求该 Future 本身实现 `Unpin`，但仍要求
`FusedFuture`。这类临时 Future 的生命周期与外部持有、反复借用的 Future 不同。

同样，`select` 需要用 `FusedFuture::is_terminated()` 判断某个分支是否应排除，避免完成后再次轮询。
“已终止”并不保证曾返回输出：例如 `Fuse::terminated()` 和 `futures::future::pending()` 都会报告终止。
后者永不产生结果；若要演示一个仍被选择器视为活跃的等待，可使用 `pending().fuse()`。
实现了 `FusedFuture` 的 future 会跟踪自身是否已经完成。这样就可以在循环中使用 `select`，
只轮询尚未完成的 future。上面的示例展示了这一点：第二次进入循环时，`a_fut` 或 `b_fut`
已经完成。由于 `future::ready` 返回的 future 实现了 `FusedFuture`，它能够告知 `select`
不要再次轮询它。

需要跨多次选择保留完成状态时，应在循环外创建并 `.fuse()`，不要每轮重新包装同一个底层 Future，
否则新包装器丢失先前的完成状态。循环中每次新建一个全新的操作则是另一回事。

请注意，流有一个对应的 `FusedStream` 特性。实现了该特性的流，或经 `.fuse()` 包装的流，
其 `.next()` / `.try_next()` 组合器会产生实现 `FusedFuture` 的 Future（调用这些方法还要求接收者满足 `Unpin`）。
这些 Future 的终止状态来自底层流，所以例子在循环内重新创建 `next()` 仍能识别流已结束。
一个流结束后，循环继续处理另一个流，直到二者都结束。

```rust,edition2018
{{#include ../../examples/06_03_select/src/lib.rs:fused_stream}}
```

## 在 `select` 循环中结合 `Fuse` 与 `FuturesUnordered` 并发运行任务 {#concurrent-tasks-in-a-select-loop-with-fuse-and-futuresunordered}

有一个不太容易发现但很实用的函数：`Fuse::terminated()`。它可以构造一个已经终止的空 future，
之后可以将需要运行的 Future 放入其中。
它最初会被 `select!` 跳过，并非一个立即返回某个值的 Future。
示例的 `.set(...)` 是 [`Pin::set`](https://doc.rust-lang.org/std/pin/struct.Pin.html#method.set)：
它在原位置析构旧值，再放入同类型新值，因此可替换已固定的 `!Unpin` Future，而不移动旧值。
参见 [`Fuse::terminated`](https://docs.rs/futures/latest/futures/future/struct.Fuse.html#method.terminated)。

当某个任务需要在 `select` 循环期间运行，但又是在 `select` 循环内部创建时，这种方式会很方便。

请留意 [`select_next_some()`](https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html#method.select_next_some)：
它要求流实现 `FusedStream + Unpin`，返回的是直接产生 `Item` 的 Future，而不是产生 `Option<Item>`。
流终止后，该 Future 报告自己已终止，`select!` 会排除该分支；这并非简单吞掉 `None` 后继续等待。
如果绕过终止检查继续轮询它，会 panic，所以不能无条件对它 `.await` 来处理可能为空的流。
底层流若在本次轮询才首次报告结束，适配器会先返回 `Pending` 并唤醒任务，下一次选择再依据
终止状态排除分支。因此带 `default` 的代码可能先走一次 `default`，之后才走 `complete`。

```rust,edition2018
{{#include ../../examples/06_03_select/src/lib.rs:fuse_terminated}}
```

前一个例子只在取数操作空闲时启动新的 `get_new_num_fut`；取得新数后，会替换旧的
`run_on_new_num_fut`，丢弃它尚未完成的状态。这个“取消”效果只涉及该 Future，不保证停止后台工作或撤销副作用。
`interval_timer` 被假定为无限流，所以全部分支终止才 panic；这不是实际定时器实现，取数和处理函数也只是返回常量或空结果的占位。

如果需要同时保留同一种 Future 的多个实例，可以使用 `FuturesUnordered` 类型。下面的示例与
上一个示例类似，但新增操作时不会替换已有的 `run_on_new_num_fut`，而是保留它们供后续轮询至完成，
并打印各自返回的值。前提是循环持续运行并驱动该集合；退出或丢弃集合时仍会丢弃内部未完成 Future。

```rust,edition2018
{{#include ../../examples/06_03_select/src/lib.rs:futures_unordered}}
```

`FuturesUnordered::push` 只是加入 Future，不会轮询或自动 `spawn`；这里由
`select_next_some()` 所在分支驱动集合。结果按观察到就绪的顺序产生，不保证加入顺序或严格的物理完成顺序。
集合暂时排空时，该分支会被排除，但后续 `push` 可再次激活它；不要先把集合永久封装为一个已终止的流适配器，
再期待底层新增工作能自动恢复。

此处没有设置在途处理数量上限：若新数到达比处理完成更快，集合可能持续增长，需要另设并发限制或背压。
无限定时器的假设也意味着这两个 `run_loop` 没有正常退出协议；真实应用应明确停止、排空或取消策略，
参见[异步迭代器（Stream）](../part-guide/streams.md)与[取消与取消安全](../part-reference/cancellation.md)。
