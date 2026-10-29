<a id="cancellation-and-timeouts"></a>
# 取消与超时

上游英文目录只有本节标题，没有正文。标题由 GPT-6 Luna 翻译；以下教程与示例由 GPT-6.1 Sol 原创补写，并非原英文正文的翻译。

`join!` 等待所有分支，`select!` 可以先采用一个结果。那么，剩下的工作怎么办？超时又能否保证它立即停止？答案首先取决于我们拥有的是工作本身，还是一个等待工作的句柄。本节使用 Tokio 的定时器与任务接口；前一节的 async-std 句柄遵循它自己的接口，不能直接套用这里的方法名。

## 丢弃的是谁

普通 Future 是保存执行进度的值。调用者拥有它时，销毁它会停止继续轮询，并同步析构它持有的资源。已经发生的数据库写入、消息发送或缓冲区修改不会撤销。Future 还没第一次被轮询时也可以被销毁；此时 async 函数体尚未执行，但已经捕获的值仍会被释放。

如果组合器接收的是 `&mut Future` 或 `Pin<&mut Future>`，它拥有的只是借用。组合器返回并释放借用后，外部拥有者仍可继续驱动原 Future。反过来，如果把 Future 的所有权交给组合器，返回后通常就不能再访问它。`select!` 的其他分支被取消，也要沿着同一条所有权链判断，不能仅凭“输掉竞争”推断底层任务已结束。

取消安全关心的是：在某个等待点停止并重新构造操作，会不会丢失进度或破坏预期？例如 `tokio::sync::mpsc::Receiver::recv` 可在选择循环中取消并重建；某些逐步读写操作却可能已消费部分输入。丢弃这类操作后从头重试，可能错过数据或重复写入。是否适合重试需要逐个检查 API 和业务协议，而不是由 `async` 统一保证。更多边界见[取消与取消安全](../part-reference/cancellation.md)。

## 给一次等待设置期限

`tokio::time::timeout(duration, future).await` 的结果是 `Result<T, Elapsed>`。`Ok(value)` 表示内部 Future 完成；`Err(_)` 表示这次等待观察到了超时。如果内部返回 `Result<T, E>`，整体就是两层 Result：业务错误与超时错误应分别处理。

下面故意把固定后的工作借给超时器。第一次等待如果超时，就继续等待原工作，最终得到 42；返回值中的布尔值记录是否走过超时路径：

```rust
{{#include ../../examples/guide-tests/src/legacy_timeouts.rs:borrowed}}
```

这里超时没有重新启动两秒的 sleep，而是继续同一个 Future。暂停时间的测试会验证 `(true, 42)`；真实运行时也可能延迟到工作已经就绪才再次轮询，因此函数同时接受首轮成功的路径，返回 `(false, 42)`。如果把 `work` 的所有权直接传入 `timeout`，超时表达式结束时它会被销毁；若要主动取回包装器中的 Future，可以保留包装器并使用 `Timeout::into_inner`，但必须继续遵守固定与所有权约束。

定时器必须在已启用时间驱动的 Tokio runtime 内创建和轮询。`#[tokio::main]` 的默认配置会启用驱动；自己构建 runtime 时需要 `enable_time()` 或 `enable_all()`。在同一项操作的多轮重试中，如果每次都重新设置完整 duration，总耗时会不断延长。可以先计算一次 `Instant` 截止点，并用 `timeout_at` 共享这份总预算。

超时是协作调度下的时间限制，不能抢占正在执行的同步代码。当前 Tokio 的 timeout 会先轮询内部工作；内部工作若一直占用线程，可能超过期限后仍返回 `Ok`。零时长也不能表达“绝不执行内部工作”。运行时繁忙或线程阻塞还会延迟观察期限，因此不要把它当成硬实时保证。应把长计算分块并让出执行机会；已经开始的 `spawn_blocking` 工作也不能靠 abort 强制中断。

## 给后台任务设置期限

Tokio 的 `JoinHandle` 是等待任务结果的句柄，本身也是 Future。把它按值传给 timeout，超时后丢弃的是句柄，任务会分离并继续运行。若业务要求请求任务中止，应保留句柄的可变借用：

```rust
# async fn example() {
use std::time::Duration;
let mut task = tokio::spawn(async {
    std::future::pending::<()>().await;
});
match tokio::time::timeout(Duration::from_secs(1), &mut task).await {
    Ok(result) => { result.expect("任务未正常完成"); }
    Err(_) => {
        task.abort();
        // 请求中止不等于已经结束；等待以观察终止和析构。
        let result = task.await;
        match result {
            Err(error) if error.is_cancelled() => {}
            other => { other.expect("任务发生 panic"); }
        }
    }
}
# }
```

任务可能在超时与 abort 之间正常完成，因此中止后 await 也可能得到正常结果。若任务正在执行不返回的 poll，abort 和随后的 await 都无法保证及时结束。观察句柄终止时同步析构已完成，但这不代表异步 flush、远程撤销或业务补偿已经执行。

## 先请求关闭，再等待清理

需要排空队列、发送结束消息或异步持久化时，让任务自己响应关闭更合适。下面用 oneshot 作为一次关闭通知，用有界 mpsc 保存工作。关闭分支优先处理已就绪的通知，退出选择循环后关闭接收端并排空缓冲区：

```rust
{{#include ../../examples/guide-tests/src/legacy_timeouts.rs:shutdown}}
```

这个示例只收集整数，处理动作立即完成；它不是耗时作业或持久化的模拟。真实的单项处理如果需要 `.await`，必须决定是否让当前作业完成、是否在处理中响应关闭、以及中间进度保存在哪里。这里关闭后排空接收端；若其他发送端此前已取得发送许可，Tokio 仍允许通过该许可发送，排空结束可能需要等待这些许可释放或使用。

宿主应发送通知，然后 await worker 的 JoinHandle，以确认清理结束。清理阶段放在选择循环外，避免再次与已就绪的关闭信号竞争而被立即取消。如果清理也需要总期限，可以另设期限并制定超时后的丢弃或中止策略；这份策略应明确可能遗留的外部工作。关闭通知只是请求，成功发送通知不代表工作已经退出。

## 运行与核查

本章源码位于独立的 `examples/guide-tests` crate。运行 `cargo test --manifest-path examples/guide-tests/Cargo.toml legacy_timeouts --locked --offline`，可验证拥有的 Future 被销毁但副作用保留、借用的 Future 超时后继续完成、句柄分离与显式中止的区别，以及关闭后排空并 join。超时测试使用 Tokio 暂停时间，没有按真实秒数休眠，也不声称验证了任意系统调度或真实 I/O。

接口依据：[Tokio timeout](https://docs.rs/tokio/latest/tokio/time/fn.timeout.html)、[JoinHandle](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html)、[select 的取消安全](https://docs.rs/tokio/latest/tokio/macro.select.html#cancellation-safety)。定时器机制见[定时器与信号处理](../part-guide/timers-signals.md)，异步清理的完整设计见[析构与清理](../part-guide/dtors.md)。
