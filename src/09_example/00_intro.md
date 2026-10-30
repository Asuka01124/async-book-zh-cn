# 最终项目：使用异步 Rust 构建并发 Web 服务器 {#final-project-building-a-concurrent-web-server-with-async-rust}

在本章中，我们将使用异步 Rust 改造《Rust 程序设计语言》一书中的[单线程 Web 服务器](https://doc.rust-lang.org/book/ch20-01-single-threaded.html)，使其能够并发处理请求。

## 回顾 {#recap}

以下回顾原教程的同步版本，作为后续异步改造的对照。原链接现重定向到 Rust Book 的[第 21 章对应小节](https://doc.rust-lang.org/book/ch21-01-single-threaded.html)；这里保留本书采用的简化版本，并修正读取长度与空连接的处理。

`src/main.rs`：
```rust
{{#include ../../examples/09_01_sync_tcp_server/src/main.rs}}
```

`hello.html`：
```html
{{#include ../../examples/09_01_sync_tcp_server/hello.html}}
```

`404.html`：
```html
{{#include ../../examples/09_01_sync_tcp_server/404.html}}
```

请先切换到 `examples/09_01_sync_tcp_server` 目录，再运行服务器：

```console
cargo +1.96.0 run --offline --locked
```

然后在浏览器中访问 `http://127.0.0.1:7878/`，就会看到来自 Rust 的问候页面。访问其他路径会得到这里的 404 页面；退出时按 `Ctrl+C`。`hello.html` 和 `404.html` 的路径相对于**进程当前工作目录**，并不相对于源码或可执行文件；只在仓库根目录指定 `--manifest-path` 不会改变工作目录。

这是同步、单线程的教学基线。等待连接、读取和写入 TCP、读取 HTML 文件都会阻塞当前线程；一个慢连接就可能阻止后续连接被处理。后续章节将逐步改变这一点。

这个示例也不是完整的 HTTP 实现。[`read`](https://doc.rust-lang.org/std/io/trait.Read.html#tymethod.read) 单次最多读取缓冲区大小，并不保证得到整个请求，甚至可能只得到请求行的一部分。代码只检查本次实际读取的字节，读到 EOF 时直接返回；请求行分段到达时仍可能误判为 404。它没有完整解析请求头、请求体或持久连接，也没有配置超时和健壮的错误恢复，`unwrap` 遇错会终止程序。

`write_all` 会尝试写完所给字节，但成功并不表示对端应用已经接收或处理响应。这里保留 `flush` 展示写入接口；对于未加用户态缓冲的 [`TcpStream`](https://doc.rust-lang.org/std/net/struct.TcpStream.html)，它不能充当对端确认机制。示例未发送 `Content-Length`，响应体以连接关闭结束；不要把这种简化格式当作通用 HTTP 服务端方案。
