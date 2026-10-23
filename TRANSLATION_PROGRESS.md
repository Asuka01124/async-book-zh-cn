# 翻译与补全进度

已完成新版指南第 1—12 章：第一轮第 1—6 章（2026-10-22），第二轮第 7—12 章（2026-10-23）。第 13 章不在第二轮范围。翻译作者为 GPT-6 Luna；技术补全、校审与验收作者为 GPT-6.1 Sol。上游基线：`840327a`。各章逐章翻译、校审、验证、提交并推送后，再开始下一章。书前导言暂保留上游英文，基础设施由主代理负责。第一轮日期与结果保留为历史证据，第二轮结果单独记录。

## 第二轮最终本地复核（2026-10-23）

第 7—12 章主提交及第 10 章措辞复核提交均已上传，提交索引见 README。主代理已逐章阅读六章成品，对照上游原始提纲确认主题覆盖，并在第 12 章完成后执行以下累计复核：

- mdBook 0.5.4：`mdbook build --dest-dir .tools/round2-final-book` 成功。
- `python scripts/check_chapters.py --output .tools/round2-final-book`：12 章通过未完成标记、空链接、本地文件、锚点和重复 HTML id 检查。
- 用户已有 Rust 1.96.0：`cargo +1.96.0 test --offline --workspace --manifest-path examples/Cargo.toml --target-dir target`，13 个测试通过、0 失败，保留上游原有警告。
- `cargo +1.96.0 test --offline --locked --manifest-path examples/guide-tests/Cargo.toml --target-dir target/guide-tests`，61 个测试通过、0 失败，无编译警告。本轮新增 32 个，包含 Loom 正反模型和实际本机 TCP 测试。
- `cargo +1.96.0 fmt --manifest-path examples/guide-tests/Cargo.toml --check`、`git diff --check` 通过；禁用增量缓存以避免 OneDrive 缓存干扰。
- 检查各章提交均为当前 HEAD 的祖先；与第二轮开始前的 `9aa2a74` 比较，第 13 章 `streams.md` 保持原样。

累计 74 个测试通过。Windows 信号函数已在本机编译但未调用；没有发送真实控制台中断。Unix cfg 分支、平台专用工具、实验 AsyncDrop 及其他未运行范围仍按逐章记录说明，不把注入 Future 测试算作操作系统信号端到端验证。不查询、监听或等待 CI。

## 首轮最终本地复核（2026-10-22）

2026-10-22，六章均已逐章提交并上传。章节提交索引见 README。主代理在第六章上传后重新执行以下检查，全部通过：

- mdBook 0.5.4：`mdbook build --dest-dir .tools/final-book`。
- `python scripts/check_chapters.py --output .tools/final-book`：六章均无 TODO/FIXME、空链接、无效本地文件或锚点、重复 HTML id。
- 用户既有 Rust 1.96.0：`cargo +1.96.0 test --offline --workspace --manifest-path examples/Cargo.toml --target-dir target`，13 个测试通过、0 失败；保留上游既有警告。
- `cargo +1.96.0 test --offline --locked --manifest-path examples/guide-tests/Cargo.toml --target-dir target/guide-tests`，29 个测试通过、0 失败；本机 TCP 测试真实执行。
- `cargo +1.96.0 fmt --manifest-path examples/guide-tests/Cargo.toml --check` 与 `git diff --check` 通过。

共计 42 个测试通过。最终测试禁用增量缓存，使用已有工具链，不跳过本机网络测试。外部服务、特定文件输入和 Linux 专属路径的验证边界仍按各章记录，不将其计入已运行范围。

按用户后续要求，不再查询或等待 GitHub Actions；CI 由用户自行查看。首轮在第六章处结束，第二轮续译结果见本文件对应记录。

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

## 首轮范围完成（第 1—6 章）

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

## 第二轮（2026-10-23）：第 10 章，Future 详解

