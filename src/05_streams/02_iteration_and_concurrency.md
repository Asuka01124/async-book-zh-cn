# 迭代与并发 {#iteration-and-concurrency}

与同步的 `Iterator` 类似，迭代并处理 `Stream` 中的值有许多不同方式。可以使用
`map`、`filter` 和 `fold` 等组合器风格的方法。处理元素为 `Result<T, E>` 的流时，
还可以使用 `TryStreamExt` 提供的 `map_ok`、`and_then`、`try_filter` 和 `try_fold` 等方法。

原文这里写作 `try_map`，但它不是本书使用的 `futures` 0.3 的 `TryStreamExt` 方法，因而校订为
`map_ok`（同步变换成功值）与 `and_then`（用返回可失败 Future 的操作变换成功值）。它们以及
`try_filter` 会把错误作为流元素传给下游，并不统一表示整个流立即终止；`try_fold` 等消费操作则会在
遇到错误时提前返回。接口细节见 [TryStreamExt 文档](https://docs.rs/futures/0.3.34/futures/stream/trait.TryStreamExt.html)。

普通 `for` 循环使用的是同步 `IntoIterator` 协议，不能直接遍历 `Stream`。对于命令式风格的代码，可以使用
`while let` 以及 `next`/`try_next` 函数：

```rust,edition2018,ignore
{{#include ../../examples/05_02_iteration_and_concurrency/src/lib.rs:nexts}}
```

`next().await` 返回 `Option<Item>`；`try_next().await` 则把结果整理为
`Result<Option<T>, E>`，所以示例用 `?` 传播错误，再用 `while let` 判断流是否结束。
这里传入的是已经固定的可变流引用；`next`/`try_next` 对接收者的 `Unpin` 要求由该引用满足，
并不要求被引用的 `dyn Stream` 本身实现 `Unpin`。

如果循环体在处理完当前元素后才取下一项，该循环内的处理便是顺序执行的。需要同时推进多个处理操作时，可以使用 `for_each_concurrent` 和
`try_for_each_concurrent` 方法：

```rust,edition2018,ignore
{{#include ../../examples/05_02_iteration_and_concurrency/src/lib.rs:try_for_each_concurrent}}
```

参数 `MAX_CONCURRENT_JUMPERS` 将同时保留的未完成处理 Future 限制为 100 个；达到上限后，
组合器先推进已有处理，再继续从源流取得元素。传入 `1` 就是逐项处理；传入 `0`、`Some(0)` 或
`None` 都表示不设置并发上限，而不是不执行任何操作。无限制并发可能积累大量内存和外部请求。
这些语义见 [for_each_concurrent 文档](https://docs.rs/futures/0.3.34/futures/stream/trait.StreamExt.html#method.for_each_concurrent)。

这些子 Future 在同一个调用任务中被交错轮询，组合器本身不会 `spawn` 任务或创建线程。
同一元素内仍按顺序先执行 `jump_n_times`，成功后再执行 `report_n_jumps`；不同元素之间的完成和报告顺序没有保证。
阻塞调用或长时间不让出的 CPU 计算仍会阻碍同任务内其他 Future，不能靠提高此上限解决。

`try_for_each_concurrent` 在源流或任一处理 Future 返回错误时，立即返回它观察到的错误，
并丢弃内部尚未完成的子 Future，不等待它们收尾，也不会收集所有错误。这里组合器持有的是流的
可变借用，返回时释放借用，并不销毁调用方拥有的源流；已从源流取出的元素也不会自动放回。
丢弃 Future 不会回滚已经发生的跳跃、报告或其他外部副作用，也不保证停止它另行启动的后台工作。
需要完成全部处理、聚合错误或显式清理时，应另行设计策略，参见[取消与取消安全](../part-reference/cancellation.md)。

示例中的 `jump_n_times` 与 `report_n_jumps` 是返回 `Ok(())` 的教学占位函数，没有实际跳跃、
报告、I/O 或挂起行为，因此这段代码展示的是 API 组织方式，不能用来验证真实并发交错。
可以结合[异步迭代器（Stream）](../part-guide/streams.md)中的完整示例理解有界并发与完成顺序。
