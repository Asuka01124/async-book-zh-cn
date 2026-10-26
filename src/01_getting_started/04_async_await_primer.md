<a id="asyncawait-primer"></a>
# `async`/`.await` 入门

旧版英文正文与示例注释由 GPT-6 Luna 完整翻译，GPT-6.1 Sol 校订，并补充真实挂起的行为示例。

`async`/`.await` 是 Rust 内置的工具，可用于编写外观类似同步代码的异步函数。`async` 会将代码块转换为实现 `Future` 的状态机。调用同步阻塞函数会占住当前线程；异步计算则可以在不能继续推进时返回 `Poll::Pending`，把控制权交还调用者，让执行器有机会推进其他工作。这个区别依赖被调用操作的实现：async 不会自动把同步阻塞或长时间计算变成非阻塞操作。

先向 `Cargo.toml` 文件中添加一些依赖：

```toml
{{#include ../../examples/01_04_async_await_primer/Cargo.toml:9:10}}
```

要创建异步函数，可以使用 `async fn` 语法：

```rust,edition2018
async fn do_something() { /* ... */ }
```

`async fn` 返回的值是一个 `Future`，函数体尚未执行；调用处的实参表达式仍会先求值。要推进函数体，必须让这个 Future 被轮询。通常由执行器驱动根 Future，再通过 `.await` 和组合器推进其中的子 Future。以下示例用 `futures::executor::block_on` 在当前线程上完成这一工作：

```rust,edition2018
{{#include ../../examples/01_04_async_await_primer/src/lib.rs:hello_world}}
```

在 `async fn` 内部，可以使用 `.await` 等待另一个异步计算，例如另一个 `async fn` 返回的 Future。更准确地说，`.await` 接受实现 [`IntoFuture`](https://doc.rust-lang.org/std/future/trait.IntoFuture.html) 的值，先转换成 Future 再轮询；所有 Future 都实现了相应转换。与在同步边界调用 `block_on` 不同，`.await` 自身不会用阻塞等待占住线程：子 Future 返回 `Pending` 时，当前 async 计算才挂起并返回 `Pending`；若子 Future 已经 `Ready`，则继续当前这次轮询，不必让出控制权。参阅 [Reference 的 await 规则](https://doc.rust-lang.org/reference/expressions/await-expr.html)。

例如，假设有三个 `async fn`：`learn_song`、`sing_song` 和 `dance`。下面的省略函数体仅表示接口，`Song` 代表学会的歌曲：

```rust,ignore
async fn learn_song() -> Song { /* ... */ }
async fn sing_song(song: Song) { /* ... */ }
async fn dance() { /* ... */ }
```

一种学习歌曲、唱歌和跳舞的方式，是分别对每个函数调用 `block_on`：

```rust,ignore
{{#include ../../examples/01_04_async_await_primer/src/lib.rs:block_on_each}}
```

这种方式会先完成学习，再完成唱歌，最后才开始跳舞，没有利用等待区间的重叠。唱歌之前必须学会歌曲，但跳舞与这条依赖链相互独立。若这些步骤有可异步等待的阶段，可以把“学习后唱歌”与“跳舞”作为两个 Future 并发推进；这提供重叠工作的机会，不等于对实际耗时或性能作保证：

```rust,ignore
{{#include ../../examples/01_04_async_await_primer/src/lib.rs:block_on_main}}
```

在这个结构中，`learn_and_sing` 内部先等待学习完成再唱歌；`join!` 同时管理它与 `dance`，等待二者都完成。某个分支返回 `Pending` 后，组合器可以继续轮询另一个分支；两者都未完成时，组合 Future 也会返回 `Pending`，由根执行器在收到唤醒后继续推进。它们属于同一任务，并非由 `join!` 创建的两个独立任务，也不保证在两个线程上并行。具体行为见 [`futures::join!`](https://docs.rs/futures/latest/futures/macro.join.html)。

不要把 `learn_and_sing` 里的 `learn_song().await` 换成 `block_on(learn_song())`。一般而言，在异步任务内引入阻塞等待会占住执行线程，妨碍同线程其他工作；对本例则还有更直接的限制：外层已经进入 `futures::executor::block_on(async_main())`，内层再调用同一执行器的 `block_on` 会被其进入检查拒绝并 panic，而不只是简单变成顺序等待。[官方执行器源码](https://docs.rs/futures-executor/0.3.34/src/futures_executor/local_pool.rs.html) 中的 `run_executor` 使用 `enter().expect(...)` 检查这种嵌套。前面的逐次 `block_on` 都在同步 `main` 边界各自完成后返回，属于不同情况。[block_on 文档](https://docs.rs/futures/latest/futures/executor/fn.block_on.html)也说明它会阻塞调用者直到 Future 完成。

仓库保留的 `learn_song`、`sing_song` 和 `dance` 都是立即完成的桩：它们没有真实等待，原三个测试只确认控制结构能编译运行，不能据此声称已经观测到交错或学习与跳舞重叠。为了实际看到挂起，下面的共享示例复用 [rustc 章节](../part-reference/rustc-async-await.md) 的 `YieldOnce`：每个新实例首次 poll 都先请求唤醒并返回 `Pending`，再次 poll 才完成。事件记录的借用只持续一次 `push`，不会把 `RefCell` 借用守卫带过 `.await`。

```rust,ignore
{{#include ../../examples/guide-tests/src/primer.rs:primer-pending-song-dance}}
```

对应测试手动轮询根 Future：第一次得到 `Pending` 时，“开始学歌”和“开始跳舞”都已记录，但歌曲还没学会；第二次仍是 `Pending`，此时学习已完成、唱歌已开始、跳舞也继续推进，两个分支都未结束；最后一次返回 `Ready(())`。测试检查先学后唱的顺序与等待重叠，不把某个执行器的线程调度顺序作为前提。这里使用无操作 Waker，因为测试按已知步骤主动重轮询；正常执行器应响应 Future 的唤醒请求。

这是一项确定性、无 I/O 的状态机演示，不是真正的音乐、动画或计时器，也不是性能基准。它说明并发来自可挂起的步骤以及共同推进它们的调用者；同步阻塞的学习实现仍会占住线程，光写 `.await` 不能修复这一点。更一般的执行器、组合、阻塞与取消边界见新版 [Async 与 Await](../part-guide/async-await.md)、[并发组合 Future](../part-guide/concurrency-primitives.md)和[同步异步桥接](../part-reference/sync-async.md)。
