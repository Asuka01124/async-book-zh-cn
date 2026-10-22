<a id="async-and-await"></a>
# Async 与 Await

本章将带你开始使用 Rust 进行异步编程，并介绍 `async` 和 `await` 关键字。

`async` 可以修饰函数声明，也可以构造异步块和异步闭包；trait 中也可以声明异步方法，但不能直接写 `async trait`；`await` 则是在表达式中使用的运算符。不过，在深入介绍这两个关键字之前，我们需要先了解 Rust 异步编程中的几个核心概念。这些内容承接上一章的讨论，并将直接联系到 Rust 编程。

<a id="rust-async-concepts"></a>
## Rust 异步概念

<a id="the-runtime"></a>
### 运行时

异步任务必须经过管理和调度。任务数量通常多于可用核心，因此无法一次全部运行。一个任务停止执行时，必须选择另一个任务运行。如果任务正在等待 IO 或其他事件，就不应被调度；等事件完成后，则应当再次调度它。这需要与操作系统交互并管理 IO 工作。

许多编程语言都提供运行时。运行时通常不只管理异步任务——它还可能管理内存（包括垃圾回收）、参与异常处理、提供操作系统抽象层，甚至就是一台完整的虚拟机。Rust 是一种底层语言，力求将运行时开销降到最低。因此，Rust 的异步运行时职责范围比许多其他语言的运行时小得多。异步运行时的设计和实现方式也有很多种，所以 Rust 不会自带一个运行时，而是让你根据需求自行选择。这也意味着，开始异步编程需要多做一个步骤。

Tokio 这类通用运行时除了运行和调度任务，还提供与操作系统交互的 IO 驱动和计时器。只有执行器的库或嵌入式运行时不一定具有这些组件。运行时的结构没有严格规定，但常见的术语和职责划分如下：

