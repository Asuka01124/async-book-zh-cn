<a id="part-1-a-guide-to-asynchronous-programming-in-rust"></a>
# 第 1 部分：Rust 异步编程指南

本部分以教程的形式介绍 Rust 异步编程，面向刚开始学习 Rust 异步编程的读者。无论你是否接触过其他语言的异步编程，本部分都应该对你有所帮助。如果你有相关经验，可以跳过第一节，或快速浏览以温习相关知识。你也可以尽早阅读这篇[与其他语言异步编程的比较](#comparing-other-languages)。

<a id="core-concepts"></a>
## 核心概念

我们将先讨论不同的[并发编程](concurrency.md)模型，包括使用进程、线程或异步任务。第一章会介绍 Rust 异步模型的核心内容；接着在[第二章](async-await.md)中，我们将引入 async 和 await 编程范式，并深入探讨异步编程的细节。[下一章](more-async-await.md)还会介绍更多异步编程概念。

异步编程的主要动机之一是提升 IO 性能，这部分内容将在[下一章](io.md)介绍。同一章还会详细讨论*阻塞*。阻塞是异步编程中的一大隐患：某个操作（通常是 IO 操作）以同步方式等待，导致线程无法继续执行。

异步编程的另一个动机是，它为[并发代码的抽象与组合](concurrency-primitives.md)提供了新的模型。介绍完这些内容后，我们将转向并发任务之间的[同步](sync.md)。

本书还专门用一章介绍[异步编程工具](tools.md)。

最后几章会讨论一些更专门的主题，首先是[异步销毁与清理](dtors.md)（这是常见需求，但目前还没有完善的内置解决方案，因此属于较为专门的话题）。

接下来的两章将详细介绍异步编程的两个基础构件：[Future](futures.md) 和[运行时](runtimes.md)。

最后，我们会介绍[计时器与信号处理](timers-signals.md)以及[异步迭代器](streams.md)（也称为流）。异步迭代器用于处理一系列异步事件（相比之下，单个异步事件由 Future 或异步函数表示）。语言目前仍在积极完善这一领域，因此有些地方还不够成熟。
<a id="comparing-other-languages"></a>
## 与其他语言异步编程的比较

如果你用过 JavaScript、Python 等语言的异步功能，需要特别留意 Rust 的执行模型：调用 Rust 的 `async fn` 会返回一个 Future，函数体此时还没有执行。Future 需要通过 `.await` 或执行器的轮询才能推进。创建 Future 与把它提交给运行时作为任务执行是两件事；例如，`tokio::spawn` 会立即安排任务，而返回的 `JoinHandle` 用于等待该任务的结果。

Rust 标准库提供 `Future` trait 和语言所需的基本类型，应用程序则按需选择 Tokio 等运行时。Future 可以借用数据，因此不必总是分配到堆上或满足 `'static`；但生成独立任务的 API 通常有额外的所有权和线程安全约束。后续章节会具体解释这些差异。

技术依据：[标准库 `Future` 文档](https://doc.rust-lang.org/std/future/trait.Future.html)、[Rust Reference 的 `.await` 语义](https://doc.rust-lang.org/reference/expressions/await-expr.html)和 [Tokio 任务文档](https://docs.rs/tokio/latest/tokio/task/)。
