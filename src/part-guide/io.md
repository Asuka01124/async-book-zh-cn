<a id="io-and-issues-with-blocking"></a>
# IO 与阻塞问题

高效处理 IO（输入/输出）是异步编程的主要动机之一，而且大多数异步程序都会进行大量 IO。从根本上说，IO 的问题在于它所需的时间比计算长好几个数量级，因此只等待 IO 完成、不继续处理其他工作，效率会非常低。理想情况下，异步编程可以让程序在等待 IO 时继续处理其他工作。

本章介绍异步上下文中的 IO。我们会讲解阻塞式 IO 与非阻塞式 IO 的重要区别，以及为什么阻塞式 IO 与异步编程不能很好地配合（至少需要经过一些思考和努力才能结合使用）。我们会介绍如何使用非阻塞式 IO，然后讨论 IO 与异步编程中可能出现的一些问题。我们还会了解操作系统如何处理 IO，并简要看看 io_uring 等替代 IO 方法。

最后，我们会介绍其他阻塞异步任务的方式（这很不好），以及如何恰当地将异步编程与阻塞式 IO 或长时间运行的 CPU 密集型代码结合起来。


<a id="blocking-and-non-blocking-io"></a>
## 阻塞式与非阻塞式 IO

IO 由操作系统实现；IO 工作发生在独立进程和/或专用硬件中，无论哪种情况，都在程序进程之外。这里需要区分阻塞/非阻塞调用与同步/异步完成模型，两组术语并不严格等价。阻塞调用等待结果；非阻塞调用在暂时无法推进时返回 WouldBlock；基于完成的异步接口则提交操作并在稍后报告结果。同步 IO 意味着 IO 进行时，程序（或至少线程）会等待（也就是阻塞），直到 IO 完成并从操作系统收到结果后才开始处理。异步 IO 则意味着程序可以在 IO 进行期间继续推进，并在之后取得结果。操作系统为这两种 IO 都提供了许多不同的 API，不过异步 IO 的实现方式更多样。

异步 IO 与异步编程并没有内在联系。不过，异步编程让异步 IO 更容易使用且性能更好，这是异步编程的一项主要动机。同步 IO 引起的阻塞是异步编程性能问题的主要来源之一，我们必须谨慎避免（下文会进一步讨论）。

