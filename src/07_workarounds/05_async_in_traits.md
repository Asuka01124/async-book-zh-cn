<a id="async-in-traits"></a>
# Trait 中的 `async`

从 Rust 1.75 起，稳定版已经支持 trait 中的 `async fn` 和返回位置的 `impl Trait`。原文“稳定版还不能在 trait 中使用 async fn”的说法已经过时；[2022 年 11 月 17 日的 nightly 最小可行实现介绍](https://blog.rust-lang.org/inside-rust/2022/11/17/async-fn-in-trait-nightly.html)应当作为历史材料阅读。稳定时的功能与限制见原文后来补充的 [2023 年稳定公告](https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits.html)。本节示例以 Rust 1.96.0 验证。

## 原生方法与静态分发

当调用方知道实现类型，或通过泛型参数调用时，可以直接使用原生异步方法：

```rust
{{#include ../../examples/guide-tests/src/legacy_async_traits.rs:native}}
```

`&impl Greeting` 是泛型参数形式，采用静态分发；实现返回的具体 Future 类型由编译器处理，不要求为每次调用额外装箱。这不保证方法内部没有分配：本例的 `format!` 就会构造拥有所有权的 String。也不能仅凭静态分发就推断某个应用必然更快，性能还取决于实际工作、分配、代码体积与调用方式。

## 泛型代码需要什么 Send 保证

只写 `trait Greeting { async fn greet(...); }` 并不向泛型调用方保证返回的 Future 是 Send。给实现类型加 `Send` 也不能替代这个保证。如果公共接口需要让调用方把等待该方法的任务交给 `tokio::spawn`，可以显式约束返回的 Future：

```rust
{{#include ../../examples/guide-tests/src/legacy_async_traits.rs:send}}
```

trait 用 `fn -> impl Future + Send` 声明契约，实现仍可写 `async fn`，只要生成的 Future 满足要求。示例同时要求被移入任务的 service 是 `Send + 'static`；方法产生的 Future 则可借用任务自己拥有的 service，不要求每次方法调用都返回 static Future。若某个实现跨 await 保存共享的 `&self`，通常还需要 Self 为 Sync；具体条件应从保存的状态推导，不应把 Send/Sync 当成相同概念。

公共 trait 应尽早决定是否承诺 Send，因为仅靠方法的裸 async 签名，泛型调用方无法假定这个额外属性。原生异步方法支持在本地执行非 Send Future，这同样是有效的接口选择。

## 动态分发与手动类型擦除

原生 async 方法目前仍不能直接通过 `dyn Trait` 调用；它的隐藏 Future 返回类型不满足 [dyn 兼容规则](https://doc.rust-lang.org/reference/items/traits.html#dyn-compatibility)。给方法添加 `where Self: Sized` 可以把它排除在动态分发之外，但不能让该方法在 trait 对象上可调用。需要动态调用时，可以显式返回装箱 Future：

```rust
{{#include ../../examples/guide-tests/src/legacy_async_traits.rs:erased}}
```

`BoxFuture<'a, String>` 等价于 `Pin<Box<dyn Future<Output = String> + Send + 'a>>`，这里保留了对 self 和 name 的借用期限，未强制 static。方法通过 trait 对象分派，内部 Future 也经类型擦除后轮询。这个具体实现每次调用执行 `.boxed()`；存放服务对象的 `Box` 是另一层包装，不应与每次返回的 `BoxFuture` 混为一谈。BoxFuture 的 Send 保证也不自动使持有 service 引用的外层调用 Future 为 Send；需要跨线程时，还要检查 trait 对象本身的 Send/Sync 与引用条件。

## async-trait 宏与开销

原文推荐的 [async-trait crate](https://github.com/dtolnay/async-trait)仍可用于需要类型擦除等场景。它通常把异步方法改写为返回 `Pin<Box<dyn Future<...> + Send + 生命周期>>` 的普通方法，并产生对应的约束；`#[async_trait(?Send)]` 可选择不要求 Send。具体生成的生命周期与接收者约束应以该宏文档为准。宏只改变接口与包装方式，不会自动生成任务或提供运行时。

原文“每次调用都有堆分配”的讨论针对这样的装箱方案，不能套用于所有原生 async trait 调用。装箱、动态分派以及宏引入的额外约束都应纳入 API 选择；对于调用频繁的底层接口，应根据真实负载测量，而不是预先断言开销对绝大多数应用不重要，或把调用次数当成统一阈值。

本章没有添加或运行 async-trait 宏依赖。可运行验证使用原生 trait 与手动 BoxFuture：`cargo test --manifest-path examples/guide-tests/Cargo.toml legacy_async_traits --locked --offline` 检查静态调用、声明 Send 后的生成任务，以及借用局部 String 的 dyn 调用结果一致。示例方法立即完成，只用于接口与分发验证，不代表真实 I/O 或性能基准。相关生命周期与任务约束见[更多 async/await 主题](../part-guide/more-async-await.md)。
