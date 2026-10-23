# 翻译与补全进度

已完成第一轮新版指南第 1—6 章（2026-10-22）；第二轮在 2026-10-23 按顺序续做第 7—12 章，第 13 章不在本轮范围。翻译作者为 GPT-6 Luna；技术补全、校审与验收作者为 GPT-6.1 Sol。上游基线：`840327a`。各章逐章翻译、校审、验证、提交并推送后，再开始下一章。书前导言暂保留上游英文，基础设施由主代理负责。下文第一轮日期与结果保留为历史证据；第二轮结果逐章追加。

## 本轮最终本地复核

2026-10-22，六章均已逐章提交并上传。章节提交索引见 README。主代理在第六章上传后重新执行以下检查，全部通过：

- mdBook 0.5.4：`mdbook build --dest-dir .tools/final-book`。
- `python scripts/check_chapters.py --output .tools/final-book`：六章均无 TODO/FIXME、空链接、无效本地文件或锚点、重复 HTML id。
- 用户既有 Rust 1.96.0：`cargo +1.96.0 test --offline --workspace --manifest-path examples/Cargo.toml --target-dir target`，13 个测试通过、0 失败；保留上游既有警告。
- `cargo +1.96.0 test --offline --locked --manifest-path examples/guide-tests/Cargo.toml --target-dir target/guide-tests`，29 个测试通过、0 失败；本机 TCP 测试真实执行。
- `cargo +1.96.0 fmt --manifest-path examples/guide-tests/Cargo.toml --check` 与 `git diff --check` 通过。

共计 42 个测试通过。最终测试禁用增量缓存，使用已有工具链，不跳过本机网络测试。外部服务、特定文件输入和 Linux 专属路径的验证边界仍按各章记录，不将其计入已运行范围。

按用户后续要求，不再查询或等待 GitHub Actions；CI 由用户自行查看。其余章节及书前导言尚未翻译，本轮在六章处结束。

## 第 1 章：指南导读

- 文件：`src/part-guide/intro.md`；目录条目同步为中文。
- 翻译：GPT-6 Luna；补全及技术校审：GPT-6.1 Sol。
- 补全：上游“与其他语言比较”的空链接改为本章实际比较小节，说明惰性 Future、任务生成、运行时选择及借用约束；保留原有章节概览与链接。
- 官方核查：标准库 Future、Rust Reference await 表达式、Tokio task 文档，均于 2026-10-22 查阅，链接见正文。
- 验证：`mdbook 0.5.4 build --dest-dir .tools/build-chapter-1` 成功（默认旧输出目录清理被 Windows 拒绝，改用新输出目录）；`git diff --check` 通过；相对文件链接、显式锚点、空链接与 Markdown 代码围栏检查通过。此章无可执行代码示例。

## 第 2 章：并发编程

- 文件：`src/part-guide/concurrency.md`；翻译：GPT-6 Luna；技术校审：GPT-6.1 Sol。
- 原文无 TODO。完整翻译正文、伪代码及全部脚注；保留原英文锚点，更新目录条目。
- 校正：进程与可执行文件并非一一对应；操作系统仍能抢占底层线程；阻塞只直接影响所在工作线程；并发可以在单核上重叠 IO 等待；顺序可观察不等于没有并发；限缩异步性能绝对断言，修正缓存切换与忙等的描述。
- 官方核查：Rust Book 第 17 章、标准库 Command 和 available_parallelism、Tokio task/runtime 文档（2026-10-22）；依据链接见正文。
- 验证：`mdbook build --dest-dir .tools/build-chapter-2` 成功；`scripts/check_chapters.py` 同时检查前两章通过；`git diff --check` 通过。正文仅有明确的伪代码与调用片段。

## 第 3 章：Async 与 Await