Rust 标准库提供了用于阻塞式 IO 的函数和 trait。标准库的一些类型也支持非阻塞模式，例如 [`TcpStream::set_nonblocking`](https://doc.rust-lang.org/std/net/struct.TcpStream.html#method.set_nonblocking)，但不会自动与异步任务的唤醒机制集成。要使用方便的异步 IO，通常使用专门的库；这些库往往是异步运行时的一部分，例如 Tokio 的 [`io`](https://docs.rs/tokio/latest/tokio/io/index.html) 模块。

我们快速看一个例子（改编自 Tokio 文档）：

```rust,no_run
use tokio::{io::AsyncWriteExt, net::TcpStream};

async fn write_hello() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
    stream.write_all(b"hello world!").await?;

    Ok(())
}
```

`write_all` 是一个异步 IO 方法，用于向 `stream` 写入数据。操作可能立即完成，但更有可能需要一些时间。因此，`stream.write_all(...).await` 会暂停当前任务，使其等待操作系统处理写入。调度器会运行其他任务；写入完成后，它会唤醒该任务并安排它继续工作。

然而，如果我们使用标准库中的写入函数，异步调度器就不会参与其中，操作系统会在 IO 完成前暂停整个线程。这意味着不仅当前任务会暂停，该线程也无法执行其他任务。如果运行时线程池中的所有线程都发生这种情况（某些情况下线程池可能只有一个线程），整个程序就会停下来，无法继续推进。这称为阻塞线程（或程序），会严重影响性能。应避免在异步工作线程上直接执行可能长时间阻塞的操作；必要的阻塞操作可隔离到专用线程或阻塞池，见下文。

除了阻塞式 IO，长时间运行的任务或等待锁的任务也可能阻塞线程。本章[末尾](#other-blocking-operations)会进一步讨论这个问题。

反复读写是常见模式，而流（异步迭代器）和 sink（异步写入端）是实现这一模式的便捷机制。我们会在[专门的一章](streams.md)中介绍它们。


<a id="reading-and-writing"></a>
## 读取与写入

Tokio 用 [`AsyncRead` 和 `AsyncWrite`](https://docs.rs/tokio/latest/tokio/io/) 表示异步字节读取和写入。它们的核心方法采用 poll 接口；应用通常导入 `AsyncReadExt`、`AsyncWriteExt`，使用 `read`、`read_exact`、`write_all`、`flush` 等可 await 的扩展方法。这些 trait 属于 Tokio，不是标准库 trait，也不能直接替换 futures-rs 的同名 trait。

一次 `read` 可能只填充缓冲区的一部分，应只处理返回长度 n 对应的 `buffer[..n]`。对于非空目标缓冲区，返回零通常表示 EOF。一次 `write` 也可能只写入前缀；需要发送完整数据时使用 `write_all`，需要固定字节数时使用 `read_exact`，但后两者的取消语义不同，见下文。

TCP 提供字节流，不保留应用消息边界。下面用四字节大端长度前缀定义一个简单协议，并在分配内存前限制消息不超过 1 MiB。`read_frame` 收到不完整的头部或内容会返回 UnexpectedEof；`write_frame` 会处理部分写入。这些是可复用函数，依赖 Tokio 和 bytes：

```rust,edition2021
{{#include ../../examples/guide-tests/src/io.rs:frames}}
```

下面是无需外部服务器的完整程序。它使用内存中的 [`duplex`](https://docs.rs/tokio/latest/tokio/io/fn.duplex.html) 流模拟容量有限的连接；同时推进读写端，才能在缓冲区满时继续传输：

```rust,edition2021
{{#include ../../examples/guide-tests/src/bin/io-demo.rs}}
```

在仓库根目录运行 `cargo run --manifest-path examples/guide-tests/Cargo.toml --bin io-demo`，输出为 `hello async IO`。`guide-tests` 是独立 crate，测试复用上面同一份分帧代码。

实际类型的选择也很重要：

- `TcpStream` 实现读写字节流；UDP 的 `UdpSocket` 使用 recv_from/send_to 等数据报 API，要考虑消息边界、截断、丢包和乱序，不能照搬 TCP 的读满协议。
- [`tokio::fs::File` 与文件工具函数](https://docs.rs/tokio/latest/tokio/fs/) 提供异步接口。默认稳定路径在阻塞池执行普通文件操作；当前版本在 Linux 的 tokio_unstable 配置下还提供 io_uring 相关实验路径，不能把异步接口一概理解为内核原生异步操作。
- 标准输入输出可使用 tokio::io 的相应句柄。异步接口仍可能在内部借助阻塞线程；特别是 [`stdin`](https://docs.rs/tokio/latest/tokio/io/struct.Stdin.html) 的底层读取不可取消，可能影响运行时退出。交互式输入通常应交给专用线程。
- 管道和文件描述符需要匹配平台与资源类型。Unix 可考虑 tokio::net::unix::pipe 或 tokio::io::unix::AsyncFd；后者面向支持就绪通知的非阻塞描述符，不应随意用于普通磁盘文件。不要用 tokio::fs 读取可能永久等待的特殊文件。

<a id="memory-management"></a>
## 内存管理

读取数据时需要将其存放在某处；写入数据时，在写入完成之前也需要保留数据。异步操作可能跨越多次轮询，因此缓冲区的生命周期也是 IO 接口设计的一部分。

Tokio 的 read 借用 `&mut [u8]`，write 借用 `&[u8]`。Future 存活期间，这些借用遵循普通借用规则；调用方不能同时改写仍被写操作借用的数据，也不能移动或释放正在被读取的缓冲区。直接 await 时借用通常很方便；跨独立任务传递时，则可能需要移动 Vec 等拥有所有权的缓冲区。

分块复用一个固定大小的 Vec 或数组，可以避免每次读取都重新分配。不要对不可信的流无上限地 read_to_end，也不要直接按对端声明的任意长度分配内存。上面的消息长度检查、容量有限的队列，以及让生产者等待消费者的背压机制，可以共同限制内存占用；还要考虑每连接限制与总连接数相乘后的成本。

[`bytes`](https://docs.rs/bytes/latest/bytes/) 提供 BytesMut、Bytes、Buf 与 BufMut。BytesMut 便于逐步接收并解析数据；freeze 后的 Bytes 是不可变视图，可以廉价克隆、切片并共享底层存储：

```rust,edition2021
{{#include ../../examples/guide-tests/src/io.rs:shared-buffer}}
```

这里的 prefix 与 message 共享相同字节存储，而不是复制 "hello"。这减少的是用户空间缓冲区复制，不代表系统调用或网络协议栈也实现了零拷贝。共享视图还可能让很小的一段数据长期保留一大块底层分配；必要时复制小片段反而能更早释放大缓冲区。

基于完成的 IO 通常有更严格的缓冲区存活要求：内核在提交之后仍可能访问缓冲区，即使等待结果的 Future 已被取消。为此，tokio-uring 等接口接收缓冲区的所有权，并在操作完成时连同结果一起返回。不能简单用借用式接口包装一个内核仍在使用的指针，或在丢弃 Future 时立刻释放它；运行时必须负责在途操作与资源生命周期，见下一节。

<a id="advanced-topics-on-io"></a>
## IO 高级主题

`BufReader` 缓存多次小读取，`BufWriter` 将小写入合并为较大批次，通常可以减少调用次数。它们不是所有场景都更快：如果原本就在写大块数据，再增加一层缓冲可能没有收益。Tokio 的 BufWriter 在 drop 或 into_inner 时会丢弃未刷新的内容，**不会自动异步 flush**；需要交付数据时显式 `flush().await?`。刷新到下层也不等于落盘持久化，持久化需要按文件系统与协议要求额外处理。

下面在一对内存流上写入两行，显式刷新并关闭写端，然后让读端读到 EOF。`lines` 对本例固定输入很方便；处理外部输入时还应为行长和累计结果设置限制：

```rust,edition2021
{{#include ../../examples/guide-tests/src/io.rs:buffered-lines}}
```

同一对象可以同时实现 AsyncRead 与 AsyncWrite，表示双工通道。`tokio::io::split` 将其拆成读写两半；TcpStream 自带的 split 借用原对象，into_split 则返回可移动到不同任务的拥有所有权的两半。通用两半可以 unsplit；TCP 的对应拥有两半可以 reunite，不同来源的两半不能随意合并。应为每个方向安排明确的所有者，并正确处理 EOF 和半关闭。

[`copy`](https://docs.rs/tokio/latest/tokio/io/fn.copy.html) 将单向字节流从读端复制到写端，并返回复制的字节数；[`copy_bidirectional`](https://docs.rs/tokio/latest/tokio/io/fn.copy_bidirectional.html) 同时推进两个方向。后者在一个方向 EOF 后关闭对端的对应写方向，仍继续另一个方向，适合 TCP 代理的半关闭语义。测试中用两对 duplex 流验证了请求、响应及两个方向的字节数。duplex 是双工内存连接；仅需一个方向时可以使用 simplex 或单向管道。

取消安全必须逐个方法核对。下表指的是在 select 竞争失败、当前方法的 Future 尚未完成时被丢弃的情况；不代表所有底层操作都能撤销：

| 方法 | 取消后的关键行为 |
| --- | --- |
| `read` / `read_buf` | Tokio 文档保证取消安全：等待中的调用被取消时，不会悄悄消耗输入。已完成并返回的字节由调用方处理。 |
| `read_exact` / `read_to_end` | 不保证取消安全；内部可能已经消费了一部分输入，调用方不能假设重新调用会从相同位置开始。 |
| `write` | 等待中的调用取消安全；成功时仍可能只返回部分写入长度。 |
| `write_all` | 不保证取消安全；取消时可能已发送前缀，直接从头重试可能重复发送。 |
| `Lines::next_line` | 本方法取消安全，需保留同一个 Lines 对象，使解析状态继续存活；不能推广到 `read_line`。 |

依据：[`AsyncReadExt`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncReadExt.html)、[`AsyncWriteExt`](https://docs.rs/tokio/latest/tokio/io/trait.AsyncWriteExt.html)、[`Lines::next_line`](https://docs.rs/tokio/latest/tokio/io/struct.Lines.html#method.next_line) 和 [`BufWriter`](https://docs.rs/tokio/latest/tokio/io/struct.BufWriter.html)。本章测试用容量有限的 duplex 和显式 poll 确认了部分消费、部分发送与取消之间的区别。

前面的 read_frame 将头部和消息缓冲区保存在函数内部，因此**不是取消后可无损重试的协议读取器**。不要在 select 循环里不断创建它，取消后又直接从同一流重读。可选择让一个长期任务拥有读取器并通过通道传递完整消息，或把解码状态保存在长期存在的外部对象中。取消 Future 只意味着不再推进或等待它，并不回滚数据消费、网络写入或已经提交的磁盘操作。

若必须使用同步 IO，可以在 spawn_blocking 或专用线程里运行，再用通道或任务结果桥接回异步代码；具体取舍见本章后半部分。

<a id="the-os-view-of-io"></a>
## 从操作系统角度看 IO

异步 IO 常见两类底层机制：

| 机制 | 事件意味着什么 | 程序需要做什么 |
| --- | --- | --- |
| 就绪通知 | 资源现在可能可读或可写 | 收到事件后尝试非阻塞读写，若得到 WouldBlock 则重新等待。通知可能是过时或虚假的。 |
| 完成通知 | 已提交的操作产生了结果 | 收取结果和实际字节数，并在完成前保持缓冲区和资源有效。 |

不能把这张表简单等同于不同操作系统的固定分类：运行时可能在一个平台上模拟另一种抽象。Tokio 网络接口通常以就绪式模型向用户暴露能力；默认文件接口则可用阻塞池实现异步等待。

[`mio`](https://docs.rs/mio/latest/mio/) 提供底层非阻塞 IO 与事件通知，是构建事件驱动系统的基础工具；它不是完整的 async/await 运行时。普通应用通常不必自行实现事件循环。确实需要直接使用 Tokio 就绪接口时，要把“等待可写”与“尝试写入”放入循环，并处理部分写入与 WouldBlock：

```rust,edition2021
{{#include ../../examples/guide-tests/src/io.rs:readiness}}
```

即使 writable 已完成，try_write 也可能返回 WouldBlock；应重新等待，不能把一次就绪当作永久可写。详见 [`TcpStream::writable`](https://docs.rs/tokio/latest/tokio/net/struct.TcpStream.html#method.writable)。本章在绑定到 127.0.0.1 随机端口的真实 TCP 连接上验证此循环。

Linux 的 io_uring 提供提交队列与完成队列，可用于文件和网络等操作。[`tokio-uring`](https://docs.rs/tokio-uring/latest/tokio_uring/) 在独立的启动入口中管理 Tokio 与 io_uring 驱动，并使用拥有所有权的缓冲区；例如 read_at 返回 `(结果, 缓冲区)`。它与常规 Tokio 的借用式 IO API 不能直接混用。使用前必须检查内核版本、运行环境限制和库的支持范围。本章在 Windows 验证了通用 Tokio 示例，没有运行 Linux 专用的 io_uring 示例。

<a id="other-blocking-operations"></a>
## 其他阻塞操作

正如本章开头所说，不阻塞线程对异步程序的性能至关重要。各种阻塞式 IO 是造成阻塞的常见原因，但大量计算，或以异步调度器无法协调的方式等待，也会导致阻塞。

等待最常见的原因是使用了不感知异步的同步机制，例如锁竞争时等待 std::sync::Mutex，或阻塞接收同步通道。短小、低竞争且不跨 await 的数据临界区可以使用同步互斥锁，不能仅因名字里没有 async 就判定它一定不合适。本书将在[通道、锁与同步](sync.md)一章讨论这个问题。还有其他方式会造成阻塞式等待；通常需要找到非阻塞或适合异步环境的机制，例如使用异步 `sleep` 函数，而不是 std 提供的版本。等待也可能是忙等（实际上就是不做任何工作的循环，也称为自旋锁），通常应当避免这种做法。

<a id="cpu-intensive-work"></a>
### CPU 密集型工作

执行长时间运行的工作（即 CPU 密集型或 CPU 受限工作）会阻止调度器运行其他任务。这*确实*是一种阻塞，但它没有 IO 阻塞或等待那么糟糕，因为至少程序仍在取得进展。不过，如果不加以谨慎处理，某些性能指标（例如尾延迟）很可能不理想；如果无法运行的任务必须在特定时间运行，还可能影响正确性。有人会说，CPU 密集型工作根本不该使用 Rust 异步编程（或 Tokio 等通用异步运行时），但这过于简化了。准确地说，如果不做特殊处理，就不能把 IO 受限任务和 CPU 受限任务（更精确地说，是长时间运行的任务与对延迟敏感的任务）混在一起，还指望一切顺利。

本节余下部分将假设你同时有对延迟敏感的任务，以及长时间运行的 CPU 密集型任务。如果没有对延迟敏感的工作，情况会有所不同（通常也更简单）。

运行长时间任务或阻塞任务，基本上有三种方案：使用运行时内置功能、使用独立线程，或使用独立运行时。

在 Tokio 中，可以使用 [`spawn_blocking`](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html) 创建可能发生阻塞的任务。它的用法类似于用于创建任务的 [`spawn`](https://docs.rs/tokio/latest/tokio/task/fn.spawn.html)，但会把任务放在一个专为可能阻塞的任务优化的独立线程池中运行（任务很可能在自己的线程上运行）。请注意，这里运行的是普通同步代码，而不是异步任务。已开始运行的阻塞闭包不能被 abort 中断；尚未开始的排队任务可能被取消，但不保证。运行时退出通常还会等待已开始的阻塞任务结束，shutdown_timeout 只是停止等待，并不会终止它们。大量 CPU 工作应通过信号量或专用计算池限制并行度，以免阻塞池产生过多线程。其他运行时也提供类似功能。

这个示例使用 `spawn_blocking` 执行阻塞式 IO，具体做法是调用标准库中的同步文件系统函数。请注意，[`tokio::fs`](https://docs.rs/tokio/latest/tokio/fs/index.html) 也提供异步文件系统 API；不过，默认稳定路径同样通过阻塞池包装普通文件操作；实验性 Linux IO 路径的配置应以当前文档为准。

```rust,no_run
#[tokio::main]
async fn main() {
    let contents = tokio::task::spawn_blocking(|| {
		std::fs::read_to_string("file.txt").unwrap()
    })
	.await
	.unwrap();

	// 对 contents 做一些处理
}
```

由于通过 `spawn_blocking` 创建且已开始执行的任务无法强制中止，它适用于最终会完成的工作。可能无限期阻塞的任务（例如监听传入请求的服务器）最好放在专用线程上运行，以免长时间占用 Tokio 阻塞线程池中的线程。你可以通过 [`std::thread::spawn`](https://doc.rust-lang.org/stable/std/thread/fn.spawn.html) 或类似 API 创建线程。

如果需要运行大量任务，你可能需要某种线程池或工作调度器。如果不断创建线程，使线程数量远超可用核心数，最终会牺牲吞吐量。[Rayon](https://github.com/rayon-rs/rayon) 是一种常见选择，可以方便地运行和管理并行任务。如果选用更贴合工作负载，和/或了解所运行任务特性的工具，性能可能会更好。

下面是一个同时使用 Rayon 和 Tokio 的例子。此例需要额外依赖 `rayon = "1"`，通过 [`rayon::spawn`](https://docs.rs/rayon/latest/rayon/fn.spawn.html) 把闭包提交到 Rayon 池。它利用 [`tokio::sync::oneshot::channel`](https://docs.rs/tokio/latest/tokio/sync/oneshot/fn.channel.html)，在 Rayon 创建的任务与 Tokio 中的当前任务之间传递结果。

```rust,no_run
use rayon::prelude::*;

#[tokio::main]
async fn main() {
    let data = 1..=10;

    let (send, recv) = tokio::sync::oneshot::channel();
    // 在 rayon 上创建任务，以免阻塞当前任务
    rayon::spawn(move || {
        // 使用 rayon 的并行迭代器并行计算结果
        let results = data.into_par_iter().map(compute).collect::<Vec<_>>();
        // 将结果发送回 Tokio。
        send.send(results).unwrap();
    });

    // 等待 rayon 任务并取得结果
    let results = recv.await.unwrap();
    println!("Results: {:?}", results);
}

fn compute(input: u64) -> u64 {
    // 通过累加大量整数
    // 模拟 CPU 密集型计算。
    let mut sum = 0u64;
    for i in 0..100_000_000 {
        sum = sum.wrapping_add(i * i);
    }
    sum % input
}
```

可以为对延迟敏感的任务和长时间运行的任务分别使用独立的异步运行时实例。这适合 CPU 受限任务，但即使在用于长时间任务的运行时上，也不应使用阻塞式 IO。对于 CPU 受限任务，这是一种不错的方案，因为它能在隔离调度资源的同时保留异步任务的编写方式；这不是支持长时间异步工作的唯一方式。它也很灵活（可以配置运行时，使其适合所运行的任务类型；事实上，要获得最佳性能，确实需要花些精力配置运行时），并且能让你受益于 Tokio 这类成熟、设计良好的子系统。你甚至可以使用两个不同的异步运行时。无论如何，这些运行时必须运行在不同线程上。

另一方面，你确实需要多加考虑：必须确保任务运行在正确的运行时上（这可能比听起来更难），而任务之间的通信也可能很复杂。接下来我们会讨论同步与异步上下文之间的同步；多个异步运行时之间的情况可能更加棘手。每个运行时都有自己的一小片任务世界，调度器完全相互独立。Tokio 的通道和锁*可以*在不同运行时（包括非 Tokio 运行时）之间使用，但其他运行时提供的原语未必也能这样使用。

由于每个运行时的调度器都不了解其他运行时（操作系统也不了解任何异步调度器），调度之间没有协调或共享优先级，运行时之间也无法窃取工作。因此，任务调度可能不是最优的（尤其是在运行时没有针对各自的工作负载进行良好调优时）。此外，由于所有调度都是协作式的，长时间运行的任务仍可能得不到资源，延迟也会受到影响。如何让长时间任务更好地协作，见[下一节](#yielding)。

作为纯粹的调度器，与专用的同步工作线程池相比，使用 Tokio 处理 CPU 工作可能会有稍高的开销。考虑到支持异步编程需要额外工作，这并不意外。对大多数用户来说，实践中这不太可能成为问题；不过，如果代码对性能极其敏感，可能值得考虑这一点。

对于上述任何方案，任务都可能运行在不同上下文中（同步与异步上下文，或不同的异步运行时）。如果需要在任务之间通信，就要注意选择正确组合的同步和异步原语（通道、互斥锁等），并在这些原语上调用正确的方法（阻塞或非阻塞）。对于互斥锁和类似的锁，如果需要跨越 await 点持有锁，或保护 IO 资源，通常应使用异步版本（在同步上下文中应能通过阻塞式加锁方法使用它）；如果是保护数据，或不需要跨越 await 点持有锁，则可使用同步版本。Tokio 的异步通道可以在同步上下文中通过阻塞方法使用；至于何时使用同步或异步通道，请参阅[这些文档](https://docs.rs/tokio/latest/tokio/sync/mpsc/index.html#communicating-between-sync-and-async-code)了解详情。

那么，上述方案该选哪一种？

- 如果要执行阻塞式 IO，通常应使用 `spawn_blocking`。自建阻塞线程池或专用线程也是有效选择；第二个异步运行时本身不会把同步阻塞调用变成异步 IO。
- 如果有一个线程会一直运行，应使用 `std::thread::spawn`，而不要使用任何线程池（因为它会一直占用池中的一个线程）。
- 如果要进行*大量* CPU 工作，应使用线程池，可以是专用线程池，也可以是第二个异步运行时。
- 异步任务生命周期很长并不意味着需要第二个运行时。关键是每次 poll 是否能及时返回；只有确有隔离或调度需求时才考虑独立运行时。
- 即使更复杂的方案更理想，你仍可能选择专用线程或 `spawn_blocking`，因为它们简单且性能令人满意。


<a id="yielding"></a>
### 让出控制权

长时间运行的代码会造成问题，因为它不给调度器调度其他任务的机会。异步并发是协作式的：调度器不能抢占一个任务并运行另一个任务。如果长时间运行的任务不向调度器让出控制权，调度器就无法停止它。不过，如果长时间运行的代码确实向调度器让出控制权，其他任务就可以得到调度，任务运行时间长这一点本身也不再是问题。对于 CPU 密集型工作，可以用这种方式替代使用另一个线程，或在单独的运行时中处理，以（可能）改善性能。

让出控制权很简单，只需调用运行时的 yield 函数。在 Tokio 中，该函数是 [`yield_now`](https://docs.rs/tokio/latest/tokio/task/fn.yield_now.html)。请注意，它不同于标准库的 [`yield_now`](https://doc.rust-lang.org/stable/std/thread/fn.yield_now.html)，也不同于用于从 coroutine 中让出的 `yield` 关键字。`yield_now().await` 可能被上层组合器吸收，例如 select 的另一个分支在同一次 poll 中完成；因此不能保证每次都向调度器让出控制权，也不保证下一次一定执行其他任务（参见[并发组合 Future](concurrency-primitives.md)一章）；这是否符合你的预期，要视情况而定。

判断何时需要让出控制权就稍微复杂一些。首先需要知道程序是否在隐式让出控制权。这只会发生在 `.await` 处，所以如果没有使用 `await`，就不会让出控制权。不过，await 并不会自动向调度器让出控制权。只有在被 await 的叶子 Future 处于 pending（未就绪）状态，或调用栈中的某处显式执行了 `yield` 时，才会让出控制权。Tokio 和大多数异步运行时会在 IO 和同步函数中这样做，但一般来说，若不调试或检查源代码，就无法知道某个 `await` 是否会让出控制权。

10 到 100 微秒内到达一个可能让出控制权的点，是面向延迟敏感负载的经验建议，而不是 Tokio 的时限保证；应结合实际延迟目标与测量结果选择分块粒度。

<a id="references"></a>
### 参考资料

- [Tokio 关于 CPU 受限任务和阻塞代码的文档](https://docs.rs/tokio/latest/tokio/index.html#cpu-bound-tasks-and-blocking-code)
- [博客文章：什么是阻塞？](https://ryhl.io/blog/async-what-is-blocking/)
- [博客文章：使用 Rust 的 Tokio 异步运行时处理 CPU 受限任务](https://thenewstack.io/using-rustlangs-async-tokio-runtime-for-cpu-bound-tasks/)

> 校订说明：本章把上游四个待写小节和相关提纲补成正文与可测试示例，并修正阻塞/非阻塞与异步的关系、sink 的定义、同步锁的适用场景、spawn_blocking 取消边界、独立运行时的取舍及 yield_now 的非保证。阻塞池与协作调度依据见 [`spawn_blocking`](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html) 和 [`yield_now`](https://docs.rs/tokio/latest/tokio/task/fn.yield_now.html) 官方文档。
