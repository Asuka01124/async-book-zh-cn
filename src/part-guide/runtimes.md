<a id="runtimes-and-runtime-issues"></a>
# 运行时及其使用边界

Future 描述计算，执行器轮询计算，IO 和时间驱动器把外部事件转换成唤醒。运行时把这些服务组织起来，通常还包含阻塞线程池。标准库没有指定唯一运行时；本书主要使用 Tokio。纯计算 Future 可以在许多执行器上运行，但依赖特定驱动器的资源不能随意互换。

<a id="running-async-code"></a>
## 运行异步代码

### 显式启动与异步 main

`#[tokio::main]` 是创建运行时并调用 `block_on` 的便利写法；操作系统入口仍是同步的。默认宏配置使用多线程调度器，可通过 `flavor = "current_thread"` 改用当前线程。手动启动则方便处理构建错误、配置线程和与同步应用集成：

```rust
{{#include ../../examples/guide-tests/src/runtime_details.rs:explicit-runtime}}
```

`Builder::new_current_thread()` 选择调度器，`enable_all()` 启用编译时已提供的 IO 和时间驱动器；Cargo 功能和运行时启用是不同层次。调用 `sleep` 时没有时间驱动器会 panic，不是自动退化为 `thread::sleep`。

`Runtime::block_on` 阻塞调用线程直到根 Future 完成，并进入对应运行时上下文。根 Future不等于一个通过 `spawn` 生成的任务，不要求与生成任务完全相同的 `Send + 'static` 约束；因此可直接借用调用栈上的数据。根 Future 返回也不代表所有后台任务完成，需要显式等待句柄或管理器。

### 上下文、Runtime 与 Handle