- GPT-6 Luna逐项翻译（首段亦译为中文），GPT-6.1 Sol扩写与技术校审。Future/Output、IntoFuture及异步builder、装箱/动态分派/递归、poll/Poll/await/唤醒/取消、原空Fusing节、futures-rs历史/用途/TryFuture/辅助Future/组合器/执行器/IO差异，以及futures-concurrency均实质补齐。
- 修正Ready后重复poll无一般保证、Fuse不缓存输出、Pin不阻止销毁、now_or_never会消费丢弃、泛型Future与IntoFuture约束不同、当前Tokio宏版本转换支持。补充race空集panic及丢JoinHandle的拥有关系边界。
- 新增 `guide-tests/src/future_details.rs`，正文include五组源码。新增4测试通过：Pending安排wake后Ready、builder/借用装箱/错误传播/join、race拥有输家销毁、纯Future由futures executor驱动。锁定futures-concurrency7.7.1并离线编译；累计48测试，本章执行新增4个，旧44个留最终统一复核。
- 2026-10-23实际打开标准库Future/IntoFuture、futures FutureExt/TryFuture/Fuse/executor、futures-concurrency及Race官方文档，正文附来源。Rust1.96、cargo fmt、`.tools/round2-chapter-10`构建与前10章HTML/链接/锚点/待写标记检查、git diff --check通过；不监听CI。

### 第 10 章提交后的措辞复核

父代理复核指出Waker的执行方式属于执行器，正文已改为不依赖轮询线程/时机；将“某个ready()仍可轮询”改为某些具体Future支持重复轮询，避免误指标准库Ready。修改仅限描述，不改源码和测试；重新构建并检查前10章通过。

## 第二轮（2026-10-23）：第 11 章，运行时及其使用边界

- GPT-6 Luna翻译，GPT-6.1 Sol补全与校审。原运行启动/异步main、上下文/block_on/Runtime/Handle/关闭，线程任务/Send与static/yield/spawn_local/spawn_blocking/block_in_place/队列，配置/线程池/单线程/每核线程，替代运行时/自建目的与设计差异等主题逐项补齐。
- 特别解释enter不驱动、构造计时器参数的求值时机、current_thread上的Handle::block_on不驱动IO/时间、Handle不保活、LocalSet根上下文及run_until非全量join、join分支在block_in_place中同任务暂停、关闭超时不杀线程；队列行为只作版本实现说明。
- 新增 `guide-tests/src/runtime_details.rs` 并正文include。Rust1.96新增7测试全部真实通过：显式启动与enter差异、LocalSet剩余任务恢复、多线程block_in_place重新进入、单线程拒绝（预期panic）、嵌套Handle block_on拒绝（预期panic）、Handle不延长运行时生命周期/任务析构、shutdown_timeout后已开始阻塞任务仍完成。累计55测试，本章过滤执行7个，旧48个留最终统一复核。
- 2026-10-23实际打开Tokio runtime/Runtime/Handle/Builder/LocalSet/block_in_place/spawn_blocking/yield以及smol、Glommio、tokio-uring官方文档；必要来源在正文。其他运行时、Linux每核线程和io_uring未在Windows执行。
- cargo fmt、`.tools/round2-chapter-11`构建、前11章HTML/链接/锚点/待写标记检查与git diff --check通过，不监听CI。

## 第二轮（2026-10-23）：第 12 章，定时器与信号处理

- GPT-6 Luna翻译，GPT-6.1 Sol补全。原运行时集成/避免thread::sleep、Duration/Instant、sleep、interval、timeout及专用包装与select/race区别、信号含义/异步接入/操作系统差异/Tokio文档全部扩为正文。
- 增补Sleep固定与reset、总期限/空闲期限、首次tick立即完成及Burst/Delay/Skip、非实时与非抢占超时/嵌套Result/JoinHandle分离、虚拟时钟；信号接入分别介绍Unix SIGINT/SIGTERM/SIGHUP、合并与默认处理器不恢复，Windows控制台事件/关闭窗口与服务协议差异，以及注入触发器测试方案。
- 新增 `guide-tests/src/time_signals.rs`，正文include周期等待、超时取消任务、Unix/Windows信号及通知转发源码。6测试真实通过：完成前后reset、首次tick与三漏tick策略、零超时已就绪工作、借用JoinHandle超时继续、显式abort后析构完成、注入关闭成功/失败；累计61测试，本章执行6个，旧55个留主代理统一复核。
- Rust1.96/Tokio1.54.1的cargo check --all-targets通过，Windows信号函数编译；没有调用它注册监听或向用户会话发送真实中断。Unix分支被cfg排除，仅核查官方API与源码，未编译/运行；注入Future测试不计作OS信号端到端测试。
- 2026-10-23实际打开Tokio time/sleep/interval/MissedTickBehavior/timeout/pause/signal/ctrl_c/Unix Signal及signal函数/Windows模块和ctrl_close、Rust Instant、Microsoft HandlerRoutine及服务控制处理器官方文档，正文附来源。
- cargo fmt、`.tools/round2-chapter-12`构建、全部12章HTML/链接/锚点/待写标记检查、git diff --check通过；不监听CI。到本章停止扩展，第13章streams.md保持原状。

