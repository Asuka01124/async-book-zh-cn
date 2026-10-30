# 并发处理连接 {#handling-connections-concurrently}
目前为止，我们的代码存在一个问题：`listener.incoming()` 是一个阻塞迭代器。
当 `listener` 等待传入连接时，它会阻塞执行器的当前线程，使该线程不能轮询其他 Future；
而且在处理完前一个连接之前，我们也无法处理新连接。

为了解决这个问题，我们要把 `listener.incoming()` 从阻塞的 Iterator
改为使用异步监听器返回的 Stream。Stream 与 Iterator 类似，但可以异步地消费。
更多信息请参阅[Stream 章节](../05_streams/01_chapter.md)。

我们把阻塞的 `std::net::TcpListener` 替换为非阻塞的 `async_std::net::TcpListener`，
并将连接处理函数更新为接收 `async_std::net::TcpStream`：
```rust,ignore
{{#include ../../examples/09_04_concurrent_tcp_server/src/main.rs:handle_connection}}
```

异步 `TcpListener` 的 `incoming()` 方法返回实现 `Stream` 的 [`Incoming`](https://docs.rs/async-std/latest/async_std/net/struct.Incoming.html)；不是监听器本身实现了这个 trait。
这一变化带来了两个好处。
首先，`listener.incoming()` 不再阻塞执行器。
没有待处理的 TCP 连接时，对流的轮询会返回 Pending 并注册唤醒，执行器可以调度其他就绪任务。

第二个好处是，可以选择并发处理 Stream 中的元素，
方法是使用 [`StreamExt::for_each_concurrent`](https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html#method.for_each_concurrent)。
这里我们将利用这个方法并发处理每个传入请求。
我们需要从 `futures` crate 导入扩展 trait `StreamExt`（源码中的 `use futures::stream::StreamExt;`），因此现在的 Cargo.toml 如下：
```diff
+[dependencies]
+futures = "0.3"

 [dependencies.async-std]
 version = "1.12"
 features = ["attributes"]
```

现在，我们可以通过闭包函数传入 `handle_connection`，并发处理每个连接。
流每次产出一个 `Result<TcpStream>`，闭包创建处理它的 Future；组合器随后轮询多个尚未完成的处理 Future。只要处理函数及时让出执行权，等待一个连接的 I/O 时就能推进其他连接。这里不保证每个请求立即执行或获得公平的时限。

这个 `09_04_concurrent_tcp_server` 示例仍是阶段性占位实现：它异步读取一次，再写入字符串 `hello`，**并不生成 HTTP 响应**。它用于展示并发调度，不能据此宣称浏览器已得到完整页面。
```rust,ignore
{{#include ../../examples/09_04_concurrent_tcp_server/src/main.rs:main_func}}
```

# 并行处理请求 {#serving-requests-in-parallel}
目前为止，我们的示例主要把协作式多任务并发（使用 async 代码）
作为抢占式多任务（使用线程）的替代方案来介绍。
不过，async 代码和线程并非互斥。
`for_each_concurrent` 在**同一个外层任务**中轮询各处理 Future，不会自行生成线程或独立任务。同一时刻一次轮询发生在一个线程上；如果外层任务由允许迁移的调度器驱动，它以后仍可能换线程，因此不能把这些 Future 理解为永远固定在某个操作系统线程。

`async-std` 的 [`task::spawn`](https://docs.rs/async-std/latest/async_std/task/fn.spawn.html) 可以将每个连接处理提交为独立任务，由运行时调度；多个任务可能在线程池上并行执行，但不保证为每个连接分配专用线程，也不保证实际并行。它要求传入的 Future 及输出满足 `Send + 'static`。传入拥有的 TcpStream 时，下面的处理任务可以满足这些接口约束；非阻塞性则是避免占用执行线程的设计要求，不是 `Send` 能保证的性质。
代码如下：
```rust
{{#include ../../examples/09_05_final_tcp_server/src/main.rs:main_func}}
```
这一版 `09_05_final_tcp_server` 恢复了简化的页面响应。网络操作使用异步 I/O，读取 HTML 改为 `async_std::fs::read_to_string(...).await`；文件系统接口通过运行时支持避免在当前执行任务中直接调用阻塞的标准库文件读取，并不表示底层磁盘操作不需要线程。处理函数返回 `std::io::Result<()>`，子任务内报告读写和文件错误；接收连接失败也会报告。绑定监听地址失败仍会因 `unwrap` 终止。

两段代码的 `None` 都表示组合器不限制同时持有的闭包 Future 数量。第一版在闭包里等待完整处理，改为 `Some(n)`（`n > 0`）可以限制该组合器内的处理并发；`Some(0)` 与 `None` 一样不设限制。第二版闭包只生成任务就返回，即使把它改为 `Some(n)`，也不能限制已经分离的后台处理任务数量。需要限制后台并发时，应持有任务句柄、使用许可等方式覆盖实际处理的整个生命周期，参见[生成任务](../06_multiple_futures/04_spawning.md)。

在 async-std 中，丢弃 [`JoinHandle`](https://docs.rs/async-std/latest/async_std/task/struct.JoinHandle.html) 会分离任务，而非自动等待或取消。示例在任务内部观察普通 I/O 错误，但没有汇总任务结果、观察 panic 或实现优雅关闭；主任务结束也不表示后台任务已完成。无限接收、无并发上限和简单日志都只是教学取舍。

如需运行这一版，请在 `examples/09_05_final_tcp_server` 目录执行 `cargo run`，以便找到 HTML 文件。当前读取仍只检查一次最多 1024 字节，分段请求行可能被误判；EOF 返回、`write_all` 完整写入及错误传播已补上，但它依然没有完整 HTTP 解析、请求大小和时间限制，`flush` 也不保证对端收到响应。这些限制不因使用 async 和 spawn 自动消失。
更多信息请参阅[多线程执行器一节](../08_ecosystem/00_chapter.md#single-threaded-vs-multi-threaded-executors)。
