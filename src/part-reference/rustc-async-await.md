<a id="the-implementation-of-asyncawait-in-rustc"></a>
# rustc 中 async/await 的实现

上游只有英文目录标题，没有英文正文；标题由 GPT-6 Luna 翻译，正文由 GPT-6.1 Sol 原创补写。

`async fn` 看起来像普通函数，却能在等待途中把控制权交还给调用者，稍后从原来的位置继续。rustc 将这种控制流编译成一个实现 `Future` 的值，其中保存恢复执行所需的数据。本章沿着语法、编译器中间表示和可观察行为解释这一过程。

先区分两层依据：[Rust Reference](https://doc.rust-lang.org/reference/items/functions.html#async-functions) 描述语言行为；rustc 的中间表示、转换过程、状态编号和字段布局属于实现细节。本章在 Windows 上使用现有 Rust 1.96.0 观察 MIR；在线编译器源码文档在核查时对应 Rust 1.99.1，主要用于说明转换职责，不将两个版本的内部细节视为完全相同。更换版本或优化参数后，输出可能不同。

## 调用先产生 Future

可以从下面的近似关系开始理解：

```rust
use std::future::Future;

async fn length(text: &str) -> usize {
    text.len()
}

// 便于理解返回类型的近似写法，并非编译器实际打印的源码。
fn length_like<'a>(text: &'a str) -> impl Future<Output = usize> + 'a {
    async move { text.len() }
}
```

`usize` 是 Future 完成时的输出，调用 `length` 当下得到的则是匿名 Future。函数体在首次 `poll` 时才开始执行；即使体内没有 `.await`，这个区别也存在。函数实参表达式仍在调用处立即求值，传入的参数被捕获进 Future，参数中的借用也必须在适当的生命周期内有效。`async fn` 连未使用的参数也会捕获，不能把它简单改写成只捕获正文使用变量的普通闭包。[Reference 的参数与生命周期规则](https://doc.rust-lang.org/reference/items/functions.html#items.fn.async.desugar) 对这种近似写法作了限定。

因此，创建连接参数时的同步处理、构造某个 Future 之前已经发起的操作，都可能先发生。这里的惰性针对 async 函数体；它不是所有返回 Future 的库函数都不做同步工作的保证。若从未轮询便丢弃 Future，函数体没有运行，但 Future 已经拥有的参数仍会按析构规则释放。

## 从语法到可恢复的控制流

rustc 会经历多个中间表示。下面这条路径标出各层作用，并不是说每层都能用一份等价的稳定 Rust 源码替代：

```text
async fn / async 块 / .await 的语法
        |
        v
HIR：表示匿名返回类型与协程体，将 await 表达为轮询和挂起控制流
        |
        v
类型检查、THIR、MIR 构建与借用检查等分析
        |
        v
MIR 协程转换：保存跨挂起的数据，生成恢复入口及析构路径
        |
        v
后续优化与机器码生成
```

在函数接口上，调用者看到的是隐藏具体类型的返回值，满足 `Future<Output = T>`；编译器内部则知道那个具体的协程类型。`opaque`（不透明类型）表达“接口不暴露具体类型”，不意味着运行时动态分配或类型擦除。匿名 Future 可以直接内联存放，是否装进 `Box` 是另一项选择。

在函数体一侧，HIR 使用协程相关节点表达 async 控制流。开发指南的[协程闭包章节](https://rustc-dev-guide.rust-lang.org/coroutine-closures.html#hir) 介绍了这套表示及参数移入内部协程体的处理；async 闭包还有额外的借用捕获规则，不应与普通 async fn 完全等同。随后，MIR 构建实际上从完成类型分析的 THIR 生成基本块、局部变量和控制流，详见 [MIR 构建](https://rustc-dev-guide.rust-lang.org/mir/construction.html)。

`.await` 的核心步骤是调用 `IntoFuture::into_future`，固定得到的子 Future，用当前任务的 `Context` 调用它的 `poll`。如果得到 `Ready(value)`，就在当前这次外层 `poll` 中继续；如果得到 `Pending`，才保存状态并让外层 Future 返回 `Pending`。下次外层被轮询时，再进入等待位置，继续轮询保存的那个子 Future。[Reference](https://doc.rust-lang.org/reference/expressions/await-expr.html) 给出语义步骤；[rustc 的 await 降低源码](https://doc.rust-lang.org/stable/nightly-rustc/src/rustc_ast_lowering/expr.rs.html) 中可查 `lower_expr_await` 与 `make_lowered_await`。

这意味着 `.await` 是可能挂起的位置，不是必然的调度切换。连续等待立即完成的 Future，可以在一次 `poll` 中全部执行完。调用同步阻塞函数或进行长时间计算，也不会被编译器自动改成后台工作。

## 挂起时保存什么

考虑这个带两个等待位置的函数。`Probe` 是记录析构事件的教学资源；完整定义与验证位于本书的独立示例 crate。

```rust,ignore
{{#include ../../examples/guide-tests/src/rustc_async.rs:rustc-stages}}
```

它等待的 `YieldOnce` 第一次真的返回 `Pending`，同时请求再次轮询，第二次返回 `Ready(())`：

```rust
{{#include ../../examples/guide-tests/src/rustc_async.rs:rustc-yield-once}}
```

这个自唤醒只适用于示例中“下次已经能继续”的情况。真正等待 I/O 的 Future 应登记通知，由就绪事件唤醒，不能照搬为无限自唤醒循环。

函数的主要状态可以画成：

```text
未开始（拥有 input、events）
    | 首次 poll：建立 first，轮询第一个子 Future
    v
等待一（保存 input、events、first、子 Future） --Pending--> 返回调用者
    | 再次 poll：子 Future Ready，释放 first，建立 second
    v
等待二（保存 input、events、second、子 Future）--Pending--> 返回调用者
    | 再次 poll：子 Future Ready，释放 second 与 input
    v
已完成：返回 Ready(7)
```

这张图表现恢复位置与资源关系，不是实际内存布局。挂起后，本次 `poll` 的普通调用栈已经退出；下次恢复所需的局部值必须留在 Future 的存储中。只在一次 `poll` 内使用的临时值可以继续作为该次调用的局部变量，不必一律变成永久字段。

判断需要保存什么，涉及值的活跃性、存储活跃范围、借用及析构需要。不能仅看“变量名在 await 之后是否再次出现”：一个之后不再读取、但仍须析构的资源，也可能需要保存。不同状态中不会同时存活的字段可能复用存储，布局优化还能进一步改变结果。[协程转换模块](https://doc.rust-lang.org/stable/nightly-rustc/rustc_mir_transform/coroutine/index.html) 与其[源码](https://doc.rust-lang.org/stable/nightly-rustc/src/rustc_mir_transform/coroutine/mod.rs.html) 描述了保存局部值、状态分派、`poll` 和析构入口的职责。

## Drop 也是状态机的一部分

Future 不一定正常完成。调用者可能在尚未轮询、第一次挂起、第二次挂起等不同阶段丢弃它。生成的析构逻辑必须根据当前状态，释放当时已经初始化且仍归它所有的值，避免释放尚未建立的局部值或再次释放已经移走的值。

以 `stages` 为例：未轮询时只持有参数，尚无 `first` 或 `second`；第一次挂起时需要释放 `first` 和参数；第二次挂起时 `first` 已经释放，只应释放 `second` 和参数；正常完成时正文已经完成相关释放。编译器还需处理初始化途中出错、展开等路径，使用状态与必要的析构标志维持这些区别。不能只生成恢复用的 `match`，却忘记每个挂起状态下的清理。

这仍是普通同步析构。丢弃 Future 不会让状态机继续跑到最后一行，也不会自动 `.await` 某项清理；不会因为编译器知道保存哪些字段，就保证外部系统中的请求也已终止。异步关闭与底层操作取消的区别见[析构与清理](../part-guide/dtors.md)和[取消与取消安全](cancellation.md)。如果程序主动泄漏 Future，或直接终止进程，也不能依靠这套析构路径保证资源清理。

在本章观察的 rustc 输出中，还有完成及 panic 后的状态。完成后再次轮询编译器生成的 Future 通常会 panic；使用者应遵守 [Future 契约](https://doc.rust-lang.org/std/future/trait.Future.html#tymethod.poll)，不在 `Ready` 之后再次 `poll`。该 trait 并不要求所有手写 Future 在这种情况下采用相同的 panic 行为。

## Pin、内部借用与 Send

普通函数的借用可以指向调用栈内的局部值；async 函数中的借用若跨越挂起，就可能指向保存在同一个 Future 中的数据。下面的 `view` 在等待后仍被使用：

```rust
{{#include ../../examples/guide-tests/src/rustc_async.rs:rustc-borrow-and-send}}
```

`borrowed_len` 的状态需要同时保留 `text` 和恢复时有效的借用关系。Future 固定后，调用方必须遵守不能随意移动其地址敏感内容的约束；这也是 `Future::poll` 接收 `Pin<&mut Self>` 的原因。编译器能为这种内部借用生成代码，不等于用户能用普通结构体和普通生命周期参数直接声明同样的自引用结构。详细规则见[固定（Pinning）](pinning.md)及[标准库 pin 文档](https://doc.rust-lang.org/std/pin/index.html)。固定前可以移动 Future；固定后的 `Box` 句柄本身也可以移动，只要所指 Future 保持原位。

`Send` 则关乎把 Future 的所有权移到其他线程是否安全。如果跨挂起状态保存了 `Rc` 或不能跨线程的守卫，Future 通常不能满足 `Send`。反过来，`scoped_rc` 用明确的内部作用域在挂起前结束 `Rc` 的生命期，只让 `usize` 跨过等待位置，其返回 Future 在本章工具链上满足 `Send`。参数捕获也要考虑：非 `Send` 参数可能在尚未开始的状态中就阻止整个 Future 实现 `Send`。不要把“函数体里出现过 Rc”或“最后一次读取已经过去”当成完整判据；以编译器实际检查及相关类型约束为准。

## 在稳定工具链中观察 MIR

本章无需 nightly、`-Z` 或自行编译 rustc。以下命令在仓库根目录执行，使用现有工具链，把与正文共享的源码编译为库并输出 MIR；未开启 `cfg(test)`，因此不需要示例中的测试依赖：

```text
rustc +1.96.0 --edition=2021 --crate-type=lib --emit=mir examples/guide-tests/src/rustc_async.rs -o .tools/round5-rustc-async.mir
```

本机版本为 `rustc 1.96.0 (ac68faa20 2026-05-25)`，目标为 `x86_64-pc-windows-msvc`。实际输出中，`stages` 的外层函数返回 `{async fn body of stages()}`，另有接收 `Pin<&mut ...>` 和 `&mut Context`、返回 `Poll<usize>` 的体。体的入口读出 `discriminant` 后以 `switchInt` 分派，两个 `Pending` 返回路径分别写入状态 `3` 和 `4`，完成路径返回 `Ready(7)` 并写入完成状态。

这些名称和编号方便阅读本次产物，但不是编写应用或 FFI 时可依赖的接口。`--emit=mir` 输出也只是某阶段的文本视图，不能当成所有编译步骤或最终机器码的完整证明。不要据它直接计算稳定字段偏移，不要将匿名 Future 的内存映射到 C 结构体；[Reference 明确规定 async 块的具体数据格式未指定](https://doc.rust-lang.org/reference/expressions/block-expr.html#expr.block.async.layout-unspecified)。

行为验证使用两个测试：第一个分别在零、一、二次 `Pending` 后丢弃，以及正常完成后丢弃，检查参数求值、函数体启动、两次唤醒和各阶段资源析构；第二个实际让内部借用跨过 `Pending` 再恢复，并在编译期检查 `scoped_rc` 返回值满足 `Send`。

```text
cargo +1.96.0 test --manifest-path examples/guide-tests/Cargo.toml --offline --locked rustc_async_
```

这两项测试在本机通过，不依赖网络或真实计时器。它们验证这些具体程序的可观察行为，不证明任意状态机布局、所有平台上的优化结果或全部 panic 展开组合。

## 编译器与运行时的交接处

rustc 生成的是一个可轮询的计算值，运行时负责决定何时、在哪个线程上轮询根任务，并提供 I/O、计时器等服务。子 Future 得到的 `Context` 来自外层任务；`wake` 通常只是把任务标记为需要再运行，不是在回调中直接从某行继续执行。

所以 await 的编译既不会自动创建 Tokio 任务，也不会凭空创建操作系统线程、事件循环或网络驱动。整个调用链能够内联为嵌套 Future；只有应用或库显式提交任务，才建立对应执行器管理的任务边界。编译器生成状态机解释“计算如何暂停与恢复”，[实现你自己的运行时](implementing-runtime.md)则解释“谁来安排下一次 poll”。

[RFC 2394](https://rust-lang.github.io/rfcs/2394-async_await.html) 是理解这一设计动机与最初状态机模型的历史资料。其中的 `await!`、早期关联类型名和当时尚未决定的语法不代表今天的稳定写法；当前使用方式应以 Reference 与标准库文档为准。
