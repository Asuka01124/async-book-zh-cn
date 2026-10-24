<a id="pinning"></a>
# 固定（Pinning）

本章由 GPT-6 Luna 完整逐段翻译，GPT-6.1 Sol 按标准库契约校订原文中的不精确表述，并补齐异步状态机说明与可运行示例。历史提案和参考链接全部保留；它们描述的设想不等于当前稳定语言功能。本章测试使用 Rust 1.96.0。

固定（pinning）是出了名地难以理解的概念，其中有一些微妙而令人困惑的性质。本节将深入讨论这个主题（也可以说，深入得有些过头）。固定是 Rust 异步编程实现的关键[^design]，不过即使从未遇到固定，也完全可以做很多事情；当然，也不必深入理解它。

第一节会概述固定，希望这对大多数异步程序员来说已经足够。本章其余部分面向实现者、从事高级或底层异步编程的人，以及好奇的读者。

概述之后，本章会先介绍移动语义的背景，再讨论固定。我们会讲解总体思路、`Pin` 和 `Unpin` 类型、固定如何实现其目标，以及在实践中使用固定时的若干主题。之后还会讨论固定与异步编程，以及固定的若干替代方案和扩展（供特别好奇的读者阅读）。本章末尾列出了一些其他解释和参考资料的链接。

[^design]: 值得指出的是，固定是专为实现 Rust 异步机制而设计的底层构件。虽然它并不与 Rust 异步机制直接绑定，也可以用于其他用途，但它并非通用机制；尤其不能开箱即用地解决自引用字段问题。固定若用于异步代码以外的场景，通常只有在厚重的抽象层包装下才行得通，因为这会需要大量繁琐且难以推理的 `unsafe` 代码。

<a id="tldr"></a>
## 简要说明

`Pin` 包装指针，为需要固定保证的被指向值建立契约：它在原址保持有效，直到原址析构完成；不能在此之前移动该值或复用其存储。`Unpin` 类型不依赖这项限制，后文会解释例外。固定的核心是库 API 的契约和访问限制，不是编译器替所有值加上的自动地址锁。`unsafe` 代码确实可以破坏契约，但和其他安全抽象一样，实现者有责任证明它不会被破坏。

保证地址稳定，是安全封装自引用的一项必要条件，但 `Pin` 不会自动建立引用、初始化字段或证明其他别名规则。异步函数会被实现为数据结构，跨暂停点的变量可能作为字段存储，其中某字段可能借用另一字段，因此需要固定契约。不是每个 Future 都实际自引用。大多数情况下，程序员无需处理这些内部细节；但直接处理 Future 时可能需要，因为 `Future::poll` 的签名要求 `self` 为固定引用。

如果要借用一个 `!Unpin` Future 并继续轮询，通常先用 `pin!(future)` 固定 Future 值，得到 `Pin<&mut F>`，而不是只固定一个普通引用值本身（这常见于 `select`）。普通 `&mut F` 实现 Future 需要 `F: Unpin`。同样，要手动调用 poll，必须提供固定引用；可以使用 pin!、Box::pin，或接受已经固定的参数。若还希望可变访问对象内部，就需要理解下文的固定字段与投影，不能直接取出任意 `&mut Self`。

<a id="move-semantics"></a>
## 移动语义

讨论固定及相关主题时，一个有用的概念是 *place*（内存位置）。place 是一块可以存放值的内存（有一个地址）。引用实际指向的并不是值，而是一个 place。这就是为什么 `*ref = ...` 有意义：解引用得到的是 place，而不是值的副本。语言实现者熟悉 place，但编程语言通常会将它隐去（Rust 也是如此）。程序员通常对 place 有直觉上的认识，只是不一定会明确地想到它。

除了引用，变量和字段访问也会求值为 place。事实上，任何能出现在赋值左侧的表达式，在运行时都必须是 place（这就是为什么编译器术语会把 place 称为“左值”，即 lvalue）。

在 Rust 中，可变性是 place 的属性；借用所导致的“冻结”也是如此（我们可以说这个 place 正被借用）。

对非 Copy 值，`let b = a;` 会在语义上把值从 a 的 place 移到 b 的 place；之后 a 未初始化，不能继续使用，而 b 拥有该值。它允许运行时地址改变，却不承诺一定发生物理复制或地址变化[^compiler]。Copy 类型的按位复制则不会使原绑定失效；Copy 与 Unpin 是不同概念。

若存在仍须使用的借用，它不能指向已经移走的值。因此对于非 Copy 的 a，`let r = &a; let b = a;` 后若还使用 r，移动就会被拒绝；若 r 后面不再使用，非词法生命周期（NLL）可能让借用提前结束，这两句本身未必报错。编译器检查的是活跃借用，而非“曾声明过引用”。原始指针本身不提供同样的借用检查，解引用仍须自行满足有效性要求。

普通借用检查可以追踪从外部借入一个对象或其字段的关系；但在普通用户定义结构体中，不能用假想的 'self 生命周期声明并让借用检查器自动维护“某字段指向同一对象另一字段”的关系。下面仅作假想布局解释，并不是说编译器生成 async 状态机时也看不见内部借用。设想我们可以这样写：

```rust,ignore
struct Bad {
    field: u64,
    r: &'self u64,
}
```

假想一个 Bad 实例 b，其中 b.r 指向 b.field。如果允许按普通可移动结构体的规则执行 `let a = b;`，却没有额外约束维护这种内部关系，那么 a.r 仍可能指向 b.field 原来的位置，而非新的 a.field，从而失效。这说明这种普通结构体声明不能仅靠假想生命周期获得安全性；编译器生成的 async 状态机则另有固定和借用规则配合。

