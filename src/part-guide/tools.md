<a id="tools-for-async-programming"></a>
# 异步编程工具

异步任务不像线程那样始终拥有一条独立调用栈。一个线程先轮询请求 A，A 等 IO 时又去运行 B；之后 A 还可能换到另一个工作线程。因此线程号不足以表示请求身份，普通栈也不包含所有尚未完成的逻辑调用链。排查时需要结合任务状态、因果日志、运行时指标和操作系统工具。

先把问题变成可观测条件：哪个请求超时，最后完成了哪一步，等待的是容量、锁、IO 还是另一个任务？给请求和工作项分配稳定的标识，记录进入、完成、取消和错误；再用可重复负载缩小范围。只增加几处 `println!` 有时足够，但高并发中同步输出也可能改变时序和吞吐。

<a id="monitoring"></a>
## 监控

[Tokio console](https://github.com/tokio-rs/console)把任务和异步资源展示出来，适合查找长时间没有进展的任务、长时间运行的 `poll`，以及唤醒后迟迟未获调度的工作。它由应用内的采集端和单独的客户端组成；不是把任意未插桩进程附加进去就能看见任务。

按 [console-subscriber 的配置说明](https://docs.rs/console-subscriber/latest/console_subscriber/)，应用需要启用 Tokio 的 `tracing` 功能、添加 `console-subscriber`，并在编译应用及其 Tokio 依赖时启用 `--cfg tokio_unstable`。例如在已有 Cargo 项目中配置：

```toml
[dependencies]
tokio = { version = "1", features = ["full", "tracing"] }
console-subscriber = "0.5"
```

在运行时启动并生成业务任务前调用 `console_subscriber::init()`。已配置自己的 subscriber 时，改用它的 layer 与原有层组合，避免重复设置全局 subscriber。PowerShell 中可在当前会话构建运行：

```powershell
$env:RUSTFLAGS='--cfg tokio_unstable'
cargo run
# 在另一个终端启动已安装的客户端：
tokio-console
```

这里的 cfg 是 Tokio 的实验 API 开关，不等于必须使用 nightly Rust；它可能随版本变化。编译标志必须覆盖 Tokio 本身，只给最终二进制设置不够。默认连接本机采集端；远程连接及采集开销要按部署环境评估。未被插桩的资源也不会凭空出现，例如某些 `const_new` 创建的同步原语。Console 显示的等待时间并不能单独证明死锁：一个消费者正常等下一条消息也可以等待很久。

<a id="tracing-and-logging"></a>
## 追踪与日志

[`tracing`](https://github.com/tokio-rs/tracing)用 event 记录某个时刻发生的事，用 span 描述一段相关工作。结构化字段如请求 ID、队列名和错误种类比拼接字符串更方便过滤和聚合。[crate 文档](https://docs.rs/tracing/latest/tracing/)区分了发出事件的库与收集事件的 subscriber：库应发事件，应用决定输出格式、过滤和采集目的地。

不要让 `span.enter()` 返回的守卫跨 `.await` 存活。任务挂起时线程可能执行别的任务，仍然处于该 span 的线程上下文会把无关事件错误归属到它。使用 `#[tracing::instrument]` 标注异步函数，或用 [`Instrument`](https://docs.rs/tracing/latest/tracing/trait.Instrument.html)包装 Future，使每次轮询进入、返回时退出 span：

```rust
{{#include ../../examples/guide-tests/src/tools.rs:instrument}}
```

使用这个片段还须配置 subscriber，例如应用中的 `tracing_subscriber::fmt::init()`；验证 crate 的测试使用 registry 检查上下文。`spawn` 一个新任务不应被当成自动继承所有当前上下文的承诺；可以把 Future `.in_current_span()` 后再生成，或为子任务显式创建带父关系的 span。对仅同步执行的小块代码，`span.in_scope(|| ...)` 是合适的选择。

注意区分 span 的存活时间和实际被轮询的忙碌时间。一个 span 存活五秒，可能大部分时间在等待 IO。不要记录密码、令牌或整个大请求体；`#[instrument(skip(...))]` 可跳过不适合记录的参数。大流量路径应设定过滤级别和采样策略，并测量记录本身的开销。

<a id="debugging"></a>
## 调试

### 理解回溯和断点

`RUST_BACKTRACE=1` 能帮助查看 panic 时的线程栈；PowerShell 中先设置 `$env:RUST_BACKTRACE='1'` 再运行程序。要更完整的符号，保留调试信息，必要时使用 `RUST_BACKTRACE=full`。[`std::backtrace`](https://doc.rust-lang.org/std/backtrace/index.html)捕获的是操作系统线程栈，优化、内联和平台符号支持也会影响内容；它不是所有挂起任务的逻辑栈列表。

断在 `poll` 内可能看见生成的状态机、执行器包装层和当前分支，已经挂起的父子工作未必形成传统连续栈。异步函数入口断点也可能多次命中。结合源文件行断点、请求 ID 和任务生成点，通常比只看函数名有效。

Windows 可用 [WinDbg](https://learn.microsoft.com/en-us/windows-hardware/drivers/debugger/)检查线程、内存和崩溃转储；MSVC 构建保留匹配的二进制及 PDB。其他平台可用相应的 GDB 或 LLDB 支持。不要假定任一原生调试器会自动重建 Tokio 任务图。暂停某条线程也可能阻止它本来要唤醒的任务，单步时的等待不一定是生产问题。

### 从停滞症状回到协议

若 CPU 很高，找长 `poll`、忙等和同步计算；若 CPU 很低，列出谁持锁、谁等消息、生产者是否仍活着。Console 中长期处于等待状态的任务是线索，最后一条 span 事件说明它在哪里停下。检查是否忘记释放发送端、在持锁时等待依赖同一锁的任务，或丢弃了本来应等待的任务句柄。

为了复现，先减到 `current_thread` 运行时，显式安排消息和同步点；用可控时间验证超时，用容量为 1 的通道放大背压。减少线程只帮助理解某些交错，不能证明多线程正确。给测试加等待上限可防止测试进程永远挂起，但不能用任意 `sleep` 代替真正的“准备就绪”握手。

### 用 Loom 验证底层并发协议

[Loom](https://docs.rs/loom/latest/loom/)通过替换线程、原子和同步类型，探索小模型的并发交错。它适合自写锁、原子状态机或唤醒协议，不是把整个网络服务放进一个函数就能穷举的工具。模型必须使用 Loom 能观察的类型；真实 socket、未替换的标准库原语和外部系统不在模型覆盖内。线程数和状态空间越大，成本增长越快。

下面测试两个并发增量不会丢失。将每次 `fetch_add` 换成分离的 `load` 和 `store`，即使用 `SeqCst`，两次操作之间仍允许插入另一个线程，Loom 就能找到最终只有 1 的执行路径。验证源码同时保留这个预期失败的反例，确保工具确实能发现错误。

```rust
{{#include ../../examples/guide-tests/src/tools.rs:loom}}
```

它是普通 `#[test]`，依赖 `loom = "0.7"`；不需要在 Tokio 运行时里启动。大型项目可通过 cfg 为真实实现切换 Loom 类型，并建立专用模型测试，具体按官方指南处理 API 差异。模型通过只说明所建模型和探索限制内未找到反例，不等于证明未建模的业务、IO 和生命周期全部正确。

<a id="profiling"></a>
## 性能剖析

### CPU 火焰图不能直接解释请求延迟

CPU 采样图回答“线程执行时在哪些栈上花了 CPU”。异步等待 IO 时线程执行别的工作或休眠，因此一个延迟很高的请求可能几乎不出现在热点图里；相反，执行器层和 Future 生成代码可能占据大量栈。不能把火焰图中的宽度直接理解成每个请求的端到端延迟。

使用代表性负载，分别测吞吐、分位延迟、并发任务数、队列深度和 CPU。发布配置可保留符号：

```toml
[profile.release]
debug = true
```

[cargo-flamegraph](https://github.com/flamegraph-rs/flamegraph)可生成采样图，Linux 使用 perf，macOS 使用 xctrace，当前也提供 Windows 后端。安装与权限依平台而定。先用 CPU 图找计算和阻塞系统调用，再结合 span 时间、服务端和客户端指标寻找 IO 等待；网络抓包及操作系统 IO 工具还能帮助区分网络延迟、对端慢与本地排队。不要只在无负载环境观察运行时自旋就下结论。

### 观察运行时

稳定的 [`RuntimeMetrics`](https://docs.rs/tokio/latest/tokio/runtime/struct.RuntimeMetrics.html)提供部分指标，其他指标仍要求 `tokio_unstable`。下面不依赖实验 cfg：

```rust
{{#include ../../examples/guide-tests/src/tools.rs:metrics}}
```

活跃任务数增长而吞吐不变，可能表示入站速度超过处理能力；队列持续增长可能意味着工作线程跟不上；长 `poll` 可能妨碍其他任务及时运行。但指标是并发快照，某些值不具备强一致性，不能拿来判断“现在可以安全销毁所有资源”。比较一段时间内的趋势，并与业务负载一起解释。

[`tokio-metrics`](https://docs.rs/tokio-metrics/latest/tokio_metrics/)还能包装任务并汇总轮询、空闲、调度等时间，适合导出监控；运行时监控的部分能力需要额外功能和实验 cfg。先确认所用版本的 API，再选需要的指标。插桩、Console、日志和采样器都会影响程序，应比较启用前后的开销。

本章于 2026-10-23 核查上述官方项目文档。验证 crate 真实执行 span 在挂起期间退出上下文、Loom 正反模型和稳定运行时指标测试；没有声称运行 Console 客户端、WinDbg、Linux perf 或采集生产火焰图。它们的操作说明和平台要求来自文档核查。
