<a id="timers-and-signal-handling"></a>
# 定时器与信号处理

计时器和信号都把外部事件转换成任务可以等待的通知：一个表示期限到达，另一个表示操作系统向进程投递了事件。它们都需要运行时配合，但用途与保证不同。等待一个事件不会自动终止任务，收到关闭通知也不会自动完成资源清理。

<a id="time-and-timers"></a>
## 时间与定时器

### 时间类型与运行时

`std::time::Duration` 表示一段时长；[`std::time::Instant`](https://doc.rust-lang.org/std/time/struct.Instant.html)表示用于测量间隔的单调时刻，不是可打印为年月日的墙上时间。日历时间应使用 `SystemTime` 或相应日期库。`Instant` 的平台实现、休眠是否计入经过时间等仍有差异，不应把它当作跨机器可交换的绝对时间戳。

[`tokio::time`](https://docs.rs/tokio/latest/tokio/time/index.html)使用自己的 `Instant` 包装类型，并复用标准库 `Duration`。它可与标准库时刻转换，同时支持测试中的虚拟时钟。普通应用常用 `use tokio::time::{Duration, Instant};`，避免混淆两个 `Instant`。

Tokio 计时器需要 `time` Cargo 功能和启用时间驱动器的运行时。`#[tokio::main]` 的常见配置已启用驱动器；手写 Builder 时调用 `enable_time()` 或 `enable_all()`。在异步任务里调用 `std::thread::sleep` 会阻塞工作线程，不能替代 `tokio::time::sleep(...).await`；长时间计算也会延迟已到期计时器获得轮询。运行时调度不是实时系统，计时器到期不保证业务代码立即执行。

### 一次等待与重置

`sleep(duration)` 返回等待相对时长的 Future，`sleep_until(deadline)` 接受绝对期限。它们不创建独立线程；丢弃计时器取消的是这次等待，不需要额外关闭动作。定时精度受操作系统和驱动器影响，不能用非常短的 sleep 保证另一个任务已经运行。

若循环需要复用同一个 [`Sleep`](https://docs.rs/tokio/latest/tokio/time/struct.Sleep.html)，先固定它，再用 `as_mut().reset(new_deadline)` 改期限。`Sleep` 不是 `Unpin`，在 `select!` 多次借用时不能随意移动。重置既能发生在完成前，也能发生在完成后；不调用 `reset`，再次等待一个已经到期的 Sleep 不会重新等待一整段时长。

总期限应在循环外计算，如 `Instant::now() + budget`，多步操作共享同一个 `deadline`；空闲超时则在每次进展后重新计算。把每步都包在同样相对时长的 timeout 中，总耗时可能超过原来的总预算。两种语义各有用途，见[并发组合章](concurrency-primitives.md)的空闲与总期限示例。

### 周期等待与漏掉的 tick

反复 `sleep(period).await` 会在“处理耗时 + period”之后开始下一轮，因处理耗时累积而漂移。[`interval(period)`](https://docs.rs/tokio/latest/tokio/time/fn.interval.html)按周期安排 tick，**第一次 tick 立即完成**，周期为零会 panic。如果要第一轮也等待，应使用 `interval_at(now + period, period)`，或明确消耗初始 tick。

```rust
{{#include ../../examples/guide-tests/src/time_signals.rs:periodic}}
```

`tick().await` 返回原计划的触发时刻，不一定等于当前执行到这里的时刻。如果处理速度低于周期，就需要选择 [`MissedTickBehavior`](https://docs.rs/tokio/latest/tokio/time/enum.MissedTickBehavior.html)：

| 策略 | 延迟后的行为 | 常见选择 |
| --- | --- | --- |
| `Burst`（默认） | 尽快补发错过的 tick，保持原节拍 | 必须处理每一轮且能承担突发 |
| `Delay` | 以发现延迟时的当前时间重新安排后续周期 | 希望后续工作重新均匀间隔 |
| `Skip` | 跳过过期节拍，后续仍对齐原时间线 | 状态快照、心跳等只需最新工作 |

这些策略不会并行运行处理逻辑，也不会凭空补回没有完成的业务工作。需要每个业务事件必达时，请使用持久队列或日志，而不是把 tick 当成事件记录。漏 tick 检测还存在实现精度阈值，不能拿亚毫秒测试来证明策略差异；本章测试使用十秒周期的虚拟时间。

### 超时包装器与竞争

[`timeout(duration, work)`](https://docs.rs/tokio/latest/tokio/time/fn.timeout.html)返回 `Result<WorkOutput, Elapsed>`。若 work 自身返回 `Result<T, E>`，整体是两层 Result，必须区分业务错误和超时。`timeout_at(deadline, work)` 可用于共享绝对期限；`select!` 或 race 更适合多个不同完成条件、需要保留其他分支状态或执行专门处理的场景。

超时不是抢占。当前 Tokio 在检查计时器之前轮询工作，立即就绪的工作即使 timeout 为零也保证成功；一个不让出的长 poll 可能在期限后才返回成功，运行时无法在中间强行打断它。不要把 timeout 当作 CPU 时间上限或终止任意阻塞代码的机制，也不要依赖临界时刻哪个分支必胜来设计事务语义。

普通拥有型 Future 超时后会被丢弃，已经产生的副作用不会撤销。如果包装的是 `JoinHandle`，丢弃句柄只分离任务，不会取消底层工作。若政策是“超时就请求取消，并确认退出”，可以先借用句柄等待，再显式 abort 和 join：

```rust
{{#include ../../examples/guide-tests/src/time_signals.rs:task-timeout}}
```

这个示例只保留简化错误类别。实际代码可保留 JoinError 和业务错误；取消请求也可能与正常完成竞争。对于已开始的 `spawn_blocking`，abort 无法保证停止，继续等待句柄可能超过期限，所以该辅助函数只适用于能够正常响应取消的异步任务。`timeout` 借用一个普通 Future 则允许调用方保留其状态，但后续重试是否合法仍取决于操作的取消安全性。

### 可控制的时间测试

真实等待几分钟既慢又难稳定复现。Tokio 的 `test-util` 功能允许 `#[tokio::test(start_paused = true)]` 在当前线程运行时中暂停时钟，用 `advance` 推进时间；运行时空闲时也会自动推进到下一个计时器。此功能影响 Tokio 的时钟，不会暂停 `std::time::Instant` 或真实网络。[pause 文档](https://docs.rs/tokio/latest/tokio/time/fn.pause.html)说明了上下文和自动推进限制。

测试应验证期限、消息或状态转变，不能只检查 sleep 调用了几次。暂停时钟也不是暂停调度器：仍要轮询任务或等待明确的握手，`advance` 返回并不表示每个到期任务的业务处理已经结束。本章共享源码测试实际验证重置、三种漏 tick 策略、零期限、借用句柄超时后继续执行，以及显式取消后的析构完成。

<a id="signal-handling"></a>
## 信号处理

### 把操作系统通知接入异步控制流

Unix 信号和 Windows 控制台事件可表达中断、终止请求或重载配置。应用通常需要在收到通知后协调多个任务，等待 IO 清理，因此异步程序希望把它们作为 Future 等待。[Tokio signal 模块](https://docs.rs/tokio/latest/tokio/signal/index.html)负责平台相关的接入，应用在正常异步上下文中处理通知，不应在原始操作系统信号处理器里直接分配复杂状态、取普通锁或执行异步代码。

跨平台的 `tokio::signal::ctrl_c().await` 返回 `io::Result<()>`，Unix 和 Windows 均支持。监听注册可能失败，应处理错误；`ctrl_c()` 的 Future 要到首次被轮询时才注册监听，单纯创建后放在一旁不保证已开始接收。库一般应让最终应用决定是否接管进程级信号。

### Unix

Unix 用 `tokio::signal::unix::{signal, SignalKind}` 选择信号。服务常同时关注 SIGINT 和 SIGTERM；SIGHUP 是否表示重载是应用约定，不是 Tokio 自动实现的业务行为。

```rust,no_run
{{#include ../../examples/guide-tests/src/time_signals.rs:unix-signals}}
```

[`Signal`](https://docs.rs/tokio/latest/tokio/signal/unix/struct.Signal.html)可能合并多次通知，一条接收事件表示至少收到一次，不能用来可靠计数每次信号。`recv()` 在 `select!` 中是取消安全的，但系统与库的合并机制本来就可能把多次投递折叠。当前该接口虽返回 `Option<()>`，文档明确它实际上不会返回 `None`，不要把这个签名误读成常规消息通道的关闭协议。

[ctrl_c 的注意事项](https://docs.rs/tokio/latest/tokio/signal/fn.ctrl_c.html#caveats)还指出：Unix 首次为某类信号安装处理器后，会替换进程的默认行为，丢弃监听对象不会恢复默认行为。监听 SIGINT 后再“忘记等待”，并不等于下次Ctrl+C自动恢复终止进程。[SIGKILL 和 SIGSTOP 不能被捕获或忽略](https://docs.rs/signal-hook-registry/latest/signal_hook_registry/fn.register.html#panics)；无论清理代码写得多完整，都不能保证强制终止时执行。

### Windows

[Windows 模块](https://docs.rs/tokio/latest/tokio/signal/windows/index.html)提供 Ctrl+C、Ctrl+Break、关闭控制台、注销和系统关闭等监听类型，基于 `SetConsoleCtrlHandler`，不是 Unix 信号编号的翻译。下面展示控制台程序接收 Ctrl+C 或 Ctrl+Break：

```rust,no_run
{{#include ../../examples/guide-tests/src/time_signals.rs:windows-signals}}
```

关闭控制台的 [`ctrl_close`](https://docs.rs/tokio/latest/tokio/signal/windows/fn.ctrl_close.html)等事件有额外生命周期限制；操作系统可在清理窗口结束后终止进程，不能假定所有异步任务都有无限时间完成。[Microsoft HandlerRoutine 文档](https://learn.microsoft.com/en-us/windows/console/handlerroutine)还说明不同事件、服务与交互式应用的差异：普通交互程序不能假定注销或系统关闭通知必定像Ctrl+C一样送达。

[Windows 服务应采用服务控制管理器的停止/关闭协议](https://learn.microsoft.com/en-us/windows/win32/services/service-control-handler-function)；GUI 应用还要考虑窗口消息及其宿主行为。无控制台、调试器附加、终端关闭与用户按键也可能有不同表现。`cfg(unix)`、`cfg(windows)` 应隔离平台接口，不能在Windows代码里直接导入 `tokio::signal::unix`。

### 通知、清理与测试边界

收到信号后，由控制任务转换成应用内关闭状态，再按[析构与清理章](dtors.md)停止接收新工作、通知工作任务并等待退出。信号处理成功只表示收到了通知，不表示清理成功；监听失败时也应决定是报告错误并退出，还是使用另一个关闭入口。

下面把外部触发器作为参数注入，便于用普通 Future 测试传播逻辑：

```rust
{{#include ../../examples/guide-tests/src/time_signals.rs:signal-routing}}
```

Unix 应用可传入 `unix_shutdown_signal()`，Windows 控制台应用可传入 `windows_console_signal()`；测试则传入立即成功或失败的 Future，验证关闭状态与错误路径，不必向测试会话发送Ctrl+C。需要端到端验证时，应在受控的独立子进程和对应操作系统里投递事件，再检查进程退出与数据完整性。

本章于2026-10-23查阅上述官方时间和信号文档，Rust1.96/Tokio1.54.1的6个行为测试真实通过。Windows信号函数在本机编译，但未注册监听、未发送真实控制台事件；Unix分支受cfg排除，仅核查官方API与源码，未在本机编译或运行。没有把注入触发器测试算成操作系统信号端到端测试。
