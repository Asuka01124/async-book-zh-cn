# I/O：输入与输出 {#io}

上游原文只有目录占位标题“I/O”，没有英文正文；标题由 GPT-6 Luna 翻译，以下教程与示例由 GPT-6.1 Sol 原创补写。本章从前面服务器示例的单次读取限制出发，介绍如何正确推进异步字节流，以及调用者仍需承担哪些协议责任。

## 一次读取不等于一条消息

I/O 是程序与外部资源交换数据的过程，包括网络、文件、管道等。异步 I/O 的价值在于：资源暂时不能推进时，让当前任务暂停等待，执行器可以运行其他就绪任务。`async fn` 本身不会把阻塞的标准库调用变成非阻塞调用，也不会自动识别应用协议。

TCP 提供有序字节流，不保留每次发送的消息边界。对端一次写入 `hello\n`，接收方可能先读到 `he`、再读到 `llo\n`；也可能一次读取就得到两条消息。分片与合并都是正常情况，不能把一次 `read` 当成一个 HTTP 请求、一行文本或一条业务消息。

使用扩展方法 `read(&mut buf).await` 时，应根据返回结果处理：

- `Ok(n)` 且 `n > 0`：只有 `buf[..n]` 是这次读取的有效字节，不要解析剩余旧内容；`n` 可以小于缓冲区长度。
- 对非空缓冲区返回 `Ok(0)`：通常表示输入已结束，即 EOF。传入空缓冲区也能得到 0，不能据此判定连接关闭。具体资源类型仍应遵守自身接口语义。
- `Err(error)`：向调用方传播或按明确策略处理，不能把错误当成 EOF。

等待发生在底层轮询接口：资源暂不可读时，`poll_read` 返回 `Poll::Pending`，并安排在可读或结束时通过 Waker 通知当前任务。实现者若只返回 Pending 而不安排唤醒，任务可能永远不再被轮询；调用方也不应忙循环反复 poll 来代替等待。

## 先选择同一套 I/O trait

标准库 `std::io::Read`/`Write` 是同步接口。异步生态有两套常见的接口，名字相似但类型不同：

| 接口 | 底层读取方式 | 常见扩展方法 |
| --- | --- | --- |
| `futures::io::AsyncRead`，以及 async-std 重新导出的 `Read` | `poll_read(..., &mut [u8]) -> Poll<io::Result<usize>>` | `futures::io::AsyncReadExt`；async-std 的对应扩展接口 |
| `tokio::io::AsyncRead` | `poll_read(..., &mut ReadBuf) -> Poll<io::Result<()>>` | `tokio::io::AsyncReadExt` |

Tokio 的 [`ReadBuf`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncRead.html) 跟踪已填充的区域，读取量由填充长度的增加表示。两种接口都表达异步读取，但不能把前一章的 `poll_read` 实现原样用于 Tokio，也不能只导入同名 trait 就完成转换。必要时可使用 I/O 兼容适配器；它只解决接口匹配，资源所需的驱动器和运行时上下文仍须正确配置，参见[异步生态系统](../08_ecosystem/00_chapter.md)。

以下示例统一使用 Tokio I/O trait，不与 futures I/O 类型混用。

## 写入、刷新与完成是不同层次

