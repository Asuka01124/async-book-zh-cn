<a id="recursion"></a>
# 递归

在内部，`async fn` 会创建一种状态机类型，其中包含每个被 `.await` 的子 `Future`。
这让没有间接层的递归 `async fn` 有些棘手，因为生成的状态机类型会要求直接包含自身。下面的枚举只是说明“保存当前阶段状态”的概念伪码，不是编译器实际生成的精确布局；它省略了初始/结束状态、捕获值等细节，整个代码块有意不可编译：

```rust,ignore,edition2018
# async fn step_one() { /* ... */ }
# async fn step_two() { /* ... */ }
# struct StepOne;
# struct StepTwo;
// 这个函数：
async fn foo() {
    step_one().await;
    step_two().await;
}
// 会生成类似这样的类型：
enum Foo {
    First(StepOne),
    Second(StepTwo),
}

// 因此，这个函数：
async fn recursive() {
    recursive().await;
    recursive().await;
}

// 会生成类似这样的类型：
enum Recursive {
    First(Recursive),
    Second(Recursive),
}
```

这样行不通——枚举直接包含自身会要求无限大小；异步递归同样需要打破这个大小依赖。单独编译下面的递归函数，会得到 E0733（Rust 1.96.0 已实测）：

```rust,compile_fail,edition2018
async fn recursive() {
    recursive().await;
    recursive().await;
}
```

诊断的核心内容为：

```
error[E0733]: recursion in an async fn requires boxing
 --> src/lib.rs:1:1
  |
1 | async fn recursive() {
  | ^^^^^^^^^^^^^^^^^^^^
  |
  = note: a recursive `async fn` call must introduce indirection such as `Box::pin` to avoid an infinitely sized future
```

要让这种写法可行，我们必须使用 `Box` 引入间接层。

在 Rust 1.77 之前，受编译器限制，仅用 `Box::pin` 包装对 `recursive()` 的调用还
不够。一种历史上的解决办法是把 `recursive` 改成一个非 `async` 函数，并让它返回一个
调用了 `.boxed()` 的 `async` 块：

```rust,edition2018
{{#include ../../examples/07_05_recursion/src/lib.rs:example}}
```

这里 `BoxFuture<'static, ()>` 是 `Pin<Box<dyn Future<Output = ()> + Send + 'static>>` 的别名，`.boxed()` 也要求 Future 满足 Send。Send 与 static 是这个具体类型选择的附加条件，并非异步递归本身的要求；借用数据或仅在本地执行的实现，可以按需要选择更短生命周期或不要求 Send 的装箱形式。指针提供有限大小的间接层，动态分派是本例采用的另一项选择。

在较新版本的 Rust 中，[这项编译器限制已经解除][that compiler limitation has been lifted]。

从 Rust 1.77 起，带有分配间接层的 `async fn` 递归支持[已稳定][becomes stable]。因此，只要递归调用
使用某种间接层来避免函数状态无限增大，就可以使用递归调用。

这意味着，下面这样的类型定义现在可以通过编译，而不必把整个函数改成返回装箱 trait 对象的普通函数：

```rust,edition2021
{{#include ../../examples/07_05_recursion/src/lib.rs:example_pinned}}
```

上面的 `recursive` 和 `recursive_pinned` 都没有基本终止条件，只用于展示类型写法。它们并不是可直接执行的算法：第一次递归调用就会继续递归，第二次调用没有机会开始，可能不断分配内存或耗尽调用栈。验证时只编译这两个函数，不执行它们。

实际算法需要终止条件。例如下面逐层递减参数，到零时返回，并在返回途中计数：

```rust,edition2021
{{#include ../../examples/07_05_recursion/src/lib.rs:bounded}}
```

本包测试只执行深度 0 和 8，验证基本分支和递归结果。`Box::pin` 解决的是 Future 类型大小及固定问题，不保证消除逐层 poll 或析构时的调用栈增长，也不保证任意深度安全。输入深度不受控时，应考虑显式栈、循环或其他迭代算法，并控制内存与工作量；装箱本身不会生成后台任务。

[becomes stable]: https://blog.rust-lang.org/2024/03/21/Rust-1.77.0.html#support-for-recursion-in-async-fn
[that compiler limitation has been lifted]: https://github.com/rust-lang/rust/pull/117703/
