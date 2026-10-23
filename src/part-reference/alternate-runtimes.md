<a id="alternate-runtimes"></a>
# 替代运行时

上游只有英文目录标题，没有英文正文；标题由 GPT-6 Luna 翻译，以下正文由 GPT-6.1 Sol 原创补写。本章比较运行时的能力与约束，帮助判断已有代码能否迁移；项目状态按 2026-10-24 实际打开的官方文档记录，不以旧版教程的流行程度推断维护状态。

## 共同的 Future，不同的环境

Rust 的 Future、Poll、Context 和 Waker 是跨执行器的协议。只使用内存状态、标准库同步和正确唤醒的 Future，可以由不同执行器驱动。上一章的教学接收端就是例子，本章在 futures executor 与 smol 上分别运行并得到相同结果。

这不表示任意 async 函数都与运行时无关。函数内部若创建 Tokio 的 Sleep、调用 tokio::spawn 或操作已绑定 Tokio 驱动器的 socket，就依赖那个运行时的上下文及持续驱动。把外层 `block_on` 换掉不能替这些内部依赖重新接线。创建资源、轮询资源和销毁资源都可能有运行时约束。

应分开看四层：执行器负责可运行任务；事件驱动器把网络和计时事件转为唤醒；库提供通道、文件和协议；应用负责生命周期和关闭。某个库只提供执行器，并不欠缺 Future 支持，但也不因此自动具备操作系统 I/O。

## 选择的主要维度

| 方案 | 调度与驱动特征 | 使用时需要核对 |
| --- | --- | --- |
| futures executor | block_on、LocalPool，或启用功能后的线程池 | 没有自动提供 Tokio 的网络/时间上下文 |
| smol | 多个小型 crate 组合，可显式运行 Executor/LocalExecutor | 任务句柄取消语义、I/O trait、驱动器及线程生命周期 |
| Glommio | Linux io_uring、线程本地和每核线程取向 | 内核/资源限制、任务分配、专用网络和文件 API |
| Monoio | 每线程执行器、租借式 I/O 缓冲模型，多种驱动 | 目标平台、驱动选择及第三方库适配 |
| tokio-uring | 当前线程 Tokio 执行器加独立 io_uring 驱动 | Linux、所有权式缓冲 API、明确关闭 |
| Embassy | 面向嵌入式的静态任务与中断唤醒 | 芯片平台、HAL、静态内存及时间驱动 |

这不是性能排行榜。工作负载不均匀时，每核线程方案可能有空闲核与繁忙核并存；能迁移任务的线程池也会付出队列、同步和缓存成本。应该以实际协议、数据规模、延迟分位、内存和故障恢复衡量收益，而不是比较“async 数量”。

## futures executor：协议验证与嵌入

