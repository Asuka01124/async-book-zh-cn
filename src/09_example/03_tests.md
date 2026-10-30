# 测试 TCP 服务器 {#testing-the-tcp-server}
接下来，我们来测试 `handle_connection` 函数。

首先，我们需要一个可供使用的 `TcpStream`。
在端到端测试或集成测试中，我们可能会建立一个真实的 TCP 连接来测试代码。
一种做法是将监听器绑定到 `127.0.0.1:0`：这里的 [bind 端口 0](https://doc.rust-lang.org/std/net/struct.TcpListener.html#method.bind) 表示请求操作系统分配可用的临时端口，然后通过 `local_addr()` 取得实际地址。它不是“无效的 UNIX 端口”，也不表示客户端应该连接到 0 号端口。真实 TCP 测试还要管理监听器生命周期、超时及任务清理。

本例中，我们改为针对连接处理器编写单元测试，检查它针对相应输入是否返回了正确的响应。
为了让单元测试彼此隔离且结果可复现，我们会用一个模拟对象替换 `TcpStream`。

上一节已经采用下面的泛型函数签名，让 `handle_connection` 更容易测试。
`handle_connection` 实际上并不需要 `async_std::net::TcpStream`；
它需要的是任何实现了 `async_std::io::Read`、`async_std::io::Write` 和 `std::marker::Unpin` 的类型。
修改类型签名以反映这一点后，我们就可以在测试中传入模拟对象。
```rust
use async_std::io::{Read, Write};

async fn handle_connection(mut stream: impl Read + Write + Unpin) -> std::io::Result<()> {
    // 函数体见上一节；这里仅展示签名。
    # Ok(())
}
```

这里的 `async_std::io::Read`/`Write` 是 futures I/O trait 的重新导出，可用 `futures::task::Context`/`Poll` 实现轮询接口；不要与 `std::io` 的同步 trait 或 Tokio 的同名异步接口混用。

接下来，我们来构建一个实现这些 trait 的模拟 `TcpStream`。
首先实现 `Read` trait 的必需方法 `poll_read`，其他有默认实现的方法可沿用。
模拟器持有输入字节及读取游标，每次最多复制缓冲区可容纳的字节，并推进游标；输入耗尽时返回 `Ready(Ok(0))` 表示 EOF，零长度缓冲区也返回 0，但不消费输入。不能每次从输入开头重新复制，否则会把有限输入错误地模拟成无限重复的数据源。

按照 [`AsyncRead` 的约定](https://docs.rs/futures-io/latest/futures_io/trait.AsyncRead.html)，返回 `Pending` 时还必须安排后续唤醒。这里使用一次性开关模拟暂不可读：先改变下一次轮询的状态，再调用当前 Waker 并实际返回 Pending；这只是确定性的协议模拟，不是网络延迟或调度公平性的测量。
```rust,ignore
{{#include ../../examples/09_05_final_tcp_server/src/main.rs:mock_read}}
```

`Write` 的实现与此很相似，
不过我们需要实现三个方法：`poll_write`、`poll_flush` 和 `poll_close`。
[`poll_write`](https://docs.rs/futures-io/latest/futures_io/trait.AsyncWrite.html) 返回本次接收的字节数，不能超过传入缓冲区长度；它可以短写，因此本例用 `max_write` 限制单次长度，并将已接收字节追加到输出，而不是覆盖先前输出。`write_all` 会继续写入余下字节；非空输入写入 0 字节时应得到 WriteZero 错误。写入的 Pending 开关也安排唤醒，且返回 Pending 时不消费数据。

内存模拟器没有真实 socket 的刷新或关闭工作，正常情况下可直接返回 `Ready(Ok(()))`。这里另提供读取、写入和刷新错误开关，以测试错误传播；`poll_close` 的空实现不能证明真实 TCP 关闭行为。
```rust,ignore
{{#include ../../examples/09_05_final_tcp_server/src/main.rs:mock_write}}
```

最后，接口要求模拟对象满足 [`Unpin`](https://doc.rust-lang.org/std/marker/trait.Unpin.html)，即该类型不依赖固定地址提供的保证。本模拟器的字段已经自动满足 Unpin，无需手写实现；下面用泛型约束检查这一点，而不是建议所有模拟类型都手动声明 Unpin。关于 pin 和 Unpin 的细节，参阅[固定与 Pin](../part-reference/pinning.md)。
```rust,ignore
{{#include ../../examples/09_05_final_tcp_server/src/main.rs:unpin}}
```

现在，我们可以开始测试 `handle_connection` 函数了。
设置好包含初始数据的 `MockTcpStream` 后，
我们可以使用 `#[async_std::test]` 属性运行 `handle_connection`，这与使用 `#[async_std::main]` 的方式类似。
这个正常路径测试提供完整请求行，并检查写入数据与预期响应**完全相等**，而不是仅检查前缀。预期页面通过 `include_str!` 从包内 HTML 编译时读取；处理函数仍在运行时按当前工作目录读取文件，因此单元测试也没有完全隔离文件系统。[Cargo 以包根目录作为单元测试工作目录](https://doc.rust-lang.org/cargo/commands/cargo-test.html#working-directory-of-tests)；直接运行测试可执行文件时也需确保该条件。
```rust,ignore
{{#include ../../examples/09_05_final_tcp_server/src/main.rs:test}}
```

本包还包含以下确定性检查（在 `examples/09_05_final_tcp_server` 目录运行 `cargo test`）：

- 未知路径返回完整 404 响应，输入 EOF 时不写响应。
- 直接轮询模拟器，验证读游标、EOF、零长度缓冲区、短写追加，以及读写实际返回 Pending 时的唤醒计数。
- 请求行完整可读，但读写各先返回一次 Pending、每次最多写 3 字节时，处理函数最终仍输出完整 200 响应。
- 读取、写入及刷新错误传回调用方；非空响应写入 0 字节时返回 WriteZero。

这些测试验证的是泛型处理函数与模拟 I/O 的契约，没有建立真实 TCP 连接，也不验证线程并行、HTTP 协议完整性或真实 socket 的关闭语义。处理函数目前仍只读取一次：**分片到达的请求行可能被误判**。测试中的短读验证的是模拟器游标行为，并没有声称当前处理函数能重组分片请求。若要保证完整 HTTP 请求解析，应使用相应协议库或进一步实现有界读取与解析，再补充对应测试。
