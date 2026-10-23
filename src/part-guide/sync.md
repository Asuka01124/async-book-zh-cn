<a id="channels-locking-and-synchronization"></a>
# 通道、锁与同步

任务需要交换工作、共享状态或限制并发。阻塞式同步原语让**线程**等待；异步原语登记等待者，返回 `Pending`，让执行器运行其他任务。在单线程运行时里阻塞等待另一个任务，很容易连那个任务也一起堵住。

并非所有 `std::sync` 都要替换：保护少量内存、临界区短且不跨 `.await` 时，标准库互斥锁通常很合适。需要等待消息或跨异步 IO 持锁时，才选择对应异步原语。下面使用 Tokio；[同步模块的兼容性说明](https://docs.rs/tokio/latest/tokio/sync/index.html#runtime-compatibility)指出这些原语通常不绑定某个 Tokio 运行时，也能跨运行时使用。但计时操作仍依赖 Tokio 时间驱动器；不要把这种兼容性推广到所有 IO 类型。

<a id="channels"></a>
## 通道

通道转移消息的所有权，既能连接同一线程中的任务，也能连接不同线程；跨线程时仍须满足元素的 `Send` 等约束。异步接收不会像标准库的阻塞接收那样占住线程。并非所有方法都是异步的：一次性通道的发送就是同步操作。

| 需求 | Tokio 类型 | 边界 |
| --- | --- | --- |
| 一次请求对应一次答复 | `oneshot` | 任一端提前丢弃，另一端可观察到失败 |
| 多生产者把工作交给一个消费者 | `mpsc` | 可克隆发送端，只有一个接收端 |
| 每个订阅者都需要事件 | `broadcast` | 慢消费者可能收到 `Lagged`，须补数据或退出 |
| 只关心最新状态，如配置 | `watch` | 中间版本可能被覆盖，不保证逐条处理 |

`oneshot::Sender::send` 消耗发送端，可从同步代码调用。接收端 `.await` 得到 `Result<T, RecvError>`；发送者消失不等于一个正常业务值。如果答复就是任务的最终返回值，直接等待 `JoinHandle` 常更简单。

有界 `mpsc::channel(capacity)` 在队列满时让 `send().await` 等待，形成背压。容量限制排队消息数，不限制总字节数，也不限制已取出后正在处理的工作。无界通道的发送不等容量，消费者跟不上时会持续占用内存。容量应结合消息大小、处理速度和排队延迟选择。

下面的任务独占状态，通过一次性通道答复总数，不需要互斥锁。示例用 `unwrap` 让意外关闭立即失败；应用应明确关闭和重试策略。

本章片段使用 `use std::sync::Arc;` 和 `use tokio::sync::{mpsc, oneshot, Semaphore};`，依赖 Tokio 的 `sync`、`rt` 与 `macros` 功能；完整验证 crate 已启用这些功能。

```rust
{{#include ../../examples/guide-tests/src/sync.rs:actor}}
```

丢弃所有强发送端后，`recv()` 取完队列再返回 `None`。接收端也可 `close()` 拒绝新发送，然后排空已接受的消息；已有容量许可可能仍允许发送。忘记丢弃自己保存的发送端副本，是接收循环不退出的常见原因。

取消发送不一定保留原值：在 `select!` 中取消 `send(value)` 会丢弃 Future 持有的 `value`。需要保留消息时，先等 `reserve()`，拿到许可再同步 `permit.send(value)`；等待期间消息仍归调用者。取消容量等待会失去排队位置。`recv()` 则是取消安全的：竞争失败不会取走消息。见[发送端文档](https://docs.rs/tokio/latest/tokio/sync/mpsc/struct.Sender.html)。

`broadcast` 的丢失检测与 `watch` 的覆盖语义不能互换。`watch` 循环通常在 `changed().await` 后用 `borrow_and_update()` 读取并标为已看过；借用守卫应在下一次 `.await` 前释放。[broadcast](https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html)和 [watch](https://docs.rs/tokio/latest/tokio/sync/watch/index.html)文档介绍了慢消费者与关闭协议。

<a id="locks"></a>
## 锁

### 先决定临界区

只保护普通数据时，把同步访问封装进普通方法，在返回后再执行异步操作：

```rust
{{#include ../../examples/guide-tests/src/sync.rs:short-lock}}
```

`std::sync::Mutex::lock()` 仍会阻塞线程，临界区须短，不包含阻塞 IO、昂贵计算或 `.await`。高竞争时也应重评设计。`try_lock()` 适合“拿不到就走另一条路径”，反复调用它会忙等；插入 `yield_now()` 也不会自动成为正确等待协议。[Clippy 的 await_holding_lock](https://rust-lang.github.io/rust-clippy/master/index.html#await_holding_lock)能帮助发现问题。局部任务能否编译通过，不是没有死锁的证明。

### 异步互斥锁

`tokio::sync::Mutex::lock().await` 在锁不可用时挂起任务。守卫借用锁，设计上可跨 `.await` 持有，并在被保护值满足相应约束时实现 `Send`。“等待锁不阻塞线程”和“可以跨等待点持有守卫”是不同性质；让出执行权不会自动解锁。额外管理有成本，所以只有确有需要时才使用。

共享一个必须串行使用的连接，可能确实要持锁完成请求和响应。但 A 持锁等待 B 的答复，而 B 必须先取得同一把锁才能答复，仍会死锁。这可能只冻结几个任务，进程和其他任务仍在运行，CPU 占用也很低。缩短持锁范围、统一多锁获取顺序、避免持锁等待外部参与者，或使用资源管理任务，都比盲目添加超时更可靠。超时不能撤销已修改状态或已发请求。

Tokio 互斥锁公平排队；取消获取会失去位置。守卫在正常离开作用域或 panic 展开时释放。Tokio **没有锁中毒机制**：panic 后下一位仍可取得锁，业务不变量须自行维护。先计算完整新状态再替换，通常比持锁分阶段修改更容易恢复。详见 [Mutex 文档](https://docs.rs/tokio/latest/tokio/sync/struct.Mutex.html)。

`Arc<Mutex<T>>::lock_owned().await` 返回持有 `Arc` 的拥有型守卫，不借用局部锁变量，可用于跨任务转移；可发送性仍取决于类型约束。`blocking_lock()` 面向同步线程，例如 `spawn_blocking` 闭包；在异步执行上下文直接调用会 panic。

### 读写锁、信号量与让出执行权

[`RwLock`](https://docs.rs/tokio/latest/tokio/sync/struct.RwLock.html)允许多个读者或一个写者。Tokio 公平且偏向等待写者，避免新读者无限插队。不要持读锁等待写锁升级；写者已排队时，持一个读守卫等待第二个读锁也可能自我死锁。读多并不自动保证读写锁更快。

[`Semaphore`](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html)用许可数限制并发，许可销毁时自动归还。在入口先拿拥有型许可再 `spawn`，还能避免生成海量等待许可的任务。`acquire_many` 也参与公平队列，大请求可能挡住小请求。`close()` 使等待者和后续获取失败，已发许可仍由持有者释放。

```rust
{{#include ../../examples/guide-tests/src/sync.rs:limited}}
```

`yield_now().await` 只提供一次调度机会，不能证明其他任务已完成，也不释放守卫。等待条件请用同步协议；CPU 工作则要有合适的线程池和并发上限。

<a id="other-synchronization-primitives"></a>
## 其他同步原语

[`Notify`](https://docs.rs/tokio/latest/tokio/sync/struct.Notify.html)不带载荷。`notify_one()` 最多保存一个未消耗许可，多次通知可能合并；它不是事件计数器。`notify_waiters()` 唤醒已注册等待者，不为未来保存许可。真实条件应存在受保护状态中，等待者循环检查条件；实现多消费者队列还须在检查前正确注册等待者，见 `Notified::enable` 的官方示例，否则可能丢唤醒。直接用通道往往更容易正确。

[`Barrier`](https://docs.rs/tokio/latest/tokio/sync/struct.Barrier.html)让固定数量的任务会合，并可重复使用。参与者提前退出会让其他任务一直等。`wait()` 不是取消安全的，不能随意放进可能输掉的竞争。参与者动态变化时，显式管理完成消息通常更合适。

[`OnceCell`](https://docs.rs/tokio/latest/tokio/sync/struct.OnceCell.html)协调异步初始化一次，之后共享结果。初始化被取消、panic 或 `get_or_try_init` 返回错误，不会留下成功值，之后可重试。初始化函数递归等待同一单元会死锁。缓存的值若依赖运行时，仍须考虑运行时关闭。

原子操作无需异步版本：单次操作不等执行器，可用于计数或标志。独立统计计数常可用 `Ordering::Relaxed`；通过标志发布其他内存则需要正确的 acquire/release 等协议。原子变量本身不提供唤醒，循环检查仍可能忙等。复杂状态优先使用成熟同步原语，并用下一章的工具验证协议。

本章 API 于 2026-10-23 按上述官方链接核查，测试锁定 Tokio 1.54.1；额外行为测试覆盖背压与取消、广播落后、状态覆盖、跨等待持锁、取消归还许可、通知合并和取消初始化后重试。
