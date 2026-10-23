<a id="async-in-sync-sync-in-async"></a>
# 同步中的异步，异步中的同步

上游只有英文目录标题，没有英文正文；标题由 GPT-6 Luna 翻译，正文由 GPT-6.1 Sol 原创补写。同步程序可以把异步限制在某个子系统，异步程序也常要调用数据库驱动、压缩库和 CPU 密集算法。桥接的关键不是到处加入 block_on，而是确定谁允许阻塞、谁持续驱动事件、以及关闭时谁先退出。

## 先确定桥接发生在哪一侧

| 调用方 | 目标工作 | 常见入口 |
| --- | --- | --- |
| 普通同步线程 | 一段需要完成后返回的异步操作 | 复用 Runtime::block_on |
| 同步 GUI/宿主 | 持续运行的异步子系统 | 专用运行时线程，加请求/响应通道 |
| 异步任务 | 有界且最终会返回的阻塞调用 | spawn_blocking，并限制并发 |
| 异步任务 | 长期驻留的同步循环 | 专用 OS 线程与关闭协议 |
| 异步任务 | 大量可并行 CPU 计算 | 有限 Rayon 池与异步结果通道 |
| Tokio 多线程任务 | 必须在当前栈执行的同步代码 | 审慎使用 block_in_place |

“调用函数不是 async”并不必然表示会阻塞；oneshot 的 send 等短内存操作可以直接调用。反过来，一个声明为 async 的包装器如果内部直接读阻塞文件或长时间计算，也不会自动让出线程。

## 在同步世界拥有运行时