[`futures::executor`](https://docs.rs/futures/latest/futures/executor/index.html) 提供当前线程 block_on 与支持非 Send 任务的 LocalPool。它适合测试与宿主程序中的小型执行环境。启用 `thread-pool` 功能后还可使用线程池；当前验证 crate 没有为此添加功能。

一个由其他事件源唤醒的 Future 可以在这里工作，但执行器不会凭空创建每个库所需的 reactor。反过来，一个资源若自带后台驱动线程，可能不要求当前执行器提供 I/O。兼容性应逐项查 API 契约，不能以“运行在 futures executor”就断定绝无外部线程。

## smol：组合式运行时与本地执行器

[`smol`](https://docs.rs/smol/latest/smol/) 主要重新导出较小组件：async-executor 调度、async-io 提供 I/O 和计时、async-net 提供网络、blocking 处理阻塞操作。这使应用可以选择组合，但仍要了解谁持续驱动工作。

下面的共享源码在 Windows 实际运行。LocalExecutor 只在创建它的线程执行，所以任务可以持有 Rc<Cell<_>>；根 Future 与子任务在 executor.run 内共同推进：

```rust
{{#include ../../examples/guide-tests/src/alternate.rs:smol-local}}
```

[`LocalExecutor::run`](https://docs.rs/smol/latest/smol/struct.LocalExecutor.html) 本身也是 Future，须被轮询；只构造执行器、调用 spawn，再让线程返回，不会完成这些任务。run 的根 Future 返回也不表示任意后台业务已优雅关闭，应用应保留需要等待的任务句柄并逐个确认。LocalExecutor 的存在也不保证所有工作都是单线程：底层文件、阻塞操作等仍可能交给其他线程。

特别注意 [`smol::Task`](https://docs.rs/smol/latest/smol/struct.Task.html)：**丢弃句柄会取消任务**，调用 detach 才分离；`cancel().await` 请求取消并等待任务结束。这与 Tokio 丢弃 JoinHandle 后任务继续的语义不同。迁移时若沿用“spawn 完忽略返回值”的习惯，可能直接取消刚创建的工作。本章测试实际先轮询到 Pending，再丢句柄并驱动取消，确认其 Future 已析构。

[`async-io`](https://docs.rs/async-io/latest/async_io/) 有后台事件线程作为后备，也能由 block_on 所在线程处理事件；其 Windows 后端使用 polling 提供的 IOCP 支持。不要把 smol 简化为“只用 epoll”，也不要把某个本地执行器等同于整套运行时的线程总数。

## 特定平台和所有权模型

[`Glommio`](https://docs.rs/glommio/latest/glommio/) 的官方文档明确依赖 Linux io_uring，推荐内核 5.8 或更新，提供本地 I/O、定时与多个调度队列。每个线程有自己的执行器，应用可以把线程绑定到 CPU。其优势依赖工作与数据分片设计；跨核共享状态、负载倾斜和长时间计算仍需处理。它不能在 Windows 本机直接运行，本章没有编译或执行它。

[`Monoio 官方仓库`](https://github.com/monoio-rs/monoio) 描述了 io_uring 与 legacy 驱动：Linux 可用 io_uring/epoll，macOS 的 legacy 路径使用 kqueue。仓库仍把 Windows 列为实验开发方向，不能据此宣称 Windows 已稳定支持。租借式读取常把缓冲区交给操作，再返回 `(结果, 缓冲区)`；这影响调用方的内存复用和取消策略，也不直接实现与 Tokio 相同的借用式 AsyncRead。这里仅核查资料，没有在本机编译或验证任何 Monoio 后端。

[`tokio-uring`](https://docs.rs/tokio-uring/latest/tokio_uring/) 不是给普通 Tokio Builder 加一个开关。其 start 同时管理当前线程 Tokio runtime 和 io_uring driver，官方要求 Linux 5.10 或更新。资源 API 把缓冲区所有权交出并在完成时还回；显式 close 用于确定关闭完成。本章未安装或运行它，具体内核完成与内存问题见后文 I/O 模型讨论。

[`Embassy executor`](https://docs.embassy.dev/embassy-executor/) 面向嵌入式：任务可静态分配，不要求堆；空闲时依赖平台睡眠/中断而非忙轮询。桌面服务器的线程池、文件系统和网络栈模型不能直接套到微控制器上。目标芯片 HAL 和中断/时间驱动是运行环境的一部分，本章不提供硬件实测结论。

## 生态与维护状态同样属于兼容性

截至本章核查，[async-std 官方 README](https://github.com/async-rs/async-std) 已明确宣布停止维护并建议迁移 smol。旧书和旧项目中仍会出现 async-std；理解它有助于维护已有系统，但不宜继续把它描述为与 smol 并列的当前维护推荐。停止维护不等于旧二进制立刻无法运行，迁移也需要检查依赖与行为差异。

smol、Glommio、Monoio 和 tokio-uring 的表中能力依据上述官方文档，不把“存在文档”“未看到归档标记”推断成未来维护承诺。投入生产前应重新核对目标版本发布、问题修复、安全通告、支持平台和团队能承担的维护成本。内核最小版本也只是门槛，容器权限、seccomp、资源限制及发行版配置可能进一步限制可用操作。

协议库也会绑定生态：不同 AsyncRead/AsyncWrite trait 即使名称相同也不是同一个 trait。可用适配器连接借用式 I/O 接口，但它不能自动把所有权式 completion API 改为可任意取消的借用接口。[`async-compat`](https://docs.rs/async-compat/latest/async_compat/) 同时涉及 I/O trait 和 Tokio 上下文；其后备运行时和功能选择会引入额外资源。适配器使用前仍须检查所依赖库是否 spawn、是否需要 block_in_place、以及运行时如何退出。本章未新增或运行该适配器。

实际迁移可以先抽出只处理字节、状态和内存通道的核心，再由边界层负责 socket、计时、spawn 和关闭；为不同后端保留清晰入口，不在深层库里偷偷创建进程级 runtime。无须为了抽象而把所有 API 包装成 trait，只有存在明确复用需求时才增加适配层。

本章新增 smol 2.0.2 并锁定依赖，三测试实际验证 Windows 上本地非 Send 任务和计时、句柄取消析构，以及同一独立 Future 在两种执行器运行。没有进行网络性能比较、Linux/macOS 内核 I/O 测试或嵌入式硬件测试。