## 第二轮逐章交付完成（2026-10-23）

第7—12章均按Luna逐章翻译→Sol实质补全/源码验证→该章独立提交并上传→下一章执行。原有实质提纲均补成解释、用法、错误边界与示例，不以删除标记代替补写。第10章上传后根据父代理复核另追加Waker/重复轮询契约措辞修正，保留原提交历史。

独立验证crate由29测试增至61测试（新增32个），六章分别运行新增6/4/5/4/7/6个测试全部通过。新增依赖均写入独立crate锁文件，未改变上游旧示例工作区；最终全量旧工作区和61测试由主代理在本轮结束后统一复核并记录。平台专用工具、Unix代码、实验AsyncDrop及真实信号端到端验证边界见对应章节，不计入本机运行通过。未查询、监听或等待CI。
## 第三轮（2026-10-24）：第 13 章，异步迭代器（Stream）

- GPT-6 Luna逐项翻译原英文提纲，GPT-6.1 Sol补齐正文并验收。涵盖Stream模型/惰性/固定/融合、next与消费、collect/into_future、异步组合器与有序/无序缓冲、并发限制、动态StreamGroup、join/select/race与循环取消、手写trait/unfold/库宏、Sink背压/flush/close，以及标准化/生成器/lending边界。
- 新增共享源码streams.rs，正文include真实锚点。4测试通过：惰性/固定/融合、有序与无序缓冲及并发上限、动态组移除与发送关闭、竞争后接收状态保留；累计65个。本章只运行新增4个。
- 实际打开futures Stream/StreamExt/FusedStream/unfold/Sink、Tokio StreamExt、StreamGroup、async-stream、标准库AsyncIterator、RFC2996与Unstable Book gen_blocks官方文档。2026-10-24 AsyncIterator仍nightly-only；第三方流宏未新增依赖/未实测，语言实验未运行。
- Rust1.96，cargo fmt --check、mdbook `.tools/round3-chapter-13`、前13章HTML/链接/锚点/未完成标记及git diff --check通过。没有监听CI。
## 第三轮（2026-10-24）：第 14 章，实现 Future 与 Stream

- 上游只含英文目录标题，没有英文正文。GPT-6 Luna翻译标题，GPT-6.1 Sol原创补写本章，新增implementing-futures-streams.md并接通目录。没有将新写正文归称为英文翻译。
- 完整解释手写状态机、poll/Pending唤醒协议、丢唤醒竞态/检查注册原子区与AtomicWaker注册重查、最新Waker/锁外唤醒、Pin/Unpin/安全字段投影、保存跨poll状态、单项Stream与FusedStream、取消/关闭/迟到通知及测试边界。
- 新增安全Rust共享源码implementing.rs，正文include真实锚点，4个行为测试通过：真实Pending后由辅助线程发送并唤醒最新Waker；完成先于注册/发送端关闭；取消注销后拒绝迟到发送并返还值；含!Unpin Future的Stream保留Pending状态并融合。累计69测试，本章仅过滤执行新增4个；无新依赖。
- 实际打开标准库Future/Pin、futures AtomicWaker/Stream/FusedStream及pin-project-lite官方文档。投影宏未新增依赖或运行，线程测试不宣称穷举所有交错；没有内核IO验证。
- Rust1.96，cargo fmt --check、mdbook `.tools/round3-chapter-14`、前14章HTML/链接/锚点/未完成标记及git diff --check通过。不监听CI。
### 第 14 章追加校审：Waker 回调的锁边界

- 父代理指出Waker的clone/drop也可能有自定义回调。已将Receiver的新Waker克隆放到锁外，旧Waker经replace/take移出，在释放Mutex后销毁，正文补充此约束。
- 新增确定性Wake::Drop探针，用try_lock验证替换旧注册与取消注销时锁已释放；14章5测试通过，累计70。没有通过真的制造死锁进行验证。
- 重建原输出目录遇OneDrive拒绝删除旧目录，改用`.tools/round3-chapter-14-review`成功构建；前14章检查、fmt/diff检查通过。独立追加提交，不重写已推历史。
