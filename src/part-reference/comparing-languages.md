<a id="comparing-async-programming-in-rust-to-other-languages"></a>
# Rust 异步编程与其他语言的比较

上游只有英文目录标题，没有英文正文；标题由 GPT-6 Luna 翻译，正文由 GPT-6.1 Sol 原创补写。

async、await、Task、Future 和 Promise 经常出现在相似的示例里，但相同写法不保证相同启动时机、线程、取消或生命周期。本章比较语言和常用执行环境的语义，帮助读者迁移已有经验，不给语言作性能排名。Rust 例子沿用本书 Rust 1.96/Tokio 1.54.1；Python 引用固定的 3.14 文档，其余引用对应官方规范或平台文档。

## 先比较运行的单位

| 环境 | 创建或调用得到什么 | 谁推进工作 | 放弃等待意味着什么 |
| --- | --- | --- | --- |
| Rust async fn | 惰性的 Future 值 | 外层 Future 或执行器 poll | 丢弃拥有的 Future 停止其后续执行；已发起外部工作另算 |
| Tokio spawn | 已提交独立任务的 JoinHandle | Tokio 执行器 | drop 句柄会分离任务，需要显式停止与 join |
| JavaScript async function | Promise，函数已执行同步前缀 | JavaScript agent 与宿主事件机制 | 丢弃 Promise 引用不等于取消操作 |
| Python async def | 尚未执行的 coroutine 对象 | await，或事件循环驱动的 Task | 取消 Task 是请求注入 CancelledError，需继续等退出 |
| Go go 语句 | 独立执行的 goroutine | Go runtime 调度到 OS 线程 | 没有由丢弃句柄触发的通用取消；用协议协作退出 |
| C# 常规 async Task 方法 | Task，方法已执行到首次未完成 await 或返回 | 调用线程及后续上下文/调度器 | 不再 await 或丢 Task 引用都不会自动发取消请求 |

这张表区分“未启动的计算描述”和“已启动工作的观察句柄”。Rust 中同步函数也可以先发出操作再返回 Future；传入 async fn 的参数表达式在调用时求值，创建参数可能有副作用。只有 async 函数体本身在首次 poll 前不执行，不能据此说“任何返回 Future 的 API 都完全惰性”。相关已测行为见 [Async 与 Await](../part-guide/async-await.md) 和 [并发组合 Future](../part-guide/concurrency-primitives.md)。

## 同时准备两个请求，不一定已经并发

假设先保存两次调用的结果，再逐个 await。JavaScript 常规 async function 和 C# 常规 async Task 方法通常已在调用时开始工作，所以先保存两个结果可以让等待区间重叠。Rust 的两个 async fn Future、Python 的两个裸 coroutine 则尚未因此各自调度；先 await 第一个再 await 第二个，通常仍是顺序推进。

在 Rust 中，可以 join 两个 Future，在同一个任务里交替 poll；或者 spawn 为独立任务，再等待句柄。在 Python 中可用 TaskGroup、create_task 或合适的聚合 API 创建 Task。Go 的普通函数调用仍是同步的，使用 go 语句才启动 goroutine，之后需要另外等待结果。不要仅把语法替换为同名 await，就假定保留了原来的并发度。

还要区分并发与并行。Rust join 在一个任务里并发驱动子 Future，不保证它们占两个核；多个 Tokio 任务能否并行取决于运行时和线程配置。其他环境也有类似限制：异步 I/O 不等于自动为任意 CPU 运算分配线程。

## JavaScript：Promise 和宿主事件循环

