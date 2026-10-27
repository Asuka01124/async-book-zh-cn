# `async`/`.await` {#asyncawait}

在[第一章]中，我们简要了解了 `async`/`.await`。本章将更详细地讨论 `async`/`.await`，说明它的工作原理，以及 `async` 代码与传统 Rust 程序有何不同。

`async`/`.await` 是 Rust 中的特殊语法，使我们能够让出当前线程的控制权，而不是阻塞线程；在等待某项操作完成时，其他代码因此可以继续推进。

本节介绍两种主要形式：`async fn` 和 `async` 块。调用异步函数或求值异步块都会得到实现了 `Future` trait 的值；异步闭包则在被调用时返回 Future：

```rust,edition2018,ignore
{{#include ../../examples/03_01_async_await/src/lib.rs:async_fn_and_block_examples}}
```

正如第一章所述，调用 `async fn` 或求值 `async` 块会构造 Future，其函数体或块体在轮询时才开始执行。调用参数的求值、同步构造函数以及独立线程或任务可能已经产生外部工作，不能把“惰性”扩大成所有工作都未开始。

使用 Future 最常见的方式是 `.await`。严格来说，它先通过 `IntoFuture` 转换操作数，再在当前任务的上下文中轮询。如果得到 `Ready`，便立即产生结果并继续执行；只有得到 `Pending` 才挂起当前异步计算，把控制权交回调用者，最终可回到执行器。因此 `.await` 不必每次都让出线程，也不创建独立任务。等待资源的通知使执行器稍后重新轮询任务；任务内部则恢复相应的等待点。同步阻塞调用不会因为放在 `async` 中就自动变成非阻塞操作。

## `async` 的生命周期 {#async-lifetimes}

接收引用或其他非 `'static` 参数的 `async fn` 会返回一个生命周期受这些参数约束的 `Future`：

```rust,edition2018,ignore
{{#include ../../examples/03_01_async_await/src/lib.rs:lifetimes_expanded}}
```

这意味着，持有借用的 Future 不能活得比被借用数据更久；若要把它驱动至完成，借用数据必须在完成前保持有效。它也可以在有效期内被丢弃，不要求每个 Future 都必须 await。通常，在调用函数后立即对 Future 使用 `.await`（例如 `foo(&x).await`）不会有问题。但如果要保存这个 Future，或将它传给另一个任务或线程，就可能遇到问题。

一种常用的解决办法是，将引用参数与对 `async fn` 的调用一起放进 `async` 块，从而把接收引用参数的 `async fn` 转换成 `'static` Future：

```rust,edition2018,ignore
{{#include ../../examples/03_01_async_await/src/lib.rs:static_future_with_borrow}}
```

`bad` 返回了借用局部 `x` 的 Future，而 `x` 会在同步函数返回时销毁，因此无法编译。`good` 在异步块内部创建 `x`，并在块内等待借用它的子 Future；外层 Future 自己管理这个局部值，所以没有借用已经返回的调用栈。这里不是把原来外部变量的生命周期凭空延长，也不是给所有借用加上 `'static`。

`Future: 'static` 表示 Future 不包含有效期不足的外部借用，不表示它必须运行到进程结束。所引源码里的 `bad` 原本被 `never_compiled` 配置排除，正常编译包不会验证它的拒绝行为；本次另将原函数提取为最小程序，以 Rust 1.96.0 实际确认借用检查报错。

## `async move` {#async-move}

与普通闭包类似，`async` 块和异步闭包也支持 `move` 关键字。`async move` 按值捕获使用到的变量：非 `Copy` 值通常被移入 Future，`Copy` 值会被复制。下面的 `String` 由 Future 拥有，因此 Future 可以离开原局部作用域；若要让其他代码继续共享数据，可以在构造前克隆 `Arc` 等共享句柄。

```rust,edition2018,ignore
{{#include ../../examples/03_01_async_await/src/lib.rs:async_move_examples}}
```

## 在多线程执行器上使用 `.await` {#awaiting-on-a-multithreaded-executor}

`async move` 不自动保证 Future 为 `'static` 或 `Send`：按值捕获一个 `&T` 仍是捕获借用，不能延长 `T` 的有效期；移入 `Rc<T>` 也不会让它变成 `Send`。本次最小编译负例实际验证了“移动非 static 引用不能返回 static Future”，而拥有内部局部数据的正例通过了 `Send + 'static` 编译约束。

多线程执行器可能在不同次轮询之间把任务移交给另一个工作线程；这不要求移动已经固定的 Future 所指值，只需移动拥有它的固定指针。需要跨线程调度的任务通常要求 Future 为 `Send`。判断依据包括构造时已捕获的参数和跨挂起点保留的状态，而不是“函数体中使用过的所有变量”。共享引用 `&T` 要为 `Send`，通常要求 `T: Sync`，因此 `Rc`、`&RefCell` 等保留在这些状态中时会阻止 Future 满足相应要求。

在异步体内创建的非 `Send` 局部值，如果放在一个在 `.await` 前已结束的独立作用域里，通常不需要跨挂起点保存，Future 仍可能是 `Send`；但调用 `async fn` 时传入的非 `Send` 参数已经被捕获，不能仅凭函数体中“在 await 前用完”就断言 Future 为 `Send`。让编译器通过 `Send` 约束检查，比单看变量文本作用域更可靠。若本来就不需要跨线程迁移，可以使用支持本地任务的执行方式，例如 Tokio 的 [`LocalSet` 和 `spawn_local`](https://docs.rs/tokio/latest/tokio/task/struct.LocalSet.html)，在同一线程运行 `!Send` Future。

也不要在 `.await` 期间持有阻塞锁：一个任务获取锁后挂起，另一个任务阻塞执行线程等待同一把锁，可能让持锁任务无法恢复，甚至耗尽整个线程池。标准库 `MutexGuard` 还不实现 `Send`，跨挂起保存它通常直接无法满足可迁移任务的要求。

`std::sync::Mutex` 并非在异步程序中一概不可用。短小、争用较低、完全不跨 `.await` 的临界区可以使用它，最好用明确的同步辅助函数或作用域让守卫先释放。如果确实需要跨等待持锁，可以考虑 `futures::lock::Mutex` 或对应运行时的异步锁，使等待锁本身不阻塞线程；异步锁仍可能产生锁顺序、重入或等待自己资源的逻辑死锁，也不是长临界区和阻塞工作的通用解决方案。参见 [Tokio 共享状态教程](https://tokio.rs/tokio/tutorial/shared-state) 与本书[同步原语](../part-guide/sync.md)。

语义依据见 Rust Reference 的 [await 表达式](https://doc.rust-lang.org/reference/expressions/await-expr.html) 与 [async 块](https://doc.rust-lang.org/reference/expressions/block-expr.html#async-blocks)。本节复用已有[真实 Pending 的入门示例](../01_getting_started/04_async_await_primer.md)和[捕获与 Send 示例](../part-reference/rustc-async-await.md)，不把这里立即返回值的片段当作调度交错实测。

[第一章]: ../01_getting_started/04_async_await_primer.md
