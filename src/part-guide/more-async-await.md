<a id="more-asyncawait-topics"></a>
# 更多 async/await 主题

<a id="unit-tests"></a>
## 单元测试

如何对异步代码进行单元测试？问题在于，只有在异步上下文中才能使用 await，而 Rust 中的单元测试本身并不是异步的。幸运的是，大多数运行时都会提供一个方便的测试属性，类似于 `async main` 所用的属性。使用 Tokio 时，代码如下：

```rust,no_run
#[tokio::test]
async fn test_something() {
  // 在此编写测试，可以按需使用任意多个 `await`。
}
```

`#[tokio::test]` 默认给每个测试建立独立的单线程运行时；需要验证多线程行为时可设置 `flavor = "multi_thread"`。计时器测试可使用 `start_paused = true`，这还需启用 Tokio 的 `test-util` 特性。用通道或通知协调测试步骤比固定休眠更可靠。测试有许多配置方式，详情请参阅[文档](https://docs.rs/tokio/latest/tokio/attr.test.html)。

异步代码测试还有一些更高级的主题（例如测试竞争条件、死锁等），本指南[异步编程工具一章](tools.md)会介绍其中一部分。


<a id="blocking-and-cancellation"></a>
## 阻塞与取消

使用 Rust 进行异步编程时，必须牢记阻塞和取消。这些概念并不局限于某项特性或某个函数，而是系统中普遍存在的属性；要写出正确的代码，就必须理解它们。

<a id="blocking-io"></a>
### 阻塞式 IO

当线程（请注意，这里指的是操作系统线程，而不是异步任务）无法取得任何进展时，我们称它被阻塞了。这通常是因为它正在等待操作系统代为完成某项任务（通常是 IO）。重要的是，线程被阻塞时，操作系统知道不应调度它，因此其他线程可以继续推进。在多线程程序中这没有问题，因为阻塞线程等待时，其他线程仍可运行。然而，在异步程序中，同一个操作系统线程上还有其他应该被调度的任务，但操作系统并不知道这些任务，于是它让整个线程都处于等待状态。这意味着原本只需等待 IO 完成的单个任务（这没有问题），变成了许多任务都必须等待（这就有问题了）。

我们很快会讨论非阻塞式/异步 IO。目前只需知道，异步 IO API 会将等待与运行时配合，因此通常只挂起当前任务。非阻塞与异步并非完全相同：例如标准库的 TcpStream 可设置为非阻塞模式，但不会自行注册异步唤醒；通用异步 IO trait 也不在标准库里。应避免在运行时工作线程中直接执行可能长时间阻塞的 IO；必要的同步操作可以交给 spawn_blocking，详见[IO 一章](io.md)。

<a id="blocking-computation"></a>
### 阻塞式计算

执行计算也可能阻塞线程（这与阻塞式 IO 不完全相同，因为操作系统没有参与，但效果类似）。如果某项计算运行时间很长（无论是否包含阻塞式 IO），并且没有向运行时交出控制权，那么该任务就不会给运行时的调度器机会去调度其他任务。请记住，异步编程采用协作式多任务处理。此时任务没有配合让出控制权，因此其他任务也就没有机会完成工作。后面会讨论缓解这一问题的方法。

还有许多其他方式会阻塞整个线程，本指南后面会多次回到阻塞这一主题。

<a id="cancellation"></a>
### 取消

取消是指停止执行某个 Future（或任务）。在 Rust 中（这与许多其他 async/await 系统不同），Future 必须由外部力量（例如异步运行时）推动才能前进；如果不再推动某个 Future，它就不会继续执行。如果拥有的 Future 被丢弃（请记住，Future 只是普通的 Rust 对象），它自身的状态机就不能再推进。但已经提交给操作系统的操作可能继续，丢弃 JoinHandle 也只会分离已生成的任务，并不取消任务。取消 Future 不等于撤销所有外部副作用。

取消可以通过几种方式发起：

- 直接丢弃一个 Future（前提是你拥有它）。
- 对任务的 `JoinHandle`（或 `AbortHandle`）调用 [`abort`](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html#method.abort)。
- 使用 [`CancellationToken`](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html)（这要求被取消的 Future 能够察觉该令牌，并协作完成自身取消）。
- 由 [`select`](https://docs.rs/tokio/latest/tokio/macro.select.html) 这样的函数或宏隐式发起。

中间两种方式是 Tokio 特有的，不过大多数运行时都提供类似功能。使用 `CancellationToken` 需要被取消的 Future 配合。Tokio 的 `abort` 是取消请求，不会抢占正在执行的一次 poll：任务必须先交回控制权，已经开始执行的 spawn_blocking 任务也不能据此中断。可以 await 任务句柄来确认退出；详见 [`abort` 文档](https://docs.rs/tokio/latest/tokio/task/struct.JoinHandle.html#method.abort)。在其他情况下，被取消的 Future 不会收到取消通知，也没有机会进行清理（析构函数除外）。请注意，即使 Future 使用了取消令牌，仍然可以通过其他方式取消它，而这些方式不会触发该取消令牌。

从编写异步代码（异步函数、代码块、Future 等）的角度看，代码可能在某次返回 Pending 的轮询之后停止执行（常表现为挂起在 `.await`，包括宏中隐藏的等待），并且永远不会再继续。未开始执行的 Future 也可能直接被丢弃；立即就绪的 await 不会自动产生取消边界。要确保代码正确（具体来说，要做到*取消安全*），就必须保证无论代码正常完成，还是在任意 await 点终止[^cfThreads]，它都能正确工作。

```rust,edition2021
{{#include ../../examples/guide-tests/src/more.rs:cancellation-paths}}
```

一个可能出错的例子是：异步函数将数据读入内部缓冲区，然后等待下一个数据。如果读取数据是破坏性的（也就是说，无法从原始来源重新读取），而异步函数又被取消，那么内部缓冲区就会被丢弃，其中的数据也会丢失。必须考虑取消 Future、重新启动 Future，或启动一个会访问相同数据的新 Future 时，该 Future 及其接触到的数据会受到怎样的影响。

本指南会多次回到取消和取消安全这一主题，参考部分还专门有一[章](../part-reference/cancellation.md)讨论它。

[^cfThreads]: 将异步编程中的取消与取消线程进行比较很有意思。取消线程是可行的（例如在 C 中使用 [`pthread_cancel`](https://pubs.opengroup.org/onlinepubs/9799919799/functions/V2_chap02.html)；Rust 没有直接实现这一功能的方式），但必须区分延迟取消与异步取消：pthread_cancel 的默认延迟取消发生在取消点，异步取消才可能在任意位置终止，后者尤其危险。Rust 没有可移植的安全线程取消 API。相比之下，对 Rust Future 的销毁发生在拥有者能重新取得它的时候，运行时不会打断它正在执行的 poll；对 async 函数通常对应挂起的 await 点。因此，除非要终止整个进程，否则很少会取消操作系统线程；作为程序员，你通常不必担心这种情况发生。然而，在 Rust 异步编程中，取消确实是*可能*发生的事情。我们会在后文讨论如何应对。

<a id="async-blocks"></a>
## 异步代码块

普通代码块（`{ ... }`）会在源代码中将代码组合起来，并为名称创建一个封装作用域。在运行时，代码块按顺序执行，并求值为最后一个表达式的值（如果没有尾随表达式，则求值为单元类型（`()`））。

与异步函数类似，异步代码块是普通代码块的延后执行版本。异步代码块会将代码和名称置于同一作用域中，但在运行时不会立即执行，而是求值为一个 Future。要执行该代码块并取得结果，必须对它使用 `await`。例如：

```rust,ignore
let s1 = {
    let a = 42;
    format!("The answer is {a}")
};

let s2 = async {
    let q = question().await;
    format!("The question is {q}")
};
```

如果执行这段代码，`s1` 会是一个可以打印的字符串，而 `s2` 会是一个 Future；此时 `question()` 尚未被调用。要打印 `s2`，必须先执行 `s2.await`。

异步代码块是开启异步上下文并创建 Future 的最简单方式。它通常用于创建只在一个地方使用的小型 Future。

遗憾的是，异步代码块中的控制流有些特殊。由于异步代码块会创建 Future，而不是直接执行，因此在控制流方面，它的行为更像函数，而不像普通代码块。`break` 和 `continue` 不能像在普通代码块中那样“穿过”异步代码块；你必须改用 `return`：

```rust,ignore
loop {
    {
        if ... {
            // 可以
            continue;
        }
    }

    async {
        if ... {
            // 不可以
            // continue;

            // 可以——继续执行 `loop` 的下一轮；不过请注意，如果异步代码块后面还有循环代码，仍会执行那些代码。
            return;
        }
    }.await
}
```

要实现 `break`，需要检查代码块的值（一种常见惯用写法是让代码块返回 [`ControlFlow`](https://doc.rust-lang.org/std/ops/enum.ControlFlow.html)，在返回类型兼容时也能使用 `?`）。

同样，在异步代码块内部，遇到错误时 `?` 会终止 Future 的执行，使 await 后的代码块求值为该错误，但不会退出外围函数（而普通代码块中的 `?` 会退出外围函数）。要实现这一点，需要在 `await` 后再使用一个 `?`：

```rust,ignore
async {
    let x = foo()?;   // 这里的 `?` 只退出异步代码块，不会退出外围函数。
    consume(x);
    Ok(())
}.await?
```

烦人的是，这通常会让编译器困惑，因为与函数不同，异步代码块的“返回”类型没有显式声明。你可能需要为变量添加类型标注，或使用 turbofish 类型语法才能让代码通过编译；例如，在上面的例子中使用 `Ok::<_, MyError>(())`，而不是 `Ok(())`。

返回异步代码块的函数与异步函数非常相似。`async fn foo() -> T { ... }` 大致等价于 `fn foo() -> impl Future<Output = T> { async move { ... } }`。从调用方看，两者都返回 Future，但不能据此断言任意改写都保持兼容：必须保留执行时机、捕获的生命周期和 Send 等约束。此外，实现异步 trait 时，也可以用其中一种形式覆盖另一种（见下文）。不过，你确实需要调整类型，在异步代码块版本中显式写出 `Future`：`async fn foo() -> Foo` 会变成 `fn foo() -> impl Future<Output = Foo>`（你可能还需要显式写出其他约束，例如 `Send` 和 `'static`）。

通常应优先选择异步函数版本，因为它更简单清晰。不过，异步代码块版本更灵活：你可以在函数调用时执行一部分代码（将其写在异步代码块之外），并在等待结果时执行另一部分代码（写在异步代码块内部）。


<a id="async-closures"></a>
## 异步闭包

异步闭包自 [Rust 1.85](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/) 起已稳定，可以写成 `async |args| { ... }`，需要移动捕获时使用 `async move |args| { ... }`。上游提到的[实现 PR](https://github.com/rust-lang/rust/pull/132706)和 [2024 年测试征集](https://blog.rust-lang.org/inside-rust/2024/08/09/async-closures-call-for-testing.html)属于稳定前的历史材料。

普通闭包 `|x| async move { ... }` 是“返回 Future 的闭包”：调用闭包时执行闭包体，随后轮询 Future 才执行异步块。原生异步闭包还可以让每次调用返回的 Future 借用闭包所捕获的状态。这类借出状态的行为常称为 lending；借用没有结束前，不能再进行相冲突的调用。

为这类回调编写泛型接口时，使用 `AsyncFn`、`AsyncFnMut` 或 `AsyncFnOnce` 的调用约束。单独的 `FnMut(&T) -> Fut` 往往无法表达“返回的 Future 类型随本次借用的生命周期而变化”。下面用 `for<'a> AsyncFnMut(&'a u32)` 接受任意单次参数借用，并顺序等待每次调用完成：

```rust,edition2021
{{#include ../../examples/guide-tests/src/more.rs:async-closure}}
```

`AsyncFn` 对应共享调用，`AsyncFnMut` 对应可变调用，`AsyncFnOnce` 允许消耗闭包。本例同时跨 await 借用参数与可变捕获的 sum。稳定代码使用上面的括号调用约束和 `visit(...).await`；不要直接调用仍受特性门控的底层 `async_call_mut` 方法或命名其关联 Future。具体限制见 [`AsyncFnMut` 文档](https://doc.rust-lang.org/std/ops/trait.AsyncFnMut.html)。

<a id="lifetimes-and-borrowing"></a>
## 生命周期与借用

Future 可以包含引用，也可以在挂起期间保存局部变量和对它们的借用。所有借用都必须在 Future 的整个相关使用期间有效；`.await` 不会放宽 Rust 的借用规则。异步函数会捕获参数，所以把 `async fn` 调用移到函数外部并不能让参数的局部数据自动延长生命周期。

下面返回的 Future 借用 text。`+ '_` 将其可用期与输入借用关联；在拥有 text 的作用域里直接等待它是正常的，不需要 `'static`：

```rust,edition2021
{{#include ../../examples/guide-tests/src/more.rs:borrowing}}
```

`async move` 移动的是捕获值。如果捕获值是引用，移入的是引用，而不是其指向的数据；因此 `move` 本身不保证 Future 满足 `'static`。若要跨任务共享拥有所有权的数据，可考虑 `Arc<T>`；只在当前任务中组合 Future 时，通常可以继续使用借用。

手写返回 `impl Future` 的普通函数还要注意 edition 的捕获规则。Rust 2024 默认捕获作用域内的生命周期参数；旧 edition 的普通函数往往需要显式约束，也可使用 `use<...>` 精确捕获。原生 `async fn` 和 trait 内返回位置的 impl Trait 在所有 edition 中本就有较广的捕获规则。不要把 `+ 'static` 当成解决借用错误的通用办法，详见 [Edition Guide](https://doc.rust-lang.org/edition-guide/rust-2024/rpit-lifetime-capture.html)及[捕获规则说明](https://blog.rust-lang.org/2024/09/05/impl-trait-capture-rules.html)。

<a id="send--static-bounds-on-futures"></a>
## Future 的 `Send + 'static` 约束

[`tokio::spawn`](https://docs.rs/tokio/latest/tokio/task/fn.spawn.html) 要求 Future 和输出都满足 `Send + 'static`。`Send` 允许运行时把任务移到其他工作线程；`'static` 表示它不能携带有效期不足的借用，并不要求任务永远存活。拥有 String 的 Future 通常可以满足 `'static`，即使它很快就被释放。

编译器依据 Future 保存的参数、捕获值与跨挂起点仍存活的局部状态来判断 Send。Rc 和某些锁守卫不满足 Send；若只在挂起前临时使用，可以用一个内部作用域使其先结束。需要共享所有权时可按类型条件改用 Arc；不要把阻塞锁的守卫跨 await 保存。Arc 也只有在其内部类型满足相应约束时才是线程安全的。

[`spawn_local`](https://docs.rs/tokio/latest/tokio/task/fn.spawn_local.html) 允许非 Send Future，但**仍要求 `'static`**。它需要本地任务上下文；仅选择 current_thread 运行时不足以允许任意位置调用它。下面使用稳定的 LocalSet，让 Rc 跨 await 保持存活：

```rust,edition2021
{{#include ../../examples/guide-tests/src/more.rs:local-task}}
```

如果 Future 必须借用外层的短期数据，直接 await 或使用同一任务内的 join 组合通常更合适；spawn_local 并不能解除所有权约束。

<a id="async-traits"></a>
## 异步 trait

从 [Rust 1.75](https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits/) 起，trait 可以声明 `async fn`，例如 `trait Service { async fn fetch(&self) -> String; }`，实现中直接写对应的异步方法即可。这适用于静态分派的泛型接口。

公共 trait 必须提前考虑返回 Future 的 Send 能力。只给 Self 加 Send 并不能保证其方法返回的 Future 也满足 Send。需要此保证时，可以在 trait 中显式返回 `impl Future + Send`，实现时仍然使用 async fn：

```rust,edition2021
{{#include ../../examples/guide-tests/src/more.rs:send-trait}}
```

这个 Future 借用 self，所以 Send 与 `'static` 仍是两个独立问题。若要用 spawn，应先将拥有的实现对象移入外层 async move 块，再在块中调用方法。反过来，实现原生 async trait 方法时，也可返回满足签名约束的显式 Future；改变写法不能丢掉原有的生命周期和自动 trait 保证。

[`trait-variant`](https://docs.rs/trait-variant/latest/trait_variant/) 可以生成本地版本与带 Send 约束的版本，减少重复声明。[返回类型标注](https://blog.rust-lang.org/inside-rust/2024/09/26/rtn-call-for-testing.html)探索在调用方直接约束方法返回的 Future，但本章的稳定代码不依赖它；其状态参见 [Unstable Book](https://doc.rust-lang.org/unstable-book/language-features/return-type-notation.html)。

原生 async 方法仍不满足通过 dyn 分派的要求（原称“对象安全”）。需要 `dyn Trait` 时，可使用返回装箱 Future 的方法做类型擦除：

```rust,edition2021
{{#include ../../examples/guide-tests/src/more.rs:dyn-trait}}
```

这里 `Future` 指 `std::future::Future`，`'_` 保留 self 的借用；不是把方法强行变成 `'static`。装箱带来分配和动态分派开销。[`async-trait`](https://docs.rs/async-trait/latest/async_trait/) 在原生 async trait 稳定前就提供类似转换，现仍可用于 dyn 接口和兼容旧编译器。它通常生成 `Pin<Box<dyn Future + Send + 'a>>`，使用 `?Send` 可选择本地 Future；这与原生静态分派的实现成本及约束不同。参见 [Reference 的 dyn 兼容规则](https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility)。

<a id="recursion"></a>
## 递归

异步函数会生成保存挂起状态的匿名类型。直接递归会使这个类型无限包含自身，因此需要引入间接层。自 [Rust 1.77](https://blog.rust-lang.org/2024/03/21/Rust-1.77.0/) 起，可以在递归调用处使用 `Box::pin`，不必把整个函数改成手写装箱的返回类型：

```rust,edition2021
{{#include ../../examples/guide-tests/src/more.rs:recursion}}
```

Box 提供堆上的间接存储，Pin 使需要固定位置的 Future 能安全地被轮询；后面的 [Future](futures.md) 与 [Pin](../part-reference/pinning.md) 章节会展开。装箱解决的是类型大小问题，不会自动减少递归深度，也不保证 poll 调用栈有界或高效；很深的遍历通常更适合显式栈与循环。

兼容更早编译器时，可以返回 `Pin<Box<dyn Future<Output = T> + ...>>`，或使用 [`async-recursion`](https://docs.rs/async-recursion/latest/async_recursion/) 宏自动装箱。历史做法参见[旧版递归章节](../07_workarounds/04_recursion.md)。

> 校订说明：本章补全了上游最后五节提纲，将“异步闭包即将推出”更新为已稳定，并区分取消 Future、分离任务、请求中止和撤销 IO。本章没有运行时上下文或使用未定义函数/省略号的旧示意片段标记为 ignore；新增示例均来自通过测试的 guide-tests 源码。
