# 运行异步代码 {#running-asynchronous-code}
HTTP 服务器应当能够同时为多个客户端提供服务；
也就是说，在处理当前请求之前，不应等待先前的请求完成。
《Rust 程序设计语言》一书通过创建线程池来
[解决这个问题](https://doc.rust-lang.org/book/ch20-02-multithreaded.html#turning-our-single-threaded-server-into-a-multithreaded-server)，
让连接分配给工作线程处理，而不是为每个连接创建新线程。
这里将用异步任务逐步实现连接并发；异步运行时本身仍可能使用多个线程，并不保证与线程池具有相同性能。

我们将 `handle_connection` 改为返回一个 Future：把它声明为 `async fn`：
```rust,ignore
{{#include ../../examples/09_02_async_tcp_server/src/main.rs:handle_connection_async}}
```

在函数声明中添加 `async`，会将它的返回类型从单元类型 `()`
改为实现了 `Future<Output=()>` 的类型。[调用异步函数](https://doc.rust-lang.org/reference/items/functions.html#async-functions)会构造 Future，函数体要等 Future 被轮询才开始执行。上面的处理函数是只展示签名的空桩，尚未实现 HTTP 读写。

如果把旧同步循环中的调用保留为 `handle_connection(stream);`，返回的 Future 会被立即丢弃。原版记录的警告如下（历史输出，包名、行号及措辞不代表当前构建；当前所引源码已使用 `.await`）：
```console
$ cargo check
    Checking async-rust v0.1.0 (file:///projects/async-rust)
warning: unused implementer of `std::future::Future` that must be used
  --> src/main.rs:12:9
   |
12 |         handle_connection(stream);
   |         ^^^^^^^^^^^^^^^^^^^^^^^^^^
   |
   = note: `#[warn(unused_must_use)]` on by default
   = note: futures do nothing unless you `.await` or poll them
```

在这种遗漏 `.await` 的写法中，Future 没有被轮询，所以处理函数体不会执行。监听器仍可接受 TCP 连接，随后连接可能被关闭或重置；这不等于必然出现“连接被拒绝”。即使等待当前空桩，它也只会结束，不会返回 HTTP 页面。

普通同步函数不能直接使用 `.await`，但可以通过执行器的 `block_on` 驱动 Future，也可以在满足 Pin、Context 与唤醒协议的前提下手动调用 `poll`；并非每个 Future 都必须配备完整运行时。使用异步 I/O、计时器等资源时，还需要相应驱动支持。
更多关于异步运行时、执行器和反应器的信息，
请参阅[选择运行时一节](../08_ecosystem/00_chapter.md)。
更换运行时需要同步调整具体 I/O、计时器及生成任务的 API，不能直接替换名称就认为兼容。这组历史示例使用 `async-std`；它的[官方文档](https://docs.rs/async-std/latest/async_std/)现已宣布停止维护并推荐迁移 smol，这里保留它以讲解原来的逐步改造过程。

## 添加异步运行时 {#adding-an-async-runtime}
下面的示例将演示如何重构同步代码以使用异步运行时；这里使用的是 `async-std`。
`async-std` 提供的 `#[async_std::main]` 属性允许我们编写异步的 main 函数。
要使用它，需要在 `Cargo.toml` 中启用 `async-std` 的 `attributes` 特性：
```toml
[dependencies.async-std]
version = "1.12"
features = ["attributes"]
```

这里与示例包的清单保持一致；`"1.12"` 是兼容版本范围，并不固定到 1.12.0，本仓库锁文件解析为 1.13.2。[`#[async_std::main]`](https://docs.rs/async-std/latest/async_std/attr.main.html) 宏为入口创建执行环境，操作系统入口本身仍是同步的。

首先，我们将 main 函数改为异步函数，
并对异步版本的 `handle_connection` 所返回的 Future 执行 `await`。
这一阶段只展示调用关系，还没有完整的响应处理。代码如下：
```rust
{{#include ../../examples/09_02_async_tcp_server/src/main.rs:main_func}}
```
这个循环每次都等待当前处理函数完成，之后才接受下一个连接，因此仍是顺序处理。它还使用 `std::net::TcpListener`，等待连接会阻塞当前线程。仅仅把函数改为 `async` 并逐次 `.await`，既不会生成并发任务，也不会把同步 I/O 自动转换成异步 I/O。

为了说明这一点，我们来模拟一个缓慢的请求。
当客户端向 `127.0.0.1:7878/sleep` 发起请求时，
下面的 `09_03_slow_request` 才加入实际的简化请求处理：匹配该路径后异步等待至少 5 秒，然后返回页面。由于仍然只读一次 TCP，分段到达的请求行可能未被正确识别：

```rust,ignore
{{#include ../../examples/09_03_slow_request/src/main.rs:handle_connection}}
```
这与《Rust 程序设计语言》一书中的
[缓慢请求模拟](https://doc.rust-lang.org/book/ch20-02-multithreaded.html#simulating-a-slow-request-in-the-current-server-implementation)
非常相似，但有一个重要区别：
我们使用的是非阻塞函数 `async_std::task::sleep`，而不是阻塞函数 `std::thread::sleep`。
要记住，即使一段代码在 `async fn` 中运行并且被 `await`，它仍然可能阻塞。
这里的 [`task::sleep`](https://docs.rs/async-std/latest/async_std/task/fn.sleep.html) 只让等待计时器的阶段不阻塞执行线程；处理函数仍调用同步 `TcpStream::read`、`write_all` 与 `fs::read_to_string`，这些操作仍可能阻塞。获得连接并发还需要同时调度多个处理 Future；避免这些同步操作阻塞执行器则是另一个必须处理的问题。

运行这一阶段时，请在 `examples/09_03_slow_request` 目录执行 `cargo run`，确保相对路径的 HTML 文件可被找到。若 `/sleep` 请求已进入等待分支，外层循环就要等它处理完毕才接受下一个连接；其他客户端可能已在系统队列中建立 TCP 连接，但本程序还没有开始处理它们。
这是顺序 `.await` 的结果，不是 `task::sleep` 阻塞了线程，也不证明整个运行时都没有其他可运行任务。不能只凭两次浏览器访问的时间差精确断言并发行为。

本阶段保留[上一节](00_intro.md)所述的教学边界：读取返回长度与 EOF 已处理，写入使用 `write_all`，但仍没有完整 HTTP 解析、超时及健壮错误恢复，`flush` 也不保证对端已收到响应。
下一节将介绍如何使用异步代码同时处理多个连接。
