<a id="async-io-readiness-vs-completion-and-io_uring"></a>
# 异步 I/O：就绪与完成，以及 io_uring

上游只有英文目录标题，没有英文正文；标题由 GPT-6 Luna 翻译，正文由 GPT-6.1 Sol 原创补写。异步 API 都可能返回 Future，但 Future 背后的操作系统协议不止一种。理解就绪与完成的差别，有助于判断缓冲区由谁持有、取消后还会发生什么，以及运行时能否安全跨平台。

## 两种通知分别说了什么

| 模型 | 通知的含义 | 接下来应用做什么 |
| --- | --- | --- |
| 就绪（readiness） | 某资源现在可能可以执行某类操作 | 尝试非阻塞 read/write，处理短操作、错误及 WouldBlock |
| 完成（completion） | 先前提交的某个操作有了完成结果 | 读取操作结果，按协议回收缓冲区/操作登记并决定后续提交 |

就绪通常围绕资源的“可读/可写”状态，完成通常围绕具体请求。它们都可能由同一个执行器转为 Waker 通知，因而仅看 `.await` 无法分辨底层。Rust 的 Poll 也只是 Future 协议，不等于 Unix poll 系统调用。

“完成”指某一层的操作完成，不自动表示对端应用收到、事务提交或数据持久化。一次写完成也可能只写了部分字节，仍要处理返回长度。收到可读也可能代表 EOF 或错误，不能等同于有完整业务消息。

## epoll 与 kqueue：状态变化不等于消息边界

