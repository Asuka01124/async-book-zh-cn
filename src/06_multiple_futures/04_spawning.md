# 生成任务（Spawning） {#spawning}

生成任务可以把一个新的异步任务交给执行器，在它被调度推进的同时，当前任务可以继续执行其他代码。
任务可能在同一线程交错执行，也可能由运行时分配到其他线程；`spawn` 不等于为每个任务创建线程。

本节保留历史示例使用的 `async-std`。截至本次校订（2026 年 10 月），其[官方仓库](https://github.com/async-rs/async-std)
已宣布停止维护并建议迁移至 `smol`；此处用于理解已有代码，新项目的运行时选择可参见[运行时及其使用边界](../part-guide/runtimes.md)。

假设有一个 Web 服务器需要继续接受连接，同时处理已经接入的连接。可以使用
[`async_std::task::spawn`](https://docs.rs/async-std/latest/async_std/task/fn.spawn.html)
创建独立任务。该函数接收 Future，并返回 `JoinHandle`，可用于等待任务结果。
它要求 Future 及输出类型都满足 `Send + 'static`；`async move` 在这里把连接所有权交给任务。
`'static` 表示不能携带不足以存活的外部借用，不是要求任务永远运行。

```rust,edition2018
{{#include ../../examples/06_04_spawning/src/lib.rs:example}}
```

`process_request` 只向连接写出固定 HTTP 响应字节，没有读取或解析请求，也没有实现完整 HTTP 协议、
连接管理或错误恢复。接受连接和写入都使用异步 API；若分支内加入阻塞调用或长时间 CPU 计算，仍可能阻碍执行器。
这里的普通 `async fn main` 是示例函数，若改成独立程序，需要运行时入口（例如由同步 `main` 调用
`async_std::task::block_on`），仅声明该函数不会启动服务器。

[`JoinHandle`](https://docs.rs/async-std/latest/async_std/task/struct.JoinHandle.html)
实现 `Future<Output = T>`，所以 `.await` 可以取得任务返回的 `T`。任务尚未完成时，等待会挂起
当前任务，而不是阻塞执行器线程；被生成的任务已独立交给运行时调度，不需要等待句柄才开始执行。
若任务返回 `Result`，必须观察或处理这个结果，`.await` 本身不会自动处理业务错误。

原文声称“外层函数提前结束会取消该任务”，这不符合 `async-std` 的行为：丢弃这里的 `JoinHandle`
会分离任务（detach），使任务可以继续运行，同时失去通过该句柄等待结果的机会。
外层函数返回、句柄被丢弃、运行时停止和整个进程退出是不同事件；分离任务不会阻止进程退出，
也不保证在程序结束前完成。例子中的连接任务内部直接报告写入错误，避免丢弃句柄时静默丢失 `Result`。

需要主动取消时，可持有句柄并调用 `handle.cancel().await`，等待该任务停止；返回 `Some(T)` 表示已取得
完成结果，`None` 表示任务被取消。取消需要执行器取得控制权，不能强行打断一直阻塞或不让出的代码，
也不回滚既有 I/O 或其他外部副作用。这些规则依赖具体运行时和句柄类型，不应推广到所有 `JoinHandle`。

下面先生成三个任务，再使用 `join_all` 等待它们的句柄，确保该函数返回前这三项都已结束。
`join_all` 按输入顺序收集输出；这里并发执行来自先前的 `spawn`，组合器只是等待已有任务的结果。

```rust,edition2018
{{#include ../../examples/06_04_spawning/src/lib.rs:join_all}}
```

要在当前任务和生成的任务之间通信，可以使用所用异步运行时提供的通道，也可选用兼容该运行时的
通道实现。关闭、错误通知与取消需要协议配合，参见[通道、锁与同步](../part-guide/sync.md)和
[取消与取消安全](../part-reference/cancellation.md)。