JavaScript async function 在调用时执行同步前缀，并返回 Promise；遇到 await 后，后续部分通过异步的 Promise 反应继续。即使 await 的值已经可用，续体也不会简单地在当前栈上像普通函数调用一样立即接着跑。依据见 [MDN async function](https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/async_function) 与 [ECMAScript 的 Promise/async 机制](https://tc39.es/ecma262/multipage/control-abstraction-objects.html)。Rust 则可能在子 Future Ready 时于同一次 poll 继续执行，所以“await 一次就会公平服务其他任务”的直觉不能直接移植。

JavaScript 的运行环境也不只有一条线程：单个 agent 里的 JavaScript 通常按事件处理模型运行，浏览器有 Worker，Node.js 有 worker_threads 和内部 worker pool；它们各自的内存与通信规则不同。把耗时同步计算写进 async function，仍会阻塞执行它的事件循环。Node 的 [避免阻塞事件循环指南](https://nodejs.org/en/learn/asynchronous-work/dont-block-the-event-loop) 和 [worker_threads](https://nodejs.org/api/worker_threads.html) 分别解释了 I/O/线程池与 CPU JavaScript 的安排，不能把“后台 I/O”推成“所有代码自动后台运行”。

Promise.race 选择首先确定的结果，但不会因另一项输了就通用地撤销其底层操作；Promise.all 的拒绝也不自动中止其他操作，见 [Promise 聚合规范](https://tc39.es/ecma262/multipage/control-abstraction-objects.html#sec-promise.race)。支持取消的 API 可接收 AbortSignal，再由 AbortController 请求取消；这是 API 合作协议，并非 Promise 自带的强制终止，见 [DOM 取消机制](https://dom.spec.whatwg.org/#interface-abortcontroller)。因此超时实现需要同时考虑失败分支还在做什么，类似 Rust 中区分丢 Future 与丢 JoinHandle。

错误通常表现为 rejected Promise，await 时可由 try/catch 观察，finally 用于该控制流的清理。对象被垃圾回收不能代替明确关闭连接或取消订阅，也不应依赖最终回收时机来完成异步清理。Rust 所有者销毁有确定的同步 Drop 路径，但 Drop 本身同样不能执行普通 await；两者都需要在应用协议里安排必要的异步关闭。

## Python：coroutine 与 Task 分开

调用 async def 得到 coroutine 对象，不会运行函数体；直接 await 它会推进调用链，create_task 则将它包装成事件循环管理的 Task。不要把 coroutine 对象与 concurrent.futures.Future、asyncio.Future 或已创建的 Task 混成一种启动模型。这里还存在版本例外：3.12 加入 eager task factory；3.14 的 create_task 接收 eager_start，可在创建期间先执行到阻塞点。依据为 [Python 3.14 coroutine 与任务文档](https://docs.python.org/3.14/library/asyncio-task.html#creating-tasks)。

asyncio 通常在一个事件循环线程里合作式运行任务；Task 等待尚未完成的操作时，循环才有机会运行其他工作。同步阻塞调用和长计算仍会拖住循环，需要 to_thread 或 run_in_executor 等桥接。默认带 GIL 的 CPython 中，线程桥接对纯 Python CPU 并行通常有限；释放 GIL 的扩展和可选 free-threaded 构建又有不同边界。不要把这个限制说成 Python 永远不能多核。见 [asyncio 阻塞代码指南](https://docs.python.org/3.14/library/asyncio-dev.html#running-blocking-code) 与 [free-threading 说明](https://docs.python.org/3.14/howto/free-threading-python.html)。取消等待线程结果也不等于能杀死已经运行的线程函数。

Task.cancel 请求在下一个合适机会向 coroutine 抛出 CancelledError，协程因而能在 try/finally 里清理，并在清理后传播取消；也可能捕获并抑制它。Rust 外部销毁 Future 不会向内部抛一个可捕获的取消异常，尚未执行的异步清理也不会因此自动运行。这是迁移 finally 清理逻辑时尤其需要改变的地方。

Python 3.11 起的 TaskGroup 在退出异步上下文时等所有任务；一般子任务异常会触发同组其他任务取消，并按规则汇总异常。gather 的默认异常路径并不等同于这个监督协议。生命周期与异常规则见 [任务取消与 TaskGroup](https://docs.python.org/3.14/library/asyncio-task.html#task-cancellation)。Rust 中 join、try_join、JoinSet 和独立 spawn 的关闭行为也各不相同，要按选择的组合器和所有权分别解释，不能将一种语言的“任务组”语义套在所有 API 上。

## Go：运行时管理的 goroutine

Go 用普通同步风格函数配合 go 语句启动 goroutine，不要求把调用链全部染成 async/await。goroutine 有 runtime 管理的可增长栈，并复用 OS 线程；常见通道/网络等待与受支持的阻塞系统调用会由 runtime 协调，让其他 goroutine 有机会运行。与 Rust 在 poll 内直接执行阻塞代码不同，这些阻塞行为的一部分已由 Go runtime 专门集成；cgo、线程亲和性、外部库与资源上限仍须单独考虑。见 [Go 官方 FAQ](https://go.dev/doc/faq#goroutines)。

Go 从 1.14 起引入异步抢占等调度改进，细节依平台和版本而异；这减少了不能靠函数调用安全点让出的某些饥饿问题，却不是实时调度或强制业务取消承诺。[Go 1.14 runtime 说明](https://go.dev/doc/go1.14#runtime) 描述了这项变化。Rust async 的 poll 则必须由代码合作交还控制，见 [饥饿一章](starvation.md)。

goroutine 不因启动它的函数返回就自动停止，也没有靠 GC 通用地回收一个仍在等待通道的 goroutine。常用 context 传递截止时间与取消信号；操作必须观察 Done/Err，或调用支持 context 的 API，才会协作退出。cancel 不等待工作结束，派生 context 的资源释放与 goroutine 退出确认也要分别处理。见 [context 文档](https://pkg.go.dev/context)。这更接近 Rust 的取消令牌，而非直接 drop 一个拥有状态的 Future。

Go 的 select 选择可进行的通道操作，多个就绪分支按规范进行均匀伪随机选择；它不是任意 Future 的通用轮询器。向已关闭通道发送、由谁关闭通道和何时结束消费者都属于协议设计。普通失败通常通过返回 error 值传播；独立 goroutine 的错误不会自动返回到启动者，需要结果通道或监督安排。panic/recover 也不是跨 goroutine 的普通异常捕获：recover 必须在相应 goroutine 的延迟函数中按规则执行，不能让父 goroutine 的 defer 捕获子 goroutine 的 panic。依据见 [Go select 与 panic/recover 规范](https://go.dev/ref/spec#Select_statements)。

## C#：常规 Task 方法通常已经启动

常规 C# async Task 方法从调用线程开始执行，直到首次尚未完成的 await 才暂停；如果等待对象已完成，也可能直接继续。async 不会自动创建线程，I/O Task 也不等于一个专属线程。续体可能通过捕获的 SynchronizationContext 或 TaskScheduler 恢复，UI、服务器与自定义上下文因此会有不同表现。见 [Microsoft TAP 模型](https://learn.microsoft.com/en-us/dotnet/csharp/asynchronous-programming/task-asynchronous-programming-model)。这与需要首次 poll 才进入 Rust async fn 函数体的行为不同；它不表示所有 Task 构造方式都已经启动。

CPU 密集工作常通过 Task.Run 分配到线程池；有真正异步 API 的 I/O 通常直接 await，不必为了“异步”再套 Task.Run，见 [异步场景指南](https://learn.microsoft.com/en-us/dotnet/csharp/asynchronous-programming/async-scenarios)。同步等待 .Result 或 Wait 会阻塞线程，某些上下文下还可能阻止所需续体运行，在线程池上也可能造成饥饿。ConfigureAwait(false) 改变的是是否继续捕获相应上下文，不会把同步阻塞代码自动变成非阻塞；详细解释见 [.NET ConfigureAwait FAQ](https://devblogs.microsoft.com/dotnet/configureawait-faq/)。Rust 里的相应决策通常是保持 async 调用链，或把阻塞边界移到受控工作池。

.NET 的 CancellationTokenSource/Token 是合作式取消：Cancel 发通知，操作自行检查、停止或抛出取消异常；它不通用地终止运行线程。丢弃 Task 引用不会自动请求取消；Dispose token source 只释放相应资源，不发取消通知，也不能替代退出确认。见 [.NET 取消模型](https://learn.microsoft.com/en-us/dotnet/standard/threading/cancellation-in-managed-threads) 与 [CancellationTokenSource.Dispose](https://learn.microsoft.com/en-us/dotnet/api/system.threading.cancellationtokensource.dispose?view=net-10.0)。

Task 方法的异常保存在返回任务中，在 await 时传播；async void 的异常观察方式不同，通常限于事件处理器，不能将它当成可可靠等待的普通服务 API，见 [异步返回类型](https://learn.microsoft.com/en-us/dotnet/csharp/asynchronous-programming/async-return-types)。Task.WhenAll 等所有输入结束，并反映失败或取消状态，但不会因一项失败就自动向其他项发出取消请求，见 [WhenAll](https://learn.microsoft.com/en-us/dotnet/api/system.threading.tasks.task.whenall?view=net-10.0)。

内存由 GC 管理不代表资源不需关闭。C# 的 using/IDisposable 提供同步资源协议，await using/IAsyncDisposable 可以等待异步清理，见 [DisposeAsync](https://learn.microsoft.com/en-us/dotnet/standard/garbage-collection/implementing-disposeasync)。Rust 稳定版常用显式 finish/shutdown().await 加同步 Drop 后备；从 C# 迁移时，不能把依赖 await using 的关闭直接塞进普通 Drop。

## Rust 的所有权改变了取消和共享方式

在 Rust 中，跨 await 的借用和移动仍受普通所有权规则约束。某个 Future 可以借用局部状态，但把它交给要求 Send + 'static 的多线程 spawn 时，就要证明它可跨线程且不借用短命对象；LocalSet 则允许合适的非 Send 任务。Pin 维护可能依赖地址的状态，Send/Sync 约束线程共享，这几个概念不能互相替代，见 [运行时边界](../part-guide/runtimes.md) 和 [固定](pinning.md)。

GC 环境常允许多个任务持有相同对象，运行时会维持内存可达性，但可变状态仍需同步，也可能发生逻辑竞态、句柄泄漏或业务生命周期失配。Rust 的借用与 trait 约束能在编译期拒绝许多这类错误，却不能证明协议无死锁、公平、无重复提交或没有取消丢数据。相关已测实例见 [通道与同步](../part-guide/sync.md)、[设计模式](design-patterns.md) 与 [取消安全](cancellation.md)。

Rust 的预期错误通常作为 Result 返回，? 短路会销毁当前拥有的局部状态；panic 与业务错误分开。Tokio spawn 的 JoinHandle 还引入一层任务退出结果。父 Future 退出能取消它拥有的未完成子 Future，却不会自动结束所有已 spawn 工作。其他语言的异常聚合、context 和 token 也不等价于自动拥有完整子任务树；明确等待、通知与关闭仍是共同要求。

## 用一个业务场景检查迁移是否正确

把“同时读取两项数据，任一项失败后退出，并在一秒内关闭连接”迁移到另一环境时，应该逐项确认：调用是否已经启动两项工作；失败时另一项还在不在运行；取消是否只是通知；连接清理是否需要 await；最后由谁确认没有后台工作继续访问数据。只验证正常情况下拿到两个结果，会遗漏最重要的边界。

背压也必须明确：无论创建的是 Future、Promise、Task 还是 goroutine，一次性产生百万个等待者都可能积累内存、连接与调度成本。限制队列和在途工作、保留错误信息、为停止协议留容量，比按语言名称猜测性能更可靠。吞吐量与尾延迟取决于实际库、运行时配置、分配、I/O、CPU 工作和负载，需要相同工作量的测量，本文不作“Rust 必然更快”之类的绝对结论。

## 本章验证边界

本章核查了 ECMAScript/WHATWG、Mozilla、Node.js、固定版本 Python 文档、Go 规范与 runtime/context、Microsoft C#/.NET 官方资料。Rust 说明交叉引用本书已有的可运行源码与已测行为，本章没有增加仅验证文字比较的空泛测试。

未安装或运行 JavaScript、Python asyncio、Go、C# 的比较程序，没有测跨语言性能，也没有把不同平台/版本的全部调度策略视为一致。Python TaskGroup/eager 行为明确注明引入版本，浏览器与 Node 宿主、C# 同步上下文、Go runtime 和 Rust 执行器的差异都应按目标项目实际环境再次核对。
