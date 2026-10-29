<a id="send-approximation"></a>
# `Send` 的近似判定

有些 `async fn` 状态机可以安全地在线程间传递，有些则不行。`async fn` 返回的 `Future` 是否实现 [`Send`](https://doc.rust-lang.org/std/marker/trait.Send.html)，取决于其保存的状态能否安全地在线程间转移：既包括初始捕获的参数，也包括跨挂起点仍存活的局部值。编译器需要分析这些状态；原文介绍过一些过于保守的判定，不过分析精度会随版本改进。本节使用 Rust 1.96.0 实际编译区分仍然成立的限制与已经过时的变通经验。

例如，考虑一个简单的非 `Send` 类型，比如包含 `Rc` 的类型：

```rust
use std::rc::Rc;

#[derive(Default)]
struct NotSend(Rc<()>);
```

即使 `async fn` 返回的 `Future` 类型必须实现 `Send`，`NotSend` 类型的变量也可能
短暂地作为临时值出现在 `async fn` 中：

```rust,edition2018
# use std::rc::Rc;
# #[derive(Default)]
# struct NotSend(Rc<()>);
async fn bar() {}
async fn foo() {
    NotSend::default();
    bar().await;
}

fn require_send(_: impl Send) {}

fn main() {
    require_send(foo());
}
```

但是，如果修改 `foo`，把 `NotSend` 存入变量，这个示例就无法编译：

```rust,compile_fail,edition2018
# use std::rc::Rc;
# #[derive(Default)]
# struct NotSend(Rc<()>);
# async fn bar() {}
async fn foo() {
    let x = NotSend::default();
    bar().await;
}
# fn require_send(_: impl Send) {}
# fn main() {
#    require_send(foo());
# }
```

下面摘录 Rust 1.96.0 的实际诊断；路径和行号来自本地核查文件，省略警告：

```text
error: future cannot be sent between threads safely
  --> .tools/round8-send-check/named.rs:11:26
   |
11 | fn main() { require_send(named()); }
   |                          ^^^^^^^ future returned by `named` is not `Send`
   |
   = help: within `impl Future<Output = ()>`, the trait `Send` is not implemented for `Rc<()>`
note: future is not `Send` as this value is used across an await
  --> .tools/round8-send-check/named.rs:7:54
   |
 7 | async fn named() { let x = NotSend::default(); bar().await; }
   |                        -                             ^^^^^ await occurs here, with `x` maybe used later
   |                        |
   |                        has type `NotSend` which is not `Send`
note: required by a bound in `require_send`
  --> .tools/round8-send-check/named.rs:5:25
   |
 5 | fn require_send(_: impl Send) {}
   |                         ^^^^ required by this bound in `require_send`
```

这个错误是正确的。这里 `x` 的析构作用域覆盖 `.await`，即使之后不再读取它，仍要保存它以便稍后销毁。允许这个包含 `Rc` 的状态被转移到其他线程是不安全的。`bar` 虽然立即完成，编译器仍按生成的 Future 类型及其状态判断 Send，而不是把某次运行的执行路径当成类型保证。

原文接着说“在 `.await` 前显式 `drop(x)` 仍无法解决”，这对本节示例已经过时。下面的局部值版本在 Rust 1.96.0、edition 2018 上通过了 `require_send(foo())` 的编译检查：

```rust,edition2018
# use std::rc::Rc;
# #[derive(Default)]
# struct NotSend(Rc<()>);
# async fn bar() {}
async fn foo() {
    let x = NotSend::default();
    drop(x);
    bar().await;
}
# fn require_send(_: impl Send) {}
# fn main() { require_send(foo()); }
```

另一种清晰的写法是添加块作用域，把只在挂起前使用的非 `Send` 局部值包起来。它明确划定资源的生命周期，也便于处理编译器在更复杂控制流中的分析限制；不必把它当成当前示例中替代显式 drop 的唯一办法。

```rust,edition2018
# use std::rc::Rc;
# #[derive(Default)]
# struct NotSend(Rc<()>);
# async fn bar() {}
async fn foo() {
    {
        let x = NotSend::default();
    }
    bar().await;
}
# fn require_send(_: impl Send) {}
# fn main() {
#    require_send(foo());
# }
```

上述成功例子中的非 Send 值是在函数体被轮询后才局部创建的。不要由此推断“所有值只要在 await 前 drop，Future 就是 Send”。异步函数会把参数捕获进初始 Future，即使函数体会立刻销毁参数，尚未被轮询的 Future 仍然持有它：

```rust,compile_fail,edition2018
# use std::rc::Rc;
# #[derive(Default)]
# struct NotSend(Rc<()>);
# async fn bar() {}
async fn takes_parameter(x: NotSend) {
    drop(x);
    bar().await;
}
fn require_send(_: impl Send) {}
fn main() {
    require_send(takes_parameter(NotSend::default()));
}
```

这个版本在 Rust 1.96.0 中仍然失败，诊断说明 `captured value is not Send`。要让这种接口返回 Send Future，可以在普通同步函数中先处理并销毁非 Send 输入，只把符合条件的结果交给返回的 async 块；或者让调用方在允许非 Send Future 的本地执行环境中运行它。`Send` 不等于 `'static`，也不要求任务一定换线程：它描述类型是否允许安全转移。

参数捕获规则参见 [Rust Reference](https://doc.rust-lang.org/reference/items/functions.html#async-functions)；关于生命周期、`async move` 与任务边界，见[更多 async/await 主题](../part-guide/more-async-await.md)。本节的正反例仅作编译验证，没有实际运行这些 Future，也没有模拟线程迁移。
