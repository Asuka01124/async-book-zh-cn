<a id="async-and-ffi"></a>
# 异步与 FFI

上游只有英文目录标题，没有英文正文；标题由 GPT-6 Luna 翻译，正文由 GPT-6.1 Sol 原创补写。

FFI 将 Rust 与 C、C++ 包装层及其他宿主连接起来，但不会替两侧统一执行器、内存所有权与取消协议。异步边界的关键不是给外部函数加 async，而是确定调用何时返回、谁继续推进工作、回调可能在哪个线程发生，以及何时才能安全释放上下文。

## 先分清边界方向

| 场景 | 常见安排 | 必须明确的责任 |
| --- | --- | --- |
| Rust 调用阻塞 C 函数 | 有界 spawn_blocking 或专用线程 | 参数/输出保活、线程安全与无法强制终止的工作 |
| Rust 调用外部异步 API | 注册回调，回调通过通道通知 Rust Future | userdata、回调并发、取消/注销确认 |
| C 宿主调用 Rust 异步服务 | C ABI 的 start/cancel/release 等句柄接口 | 运行时寿命、结果回调与句柄状态机 |
| 外部事件循环驱动 Rust | 明确的驱动/唤醒适配层 | 谁 poll、唤醒如何转投、避免重入与阻塞 |

对于阻塞外部调用，函数虽在 C 中实现，也不能直接占用异步 worker。把输入的拥有者连同工作移入阻塞闭包，外层取消后闭包仍持有它们；许可也应跟随真实工作结束。库若要求固定线程或线程局部上下文，应使用专用线程，不能假定 spawn_blocking 的每次调用都落在同一线程。已有源码见 [同步与异步桥接](sync-async.md)。

## C ABI 不认识 Rust Future