移动不只发生在局部绑定之间。Box 支持拥有所有权的移出，如 `let value = *boxed;`，目标位置可能是局部栈或另一个对象；单纯解引用或借用 `&*boxed` 不会移出。`take`、`replace` 和 `swap`（都在 [`std::mem`](https://doc.rust-lang.org/std/mem/index.html) 中）能通过 `&mut T` 把旧值移走，同时留下默认值、新值或交换后的值。借用的存储位置仍有效，但原对象已经离开，因而不能对固定的 `!Unpin` 值随意执行这些操作。

[^compiler]: 这里必须区分源代码绑定与运行时存储。编译器可以消除绑定、复用存储或省略物理复制；循环或多次调用中的同一源代码变量，也不必每次对应不同地址。应依据语言的有效性与所有权规则推理，而不是假设某条赋值一定对应特定内存指令。

可以把移动想象成转移一份按位表示并使源位置在语义上未初始化，但 Rust 不保证清零或擦除原内存。旧位仍然留在那里也不表示旧值还能使用；移动同样不是敏感数据擦除机制。

<a id="pinning-1"></a>
## 固定

重要提示：我会先讨论固定这一抽象概念，它与任何特定类型所表达的内容都不完全相同。接下来我们会逐步让这个概念具体化，最终给出不同类型所代表含义的精确定义；但这些类型都不与我们一开始讨论的固定概念完全相同。

如果一个对象不会移动或以其他方式失效，我们就说它已固定。如上所述，这并不是一个新概念：借用对象会在借用期间阻止对象移动。不过，Rust 的类型中并不会显式表明对象能否移动，尽管编译器知道这一点（因此会产生“无法移出……”之类的错误信息）。与借用（以及借用导致的临时移动限制）不同，固定是永久性的。对象可以从未固定变为已固定，但一旦固定，就必须一直保持固定，直到对象被销毁[^inherent]。

就像指针类型反映被指向对象的所有权和可变性（例如 `Box` 与 `&`、`&mut` 与 `&`），固定状态也通过指针类型表达。`Pin<Ptr>` 固定的是被指向的 place，并非 Ptr 句柄本身；移动一个 `Pin<Box<T>>` 句柄不会移动堆中的 T。不能把“固定指针”误解成指针变量自身必须占据固定地址。

粗略地说，`Pin<Box<T>>` 是指向一个拥有所有权且已固定对象的指针；`Pin<&mut T>` 是指向一个被独占借用、可变且已固定对象的指针（对比之下，`&mut T` 指向一个被独占借用且可变的对象，而该对象可能固定，也可能未固定）。

固定概念是在 Rust 1.0 之后加入的；出于向后兼容的原因，没有办法显式表示一个 *对象* 是否已固定。我们只能表示一个引用指向的是已固定还是未固定的对象。

固定与可变性正交。`Pin<&mut T>` 可以通过合适的投影修改允许修改的部分，但不能任意替换固定的 !Unpin 值；普通 `&mut T` 允许替换值。`Pin<&T>` 与 `&T` 都提供共享访问，后者只在借用有效期内限制移动，并不建立直到析构完成的固定契约。一个不可变绑定的拥有者仍可以把值移到别处；内部可变性则仍受相应类型的规则约束。

[^inherent]: 永久性不是所有可能语言设计中的必然要求，而是 Rust 当前固定契约的一部分。理论上可以研究生命周期可被使用者依赖的临时固定；这里不把历史讨论中对扩展难度的判断视为不可能性证明。对当前 Pin，销毁一个临时 Pin 引用并不会自动撤销已经建立的固定承诺。

<a id="unpin"></a>
### `Unpin` 标记

虽然我们是通过移动与不移动来引入固定的，而且名称也有这个暗示，但 `Pin` 实际上并不能告诉你被指向对象是否真的会移动。

什么？唉。

实际上，固定是一种关于有效性的契约，而不是关于移动的契约。它保证的是：*如果一个对象对地址敏感，那么*它的地址不会改变（因此从它派生出的地址，例如它的字段地址，也不会改变）。Rust 中的大多数数据都对地址不敏感，可以随意移动而不会出问题。`Pin` 保证被指向对象相对于其地址而言保持有效。如果被指向对象对地址敏感，就不能移动；如果它对地址不敏感，那么移不移动都无关紧要。

`Unpin` 表达类型不依赖 Pin 所提供的固定限制。若 T: Unpin，就能从固定指针中安全取得通常的可变访问或取回底层指针。反过来，`!Unpin` 只表示不能据此解除限制，不证明类型内部真的有自引用或其他地址依赖；PhantomPinned 就能让一个普通类型变为 !Unpin。尚未被固定的 !Unpin 值仍可以正常移动，类型实现者必须在进入固定阶段后才依赖地址稳定。

`Unpin` 是自动 trait，大多数普通类型都是 Unpin；字段通常决定自动实现。包含 [`PhantomPinned`](https://doc.rust-lang.org/std/marker/struct.PhantomPinned.html) 可阻止自动实现；在相应 nightly 功能下还可使用否定实现 `impl !Unpin for Type {}`，本章不运行该实验语法。实现者也能显式实现 Unpin，但必须确保固定 API 从不依赖会被该实现取消的保证，尤其要与结构性固定保持一致。

对于 Unpin 目标，可以用 Pin::new 与 Pin::into_inner 在固定指针和对应普通指针之间安全转换，因此能恢复 `Box<T>` 或 `&mut T` 的通常能力。Pin 不对这类目标施加额外的不可移动约束；对已经固定的 !Unpin 目标，则不能这样安全地取回指针并移走值。

实践中，使用 Unpin 类型更容易，因为可以用普通借用和移动 API。但 Unpin 不等于 Copy、Send 或“保证不移动”，也不会放宽所有权、借用与内部可变性的其他限制；它只解除固定带来的额外要求。

不应把 `Unpin` 理解成对象自身的属性；`Unpin` 唯一改变的是对象与 `Pin` 的交互方式。在固定语境之外使用 `Unpin` 约束，不会影响编译器的行为，也不会改变对象可执行的操作。使用 `Unpin` 的唯一理由，是配合固定使用，或将这个约束传递到使用固定的地方。

<a id="pin"></a>
### `Pin` 类型

[`Pin`](https://doc.rust-lang.org/std/pin/struct.Pin.html) 是指针包装类型，保证 `Pin<Ptr>` 与 Ptr 的内存布局和 ABI 相同，不增加额外分配或间接访问层。可以把它看作改变可用 API 的指针修饰符；“零额外表示成本”并不表示运行时连底层指针值也不存在。它影响类型检查与安全契约，而非执行一次动态地址锁定。

`Pin<Ptr>` 的契约针对 Ptr 的被指向对象，而非 Ptr 本身。对于固定的 !Unpin 值，要保持其地址与存储有效直到原址析构完成；对 Unpin 值，API 允许恢复普通移动能力。不要把 !Unpin 与“内部必然自引用”画等号。

<a id="pinning-values"></a>
### 固定值

对象创建时并未固定。对象起初是未固定的（可以自由移动），当创建一个指向该对象的固定指针时，对象才变为已固定。如果对象是 `Unpin`，使用 `Pin::new` 就能轻松完成；如果对象不是 `Unpin`，固定它时则必须确保它不会通过别名被移动或失效。

要在堆上固定对象，可以使用 [`Box::pin`](https://doc.rust-lang.org/std/boxed/struct.Box.html#method.pin)，或使用 [`Box::into_pin`](https://doc.rust-lang.org/std/boxed/struct.Box.html#method.into_pin) 转换既有 Box，两者都得到 `Pin<Box<T>>`。Arc 和 Rc 也有安全构造器。对于没有适用安全构造器的自定义指针，可能需要用 [`Pin::new_unchecked`](https://doc.rust-lang.org/std/pin/struct.Pin.html#method.new_unchecked)[^box-pin]。它要求实现者证明：固定的 !Unpin 目标在原址析构完成前保持有效，不能通过别名移走或提前复用存储。细节见该函数[文档](https://doc.rust-lang.org/std/pin/struct.Pin.html#method.new_unchecked)及下文[固定如何工作](#how-pinning-works)。

`Box::pin` 把对象放入堆上的固定位置；Box::into_pin 不移动既有堆值。使用 [`pin`](https://doc.rust-lang.org/std/pin/macro.pin.html) 宏可以在局部存储中固定值，并取得 `Pin<&mut T>`，无需额外堆分配[^not-stack]。宏拥有被固定的值，保证其存储在作用域内不会被普通安全代码移出；只对一个引用调用 pin!，固定的是那个引用值，不会神奇地固定它所指的原对象。

Tokio 也有一个 [`pin`](https://docs.rs/tokio/latest/tokio/macro.pin.html) 宏，用于固定局部值，并支持在宏中绑定变量。futures-rs 与 pin-utils 还有历史上常用的 pin_mut。当前 [futures::pin_mut 文档](https://docs.rs/futures/latest/futures/macro.pin_mut.html) 将其标为自 Rust 1.68 起建议迁移到标准库 pin! 的“软弃用”，这不同于编译器的 deprecated 警告；pin-utils 整个 crate 的维护状态则另见下文。

也可以用 `Pin::static_ref` 与 `Pin::static_mut` 分别接受 `&'static T` 与 `&'static mut T`。本机 Rust 1.96 源码标记它们稳定自 1.61，const 调用稳定自 1.84；它们不要求 T: Unpin。安全前提由永久借用满足；这不是允许任意读取或借用 static mut，全局可变静态仍有别名与数据竞争规则。

[^box-pin]: 固定实现和编译器都不会对 `Box`（或其他标准库指针）作特殊处理。`Box` 使用 `Pin` API 中的 `unsafe` 函数来实现 `Box::pin`。由于 `Box` 的安全保证，`Pin` 的安全要求得以满足。

[^not-stack]: “局部固定”比“栈固定”精确。跨 await 存活的局部值可能成为外层 Future 的字段；外层 Future 自身可在栈上，也可被执行器放在堆上。不能说异步函数的所有局部变量都在堆上，或 pin! 自己执行了堆分配。

<a id="using-pinned-types"></a>
### 使用固定类型

理论上，使用固定指针就像使用其他指针类型一样。但由于这并不是最直观的抽象，而且语言本身也不支持它，使用固定指针往往不太方便。最常见的固定使用场景是处理 future 和 stream；稍后会更详细地讨论这些具体情况。

将固定指针用于共享访问很简单，因为在相应 Deref 约束下，`Pin<Ptr>` 可解引用为 `&T`；原文这里的 `Poll<Ptr<T>>` 是 `Pin<Ptr>` 的笔误。同样，as_ref() 可得到 `Pin<&T>`。

使用固定类型最常见的方式是 `Pin<&mut T>`（例如在 [`Future::poll`](https://doc.rust-lang.org/std/future/trait.Future.html#tymethod.poll) 中）。不过，最容易创建的固定对象是 `Box::pin` 返回的 `Pin<Box<T>>`。可以用 [`Pin::as_mut`](https://doc.rust-lang.org/std/pin/struct.Pin.html#method.as_mut) 将后者转换成前者。但由于语言不支持重用引用（隐式重新借用），必须不断调用 `as_mut`，不能重复使用转换后的结果。例如（摘自 `as_mut` 文档）：

```rust,ignore
impl Type {
    fn method(self: Pin<&mut Self>) {
        // 做一些操作
    }

    fn call_method_twice(mut self: Pin<&mut Self>) {
        // `method` 会消耗 `self`，因此通过 `as_mut` 重新借用 `Pin<&mut Self>`。
        self.as_mut().method();
        self.as_mut().method();
    }
}
```

如果需要以其他方式访问固定的被指向对象，可以使用 [`Pin::into_inner_unchecked`](https://doc.rust-lang.org/std/pin/struct.Pin.html#method.into_inner_unchecked)。不过这是不安全的，必须非常谨慎地确保遵守 `Pin` 的安全要求。

<a id="how-pinning-works"></a>
### 固定如何工作

`Pin` 是指针的简单包装结构体（也称 newtype）。它通过对泛型参数要求 `Deref` trait，来确保它只用于指针并能发挥作用；但这只是为了表达意图，而不是为了维护安全性。和大多数 newtype 包装一样，`Pin` 的作用是在编译时表达不变量，而不是产生运行时效果。事实上，在大多数情况下，`Pin` 及固定机制都会在编译时完全消失。

更精确地说，`Pin` 表达的不变量关乎有效性，而不只是可移动性。它也是一个有效性不变量，仅在指针被固定后才适用——在此之前，`Pin` 没有任何效果，也不对固定之前发生的事情提出要求。指针一旦固定，`Pin` 就要求（并在安全代码中保证）被指向对象在被销毁之前始终在内存中的同一地址保持有效。

`Pin<&T>` 不提供比 &T 更多的直接修改能力，但不是“没有作用”：构造它仍需满足固定契约，可用来向接收方证明持续的地址保证。有限期的普通 &T 不能安全地固定任意 !Unpin 值，因为借用结束后拥有者可能移动它；`Pin::static_ref` 则利用永久共享借用满足条件。

对于允许修改的指针（例如 Box 或 &mut），普通可变访问可能移走被指向对象。因此对 !Unpin 目标，Pin 不会安全地提供任意访问底层指针或 &mut T 的能力。指针通常通过 [`DerefMut`](https://doc.rust-lang.org/std/ops/trait.DerefMut.html) 提供可变引用；Pin 的 DerefMut 则要求目标 Unpin，同时底层指针自身实现 DerefMut。

基本的访问限制很简单：Pin 通常提供共享访问，对 Unpin 对象还允许普通可变访问；此外也有固定投影与 Pin::set 等受约束的能力。剩下的难点是实现者必须维护的生命周期与别名不变量。as_ref/as_mut 等特定转换是安全的，但不能笼统说所有 Pin 类型之间的转换都安全；直接取回底层 !Unpin 指针的 unchecked API 仍需证明。

`Pin` 还提供了用于创建固定指针和访问底层数据的 `unsafe` 函数。和所有 `unsafe` 函数一样，维护安全不变量是程序员的责任，而不是编译器的责任。遗憾的是，固定的安全不变量分散在不同位置：它们在不同地方得到约束，很难用统一的方式整体描述。我不会在这里详细说明，而是请读者参阅文档；下面尝试做一个概述（详细说明见[模块文档](https://doc.rust-lang.org/std/pin/index.html)）：

- 创建新的固定指针 [`new_unchecked`](https://doc.rust-lang.org/std/pin/struct.Pin.html#method.new_unchecked)。程序员必须确保被指向对象已固定（即遵守固定不变量）。这项要求可能仅由指针类型本身满足（例如 `Box`），也可能需要被指向对象类型的配合（例如 `&mut`）。要求包括但不限于：
  - 指针的 Deref/DerefMut 必须稳定地返回同一被指向对象，不得在解引用过程中移动或替换该对象。
  - 正确实现 `Drop`，参见[析构保证](https://doc.rust-lang.org/std/pin/index.html#subtle-details-and-the-drop-guarantee)。
  - 如果需要固定保证，就要选择不实现 `Unpin`（使用 [`PhantomPinned`](https://doc.rust-lang.org/std/marker/struct.PhantomPinned.html)）。
  - 必须审查 `#[repr(packed)]`：不能从未对齐字段创建引用，且编译器可能把 packed 字段移到对齐临时位置析构，因此不能这样对字段承诺结构性固定。Box::pin 可以接收 packed 类型，但这并不使任何字段投影或地址依赖自动安全；固定抽象通常应避免 packed 布局。
- 访问固定值 [`into_inner_unchecked`](https://doc.rust-lang.org/std/pin/struct.Pin.html#method.into_inner_unchecked)、[`get_unchecked_mut`](https://doc.rust-lang.org/std/pin/struct.Pin.html#method.get_unchecked_mut)、[`map_unchecked`](https://doc.rust-lang.org/std/pin/struct.Pin.html#method.map_unchecked) 和 [`map_unchecked_mut`](https://doc.rust-lang.org/std/pin/struct.Pin.html#method.map_unchecked_mut)。实现者必须继续维护已经建立的固定保证，直到原址析构完成；责任跨越这个 unsafe 调用、当前借用乃至临时 Pin 指针的生命周期，不能把指针取出后就当成普通可移动值。
- 不能以其他方式提供从固定类型中移出数据的途径（因为实现这种途径需要使用 `unsafe`）。

<a id="pinning-pointer-types"></a>
#### 固定指针类型

前面提到，Pin 包装指针；常见形式包括 `Pin<Box<T>>`、`Pin<&T>` 与 `Pin<&mut T>`。对它进行有意义的解引用操作需要 Ptr: Deref。标准库还有安全的 [Rc::pin](https://doc.rust-lang.org/std/rc/struct.Rc.html#method.pin) 与 [Arc::pin](https://doc.rust-lang.org/std/sync/struct.Arc.html#method.pin)；对 Unpin 目标可直接用 Pin::new。没有安全构造器的自定义指针，可能需要在内部使用 new_unchecked，并满足以下契约：

- 指针的 `Deref` 和 `DerefMut` 实现不得从被指向对象中移出数据。
- 对 !Unpin 目标，不能通过任何安全路径重新取得足以移走固定值的普通可变访问；即使临时 Pin 引用已销毁，承诺也仍成立。这不禁止实现者在严格证明不移走值的 unsafe 代码中取得 &mut，也不禁止受约束的字段投影。
- 任意已有 Rc/Arc 别名不能随便套 new_unchecked：其他别名日后可能取得独占访问并移动值；Rc::pin/Arc::pin 的安全构造与封装会维护约束，不能把它们概括成完全不能用于固定。
- 指针的 Drop 不得先移动或使被指向值失效，再执行该值的析构；拥有型指针可以在原址完成析构后释放存储。

更多详情请参见 `new_unchecked` 的[文档](https://doc.rust-lang.org/std/pin/struct.Pin.html#method.new_unchecked)。

<a id="pinning-and-drop"></a>
### 固定与 `Drop`

固定契约在固定对象被销毁之前一直有效（严格来说，是在它的 `drop` 方法返回时，而不是调用时）。通常这很直接，因为对象被销毁时会自动调用 `drop`。如果你手动管理对象的生命周期，就可能需要额外考虑。如果一个对象已经或可能被固定，且该对象不是 `Unpin`，那么在释放或重用对象的内存或地址之前，必须先调用它的 `drop` 方法（使用 [`drop_in_place`](https://doc.rust-lang.org/std/ptr/fn.drop_in_place.html)）。详情见[标准库文档](https://doc.rust-lang.org/std/pin/index.html#drop-guarantee)。

Rust 不保证析构一定执行：[mem::forget](https://doc.rust-lang.org/std/mem/fn.forget.html)、引用计数环与进程退出都可能跳过它。固定的 Drop 保证是：在原址析构完成前，不得使固定存储失效或复用；泄漏拥有型分配可以永不析构，却仍保留存储。不能靠“忘记临时 Pin 引用”来提前复用其所指位置，unsafe 抽象也不能把调用者必定执行 Drop 当作前提。

实现依赖地址稳定的类型时，Drop 必须把已经固定的 self 视为仍受 Pin 约束。签名虽然是 &mut Self，也不能用 replace/swap 移走结构性固定字段。对象也可能在固定之前被正常销毁，所以析构要兼容两种阶段。下面是示意写法，Type 尚未定义，不作为可运行示例；内部函数使用具体 Type，避免嵌套函数不能捕获外层 Self 的问题：

```rust,ignore
impl Drop for Type {
    fn drop(&mut self) {
        // 必须同时证明：本次原址析构期间不移动值，且之后不再把它作为活值使用。
        inner_drop(unsafe { Pin::new_unchecked(self)});

        fn inner_drop(this: Pin<&mut Type>) {
            // 在这里编写实际的析构代码。
        }
    }
}
```

注意，有效性要求取决于正在实现的类型。建议明确规定这些要求，尤其是对象销毁相关的要求；如果可能涉及多个对象（例如侵入式链表），更应如此。确保这里正确无误会很有意思！

<a id="pinned-self-in-methods"></a>
### 方法中的固定 `self`

对固定类型调用方法时，需要考虑这些方法的 `self` 类型。如果方法不需要修改 `self`，仍然可以使用 `&self`，因为 `Pin<...>` 可以解引用为借用引用。不过，如果需要修改 `self`（且类型不是 `Unpin`），就需要在 `&mut self` 和 `self: Pin<&mut Self>` 之间作出选择（固定指针不会隐式强制转换为后一种类型，但可以用 `Pin::as_mut` 轻松转换）。

使用 `&mut self` 能让实现更简单，但意味着该方法不能在固定对象上调用。使用 `self: Pin<&mut Self>` 就需要考虑 pin projection（参见下一节），而且只能在固定对象上调用。虽然这一点令人困惑，但只要记得固定是分阶段的概念，就很容易理解：对象起初未固定，之后会经历状态变化而变为已固定。`&mut self` 方法可以在第一阶段（未固定）调用，`self: Pin<&mut Self>` 方法可以在第二阶段（已固定）调用。

注意，`drop` 接收 `&mut self`（尽管它可能在任一阶段调用）。这是由于语言的限制以及向后兼容的需要。编译器会对它特殊处理，并附带安全要求。

<a id="pinned-fields-structural-pinning-and-pin-projection"></a>
### 固定字段、结构性固定与固定投影

一个对象已固定，这能告诉我们它的字段是否也已“固定”吗？答案取决于数据类型实现者的选择，并没有普遍适用的答案（事实上，同一对象的不同字段也可能不同）。

如果对象的固定状态传递到某字段，就称其为结构性固定。投影 API 通常是 `fn get_field(self: Pin<&mut Self>) -> Pin<&mut Field>`；非结构性固定字段则可返回 &mut Field。一般的 !Unpin 聚合类型手工实现投影需要 unsafe 证明；若整个 Self: Unpin，也可能直接安全地 get_mut，投影宏则可代为生成并校验所需限制。公开 API 必须一致，不能既向外承诺同一 !Unpin 字段固定，又允许其他安全方法移走它。

如果聚合类型依赖某字段被固定，就必须对它维护结构性固定；例如内部保存指向该字段的指针，或要直接轮询一个可能依赖地址的 Future 字段。集合并不会自动把固定传给元素：`Pin<&mut Vec<T>>` 不固定堆中每个 T，Vec 的重新分配、删除、排序等操作可能移动元素；若需要元素地址稳定，可另外使用 `Vec<Pin<Box<T>>>`。移动 Vec 或 Box 句柄与移动其所拥有的值也要区分。

编写 `unsafe` 代码时，只能假设固定保证适用于对象中具有结构性固定的字段。相反，可以安全地把非结构性固定字段视为可移动，而无需担心它们的固定要求。尤其是，只要一个结构体始终把某字段视为非结构性固定，即使该字段不是 `Unpin`，这个结构体仍然可以是 `Unpin`。

结构性固定把聚合对象的固定要求延伸到字段。对 !Unpin 的结构性固定字段，不能移走其内容，也不能在原址析构完成前释放或复用存储；即使其他字段析构发生 panic，也要维持应有的析构/泄漏路径。因此必须审查聚合对象的 Drop。若要安全实现聚合对象的 Unpin，所有依赖其固定保证的结构性固定字段也必须满足 Unpin。字段自身若为 Unpin，则仍按它自己的契约允许普通移动，不应将结构性固定解释成取消这项豁免。

<a id="macros-for-pin-projection"></a>
#### 固定投影宏

有一些宏可以帮助实现固定投影。

[pin-project](https://docs.rs/pin-project/latest/pin_project/) crate 提供了 `#[pin_project]` 属性宏（以及 `#[pin]` 辅助属性），会为你实现安全的固定投影：它会为被标注的类型创建一个固定版本，并可以通过该类型上的 `project` 方法访问。

[Pin-project-lite](https://docs.rs/pin-project-lite/latest/pin_project_lite/) 是一种替代方案，它使用声明式宏（`pin_project!`），工作方式与 pin-project 非常相似。Pin-project-lite 的轻量体现在它不是过程宏，因此项目无需为实现过程宏而添加依赖。不过，它的表达能力不如 pin-project，也不会提供自定义错误消息。如果你希望避免添加过程宏依赖，推荐使用 pin-project-lite；否则推荐使用 pin-project。

Pin-utils 提供了 [`unsafe_pinned`](https://docs.rs/pin-utils/latest/pin_utils/macro.unsafe_pinned.html) 宏来帮助实现固定投影，但整个 crate 已弃用，建议使用上述 crate 以及现在标准库中的功能。

本章将已有锁包 pin-project-lite 0.2.17 提升为独立验证 crate 的直接依赖。下面的包装器把 inner 投影为 `Pin<&mut F>`，把普通计数器投影为 `&mut usize`；无需对 F 添加 Unpin：

```rust
{{#include ../../examples/guide-tests/src/pinning.rs:projection}}
```

宏还约束 Unpin 与 Drop 实现，防止投影 API 和其他可移动访问相冲突。若需要自定义析构，应使用所选宏提供的固定析构机制，并遵照其文档，不能绕开宏自行移走字段。本章测试让内层 async Future 在真实 Pending 期间保存对自己局部 String 的借用，恢复后正确读取，并验证两次投影轮询计数。

<a id="assigning-to-a-pinned-pointer"></a>
### 对固定指针赋值

通常可以安全地[对固定指针赋值](https://doc.rust-lang.org/std/pin/index.html#assigning-pinned-data)。虽然不能用通常的方式（`*p = ...`）完成，但可以使用 [`Pin::set`](https://doc.rust-lang.org/std/pin/struct.Pin.html#method.set)。更一般地说，可以使用 `unsafe` 代码为被指向对象的字段赋值。

使用 `Pin::set` 始终是安全的，因为之前固定的被指向对象会先被析构，从而满足固定要求；新的被指向对象只有在移动到固定位置完成后才会被固定。对单个字段赋值不会自动违反固定要求，但必须谨慎确保整个对象仍然有效。例如，如果给一个字段赋值，那么任何引用该字段的其他字段都必须在新对象中继续有效（这不是固定要求的一部分，但可能是对象其他不变量的一部分）。

原文还讨论把固定对象“复制”到另一位置并修正自引用。这不能当成通用替换办法：对非 Copy 值做原始位复制会带来双重所有权、双重析构、别名和初始化问题，修正一个内部指针并不足以证明安全。某些类型可以设计专门的复制/赋值协议，但必须满足所有不变量，并正确销毁目标旧值；不能移走或使原固定值失效。对一般安全代码，优先使用 Pin::set 或类型公开的受支持操作，不运行原始复制错例。

<a id="pinning-and-async-programming"></a>
## 固定与异步编程

希望你在使用 Rust 异步编程时可以一直不必担心固定。有时你会遇到需要固定的边界情况；如果要实现 future、运行时或类似的内容，就需要了解固定。本节会解释原因。

异步函数会被实现为 Future；[Future 详解](../part-guide/futures.md) 说明了其惰性和状态推进，[实现 Future 与 Stream](implementing-futures-streams.md) 展示了手写状态机。await 遇到 Pending 时，跨暂停点仍需使用的变量、借用与子 Future 必须保存在外层 Future 中。概念上可以把它想成带执行状态标记的结构体；真实编译器布局并不承诺就是某个手写 enum。这样的变量可以引用另一个保存在 Future 中的变量，例如下面这个合法函数：

```rust
async fn foo(wait: impl std::future::Future<Output = ()>) {
  let a = String::from("abc");
  let b = &a;
  wait.await;
  assert_eq!(b.len(), 3); // 暂停后仍使用借用，不能由NLL提前结束。
}
```

这里生成的 future 对象大致会是这样：

```rust,ignore
struct Foo {
  a: A,
  b: &'self A,  // 不变量：`self.b == &self.a`
}
```

（这里做了一些简化，忽略了执行状态等信息；重要之处在于变量/字段。）

这在直觉上说得通，但遗憾的是 Rust 中并不存在 `'self`。而且这是有理由的！请记住，Rust 对象可以移动，因此下面这样的代码是不健全的：

```rust,ignore
let f1 = Foo { ... }; // f1.b == &f1.a
let f2 = f1; // f2.b == &f1.a，但 f1 已不复存在，因为它已经移动到 f2
```

注意，这不仅仅是无法命名生命周期的问题；即使使用原始指针，这样的代码仍然不正确。

解决办法是分阶段：刚构造的 Future 可以移动；在固定后进行 poll，才可能进入内部借用已经建立的状态；从此直到原址析构完成都不能移走它。编译器负责生成状态机与借用关系，手写同类状态机则可能需要原始指针和严密的 unsafe 证明。上面的 Foo 与 'self 只是解释布局关系的伪代码，不是可声明的 Rust 类型。

Future::poll 的 self 类型因此是 `Pin<&mut Self>`。普通 await 和运行时 spawn 通常把固定安排隐藏在实现里；它们不会在 !Unpin Future 暂停后任意“解除固定”并移走它。完成也不自动解除固定：可能还有依赖地址的析构逻辑，所以仍应在原址析构。Unpin 类型则按其自身契约允许普通移动。

<a id="manual-pinning"></a>
### 手动固定

有些情况下，固定会穿透 async/await 抽象而暴露出来。根本原因在于 `Future::poll` 和 `Stream::poll_next` 的签名中有 `Pin`。直接使用 future 和 stream（而不是通过 async/await）时，可能需要考虑固定才能让程序正常工作。需要固定类型的一些常见原因如下：

- 对 future 或 stream 进行轮询——无论是在应用代码中，还是在实现自己的 future 时。
- 使用装箱的 future。如果你使用装箱的 future（或 stream），因此显式写出 future 类型而不是使用异步函数，就很可能会在这些类型中看到很多 `Pin<...>`，并需要使用 `Box::pin` 创建 future。
- 实现 future——在 `poll` 内部，`self` 已固定，因此你需要使用固定投影和/或 `unsafe` 代码来获得对 `self` 字段的可变访问。
- 组合 Future 或 Stream。若在循环外保留一个 !Unpin Future，再于 select 中借用它，应固定 Future 值并借用所得的 `Pin<&mut F>`，不是固定一个普通引用本身；[取消一章](cancellation.md) 的共享测试展示了在竞争失败后继续同一 Future。
- 使用 stream——Rust 对 stream 的抽象目前比 future 少，因此与使用 future 相比，你更可能使用组合器方法（严格来说不需要固定，但似乎更容易遇到引用或创建 future/stream 的问题），甚至手动调用 `poll`。

<a id="alternatives-and-extensions"></a>
## 替代方案与扩展

本节面向对固定相关语言设计感兴趣的读者。如果你只想阅读、理解和编写异步程序，完全不必读这一节。

固定很难理解，而且有时显得有些笨拙，所以人们常会想：有没有更好的替代方案或变体？我会介绍几种方案，并说明它们为什么行不通，或为什么比预想的更复杂。

了解历史背景很重要。新语言可以从一开始为 async/await、自引用或不可移动类型设计不同机制；Rust 的 Future、Pin 和 async/await 则在 1.0 后逐步稳定，必须考虑向后兼容与交付成本。原作者认为涉及线性类型等方案可能需要以数十年计的基础研究、设计和实现工作；这是历史设计判断，不是所有替代方案都更好或必然耗时多久的定论。

<a id="alternatives"></a>
### 替代方案

首先考虑一类方案：默认让 Rust 类型不可移动。注意，这会显著改变 Rust 的基本语义；这类方案很可能需要付出大量工作才能实现向后兼容（我不猜测某个具体方案是否可能实现，但借助自动 trait、derive 属性、edition、迁移工具等技术，或许存在可能性）。

一种提案（确切地说是一组提案，因为语义有多种定义方式）是引入一个 `Move` 标记 trait（类似 `Copy`），用来标记对象可移动，而其他所有类型都不可移动。与 `Pin` 不同，这属于值的属性，而不是指针的属性，因此影响范围大得多。例如，如果 `b` 没有实现 `Move`，那么 `let a = b;` 就会报错。

这种方法的根本问题在于，当前的固定是分阶段的概念（一个 place 起初未固定，之后变为已固定），而类型适用于值的整个生命周期。（最好也把固定理解为 place 的属性，而非值或类型的属性；不过类型适用于值。对于任何基于 trait 的方案，这是否是根本问题，我也不确定。）这两个博文探讨了该问题：[两种不要移动的方式](https://theincredibleholk.org/blog/2024/07/15/two-ways-not-to-move/)和 [Rust 中易用的自引用类型](https://blog.yoshuawuyts.com/self-referential-types/#immovable-types)。

此外，任何 `Move` trait 很可能都会遇到[向后兼容问题](https://without.boats/blog/pin/)，并导致“传染性约束”（也就是说，许多地方都需要 `Move` 或 `!Move`）。

另一种提案是支持类似 C++ 的移动构造函数。但这会打破 Rust 的基本不变量：对象始终可以按位移动。这会让 Rust 更难预测，因此 Rust 程序也更难理解和调试。这是最糟糕的一类不向后兼容变更，因为它会悄无声息地破坏 `unsafe` 代码：它改变了代码作者可能依赖的基本假设。此外，这种根本性变更需要巨大的设计与实现工作。除了这些实际问题之外，它能否奏效也不明确：移动构造函数可以用来修正被移动对象中的引用，但指向该对象、来自对象外部的引用可能无法修正。

另一类潜在方案是偏移引用。偏移引用是一种相对引用，而不是绝对引用；也就是说，即使对象在内存中移动，作为偏移引用的字段仍会指向同一对象内部的另一个字段。偏移指针的问题是，一个字段必须是偏移指针或绝对指针。但异步函数中的引用会变成字段，而这些字段有时引用 future 对象内部的内存，有时引用其外部的内存。

<a id="extensions"></a>
### 扩展

有多个提案旨在让固定更强大和/或更容易使用。这些提案大多希望以各种方式让固定成为语言中的一等功能，而不再只是库概念（通常还会扩展标准库）。我会介绍几种较成熟的想法。它们彼此相关，总体目标都是让固定位置更容易创建和使用，从而改善固定的易用性，特别是结构性固定和 `drop` 方面。

以下保留历史提案的名称和原链接；它们是设计讨论，不是本章 Rust 1.96 代码可以直接使用的稳定语法或标准库接口。核查提案原文不等于实现或运行了这些机制。

[固定位置](https://without.boats/blog/pinned-places/)沿着“固定是 place 的属性，而不是值或类型的属性”这一思路，给引用加入类似 `mut` 的 `pin`/`pinned` 修饰符。它会与重新借用和方法解析相结合，从而改善调用带固定 `self` 的方法时的易用性。

[`UnpinCell`](https://without.boats/blog/unpin-cell/) 扩展了固定位置的想法，以支持字段的原生固定投影。[MinPin](https://smallcultfollowing.com/babysteps/blog/2024/11/05/minpin/) 是一种更精简（且向后兼容）的原生固定投影与更好的 `drop` 支持提案。

[`Overwrite` trait](https://smallcultfollowing.com/babysteps/series/overwrite-trait/) 是一个提议中的 trait，它明确区分了修改对象某一部分的权限（`foo.f = ...`）和覆盖整个对象的权限（`*foo = ...`）；目前对所有可变引用来说，这两种操作都允许。该提案还包括不可变字段。`Overwrite` 可以说是 `Unpin` 的一种替代方案；结合固定位置的一些想法，它有望改善固定的使用体验。遗憾的是，尽管它可以向后兼容地采用，迁移所需的工作量会比其他扩展大得多。

<a id="references"></a>
## 参考资料

- [标准库文档](https://doc.rust-lang.org/std/pin/index.html) 是关于 `Pin` 等行为和保证的权威来源，文档质量很好。
  - [`Pin`](https://doc.rust-lang.org/std/pin/struct.Pin.html)、[`Unpin`](https://doc.rust-lang.org/std/marker/trait.Unpin.html)、[`pin` 宏](https://doc.rust-lang.org/std/pin/macro.pin.html)
- [RFC 2349](https://rust-lang.github.io/rfcs/2349-pin.html) 是提出固定机制的 RFC。稳定版 API 与 RFC 中提出的 API 略有不同，但 RFC 对核心概念及设计理由有很好的解释。
- 一些解释固定机制的博文或其他资料：
  - [Pin](https://without.boats/blog/pin/)：WithoutBoats（固定机制的主要设计者）撰写，介绍固定机制的历史、背景、设计理由，以及它为何难以理解。
  - [为什么 std::pin::Pin 如此奇怪？](https://sander.saares.eu/2024/11/06/why-is-stdpinpin-so-weird/)：深入探讨固定机制的设计理由和实践用法。
  - [Rust 中的 Pin、Unpin 及其必要性](https://blog.cloudflare.com/pin-and-unpin-in-rust/)
  - [async/await 中的固定章节](https://os.phil-opp.com/async-await/#pinning)
  - [Pin 与痛苦](https://fasterthanli.me/articles/pin-and-suffering)：一篇非常详尽、语气轻松的博文，通过大量示例讲解如何理解异步代码和固定机制。
  - Jon Gjengset 所著的 *Rust for Rustaceans* 一书，对固定为何是实现 async/await 所必需以及固定如何工作有很好的说明。

## 本章验证边界

三个新测试使用安全 Rust：PhantomPinned 地址探针在真实 Pending/恢复/Drop 中检查同址，验证移动 Box 句柄不会移动所指对象；Pin::set 测试确认旧值在原址析构后，新值才被轮询并在同址析构；投影测试验证 !Unpin 内层 Future 保存跨 Pending 借用并恢复。地址探针没有存储或解引用悬空指针，未运行任何破坏固定的 unsafe 错例。

历史伪代码仍标为 ignore，提案未编译运行。标准库在线文档可能晚于测试工具链，因此 static_ref/static_mut、Rc/Arc::pin 等版本事实还对照了本机 Rust 1.96 源码。没有运行 Miri 或给任意 unsafe 抽象做完备健全性证明；这些行为测试也不能替代实现者的契约证明。