[`Runtime`](https://docs.rs/tokio/latest/tokio/runtime/struct.Runtime.html)拥有运行时生命周期；[`Handle`](https://docs.rs/tokio/latest/tokio/runtime/struct.Handle.html)提供可克隆的访问句柄，可传给其他同步线程，用于生成任务或进入上下文。`Handle::current()` 在没有上下文时 panic；不确定是否处于运行时内时，可用 `try_current()`。

`runtime.enter()` 或 `handle.enter()` 设置当前线程上下文，返回一个守卫，使 `tokio::spawn` 等知道使用哪个运行时。它**不等于驱动执行**。在当前线程运行时中，只进入上下文再生成任务，并不会让任务自行跑起来。上下文守卫应按进入顺序逆序销毁，也不要让它跨异步等待点。

有些资源在构造时就要求上下文。例如从同步代码调用 `rt.block_on(tokio::time::sleep(duration))`，参数先于 `block_on` 求值，可能还没进入上下文就 panic；应写 `rt.block_on(async { tokio::time::sleep(duration).await })`。把资源的创建和使用放在运行时内，通常最清楚。

`Handle::block_on` 也会阻塞调用线程，不能在正在执行的异步上下文里嵌套调用。在 `current_thread` 运行时上，它**不能驱动 IO 和时间驱动器**；如果没有另一个线程正在同一运行时上执行 `Runtime::block_on`，需要这些驱动器的 Future 就无法正常前进。同步线程可以用 Handle 连接到正在工作的多线程运行时，但须同时管理其关闭和线程等待，避免彼此阻塞。

### 关闭运行时

丢弃 `Runtime` 会开始关闭；保留 `Handle` 不会阻止它关闭，保留 `Arc<Runtime>` 的强引用则会延长其生命周期。关闭不承诺完成所有任务：普通异步任务在执行器重新取得控制后会被销毁，已经开始的阻塞任务则继续到函数返回。绑定到已关闭运行时的 IO 会失效，计时器操作可能 panic。

默认析构可能等待很久，尤其是仍在运行的阻塞函数。`shutdown_timeout` 和 `shutdown_background` 可以结束调用方的等待，但不会强制停止这些函数；未停止的工作仍可能继续占用线程和资源。因此应先按[清理章](dtors.md)停止入站、通知、等待和关闭资源，再同步销毁运行时；不要在异步任务内部直接丢弃一个需要阻塞关闭的自建运行时。[运行时关闭文档](https://docs.rs/tokio/latest/tokio/runtime/struct.Runtime.html#shutdown)说明了这些差异。

<a id="threads-and-tasks"></a>
## 线程与任务

### 多线程调度不等于每个任务一条线程

Tokio 的多线程调度器把许多任务放在有限工作线程上，通常使用工作窃取平衡负载。任务可能在两次轮询之间迁移线程，不能把线程局部状态当成任务局部状态。`tokio::spawn` 因此要求 Future 及输出满足 `Send + 'static`；`'static` 表示不借用短生命周期的外部数据，不表示任务会永远活着。

当前线程调度器在调用它的线程上交错运行任务，但普通 `tokio::spawn` 的接口仍要求 `Send + 'static`。仅把运行时改成单线程，不会自动允许传入持有 `Rc` 的 Future。直接等待的根 Future 与本地任务是不同途径。

[`yield_now`](https://docs.rs/tokio/latest/tokio/task/fn.yield_now.html)主动提供调度机会，但不承诺下一次轮询哪个任务，也不保证每个上层组合器都把让出传播到执行器。Tokio 的公平性保证依赖任务总数有界、每次轮询耗时有界等条件，且不等于所有任务获得相同 CPU 配额。大量 CPU 计算或循环轮询立即就绪 Future，应分块、设置并发上限或交给专用计算线程池。

### LocalSet 与非 Send 任务

[`LocalSet`](https://docs.rs/tokio/latest/tokio/task/struct.LocalSet.html)让一组任务在同一线程执行，从而容纳 `Rc`、`RefCell` 等非 `Send` 状态。通过它的 `run_until` 或 `block_on` 驱动，在其中调用 `spawn_local`：

```rust
{{#include ../../examples/guide-tests/src/runtime_details.rs:local-runtime}}
```

本地任务仍须满足 `'static`，所以这里把 `Rc` 的拥有权移入任务，而不是借用将退出的栈变量。`spawn_local` 要有本地任务上下文，单独的 `current_thread` 运行时不等价于 `LocalSet`。`tokio::spawn` 即使在 LocalSet 里调用，也仍生成普通任务，不会自动变成本地任务。

`run_until(root)` 在 root 完成时返回，不保证该集合的其他任务全部完成；后续再次驱动集合可继续运行它们，直接等待 `local.await` 则会等集合中任务都结束。集合只有被驱动时才运行，丢弃集合会取消剩余本地任务。按官方限制，应在 `#[tokio::main]`、`#[tokio::test]` 或直接的 `Runtime::block_on` 根上下文中使用 `run_until`，不能把 LocalSet 随意塞入 `tokio::spawn` 的任务里。

### 阻塞工作与 block_in_place

[`spawn_blocking`](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html)把闭包送到阻塞池，等待句柄时不占住异步任务所在的工作线程。它要求闭包和结果可跨线程且拥有所需状态，适合最终会自行返回的阻塞调用。池达到上限后会排队；默认上限面向阻塞 IO，不能当作 CPU 密集型任务的合理并行数。长驻同步循环更适合有明确停止协议的专用线程。

[`block_in_place`](https://docs.rs/tokio/latest/tokio/task/fn.block_in_place.html)在**当前线程**运行闭包，同时通知多线程调度器转交其他工作。它可借用当前栈数据，但整个当前任务在闭包返回前都暂停了，`join!` 中同任务的其他分支也不能前进。在 `current_thread` 运行时中调用会 panic；在运行时外调用则像普通闭包调用。通常优先用 `spawn_blocking` 保持边界清楚。

需要从一个已知的多线程异步上下文桥接到必须同步返回的接口时，可在这个受限区域重新进入异步上下文：

```rust
{{#include ../../examples/guide-tests/src/runtime_details.rs:blocking-bridge}}
```

这不是在任意异步代码中嵌套 `block_on` 的许可，也不能解决持锁等待自身依赖的死锁。阻塞区域开始后不能通过取消外层任务强行打断；关闭期限也不能杀掉它。请避免在库的普通方法中偷偷创建运行时或同步等待，让调用者明确知道这种桥接限制。

### 本地队列与全局队列

Tokio 会把就绪任务放进工作线程本地队列或全局队列，其他线程可窃取工作，并周期性检查 IO 与计时事件。当前多线程实现还使用局部优化减少任务互相唤醒的延迟。它们解释了为何唤醒不意味着立刻运行，也解释了外部线程投递和本线程唤醒可能有不同开销。

这些队列策略、检查间隔和优化是版本相关的实现细节。不要依赖任务按生成顺序完成、某次yield必定让特定任务先执行，或“唤醒几次就轮询几次”。[运行时详细行为](https://docs.rs/tokio/latest/tokio/runtime/index.html#detailed-runtime-behavior)有明确的保证和非保证；应用正确性应来自消息与同步协议，调优则通过测量决定。

<a id="configuration-options"></a>
## 配置选项

[`Builder`](https://docs.rs/tokio/latest/tokio/runtime/struct.Builder.html)控制运行时的资源配置。多线程调度器的 `worker_threads(n)` 设置工作线程数，当前线程调度器不会因此变成多线程。`max_blocking_threads(n)` 限制额外阻塞线程池，与工作线程数不同；限制它也不是完整的业务背压，排队任务仍可能增长。

`enable_io`、`enable_time` 可分别启用驱动器；`enable_all` 适合普通应用。线程名称、栈大小、启动/停止回调有助于集成和诊断。先使用默认值并观察[工具章](tools.md)的指标，再调整线程数、事件检查间隔和队列选项；更大的线程池可能增加争用、缓存失效和上下文切换，不保证更高吞吐。

单线程适合不需要并行或含线程亲和状态的应用；多线程适合可迁移任务和并行处理。每核一线程的设计通常将状态分片，减少跨核共享，但需要明确跨分片通信和负载倾斜处理。Tokio 默认工作线程并不等于自动固定在指定 CPU 上，也不保证 NUMA 局部性；这些需要专门设计，而不是改一个线程数。

<a id="alternate-runtimes"></a>
## 其他运行时

选择其他运行时的理由可能是生态兼容性、嵌入式环境、GUI事件循环、线程亲和性、操作系统IO模型或更小的依赖范围。比较时先列出要使用的库、平台和资源语义，再比较吞吐基准。

[smol](https://docs.rs/smol/latest/smol/)以多个小型异步crate组成运行时；[Glommio](https://docs.rs/glommio/latest/glommio/)面向Linux的线程本地IO与每核线程设计；[tokio-uring](https://docs.rs/tokio-uring/latest/tokio_uring/)在Linux上提供提交式IO及拥有型缓冲区接口。它们的IO trait、任务句柄、定时器和取消语义并不完全相同。纯Future组合容易复用，持有特定运行时资源的库则必须核查适配层和生命周期，不能只把main宏换掉。

为了学习，可以实现只轮询、入队和唤醒的[小型执行器](../02_execution/04_executor.md)。面向生产实现则还需处理就绪登记、丢唤醒、取消与内存安全、阻塞边界、关闭和诊断；它与一个能跑 `ready()` 的循环差别很大。继续理解[执行器与系统IO](../02_execution/05_io.md)，再决定是否真的需要自建完整运行时。

本章于2026-10-23核查Tokio1.54.1及上述运行时官方文档，源码用Rust1.96验证。测试实际覆盖显式启动/仅enter不驱动、LocalSet继续驱动、block_in_place多线程桥接与单线程panic、嵌套block_on拒绝、Handle不保活、shutdown_timeout后阻塞工作仍可完成。其他运行时及Linux线程每核/io_uring仅核查文档，未在Windows上运行。