- 文件：`src/part-guide/async-await.md` 及四个 hello-world 示例注释。翻译：GPT-6 Luna；补全及技术校审：GPT-6.1 Sol。
- 补全：futures-rs 的历史、标准库 Future 的角色、可选依赖与生态选择、IO trait 与组合器语义差异。上游的两段待写提纲均已改为正文。
- 校正：不能直接写 async trait；tokio::main 使用 block_on 而非 spawn 根任务；async 函数的惰性不能推广到已生成任务的 JoinHandle；await 使用任务上下文而非隐含全局运行时；spawn 不保证并行或每种输出顺序；立即等待首个任务不会让两个新任务重叠；unwrap 不会原样重抛 panic 载荷。补齐 mini-redis 导入与依赖/服务前提，改正 no_run 围栏。
- 示例：新增独立 `examples/guide-tests`（自带 `[workspace]`），正文 add/wait_to_add 直接 include 可测试源码；修复上游未定义的 sleep(1000)。
- 官方核查：Rust 1.39 公告、Future/await 规范、futures 与 futures::io、Tokio main/spawn/JoinHandle 文档（2026-10-22），链接见正文。
- 验证：Rust 1.96 `cargo test --offline --manifest-path examples/guide-tests/Cargo.toml --target-dir target/guide-tests`：3 个行为测试通过（Future 惰性与时间、分离任务、取消错误）；四个 hello-world 二进制均真实运行并正常退出；`cargo fmt --check`、mdbook 构建以及前三章 HTML/相对链接/锚点/重复 id 检查、`git diff --check` 通过。mini-redis 为需外部服务的 no_run 教程示例，未连接或运行该服务。

## 第 4 章：更多 async/await 主题

- 文件：`src/part-guide/more-async-await.md`；翻译：GPT-6 Luna；补全及技术校审：GPT-6.1 Sol。
- 上游五个仅含提纲的章节均补为实质正文：异步闭包、生命周期与借用、Send 与 static、异步 trait、递归。保留相关历史参考，将空链接指向工具章和取消参考章。
- 更新异步闭包为 Rust 1.85 已稳定；解释 lending 与 for<'a> AsyncFnMut 约束；区分 Send/static，说明 spawn_local 仍要求 static 和 LocalSet；解释原生 async trait、Send 返回 Future、trait-variant、dyn 装箱与 async-trait 的边界；递归使用 Rust 1.77 后的 Box::pin。
- 校正取消并非撤销 IO，丢弃 JoinHandle 不取消任务，abort 不抢占 poll 且不能中断已运行 spawn_blocking；生命周期捕获的 edition 差异与改写 async 函数的兼容性；区分 POSIX 延迟取消和异步取消。旧上下文不完整的示意片段标记 ignore；原错误返回类型的取消示例替换为可编译共享源码。
- 示例：正文直接 include `guide-tests/src/more.rs` 的七组示例。Rust 1.96 离线测试：累计 9 个通过，本章新增 6 个（lending、借用、非 Send 本地任务、Send trait/dyn 分派、递归、正常及错误控制流）。仅有 OneDrive 增量缓存硬链接/清理告警，不影响测试。
- 官方核查：Rust 1.85/1.75/1.77 公告、AsyncFnMut、Reference、Edition Guide、Unstable Book、Tokio test/spawn/spawn_local/abort、async-trait、trait-variant 及 POSIX pthread_cancel 文档（2026-10-22），链接见正文。
- 文档验证：`mdbook build --dest-dir .tools/build-chapter-4-final` 成功；前四章 HTML/本地链接/锚点/重复 id 检查与 `git diff --check`、`cargo fmt --check` 通过。禁用增量缓存后重复运行 9 个测试，无编译告警。

## 第 5 章：IO 与阻塞问题