`write(data).await` 可以只接收一部分字节，返回本次接收的数量；剩余部分需要继续写。发送一段完整数据时，通常用 [`write_all`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncWriteExt.html#method.write_all)，它会持续写入直到全部接受或出错。对非空剩余数据写入 0 字节无法取得进展，完整写入操作会返回 WriteZero。

`write_all` 成功不等于对端应用已经读取或处理数据。缓冲写入器还可能把数据留在用户态缓冲区，`flush().await` 用于推进刷新；它不提供业务确认，也不保证磁盘持久化。文件持久化需要相应同步接口及清楚的崩溃一致性策略。`shutdown` 的含义同样取决于资源和协议，不能用内存模拟器的成功结果替代真实连接关闭验证。

## 用有界帧把字节组成消息

先定义一个小协议：每帧以单个 LF（`\n`）结束，返回值不含这个分隔符；载荷最多 `max_payload` **字节**。空帧合法，干净 EOF 返回 `None`；已经收到部分载荷却遇到 EOF，则返回 UnexpectedEof。这里不处理 CRLF、转义、Unicode 字符边界或 HTTP，它们属于各自协议的规则。

使用 [`BufReader` 的缓冲读取接口](https://docs.rs/tokio/latest/tokio/io/trait.AsyncBufReadExt.html#method.fill_buf)，我们可以查看已有字节，先查找 LF，再检查限额，最后消费确定属于这一帧的字节：

```rust
{{#include ../../examples/guide-tests/src/legacy_io.rs:bounded-frame}}
```

`fill_buf` 借出缓冲区中已有的数据，必要时等待底层读取；`consume(n)` 标记前 n 个字节已经处理。找到 LF 时只消费到分隔符，后续帧的数据留在同一个 reader 中。下一次读取必须继续使用它；若丢弃带有预读数据的 BufReader 后直接读底层流，就可能丢失尚未处理的字节。

检查先于追加，保持 `frame.len() <= max_payload`，因此不会先无限读取再事后发现超限。这个限制只约束当前帧的载荷长度，不是整个程序的精确内存上限：Vec 容量、BufReader 缓冲区、并发连接及其他状态都占内存。调用方还要选择合理的 reader 容量、连接数和超时。报错前可能已消费前面的若干段数据；超限后的恢复或关闭策略也必须由协议规定。

## 在内存流中运行

Tokio 的 `duplex` 是有容量限制的双向内存字节流。下面把每个方向的容量设为 2 字节，让 4 字节载荷和 LF 必须分多次传输，并把两帧依次读回。生产者与消费者由 `try_join!` 在同一任务中并发推进；如果先完整等待写入再开始读取，小容量缓冲区就可能让双方互相等待。

```rust
{{#include ../../examples/guide-tests/src/legacy_io.rs:frame-demo}}
```

这段示例已经收录在独立验证包中。在仓库根目录执行：

```console
cargo test --manifest-path examples/guide-tests/Cargo.toml legacy_io
```

对应测试还会直接轮询一个暂时无数据的 duplex 读取 Future，确认实际得到 Pending，写入后观察 Waker 被调用，再读取有效字节并验证 EOF；另检查超限、未终止末帧和允许零字节载荷的空帧。验证不依赖墙钟延迟，没有真实网络或线程并行结论。

## 资源驱动、文件和取消

内存流并不需要网络事件驱动，这不意味着任意网络 Future 都能随意换执行器。真正的 socket 和计时器可能绑定运行时驱动器，且驱动器必须持续运行。只满足 AsyncRead trait 不代表资源没有额外的环境约束。

普通文件也不应简单等同于非阻塞 socket。稳定接口下 Tokio 的 [`fs`](https://docs.rs/tokio/latest/tokio/fs/) 通常通过阻塞线程池执行普通文件操作，避免占住异步工作线程；异步 API 不保证操作系统直接提供异步磁盘接口。批量小文件操作仍会消耗线程池资源，专用文件类型、平台与版本的限制也应分别核对。

取消 Future 表示停止等待，不表示撤销已经发生的 I/O。`write_all` 被取消前可能已经发送部分字节，重新从头发送可能重复数据。本章的帧读取函数也**不是可无损重启的操作**：局部 Vec 会在取消时丢弃，但先前通过 consume 消费的字节不会回到流里。虽然单次 fill_buf 有自身的取消保证，整个多步函数不因此自动可取消重试。需要跨超时恢复时，应把解析进度存放在调用方持有的状态机中，或在取消后按协议关闭/重建连接。

继续学习时，可阅读[IO 与阻塞问题](../part-guide/io.md)以及[取消与超时](../06_multiple_futures/05_cancellation_and_timeouts.md)。本章的有界换行协议只用于建立基础模型，不能直接充当 HTTP 解析器或完整服务器。