[Tokio 桥接指南](https://tokio.rs/tokio/topics/bridging) 展示了保存 Runtime 并用同步方法调用 block_on 的模式。运行时应属于应用或长期存活的客户端对象，避免每次请求都创建线程池、驱动器和清理上下文。异步资源必须在相应运行时仍可驱动时使用。

current_thread Runtime 适合一次只做一件事的同步客户端，但只有 Runtime::block_on 执行期间才推进其任务和 I/O/时间；方法返回后，后台任务会暂停。若订阅、心跳或连接管理必须在同步调用之间继续运行，应持续驱动专用线程，或使用生命周期明确的多线程 runtime。

`Runtime::block_on(async { sleep(...).await })` 把资源构造也放在运行时上下文内。写成 `runtime.block_on(tokio::time::sleep(...))` 会先在调用方构造 Sleep，缺少时间上下文时可能 panic。`enter()` 只是设置当前上下文，不等于驱动 runtime。

根 Future 由调用 block_on 的线程执行，并不因此自动迁移到 worker。同步 GUI 主线程如果直接 block_on 网络请求，界面仍会停顿；应该把结果通过 GUI 的线程安全事件入口送回主线程。

## 句柄不是第二个运行时

[`Handle`](https://docs.rs/tokio/latest/tokio/runtime/struct.Handle.html) 可以跨线程克隆并 spawn，适合把执行入口交给同步调用方。Handle 本身不阻止 Runtime 被销毁，应用必须另外持有 Runtime。`try_current` 可检查当前上下文，但“有上下文”不能证明当前阻塞是安全的。

`Handle::block_on` 可以由合适的同步线程调用；在普通异步任务内部直接调用会因嵌套阻塞执行而 panic。current_thread runtime 上，Handle::block_on 不负责驱动 I/O 和时间，除非另一个线程仍在 Runtime::block_on，否则等待定时器可能无法推进。不要把 `Handle::current().block_on` 当成随处可用的“把 async 转 sync”函数。

同样，换成 futures::executor::block_on 并不会修复依赖关系：即使它没有立即 panic，当前线程被阻塞后，所等待的 Tokio 子任务仍可能没有执行机会。解决方法是把桥接移到线程边界，或在上层继续传播 async。

## 专用运行时线程与请求/响应

下面的服务在单独 OS 线程持续运行 current_thread Runtime，用容量为 1 的 mpsc 传请求，用 oneshot 返回结果。同步调用者调用 blocking_send/blocking_recv，异步调用者调用 send/await，两边共用同一服务：

```rust
{{#include ../../examples/guide-tests/src/bridging.rs:service}}
```

这是教学 API：错误用简化字符串表示，输出采用 saturating_mul 避免溢出；真实应用应保留业务错误、运行时启动错误和线程 panic，并决定断线是否重试。调用端取消等待只会让回复失败，服务可能已经处理请求；示例明确忽略这种“调用端不再接收”的发送错误，业务副作用不会回滚。

shutdown 消耗整个 Service，先丢唯一发送端，接收循环排空已收到的消息并遇到 None，再 join 线程。它不能在异步 worker 内直接执行，因为 join 阻塞当前线程；必要时在 spawn_blocking 中调用，或给宿主提供异步 join 包装。默认 Drop 不执行这个显式协议：字段销毁会关闭发送端，而线程句柄被丢弃只会分离，不能当作完成确认。

这个最小接口不向外暴露发送端克隆，避免 shutdown 后仍有隐藏发送端使接收循环永不结束。如果扩展为可克隆客户端，必须增加明确的关闭消息/接收端 close，停止新请求并处理尚存客户端，不能只依赖“某个发送端已经 drop”。若服务需要异步资源清理，应在退出循环后 await 清理再让线程返回。

## 背压决定系统会不会积累失控

有界 [`mpsc::Sender`](https://docs.rs/tokio/latest/tokio/sync/mpsc/struct.Sender.html) 满时，send().await 暂停异步任务，blocking_send 阻塞同步线程，try_send 返回 Full。blocking_send 在异步上下文会 panic；把函数名改成同步不改变所处线程的约束。try_send 适合允许拒绝、合并或重试的业务，不应通过空转重试来绕过背压。

[`Receiver::close`](https://docs.rs/tokio/latest/tokio/sync/mpsc/struct.Receiver.html) 拒绝新的普通发送，但已有队列消息仍可排空；此前获得的容量许可也必须归还或完成，接收端才会最终结束。关闭协议要考虑在途工作，而不只是当前缓冲区长度。

典型死锁是同步线程塞满请求队列后等待更多容量，服务却同步等待该线程处理回调；或者调用方持着服务需要的锁等待响应。另一个常见环是先 join 生产线程、再排空它已经填满的结果队列。应在等待期间继续驱动对方需要的消费者，或先明确关闭通道使生产者从阻塞发送中退出。

一个 send 成功只代表通道接收了值，不代表请求已处理。需要严格确认时要 await reply，必要时加入请求 ID、超时和幂等。取消发送等待还可能销毁已移入 Future 的消息；需要保留值时先 reserve 容量或在外层持久化待发送记录。

## 从异步侧执行阻塞工作

[`spawn_blocking`](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html) 接受 `FnOnce + Send + 'static` 并把闭包放到允许阻塞的线程上，结果通过 JoinHandle 返回。若闭包本身返回 Result，await 后还有一层 JoinError，分别处理任务失败与业务失败。

线程池默认上限较大，适合阻塞 I/O 的等待，不代表同时启动大量 CPU 工作就合理。下面先获取 Semaphore 许可，再把许可移到真实工作闭包：

```rust
{{#include ../../examples/guide-tests/src/bridging.rs:bounded-blocking}}
```

若等待方被取消，已开始的阻塞闭包仍持有许可，直到工作返回才释放。若把许可留在外层 async 栈上，取消等待会提前归还容量，实际后台计算还在进行，所谓并发限制就失效。尚未开始的工作可能被 abort 阻止启动，但一旦开始，abort 无法强行终止它。

这只是“已提交和运行中的工作”的限流。若先为无限输入 spawn 大量 async 包装任务，它们仍会堆积在许可等待队列里；还应限制入口队列或用有界并发消费。示例在许可关闭时 panic，生产 API 可把 AcquireError 明确返回。

长期循环不宜永久占据阻塞线程池，应使用 std::thread::spawn，加通道、原子停止标记或库自己的可中断机制，并在关闭时确认线程结束。shutdown_timeout 只让调用方停止等待，不会杀死已经运行的闭包。

## CPU 并行与 Rayon

[`Rayon ThreadPool`](https://docs.rs/rayon/latest/rayon/struct.ThreadPool.html) 适合把可拆分计算放到有限工作线程。不要在 Tokio worker 上直接调用同步的 `pool.install(...)` 等待长计算；可以用 pool.spawn 提交，然后通过 [`oneshot`](https://docs.rs/tokio/latest/tokio/sync/oneshot/index.html) 异步接收：

```rust
{{#include ../../examples/guide-tests/src/bridging.rs:rayon}}
```

这里 catch_unwind 把可展开的计算 panic 转为错误，避免 Rayon 的无监督任务 panic 成为进程级问题；panic=abort、进程崩溃或内存耗尽不能由它恢复。等待者离开后，计算仍会继续，发送结果可能失败。若数据可能很大或输入很多，仍需限制提交速率/队列并定义协作取消，而不只是设置 Rayon 线程数。

线程池最好由应用创建并复用。额外创建 Tokio 阻塞池、Rayon 池以及依赖库自己的池，可能过度订阅 CPU；池大小应结合整个进程的实际并行预算，不是每个子系统都使用全部核数。

## block_in_place 的适用范围

[`block_in_place`](https://docs.rs/tokio/latest/tokio/task/fn.block_in_place.html) 在当前栈执行闭包，并通知 Tokio 多线程调度器把其他任务移交给别的 worker。它不要求闭包 Send/'static，适合某些难以搬离当前栈的同步调用；但它会暂停同一任务内 join! 的其他分支，不能像 spawn_blocking 一样让这个任务继续并发推进。

它在 current_thread runtime 内会 panic；在运行时外只是普通调用。多线程上下文中可以在闭包里用 Handle::block_on 重新进入异步代码，但仍不能制造等待自身的依赖环。闭包不能被抢占取消，长时间运行仍影响关闭。本章有多线程测试实际等待计时器后返回，不用这一模式掩盖所有同步接口设计问题。

本章五测试在 Windows 通过：专用线程服务同步/异步调用并关闭；取消外层等待时阻塞工作仍占许可；Rayon计算经oneshot返回；满通道与关闭后的跨线程发送/排空；block_in_place重新进入。新增 Rayon 1.12.0 并锁定依赖；未用真实数据库/GUI/外部服务测试，也未通过故意挂死线程来演示死锁。