Linux [`epoll`](https://man7.org/linux/man-pages/man7/epoll.7.html) 维护关注的文件描述符和就绪事件。level-triggered 在条件持续成立时仍可报告；edge-triggered 更关注变化，如果收到事件后只读一小段就重新等待，可能遗漏已经留在缓冲区中的数据。通常要使用非阻塞描述符，持续读取直到 WouldBlock，再重新等待。

BSD/macOS 常见的 [`kqueue`](https://man.freebsd.org/cgi/man.cgi?query=kqueue&sektion=2) 是更一般的过滤事件机制，网络读写是其中一类。它可以合并同一来源的事件，EV_CLEAR 等标志也影响再通知方式。不要机械地把 kqueue 每个过滤器都等同于 epoll 的某一种触发方式。

这些通知不是保存每个业务事件的持久队列，也不是“一次通知恰好对应一段固定长度数据”。TCP 是字节流；拆包、合包、长度前缀和结束条件属于上层协议。多消费者、虚假事件或另一段代码抢先读取，都可能使先前的就绪变成 WouldBlock。

## 在 Tokio 上写就绪式读取

[`TcpStream::readable/try_read`](https://docs.rs/tokio/latest/tokio/net/struct.TcpStream.html) 提供跨平台接口。先等待可读，再尝试读取；WouldBlock 时回到等待，不把它当永久失败。以下共享源码在 Windows 用真实回环 TCP 实测：

```rust
{{#include ../../examples/guide-tests/src/io_models.rs:readiness}}
```

内层循环把当前数据读到 WouldBlock 或 EOF，每次用非零栈缓冲区。这里 `Ok(0)` 可以解释成读方向结束；空缓冲区的读也可能返回 0，所以通用代码不能忽略缓冲区长度。limit 限制累计接收内存，超限时已经读出的数据不会退回 socket；此错误路径应终止或重置相应应用协议，不能直接当作未消费重试。

read_until_eof 是累计操作：如果它读入一部分后再次等待，并在此时被取消，局部 out 会被销毁，其中的字节已经从 socket 消费。即使单次 read 取消安全，也不能把整个辅助函数称为可无损重试；需要保留进度时，应把累计缓冲区放在调用方或长期存活的解码状态中。

这不是每个应用都需要的低层写法。普通场景用 AsyncReadExt 的 read、read_exact 或带缓冲的帧解码更方便。read_exact 的“收齐长度”与一次 read 的“交出当前可用部分”取消语义也不同，应逐 API 检查。无限流上的读到 EOF 没有自然结束；实际服务应增加长度、期限或关闭策略。

[Mio Poll](https://docs.rs/mio/latest/mio/struct.Poll.html) 明确要求处理虚假事件以及读到 WouldBlock，并列出不同平台选择器。Windows 使用 IOCP，但 Mio 向上暴露的仍是就绪接口；Tokio 普通 TcpStream 延续这层抽象。当前 [Mio Windows AFD 源码](https://github.com/tokio-rs/mio/blob/v1.2.4/src/sys/windows/afd.rs) 也展示了通过 AFD 轮询请求桥接通知的实现。

因此，“Windows 内核使用完成机制”不能推出“Tokio 用户必须把自己的 read 缓冲区一直交给 IOCP”。桥接层可以持有自己需要保活的内部操作状态，对外遵守借用式 API 的契约。也不能把某版文档中的内部缓冲策略当作所有版本的性能保证，或绕开 Mio 直接操作原始 handle 后仍期待其状态始终一致。

## IOCP：先提交，再处理请求结果

Windows 的重叠 I/O 可提交携带 OVERLAPPED 状态的操作，[I/O Completion Ports](https://learn.microsoft.com/en-us/windows/win32/fileio/i-o-completion-ports) 把完成包集中交给工作线程处理。完成结果与具体操作关联，未必按提交顺序由业务处理；某些操作还可能同步完成，具体通知方式受 API 和配置影响。

直接使用 [`ReadFile`](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-readfile) 等原生异步接口时，调用者必须遵守缓冲区和 OVERLAPPED 的有效期要求。操作尚未终结时，内核仍可能写入；不能因为某个 Rust Future 已经 drop 就释放内存。安全包装库要在类型和运行时登记中落实这个约束。

[`CancelIoEx`](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-cancelioex) 请求取消，不等待所有目标操作结束。正常完成、取消完成和其他错误可能竞争，必须检查最终状态；相关 OVERLAPPED 在操作完成前仍不能释放或复用。取消 API 成功返回并不代表可以立即回收一切。

## io_uring：提交队列与完成队列

Linux [`io_uring`](https://man7.org/linux/man-pages/man7/io_uring.7.html) 使用用户与内核共享的提交队列 SQ 和完成队列 CQ。应用准备 SQE 描述操作，内核处理后发布 CQE，程序读取返回值及 user_data，将结果关联到原操作。批量提交、批量回收可以减少系统调用开销，是否更快仍由负载与使用方式决定。

“一个请求一个 CQE”只是普通单次操作的入门模型。[multishot 请求](https://man7.org/linux/man-pages/man7/io_uring_multishot.7.html) 可以由一个 SQE 生成多个 CQE，必须根据相关标志判断是否仍活跃。某些零复制模式也有额外的资源释放通知。运行时不能收到任意一个事件就无条件释放操作所有资源。

io_uring 也提供 poll 操作，所以“通过 io_uring 提交”不意味着每个 API 都是同样的拥有缓冲区 read 模式。反过来，文件操作标记异步也不保证内核内部完全不使用工作线程。SQPOLL、注册文件/缓冲区、direct I/O、零复制都是各有约束的可选机制，不是 io_uring 默认同时提供的免费效果。

支持程度取决于内核、具体 opcode/flags、文件系统、设备、权限与容器策略，不能只比较一个版本号。程序要有能力探测、错误处理与必要的后备路径；Windows 没有 Linux io_uring，macOS 的 kqueue 也不能直接替代同一套 SQE/CQE 调用。

## 所有权式缓冲区解决哪一个问题

当内核异步持有缓冲区指针时，仅固定 Future 还不够：Pin 不阻止 Future 被销毁，Vec 也可能在重新分配后改变数据指针。即使数据不移动，过早释放和同时可变访问仍然会破坏安全。

一种 API 设计是把 Vec 等缓冲区所有权移入操作，完成后返回 `(结果, 缓冲区)`。[tokio-uring](https://docs.rs/tokio-uring/latest/tokio_uring/) 采用这种形态；其 [`IoBuf`](https://docs.rs/tokio-uring/latest/tokio_uring/buf/trait.IoBuf.html) / [`IoBufMut`](https://docs.rs/tokio-uring/latest/tokio_uring/buf/trait.IoBufMut.html) 对稳定内存区域及初始化范围有安全要求。用户应使用库已支持的类型，不因为 Vec 常见就随意为自定义类型实现 unsafe trait。

类型约束只是其中一半：运行时仍要在用户不再等待时保留内核所需的资源，等到适当完成点才回收。另一种设计可以由运行时拥有内部缓冲区、复制数据或采用受约束的作用域；这些办法的开销和使用限制不同。不能用借用式与所有权式接口名字不同来单独判定哪一个更快。

## Future 取消不等于内核取消完成

最少要分辨四个阶段：用户不再等待、驱动器收到取消请求、目标操作终结、驱动器确认可回收资源。它们可能跨越多个 poll 或事件批次。timeout 通常只丢弃或停止等待 Future，不会自动证明内核操作已结束；同时，已经发生的写入副作用可能仍然存在。

[io_uring 取消文档](https://man7.org/linux/man-pages/man7/io_uring_cancelation.7.html) 及 [prep_cancel](https://man7.org/linux/man-pages/man3/io_uring_prep_cancel.3.html) 区分取消请求本身与目标操作的完成，目标可能已经完成、无法及时取消，或随后返回取消结果。必须按实际操作模式处理目标的最终事件和资源，而不是只观察取消请求返回值。

下面是一个**纯内存生命周期模型**，没有发出任何 OS I/O。ModelWait 的 drop 只标记不再关心并请求取消；ModelDriver 继续持有缓冲区，即使取消请求输给正常完成，也能在模拟完成后才交还缓冲区：

```rust
{{#include ../../examples/guide-tests/src/io_models.rs:completion-model}}
```

模型故意让驱动器回收缓冲区，等待端只收到完成长度，便于展示“等待者的生命周期”和“操作资源的生命周期”分开。真实所有权 API 常把结果和缓冲区一起送回等待端；等待端已取消时则转入回收路径。该模型没有处理内核指针、部分提交、multishot 或驱动器关闭，测试它不能证明任何 IOCP/io_uring unsafe 封装安全。

对就绪式的普通借用读取，Future 可以在 Pending 时只等待通知，并未让内核长期持有调用方的栈切片；取消后能重新借用缓冲区。Windows 回环测试实际先把 Tokio read 轮询到 Pending，再丢掉它，随后发送字节并再次读取，验证这一具体 API 的行为。这个结论不能推广为任意 `read_exact`、协议解码器或所有权式读都取消安全。

## 当前运行时如何选择后端

截至 2026-10-24，普通 Tokio 网络 API 仍以跨平台抽象为主，Mio 的平台选择器包括 Linux epoll、BSD/macOS kqueue 与 Windows IOCP。需要按资源/操作核查，不能一句“Tokio 是就绪式”涵盖文件、网络、进程和平台内部机制。

当前 [`tokio::fs`](https://docs.rs/tokio/latest/tokio/fs/index.html) 的默认配置通常把普通文件操作交给 spawn_blocking；取消等待后已开始的阻塞文件操作仍可能继续。同时，Tokio 1.54.1 官方文档及本机源码已经提供 Linux 上需 `tokio_unstable` 和 `io-uring` Cargo 功能的实验文件 I/O 路径，并有 [`Builder::enable_io_uring`](https://docs.rs/tokio/latest/tokio/runtime/struct.Builder.html#method.enable_io_uring)。这不是本书Windows验证配置，也不是宣称每个Tokio网络/文件操作都已换用io_uring。

独立 tokio-uring crate、Glommio 和 Monoio 具有各自平台门槛、执行器与所有权 API，见[替代运行时](alternate-runtimes.md)。不要把它们与 Tokio 的某个实验配置混为同一个产品，也不要在升级依赖时默认取消和缓冲区协议完全不变。

本章四测试通过：两项是 Windows 真实 TCP 的就绪读取/EOF与已 Pending 借用读取消后继续，两项是内存模型的取消保活和通知。没有安装 Linux 环境，没有运行 epoll/kqueue/io_uring、直接调用原生 IOCP/CancelIoEx，或启用 Tokio 实验文件后端；也没有声称做过吞吐或零复制性能验证。