async fn 返回的是一个由编译器决定类型和布局的 Future，不是“稍后返回 C 值”的普通外部函数。`dyn Future`、Rust trait 虚表、Rust 闭包、Waker、Result、Vec 和 String 都不能未经专门 ABI 协议就当成 C 数据结构传出去。`extern "C"` 决定调用约定，不会让这些 Rust 类型自动变为稳定 C 布局；给外围结构加 repr(C) 也不会递归改造内部不兼容字段。依据见 [外部块与 ABI](https://doc.rust-lang.org/reference/items/external-blocks.html)、[类型布局](https://doc.rust-lang.org/reference/type-layout.html#the-c-representation)与 [trait 对象](https://doc.rust-lang.org/reference/types/trait-object.html)。

通常给 C 一组不透明句柄与明确的 C 类型参数。下面只是协议示意，不是本章已实现的外部库接口：

```c
/* 具体类型、错误码、所有权、线程规则需在头文件中正式定义。 */
int operation_start(const unsigned char *input, size_t length,
                    void *user, void (*done)(void *, int, unsigned int),
                    operation_handle **out);
int operation_request_cancel(operation_handle *op);
int operation_unregister_and_wait(operation_handle *op);
void operation_release(operation_handle *op);
```

start 是否复制 input，失败时是否仍可能回调，成功时回调能否在 start 返回前同步触发，都必须写进协议。out 只在成功后可用，release 是否只能调用一次也要明确。不要假定上述名字对应任意真实库；有些库的取消完成事件或关闭句柄恰好承担注销/确认职责，有些根本没有可安全回收的异步注销机制。

向 C 暴露 Rust 异步服务时，也可以让内部长期运行的 Runtime 接收命令，再在完成时调用 C 回调。C 侧 cancel 通常只请求取消，release 是否表示放弃结果或等待退出需要另定。若使用整数句柄表，必须处理无效句柄与复用代数，避免陈旧句柄误操作新请求。不能每个 start 都临时创建 runtime 后立即销毁，却承诺后台任务还会继续运行。

## 注册、回调与上下文地址

典型回调接口接收函数指针和 void* userdata。安全包装可以把上下文放在稳定的拥有型分配里，把指针借给外部库；它必须在注册失败、成功、取消、重复通知和退出的每条路径上维护准确的所有权。堆分配只解决地址位置，不能解决何时释放；Pin 也不自动延长生命周期，见 [固定一章](pinning.md)。

不能把某个可移动 Future 内部字段或栈局部变量的地址交给会在函数返回后继续调用的 C 库。Future 被取消就可能销毁它们。更稳妥的组织是由独立注册对象或监督任务拥有上下文，单个 Future 只是等结果的订阅者；取消等待不立即释放注册。

回调通常借用 userdata，而不是每次取得所有权。如果用 [Box::into_raw/from_raw](https://doc.rust-lang.org/std/boxed/struct.Box.html#method.into_raw) 转移拥有权，必须恰好在协议规定的唯一回收点重建 Box；不能每次回调都 from_raw，否则后续回调会悬空或重复释放。Arc 原始指针同样需要严格的强引用配对，不能因为有 Arc 就让外部库随意复制裸指针。

回调线程也是契约的一部分：可能在外库线程、调用线程或多个线程上发生，需要上下文的同步与 Send/Sync 证明。不要未经证明就给指针包装器 unsafe impl Send/Sync；底层对象可能有线程亲和性。也不要假设回调线程处于 Tokio 上下文，直接创建/调用依赖当前 runtime 的功能。

短回调可以复制结果到 Rust 拥有的内存，再通过 [oneshot](https://docs.rs/tokio/latest/tokio/sync/oneshot/index.html) 发出完成通知；send 是同步操作，可由普通线程调用。持续事件流则要选择有界队列与明确溢出策略，不能阻塞外库回调线程等待异步容量，也不能悄悄换成无限队列。回调中的指针/长度只在回调期间有效时，必须在返回前复制或取得正式转移的所有权，不能把临时借用送入 'static 任务。

## 请求取消、注销和确认退出是三件事

必须分清以下阶段：请求停止工作；禁止新的回调进入；确认已经进入或已排队的回调全部结束；最后释放上下文、缓冲区与句柄。外库可能在取消请求成功后仍报告正常完成，也可能回调一个“已取消”结果。除非 API 明确保证，取消返回成功不能当作内存回收许可。

先把 active 标志设为 false 也不足以释放 userdata：另一个线程可能已拿到指针，还没来得及检查标志。引用计数只在各回调真正取得有效拥有权时才管用；外部裸指针不能在对象释放后再通过递增引用计数“复活”。正确顺序由库的注销/完成确认协议保障。[Nomicon 异步回调说明](https://doc.rust-lang.org/nomicon/ffi.html#asynchronous-callbacks) 强调，注销并销毁对象后，必须保证没有回调再访问它。

如果注销会同步等待回调，不能持有回调需要的锁调用它，也不能在回调线程内等待自己的退出。这会死锁。等待注销确认本身可能阻塞，可以在允许的专用线程或阻塞池等待；若库要求在事件循环中异步等待，就必须持续驱动该循环，而非阻塞它。

普通 Drop 无法 await。安全包装应提供显式 shutdown().await，并让一个寿命足够长的监督者在等待者取消后继续持有注册，或者用库支持的安全拥有权/退休队列处理晚到回调。若无法证明外库不再访问，宁可在受控错误路径保留必要分配，也不能提前 free；但泄漏不是通常的关闭方案。若库没有所需确认接口，需重新设计封装，不能把不健全的裸指针接口伪装成安全 Future。

## 一个明确的安全模型

以下代码用安全 Rust 模拟外库注册，不包含 extern 调用或裸 userdata。它用 Mutex 保护注册状态与在途计数，回调租约拥有 Arc 上下文；注销后拒绝新租约，在途归零后回收注册持有的上下文。Condvar 等待在 spawn_blocking 中完成。真实库必须提供等价的外部契约，不能靠把此模型的 Rust 结构体按布局传给 C：

```rust
{{#include ../../examples/guide-tests/src/async_ffi.rs:callback-model}}
```

结果 Future 与 Registration 分开。丢弃 response 只会让回复失败，不撤销已进入回调；request_cancel 只记录取消请求，不偷偷把它当成注销成功。示例定义 status 为零时成功、非零时返回业务错误，并只交付第一个响应；这不是任何外部库通用的错误码规则。

显式 shutdown 消耗注册对象，先注销，再等待租约归零。其等待 Future 若在已经提交阻塞等待后被丢弃，阻塞闭包仍拥有 foreign，能继续回收。若根本没有执行 shutdown，Drop 只注销；模型因回调租约的 Arc 保活而不会悬空，但外部仍保留 MockForeign 时上下文可能继续保留。真实裸指针包装不自动具备这层保护，必须另行实现并证明保活机制。

模型假设租约代表所有可能访问上下文的回调，不模拟操作系统中“已排队但尚未登记”的回调；实际外库的确认必须覆盖这些回调。Condvar 等待还假设每个租约最终释放，不承诺关闭期限；shutdown 的 JoinError 与业务状态码分别处理。本例内部锁未模拟 poisoning 恢复，也没有跨 ABI panic 隔离，因此它不能直接复制成 extern trampoline。

## panic 与外部异常不能随便跨边界

Rust 回调通常声明 extern "C"，不能让展开穿过不支持展开的 ABI。Rust panic 试图越过这种边界会中止进程；外部异常以不受支持的方式进入 Rust 则可能构成未定义行为。需要支持跨语言展开时，必须专门选择兼容的 C-unwind ABI、编译选项和双方析构模型，不能仅把一个函数名改掉就当作完成。依据见 [Nomicon 的展开边界](https://doc.rust-lang.org/nomicon/ffi.html#ffi-and-unwinding)。

若回调要把 Rust panic 转成错误，应在 Rust 函数体内部、返回 C 之前使用 catch_unwind，并确保错误路径本身不再 panic；不可在 extern 函数返回之后才捕获。[catch_unwind](https://doc.rust-lang.org/std/panic/fn.catch_unwind.html) 只捕获展开型 Rust panic，不捕获 panic=abort；捕获外部异常的行为也不能当成普通 Rust Result。panic hook 仍会先运行，捕获 panic 不自动恢复已被修改的业务不变量，必要时应把注册标为失败并触发受控关闭。

外部 C++ 异常通常应在 C++ 包装层转成错误码；输入指针的可空性、对齐、长度、有效期，以及字符串编码也要由边界校验。绑定生成器能减少签名抄写错误，不能替你证明异步生命周期、线程亲和性、取消完成或 unwinding 安全。

## 运行时和动态库的退出

关闭顺序还包括宿主：停止新 start，取消/排空工作，等待所有回调与任务，释放外库资源，然后关闭运行时或卸载动态库。代码段所属动态库也必须活到最后一次回调返回，否则不仅 userdata，连函数指针本身都会失效。

不要从 runtime worker 或外部回调中同步等待一个必须由当前事件循环推进的异步任务。需要从 C 调回 Rust 服务时，应通过已存在的安全入口发消息并异步确认，或在允许阻塞的独立线程桥接。回调可能同步重入 start，所以注册状态与输入保活应在首次外部调用之前准备好，不能先把裸指针交出去，再补写其所有者。

## 本章验证边界

三个新测试验证：真实辅助线程通过模型回调返回成功结果；丢弃等待者后请求取消仍能开始迟到回调，开始 shutdown 后拒绝新回调，已有租约退出前 Weak 上下文仍可升级，确认退出后才释放；非零状态码与重复完成只交付一次的规则。没有依赖墙钟延时，也没有释放后再访问的错例。

这些是注册/回调生命周期模型测试，没有编译 C/C++、链接真实外部库、验证实际头文件 ABI，或运行真实 panic/外部异常穿越边界。真正的 unsafe 安全包装仍需逐项证明外库协议与平台调用约定；本文的安全 Arc 模型不能替代这项证明。