- 文件：`src/part-guide/io.md`；翻译：GPT-6 Luna；补全及技术校审：GPT-6.1 Sol。
- 上游四个 TODO 均实质补全：读写 trait/扩展方法、TCP/UDP/文件/标准IO/管道；借用与拥有缓冲区、限长与背压、Bytes共享与零拷贝边界；缓冲/拆分/重组/复制/单工双工/取消；就绪与完成模型、Mio、io_uring及所有权缓冲区。
- 新增源码：`guide-tests/src/io.rs` 和完整二进制 `io-demo`，正文直接 include 同一份已验证源码。分帧协议带 1 MiB 分配上限，明确 read_frame 不可取消后在原流盲目重试。
- 校正：标准库也有非阻塞模式；sink不是异步迭代器；Tokio fs默认稳定路径blocking，当前文档另有Linux实验io_uring；同步互斥锁有合理场景；spawn_blocking已开始后不可abort但排队时可能取消；阻塞池应限制CPU并行；第二运行时不是长生命周期任务的必需品；yield_now不保证每次向执行器传播或让其他任务先跑。Rayon示例修正为实际rayon::spawn，保留依赖前提。
- 官方核查：Tokio IO/AsyncReadExt/AsyncWriteExt/BufWriter/Lines/fs/stdin/TcpStream/split/copy_bidirectional/spawn_blocking/yield_now、标准库TcpStream、bytes、Mio、tokio-uring、Rayon文档（2026-10-22），依据链接见正文。
- 测试：Rust 1.96离线累计19个测试通过，本章新增10个：部分传输/背压、截断及超长、Bytes共享、缓冲行读取、read/read_exact取消差异、write/write_all取消差异、BufWriter丢弃、真实本机TCP就绪循环、双向复制半关闭、开始后的阻塞任务abort。首次沙箱本机套接字测试因10013权限失败，提权后19个全部通过；不是跳过测试。io-demo真实运行输出hello async IO。未运行需要外部8080服务、file.txt输入或Rayon长计算的旧no_run示例；Windows未运行Linux io_uring。
- 文档验证：`mdbook build --dest-dir .tools/build-chapter-5-final`、前五章HTML/本地链接/锚点/重复id检查、`git diff --check`与`cargo fmt --check`均通过。

## 第 6 章：并发组合 Future

- 文件：`src/part-guide/concurrency-primitives.md`；翻译：GPT-6 Luna；技术校审与示例补全：GPT-6.1 Sol。原文无显式 TODO，完整保留工具、替代方案与脚注讨论。
- 校正：同任务组合不并行但任务可能迁移工作线程，仍依赖执行器驱动；CPU时间百分比仅为假想模型；join/try_join错误与取消、JoinSet/drop与JoinHandle分离的区别；select守卫先求值且禁用分支表达式仍创建；宏结果取handler而非自动取Err；借用Future可保留状态；StreamExt::next取消安全；空闲超时与总期限；固定非Unpin计时器；Fuse Future与流完成行为不同；spawn并非普遍更公平或更正确。
- IntoFuture：实际查阅Tokio1.54.1 maybe_done源码并编译仅实现IntoFuture的类型，确认当前join!/select!支持，保留原说明并补版本边界。
- 示例：`guide-tests/src/composition.rs`，四组正文include共享代码，修复原省略函数/变量与未固定计时器问题。保留一个明确标为ignore的原示意片段。
- 官方核查：Tokio join/try_join/select/JoinSet及源码，futures select/FuturesUnordered，futures-concurrency Race，标准库Future文档（2026-10-22），链接见正文。
- Rust验证：`cargo +1.96.0 test --offline --manifest-path examples/guide-tests/Cargo.toml --target-dir target/guide-tests`累计29测试全部通过，本章新增10测试：IntoFuture，join保留错误并等待其他分支，try_join销毁拥有分支，任务嵌套Result，guard/创建/轮询顺序，模式不匹配else/handler值，超时分支，借用竞争后继续，空闲/总期限差异，Fuse完成后Pending。由于含本机TCP测试，使用已授权的提权执行；无编译警告。
- 文档验证：`mdbook build --dest-dir .tools/build-chapter-6-final`、六章HTML/本地链接/英文锚点/重复id检查、`git diff --check`与`cargo fmt --check`均通过。

## 本轮范围完成

六章均由 GPT-6 Luna 逐章翻译，GPT-6.1 Sol 逐章补全与验收，每章验收后独立提交并上传，再开始下一章。上游显式待写标记（第3章1处、第5章4处）及第4章5节实质提纲已补齐；第1章和第4章共3个正文空链接已处理。原英文标题的必要别名保留，英文标题本身不重复添加同名id。

本地运行环境：Rust 1.96.0、Tokio 1.54.1、futures 0.3.34、mdbook 0.5.4。新增独立验证crate累计29测试通过，四个hello-world程序与io-demo实际运行通过。上游旧工作区的最终总检查由主代理统一执行并记录。本轮后续章节及书前导言保持上游英文；未监看或等待CI。对需要外部服务/输入和Linux专用功能的未运行示例已在相关章记录，不将静态查阅当作运行通过。

## 第二轮（2026-10-23）：第 7 章，通道、锁与同步