- *reactor*、*event loop* 或 *driver*（这里泛指相近的事件驱动职责，不同库对它们的划分并不完全相同）：分发 IO 和计时器事件，与操作系统交互，并以最低层机制推动执行；
- *scheduler*：决定任务何时执行，以及在哪些操作系统线程上执行；
- *executor* 或 *runtime*：结合 reactor 和 scheduler，并提供面向用户的异步任务运行 API；*runtime* 也可以指整套功能库（例如 Tokio crate 中的所有内容，而不只是由 [`Runtime`](https://docs.rs/tokio/latest/tokio/runtime/struct.Runtime.html) 类型表示的 Tokio executor）。

除了上面介绍的 executor，运行时 crate 通常还会包含许多实用 trait 和函数，例如 IO trait（如 `AsyncRead`）及其实现、网络或文件系统访问等常见 IO 任务的功能、锁、通道和其他同步原语、计时工具、操作系统相关工具（如信号处理）、用于处理 Future 和流（异步迭代器）的工具函数，以及监控和观测工具。本指南会介绍其中许多内容。

可供选择的异步运行时有很多。有些运行时采用截然不同的调度策略，或针对特定任务或领域进行了优化。本指南大部分内容将使用 [Tokio](https://tokio.rs/) 运行时。它是一个通用运行时，在生态系统中广泛使用。无论是入门还是用于生产环境，它都是很好的选择。在某些情况下，换用其他运行时可能会获得更好的性能，或写出更简单的代码。本指南后面会介绍一些其他运行时，以及选择不同运行时甚至自行编写运行时的理由。

要尽快运行起来，只需要少量样板代码。你需要像添加其他 crate 一样，在 Cargo.toml 中将 Tokio crate 添加为依赖：

```
[dependencies]
tokio = { version = "1", features = ["full"] }
```

然后在 `main` 函数上使用 `tokio::main` 标注，使它可以成为异步函数（Rust 本身不允许这样写）：

```rust,no_run
#[tokio::main]
async fn main() {}
```

就这么简单！现在你可以开始编写异步代码了！

`#[tokio::main]` 宏将入口改写为同步函数，构建运行时，再使用 `Runtime::block_on` 驱动异步函数体。这个根 Future 不是通过 `spawn` 创建的工作任务；详见[宏的官方说明](https://docs.rs/tokio/latest/tokio/attr.main.html)。本指南后面会更详细地解释这个标注的作用，以及如何不使用它来编写异步代码（这样会更灵活）。

<a id="futures-rs-and-the-ecosystem"></a>
### Futures-rs 与生态系统

[futures-rs](https://docs.rs/futures/latest/futures/) 是一组围绕 Future、流和异步组合工具的库。早期 Rust 异步生态在语言原生支持成熟之前就使用它；自 [Rust 1.39](https://blog.rust-lang.org/2019/11/07/Rust-1.39.0/) 稳定 `async`/`.await` 后，基本的 Future trait 位于标准库 `std::future` 中，语言语法由编译器提供，并不要求应用依赖 `futures` crate。

如果只使用 Tokio 的异步网络、计时器和任务 API，通常可以先只依赖 Tokio。需要 `FutureExt`、`StreamExt`、`FuturesUnordered`、组合器，或希望编写可供不同执行器使用的库时，`futures` 仍很有用。它提供执行器等功能，但不等同于 Tokio 这种包含网络驱动和计时器的完整运行时。

不同库有一些相似的 API，例如 Tokio 和 futures-rs 的 `join!`、`select!`，但约束与取消语义可能不同，不能只凭名字替换。两者的 [`AsyncRead`](https://docs.rs/futures/latest/futures/io/trait.AsyncRead.html) 等 IO trait 也是不同的类型；跨生态使用时需要适配器。后续章节会说明具体差异。

生态中还有 smol 等运行时，以及 futures-concurrency、async-channel 等聚焦组合或通信的库，也有探索不同 IO 模型的实验性实现。选库时应检查稳定性、平台支持、维护状态、运行时依赖及语义，而不是一次引入所有工具。本指南先使用 Tokio，再按实际需要引入其他库。

<a id="futures-and-tasks"></a>
### Future 与任务

Rust 异步并发的基本单位是 *Future*。Future 只是一个普通的 Rust 对象（通常是 struct 或 enum），它实现了 [`Future`](https://doc.rust-lang.org/std/future/trait.Future.html) trait。Future 表示一个延后执行的计算，也就是一个将在未来某个时刻就绪的计算。

本指南会多次讨论 Future，不过入门时最好先不必太在意它们。接下来几节会经常提到 Future，但直到后面才会正式定义它们或直接使用它们。Future 的一个重要特点是可以组合起来，构成新的、更“大”的 Future（之后我们会详细讨论*如何*组合它们）。

在上一章和本章中，我以非正式的方式多次使用“异步任务”这个术语，指的是一段逻辑上的执行序列；它类似于线程，但由程序内部管理，而不是由操作系统在外部管理。用任务的概念来思考通常很有帮助，但 Rust 本身并没有“任务”这一概念，而且这个词还有不同含义！这很容易造成混淆。更复杂的是，运行时确实有任务这一概念，而不同运行时对任务的理解也略有不同。

从这里开始，我会尽量精确地使用与任务有关的术语。单独使用“任务”时，我指的是一段可能与其他任务并发发生的计算序列这一抽象概念。“异步任务”也表示同一概念，用来与通过操作系统线程实现的任务相区分。“运行时的任务”表示某个运行时所定义的任务类型；“Tokio 任务”（或其他特定运行时的任务）则表示 Tokio 对任务的定义。

Rust 中的异步任务就是一个 Future（通常是由许多其他 Future 组合而成的“大”Future）。换句话说，任务就是正在执行的 Future。不过，有时 Future 会在不作为运行时任务的情况下被“执行”。直观地说，这种 Future 是一个*任务*，但不是*运行时的任务*。等我们看到相关示例时，会进一步说明这一点。


<a id="async-functions"></a>
## 异步函数

`async` 关键字用于修饰函数声明。例如，我们可以写 `pub async fn send_to_server(...)`。异步函数就是使用 `async` 关键字声明的函数，这意味着它可以异步执行；换句话说，调用方*可以选择不*等待函数完成，而是先做其他事情。

更具体地说，调用异步函数时，它的函数体不会像普通函数那样执行。函数体及其参数会被打包成一个 Future 并返回，而不是返回实际结果。调用方随后可以决定如何处理这个 Future（如果想“马上”得到结果，就可以对 Future 使用 `await`，见下一节）。

在异步函数内部，代码仍按通常的顺序方式执行[^preempt]；异步这一点不会改变执行方式。你可以在异步函数中调用同步函数，执行过程与平常一样。异步函数中还可以使用 `await` 等待其他异步函数（或 Future）；这*可能*导致让出控制权，使另一个任务得以执行。

[^preempt]: 与其他线程一样，运行异步函数的线程可能会被操作系统抢占并暂停，以便另一个线程完成一些工作。不过，从函数的角度看，除非检查可能被其他线程修改的数据，否则无法观察到这一点（而且，即使当前线程没有暂停，另一个并行执行的线程也可能修改这些数据）。

## `await`

前面说过，Future 表示一个将在未来某个时刻就绪的计算。要取得该计算的结果，我们使用 `await` 关键字。如果结果已经就绪，或无需等待就能计算出来，那么 `await` 会直接执行计算并产生结果。不过，如果结果尚未就绪，`await` 就会把控制权交给调度器，让另一个任务继续执行（这就是上一章提到的协作式多任务处理）。

在 Rust 中，await 的语法是 `some_future.await`，也就是说，它是一个后缀关键字，与 `.` 运算符一起使用。因此，在方法调用和字段访问的链式表达式中使用它很方便。这与 Python 或 JavaScript 等语言不同：这些语言把 `await` 作为前缀运算符放在表达式之前，例如 `await some_function()`。

为了理解后缀 await 为什么往往更方便，假设你调用一个会发起网络请求的异步函数，并想访问响应的状态码。使用前缀 `await` 语法时，你需要在 `fetch()` 前加上 `await`，再用括号括起表达式以便通过 `?` 传播错误，最后访问状态码，例如 `(await fetch())?.status_code`。使用后缀语法则可以写成 `fetch().await?.status_code`。在更长的调用链中，这种写法尤其方便。例如，包含两个前缀 await 的表达式是 `(await (await fetch())?.json())?.data`，而对应的后缀写法是 `fetch().await?.json().await?.data`，读起来更自然。

现在来看看如何实际使用 `async` 和 `await`。请看下面的函数：

```rust,edition2021
{{#include ../../examples/guide-tests/src/basics.rs:add}}
```

如果调用 `add(15, 3).await`，它会立即返回结果 `18`。如果调用 `wait_to_add(15, 3).await`，最终也会得到相同结果，但在等待期间，其他任务会有机会运行。

在这个简单的示例中，调用 `sleep` 是一种替代写法，代表执行某个需要等待结果的长时间任务。这通常是一个 IO 操作，其结果可能是从外部来源读取的数据，也可能是向外部目标写入成功的确认。读取操作大致如下：`let data = read(...).await?`。在这种情况下，读取期间 `await` 会让当前任务等待。读取完成后，任务会恢复执行（读取任务等待时，其他任务可以完成一些工作）。读取结果可能是成功读到的数据，也可能是错误（由 `?` 处理）。

请注意，如果调用 `add`、`wait_to_add` 或 `read` 时没有使用 `.await`，我们就得不到任何结果！

什么？

调用异步函数会返回一个 Future，而不会立即执行函数中的代码。此外，`async fn` 返回的 Future 在被轮询之前不会执行函数体[^poll]。这里不意味着任意实现 `Future` 的值所关联的工作都尚未开始；例如 `JoinHandle` 所代表的任务在 `spawn` 后就可以运行。这与其他一些语言不同：在那些语言中，异步函数返回的 Future 会立即开始执行。

这是 Rust 异步编程中一个重要的特点。熟悉之后你会觉得理所当然，但它常常让初学者困惑，尤其是有其他语言异步编程经验的人。

理解 Rust 中的 Future 时，一个重要直觉是：它们是惰性对象。要让它们执行任何工作，就必须由外部力量（通常是异步运行时）推动它们前进。

我们从操作角度描述了 `await`（它运行 Future 并产生结果），但上一章讨论了异步任务和并发；`await` 如何融入这个思维模型呢？先来看纯顺序代码：从逻辑上说，调用函数就是执行函数中的代码（并进行一些变量赋值）。换句话说，当前任务会继续执行由函数定义的下一“段”代码。同样，在异步上下文中，调用非异步函数会直接继续执行该函数。调用异步函数会找到要运行的代码，但不会运行它。`await` 运算符会继续执行当前任务；如果当前任务暂时无法继续，则让另一个任务有机会继续执行。

`await` 只能在异步上下文中使用；目前这意味着只能在异步函数内部使用（后面会看到其他类型的异步上下文）。原因是异步上下文能保存挂起时的状态，并在外层 Future 被再次轮询时恢复执行。`.await` 使用外层 `poll` 传入的任务上下文；它不隐含一个全局运行时，也不要求 Tokio。执行器是否还提供 IO 或计时服务，取决于实际使用的 API。详见 [Rust Reference](https://doc.rust-lang.org/reference/expressions/await-expr.html)。

最后再从一个角度看 `await`：前面提到，Future 可以组合成更“大”的 Future。异步函数是定义 Future 的一种方式，而 `await` 是组合 Future 的一种方式。在 Future 上使用 `await`，会将该 Future 合并到包含这段代码的异步函数所产生的 Future 中。之后我们会更详细地讨论这个角度，以及组合 Future 的其他方式。

[^poll]: 也可以说是被轮询（polled）。轮询是比 `await` 更底层的操作，使用 `await` 时会在后台发生。后面详细讨论 Future 时，我们会介绍轮询。

<a id="some-asyncawait-examples"></a>
## 一些 async/await 示例

先回顾一下“hello, world!”示例：

```rust,edition2021
{{#include ../../examples/hello-world/src/main.rs}}
```

现在你应该能认出 `main` 周围的样板代码了。它用于初始化 Tokio 运行时，并通过 `block_on` 驱动异步 `main` 函数体。

`say_hello` 是一个异步函数，调用它时必须在调用后加上 `.await`，才能让它作为当前任务的一部分运行。请注意，如果删除 `.await`，运行程序就什么也不会发生！调用 `say_hello` 会返回一个 Future，但它从未执行，因此 `println` 也不会被调用（至少编译器会给出警告）。

下面是一个稍微贴近实际的例子，摘自 [Tokio 教程](https://tokio.rs/tokio/tutorial/hello-tokio)。运行此示例还需要添加 `mini-redis = "0.4"` 依赖，并先启动该教程的 mini-redis 服务。

```rust,no_run
use mini_redis::{client, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // 连接到 mini-redis 地址。
    let mut client = client::connect("127.0.0.1:6379").await?;

    // 将键 "hello" 的值设为 "world"
    client.set("hello", "world".into()).await?;

    // 获取键 "hello"
    let result = client.get("hello").await?;

    println!("got value from the server; result={:?}", result);

    Ok(())
}
```

这段代码稍微有趣一些，但本质上做的还是同一件事——调用异步函数，然后通过 await 执行并取得结果。这次我们使用 `?` 处理错误，它的工作方式与同步 Rust 中完全相同。

尽管前面讨论了并发、并行和异步，但这两个示例都完全是顺序执行的。仅仅调用并 await 异步函数并不会引入并发，除非等待中的任务还有其他任务可供调度。为了验证这一点，我们再看一个简单（但刻意构造）的例子：

```rust,edition2021
{{#include ../../examples/hello-world-sleep/src/main.rs}}
```

打印 "hello" 和 "world" 之间，我们让当前任务休眠一秒[^async-sleep]。观察程序运行时会发生什么：它先打印 "hello"，然后什么也不做，等待一秒后再打印 "world"。这是因为单个任务的执行完全是顺序的。如果存在并发，那么这一秒的休眠时间正好可以用来做其他工作，例如打印 "world"。下一节会介绍如何做到这一点。

[^async-sleep]: 请注意，这里使用的是异步 sleep 函数。如果使用 std 中的 [`sleep`](https://doc.rust-lang.org/std/thread/fn.sleep.html)，整个线程都会休眠。在这个玩具示例中不会有区别，但在真实程序中，这意味着在此期间无法在线程上调度其他任务。这非常糟糕。


<a id="spawning-tasks"></a>
## 创建任务

我们已经讨论了如何通过 async 和 await 在异步任务中运行代码，也提到 `await` 会在当前任务等待 IO 或其他事件时让它休眠。此时可以运行另一个任务，但这些其他任务是从哪里来的呢？正如我们使用 `std::thread::spawn` 创建新任务一样，也可以使用 [`tokio::spawn`](https://docs.rs/tokio/latest/tokio/task/fn.spawn.html) 创建新的异步任务。请注意，`spawn` 是 Tokio 运行时提供的函数，而不是 Rust 标准库中的函数，因为任务完全属于运行时的概念。

下面是一个使用 `spawn` 在独立任务中运行异步函数的小例子：

```rust,edition2021
{{#include ../../examples/hello-world-spawn/src/main.rs}}
```

与上一个例子类似，我们有两个函数，分别打印 "hello" 和 "world!"。但这次我们让它们并发运行，并且可能并行，而不是顺序运行。如果多运行几次程序，你可能会看到字符串以两种顺序打印——有时先打印 "hello"，有时先打印 "world!"。这是经典的并发竞争！

我们来仔细看看这里发生了什么。这里涉及三个概念：Future、任务和线程。`spawn` 函数接收一个 Future（记住，它可以由许多更小的 Future 组成），并将其作为新的 Tokio 任务运行。Tokio 运行时调度和管理的是任务（而不是单个 Future）。Tokio 默认是多线程运行时，这意味着创建新任务时，它可能运行在与创建它的任务不同的操作系统线程上（也可能运行在同一线程上，或者先在一个线程上开始运行，之后再移动到另一个线程）。

因此，Future 被创建为任务后，会与创建它的任务以及其他任务*并发*运行。如果它被调度到不同的线程上，也可能与那些任务并行运行。

总之，在 Rust 中，前后相邻的两条语句总是顺序执行（无论是否处于异步代码中）。使用 `await` 不会改变顺序语句之间的并发关系。例如，`foo(); bar();` 严格按顺序执行——先调用 `foo`，然后调用 `bar`。无论 `foo` 和 `bar` 是否为异步函数，都是如此。`foo().await; bar().await;` 也严格按顺序执行：先完整求值 `foo`，再完整求值 `bar`。这两种情况下，另一个线程都可能与这段顺序执行交错；第二种情况下，另一个异步任务也可能在 await 点交错执行，但两条语句*彼此之间*仍然是顺序执行的。

使用 `thread::spawn` 或 `tokio::spawn` 都会引入并发，并且可能引入并行；前者在线程之间引入，后者在任务之间引入。

本指南后面会介绍并发执行 Future、但不进行并行的情况。


<a id="joining-tasks"></a>
### 等待任务完成

如果想取得被创建任务的执行结果，创建任务可以等待它完成，然后使用结果。这称为*汇合*任务（类似于线程的[汇合](https://doc.rust-lang.org/std/thread/struct.JoinHandle.html#method.join)，两者的 API 也很相似）。

创建任务时，spawn 函数会返回一个 [`JoinHandle`](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html)。如果只想让任务自行运行，可以丢弃这个 `JoinHandle`（丢弃 `JoinHandle` 不会影响已创建的任务）。如果希望创建任务等待新任务完成并使用其结果，则可以 await 这个 `JoinHandle`。

例如，我们再来看一次“Hello, world!”示例：


```rust,edition2021
{{#include ../../examples/hello-world-join/src/main.rs}}
```

代码与上次类似，但我们不再只是调用 `spawn`，而是保存返回的 `JoinHandle`，之后再 await 它们。由于我们会等待这些任务完成后才退出 `main` 函数，因此不再需要在 `main` 中调用 `sleep`。

这两个创建出来的任务仍然并发执行。如果多运行几次程序，你可能看到两种打印顺序，但运行时并不保证每种顺序都会出现。不过，await 这些 JoinHandle 会限制并发：最后的感叹号（'!'）*总是*最后打印（你可以尝试调整 `println!("!");` 相对于各个 `await` 的位置；要观察到效果，可能还需要调整 sleep 时长）。

如果我们没有保存第一个 `spawn` 返回的 `JoinHandle` 并在之后 await，而是立即 await 它（即写成 `spawn(say_hello()).await;`），那么虽然创建了另一个任务来运行“hello” Future，创建任务却会等它完成后才做其他事情。这样不会让这两个待创建的任务互相重叠执行，尽管它们仍可能与程序里的其他任务并发。若只是顺序执行且不需要独立的任务边界，直接 `.await` 原 Future 通常更简单。

### `JoinHandle`

我们再稍微深入了解一下 `JoinHandle`。可以对 `JoinHandle` 使用 `await`，这提示我们 `JoinHandle` 本身就是一个 Future。`spawn` 不是异步函数，而是一个返回 Future（`JoinHandle`）的普通函数。它会先做一些工作（调度任务），然后再返回 Future（这与异步 Future 不同），所以我们*不必* await `spawn`。Await `JoinHandle` 会等待被创建的任务完成，然后返回结果。在上面的例子中，任务没有返回结果，我们只是等待它完成。`JoinHandle` 是一个泛型类型，它的类型参数就是被创建任务的返回类型。在上面的例子中，类型会是 `JoinHandle<()>`；如果 Future 产生 `String`，则会得到类型为 `JoinHandle<String>` 的 `JoinHandle`。

Await `JoinHandle` 会返回一个 `Result`（因此上面的例子使用了 `let _ = ...`，以避免未使用 `Result` 的警告；实际程序通常应处理错误）。如果被创建的任务成功完成，任务结果会位于 `Ok` 变体中。如果任务发生 panic 或被中止（一种[取消](../part-reference/cancellation.md)），结果则会是包含 [`JoinError` 文档](https://docs.rs/tokio/latest/tokio/task/struct.JoinError.html)的 `Err`。如果项目没有通过 `abort` 使用取消机制，那么对 `JoinHandle.await` 的结果调用 `unwrap` 是合理的做法，因为失败会在等待方再次触发 panic。不过，`unwrap` 会用 `JoinError` 创建新的 panic，并不原样重新抛出原任务的 panic 值；若需保留原值，可以检查 `is_panic()` 后使用 `std::panic::resume_unwind(error.into_panic())`。运行时关闭等因素也应纳入错误策略。

> 校订说明：本章补全了上游 futures-rs 与生态提纲，并修正 `async trait`、入口宏、Future 惰性、运行时上下文及并行保证等表述。任务生成、分离与错误语义以 [`spawn`](https://docs.rs/tokio/latest/tokio/task/fn.spawn.html) 和 [`JoinHandle`](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html) 文档为准。前缀 await 的表达式只是用于语法比较的伪表达式，不是 Python 或 JavaScript 的可执行代码。
