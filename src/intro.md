说明：上游英文稿在长时间缺少维护后正在重写，仍有内容缺失或尚待完善。这是上游的状态说明；中文版已完成的翻译与校订内容可从[第一部分：指南](part-guide/intro.md)开始阅读。

<a id="introduction"></a>
# 导言

本书是一本 Rust 异步编程指南，旨在帮助你迈出入门的第一步，并进一步了解更高级的主题。我们不要求你有异步编程方面的经验（无论是 Rust 还是其他语言），但假定你已经熟悉 Rust。如果你想学习 Rust，可以从[《Rust 程序设计语言》](https://doc.rust-lang.org/stable/book/)开始。

本书主要分为两部分：[第一部分](part-guide/intro.md)是入门指南，建议按顺序阅读，内容将带你从完全的新手逐步进阶到中级水平。第二部分由若干独立章节组成，讨论更高级的主题。读完第一部分后，或是在你已经有一些 Rust 异步编程经验时，这些章节会很有帮助。

你可以通过以下几种方式阅读本书：

* 你可以从头到尾按顺序阅读。对于刚接触 Rust 异步编程的读者，建议至少按此方式阅读本书的[第一部分](part-guide/intro.md)。
* 网页左侧提供了简要目录。
* 如果你想了解某个宽泛主题，可以从主题索引开始查找。
* 如果你想找到关于某个具体主题的所有讨论，可以从详细索引开始查找。
* 你也可以看看常见问题解答（FAQ）中是否已有你的问题；该栏目目前尚未完成。


<a id="what-is-async-programming-and-why-would-you-do-it"></a>
## 什么是异步编程，为什么使用它？

在并发编程中，程序会同时（或者至少看起来同时）做多件事。使用线程编程是并发编程的一种形式。线程中的代码按顺序编写，由操作系统并发地执行各个线程。在异步编程中，任务的并发通常由程序中的异步运行时来管理；底层线程的调度和 I/O 仍可能由操作系统负责。异步运行时（在 Rust 中，通常由 crate 提供）负责管理异步任务；程序员通过 `await` 标明可能暂停的位置。只有被等待的 Future 在轮询时返回 `Poll::Pending`，`.await` 才会暂停当前执行并向外返回控制权；若返回 `Poll::Ready`，代码就继续执行，因此每次 `.await` 并不保证切换任务。详情见 [Rust Reference 的 await 表达式说明](https://doc.rust-lang.org/reference/expressions/await-expr.html)。

由于任务切换通常由异步运行时在用户态完成，异步场景中的*上下文切换*往往开销较低。此外，异步任务通常比操作系统线程占用更少的内存，但具体开销取决于 Future 保存的状态、运行时实现和工作负载。因此，对于需要处理大量并发任务、且这些任务大部分时间都在等待（例如等待客户端响应或 I/O）的系统，异步编程往往很合适。对于内存非常有限、且没有提供线程的操作系统的微控制器，异步编程也可能是合适的选择。

异步编程还让程序员能够细粒度地控制任务的执行方式（例如并行与并发的程度、控制流、调度等）。因此，在许多场景中，异步编程既有表现力，也易于使用。尤其是，Rust 异步编程具有强大的取消机制，并支持多种并发方式（可通过 `spawn` 及其变体、`join`、`select`、`for_each_concurrent` 等结构表达）。借助这些结构，可以组合并复用超时、暂停和限流等功能的实现。不过，取消 Future 不等于撤销已经提交的 I/O，丢弃 Tokio 的 `JoinHandle` 也不会取消任务，详见[取消](part-guide/more-async-await.md#cancellation)一节。


<a id="hello-world"></a>
## 你好，世界！

先通过下面这个“hello, world”示例感受一下 Rust 异步代码的样子。这里没有并发，也没有真正利用异步的优势。不过，它定义并调用了一个异步函数，也确实会打印出“hello, world!”：

```rust,edition2021
{{#include ../examples/hello-world/src/main.rs}}
```

稍后我们会详细解释其中的一切。现在先注意：我们使用 `async fn` 定义异步函数；调用它会构造一个惰性的 Future，函数体由后续的轮询（`poll`）驱动执行。这里通过 `.await` 等待它；也可以将 Future 交给 `spawn` 或 `block_on` 等执行器接口驱动，并非只有 `.await` 才能执行它[^blocking]。参见标准库的 [`Future` 文档](https://doc.rust-lang.org/std/future/trait.Future.html)。

和本书中的所有示例一样，如果你想查看完整示例（例如包括 `Cargo.toml`），或想在本地运行它，可以在本书的 GitHub 仓库中找到，例如：[examples/hello-world](https://github.com/rust-lang/async-book/tree/master/examples/hello-world)。


<a id="development-of-async-rust"></a>
## Rust 异步功能的发展

Rust 的异步功能已经开发了一段时间，但它还不是语言中一个“完全定型”的部分。Rust 异步功能（至少稳定版编译器和标准库中已有的部分）可靠且性能良好。一些规模最大的科技公司已在要求极高的生产环境中使用它。不过，仍有一些功能缺失，也存在一些不够完善之处（这里的不完善指易用性，而非可靠性）。在学习 Rust 异步编程的过程中，你很可能会遇到其中一些问题。对于大多数缺失功能，都有可行的替代方案，本书也会介绍这些方案。

目前，异步迭代器（也称为 stream）的使用仍有一些不够完善的地方。trait 中的 `async fn` 已在 [Rust 1.75](https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits/) 稳定，但这类方法仍不能直接通过原生 `dyn Trait` 调用，返回 Future 的 `Send` 约束也需要额外设计。标准库的 [`AsyncIterator`](https://doc.rust-lang.org/std/async_iter/trait.AsyncIterator.html) 和 [`AsyncDrop`](https://doc.rust-lang.org/std/future/trait.AsyncDrop.html) 目前仍是仅限 nightly 的实验性 API，异步析构尚无完善的稳定版内置方案。

Rust 异步功能仍在积极开发中。如果你想跟进进展，可以查看 Async Working Group 的[主页](https://rust-lang.github.io/wg-async/meetings.html)，其中包含他们的[路线图](https://rust-lang.github.io/wg-async/vision/roadmap.html)。你也可以阅读 Rust 项目中的异步[项目目标](https://github.com/rust-lang/goals/issues/105)。

Rust 是一个开源项目。如果你想为 Rust 异步功能的开发做贡献，可以从 Rust 主仓库的[贡献指南](https://github.com/rust-lang/rust/blob/master/CONTRIBUTING.md)开始。


[^blocking]: 其实这个示例并不好，因为 `println` 会执行*阻塞式 I/O*，而通常不应该在异步函数中执行阻塞式 I/O。我们会在[阻塞与非阻塞 I/O](part-guide/io.md#blocking-and-non-blocking-io)一节中解释什么是阻塞式 I/O，并在[其他阻塞操作](part-guide/io.md#other-blocking-operations)一节中说明为什么不应该在异步函数中执行阻塞式 I/O。