- 翻译：GPT-6 Luna；补全与技术校审：GPT-6.1 Sol。文件 `sync.md` 及目录中文化，保留英文索引锚点。
- 原始主题逐项补齐：运行时兼容性/阻塞与异步原语；oneshot、mpsc、broadcast、watch、有界/无界与背压；同步/异步Mutex、Send守卫、持锁跨await、死锁与无中毒、owned和blocking接口；RwLock、Semaphore与yield；Notify、Barrier、OnceCell、原子操作。增加关闭协议、消息取消保留和业务不变量说明。
- 正文 include 真实 `guide-tests/src/sync.rs`；新增6个测试真实通过，覆盖actor排空、容量等待取消保留消息、广播落后/watch覆盖、持锁与abort归还许可、通知合并、取消初始化后重试；累计可用测试35个。本章按过滤器执行6个，旧29个未重复运行，最终由主代理统一复核。
- 2026-10-23实际打开Tokio官方sync/Mutex/RwLock/Semaphore/Notify/Barrier/OnceCell/mpsc/broadcast/watch文档；正文附链接。mdbook构建 `.tools/round2-chapter-7`、前7章HTML/链接/锚点/待写标记检查、cargo fmt与git diff --check通过。不查询或等待CI。

## 第二轮（2026-10-23）：第 8 章，异步编程工具

- GPT-6 Luna逐项翻译，GPT-6.1 Sol补全并校审。原Monitoring、tracing/logging、debugging、profiling提纲均扩为成品：Console采集端/客户端与实验cfg配置；span跨await误归属与Instrument；线程回溯、原生调试与WinDbg/PDB；复现停滞协议；CPU火焰图与IO等待差异；RuntimeMetrics和tokio-metrics。原“其他工具/Loom”补为模型、限制与可运行正反例。
- 新增 `guide-tests/src/tools.rs`，正文include Instrument、稳定指标及Loom模型。4测试真实通过：挂起时span退出，稳定worker指标，fetch_add模型无丢失，分离load/store反例被Loom发现（预期panic）。累计39测试，本章过滤运行4个，旧35个留最终统一复核。
- 新增并锁定 tracing0.1.44、tracing-subscriber0.3.23、loom0.7.2。离线首次缺generator，按授权联网仅下载依赖，仍用已有Rust1.96；修正测试手动poll后全部通过。
- 实际打开tracing/Instrument/Span、Console/console-subscriber、Loom、RuntimeMetrics/tokio-metrics、std backtrace、Microsoft WinDbg、flamegraph官方文档（2026-10-23），必要来源在正文。未启动Console/WinDbg/perf或采集真实火焰图，明确区分文档配置说明与已执行测试。
- `.tools/round2-chapter-8` mdbook构建、前8章HTML/链接/锚点/未完成标记检查、cargo fmt和git diff --check通过；不监听CI。

## 第二轮（2026-10-23）：第 9 章，析构与清理

- GPT-6 Luna翻译，GPT-6.1 Sol补全。原Drop/一般清理需求、异步清理与正在使用资源、取消观察、panic传播/锁不变量/展开期限制、各清理模式及“为何尚无async Drop”全部扩为正文，保留优雅关闭及完成式IO前向引用。
- 明确同步销毁与远程协议完成不同，丢JoinHandle不取消、abort非抢占、取消令牌仅协作请求；运行时先于资源关闭/析构spawn无法保证完成；显式finish、析构炸弹风险、监督任务/线程/进程与关闭确认。
- 2026-10-23实际打开Rust AsyncDrop文档（仍标nightly-only）、Unstable Book async_drop、#126482、析构Reference、Tokio JoinHandle/Runtime关闭、CancellationToken和优雅关闭文档；正文链接记录状态，不执行nightly特性。
- 新增 `guide-tests/src/cleanup.rs`，正文include可编译显式finish和协作关闭。5测试真实通过：缓冲刷新与EOF、未完成缓冲丢弃、abort后同步析构已结束、panic后锁释放但数据未恢复、协作关闭先于join。累计44测试，本章运行新增5个，旧39个留最终统一复核。
- `.tools/round2-chapter-9`构建、前9章HTML/链接/锚点/待写标记检查、cargo fmt与git diff --check通过。未模拟强杀进程或实验AsyncDrop；不监听CI。
