# `join!` {#join}

`futures::join!` 宏可以让多个不同的 Future 并发执行，并等待它们全部完成。

## `join!` {#join-1}

执行多个异步操作时，很容易想到按顺序逐个对它们使用 `.await`：

```rust,edition2018,ignore
{{#include ../../examples/06_02_join/src/lib.rs:naiive}}
```

这里直到 `get_book` 完成后才调用 `get_music`。当两个操作彼此独立且等待可以重叠时，
顺序等待可能错失并发机会；若后者依赖前者的结果，顺序执行就是合理的，并不能一概断言它更慢。
某些语言的异步函数调用会启动或调度操作，所以先调用两个异步函数、再逐个等待返回对象，
也可能让操作并发推进。下面保留这种常见写法，用于说明它与 Rust 的区别：

```rust,edition2018,ignore
{{#include ../../examples/06_02_join/src/lib.rs:other_langs}}
```

Rust 的 `async fn` 调用会构造 Future，其函数体要等到 Future 被轮询时才开始执行；
`.await` 是驱动轮询的一种方式，也可以由执行器或组合器驱动。这里只构造两个 Future 后按顺序
等待，仍然会先完成取书，再推进取音乐，不能由此实现两个函数体的并发执行。
这不代表所有 Future 关联的工作都只能在被等待后启动：构造参数、普通函数中的同步工作，
以及已经交给运行时的后台任务可能早已发生。需要让这里两个操作并发推进时，可使用 `futures::join!`：

```rust,edition2018,ignore
{{#include ../../examples/06_02_join/src/lib.rs:join}}
```

`join!` 返回一个元组，按参数顺序包含每个 `Future` 的输出，与它们完成的先后无关。
它在当前任务中轮询子 Future，不会自动 `spawn` 或创建线程；某个分支阻塞线程或长时间执行 CPU 计算，
仍会妨碍其他分支推进。这里讨论的是 [futures::join!](https://docs.rs/futures/latest/futures/macro.join.html)，
不要把其他运行时同名宏的调度选项直接套用过来。

示例中的取书、取音乐函数只是立即返回值的教学占位，`Result` 版本也只返回 `Ok`；
它们没有真实 I/O 或挂起，不构成耗时对比或并发交错验证。可结合[并发组合 Future](../part-guide/concurrency-primitives.md)
中的完整示例理解等待重叠；`async` 函数体的执行规则见 [Rust Reference](https://doc.rust-lang.org/reference/expressions/block-expr.html#async-blocks)。

## `try_join!` {#try_join}

对于返回 `Result` 的 Future，可以考虑使用 `try_join!`，而不是 `join!`。由于 `join!`
只有在所有子 Future 都完成后才会结束，即使其中一个子 Future 已返回 `Err`，它仍会继续处理
其他 Future。

与 `join!` 不同，[futures::try_join!](https://docs.rs/futures/latest/futures/macro.try_join.html)
会在轮询时观察到某个子 Future 返回错误后立即返回该 `Err`；若全部成功，则返回 `Ok((...))`。
多个错误都已就绪时，不应把返回的错误解释为外部操作在物理时间上最早发生的错误。

```rust,edition2018,ignore
{{#include ../../examples/06_02_join/src/lib.rs:try_join}}
```

提前返回会丢弃宏所拥有的其他未完成 Future，不会等待它们完成。如果传入的是已固定 Future 的
可变借用，丢弃的是借用，原 Future 仍由调用方拥有，之后仍可继续驱动。
丢弃 Future 或任务句柄的效果由其类型决定，不等于自动终止另行运行的后台任务，也不会撤销已经产生的
外部副作用；相关边界见[取消与取消安全](../part-reference/cancellation.md)。

请注意，传给 `try_join!` 的 Future 必须具有相同的错误类型。可以使用
`futures::future::TryFutureExt` 中的 `.map_err(|e| ...)` 和 `.err_into()` 函数来统一
错误类型：

```rust,edition2018,ignore
{{#include ../../examples/06_02_join/src/lib.rs:try_join_map_err}}
```
