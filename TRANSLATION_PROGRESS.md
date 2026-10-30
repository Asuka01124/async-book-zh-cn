# 翻译与补全进度

已完成新版指南全部 13 篇、参考篇全部 13 篇、旧版前二十二篇（含补齐的两个空目录项）及书前导言，累计 49 篇。第一、二轮为指南第 1—12 篇（2026-10-22/23）；第三轮为指南最后一篇及参考前五篇，另加书前导言（10-24）；第四轮为参考接下来的六篇（10-25）；第五轮为参考最后两篇与旧版入门前四篇（10-26）；第六轮为旧版执行原理五篇及 async/await 详解（10-27）；第七轮为旧版 Stream 两篇及多 Future 执行四篇（10-28）；第八轮补齐取消/超时与 FuturesUnordered 两项，再完成变通方案四篇（10-29）。翻译作者为 GPT-6 Luna；技术补全、校审与验收作者为 GPT-6.1 Sol。仅有目录标题的条目由 Luna 翻译标题、Sol 原创补写正文，已有原文则完整翻译并补全。上游基线：`840327a`。各篇逐篇翻译、校审、验证、提交并推送后，再开始下一篇。其余旧版章节及其他导航内容暂保留上游内容；各轮日期与结果保留为历史证据。

## 第八轮最终本地复核（2026-10-29）

第 43—48 篇已逐篇验收、独立提交并通过用户现有 GitHub API 快进上传，确认成功才开始下一篇。Luna 翻译原有标题与正文，Sol 补全及校订；前两项原无正文，由 Sol 原创完整教程并明确归因。父代理实际审读各篇成品与新源码，修正超时示例的时序假设及分配措辞，六个主提交索引见 README。没有延伸到后续生态或项目章节。

- mdBook 0.5.4：`mdbook build --dest-dir .tools/round8-final-book` 成功。
- `python scripts/check_chapters.py --output .tools/round8-final-book`：默认全部 49 篇通过未完成标记、空链接、本地文件、锚点与重复 HTML id 检查；新增两篇和四篇原文均已纳入默认范围。
- 用户已有 Rust 1.96.0：`cargo +1.96.0 test --offline --workspace --manifest-path examples/Cargo.toml --target-dir target`，旧工作区 21 测试通过、0 失败，保留既有编译警告。新增 1 项有界递归测试；原两项无终止递归只编译、不运行。旧工作区在本次全量后未改变。
- `cargo +1.96.0 test --offline --locked --manifest-path examples/guide-tests/Cargo.toml --target-dir target/guide-tests`，独立 crate 115 测试全部通过、0 失败、无编译警告。新增超时 4 项、集合 4 项、trait 接口 1 项；包含原有真实本机 TCP 测试并获所需权限执行。
- 独立 crate 的 `cargo +1.96.0 fmt --check`、旧包 `cargo +1.96.0 fmt --manifest-path examples/Cargo.toml -p example_07_05_recursion --check` 及 `git diff --check` 均通过。禁用增量缓存，无新依赖或工具链安装，最终结果均显式选择 `+1.96.0`。
- Send 最终正文的 6 个 Rust 块逐一编译，4 个成功、2 个按预期拒绝；另核查 E0733 无间接递归负例及 E0277/E0038 trait 负例。编译验证不混入单元测试数量，不把编译成功当成线程迁移实测。
- 对照本轮基线 `476c3cb` 与上轮 HTML，四篇原文的 4 个英文锚点、2 个 include、5 条外链及递归参考式定义全部保留。前两项原仅 SUMMARY 空链接，现已有实际正文与页面。书籍源码仅改本轮六篇及 SUMMARY，既有 CI/Pages 工作流未变。

累计 136 个本地测试通过，比上轮新增 10 项。超时示例同时接受首轮成功与超时；暂停时间与通道握手验证确定路径，未假定真实调度总能准时触发。集合示例实际运行普通与限额消费，并单独验证容量准入、错误析构和 fuse 结束状态。原生 trait 与手动装箱方案已执行，未安装或运行 async-trait 宏；立即完成方法不当成性能基准。没有运行无终止递归、外部 HTTP 或实验功能，也没有把行为测试当任意调度证明。不查询、监听或等待 CI。

## 第七轮最终本地复核（2026-10-28）

第 37—42 篇已由 Luna 完整翻译正文与所引源码注释，Sol 技术校订；父代理实际阅读全文及源码验收，每篇独立提交并通过用户现有 GitHub API 快进上传，确认成功才开始下一篇。六个主提交索引见 README。没有扩展到其后的 TODO 导航条目或其他旧版章节。

- mdBook 0.5.4：`mdbook build --dest-dir .tools/round7-final-book` 成功。
- `python scripts/check_chapters.py --output .tools/round7-final-book`：默认全部 43 篇通过未完成标记、空链接、本地文件、锚点和重复 HTML id 检查，本轮六篇已纳入默认范围。
- 用户已有 Rust 1.96.0：`cargo +1.96.0 test --offline --workspace --manifest-path examples/Cargo.toml --target-dir target`，旧工作区 20 测试通过、0 失败，保留上游既有警告。相较上轮增加 join、select、spawning 各 1 项确定性测试；原通道、求和、count 和三任务等待测试也实际重跑。
- `cargo +1.96.0 test --offline --locked --manifest-path examples/guide-tests/Cargo.toml --target-dir target/guide-tests`，独立 crate 106 测试全部通过、0 失败、无编译警告。真实本机 TCP 测试获所需权限执行；此 crate 及其依赖随后未再改变，不重复执行。
- `cargo +1.96.0 fmt --manifest-path examples/Cargo.toml -p example_06_02_join -p example_06_03_select -p example_06_04_spawning --check`、独立 crate 的格式检查及 `git diff --check` 通过。保持 `CARGO_INCREMENTAL=0`，不安装工具链或新增依赖。
- 对照本轮基线 `fe7257d` 与上轮生成 HTML，六篇原有 11 个英文标题锚点及 16 个 include 全部保留，原文无外链、脚注或参考式章节引用。书籍源码仅改变本轮六篇及 SUMMARY，既有 CI/Pages 工作流未变。

累计 126 个本地测试通过。新增验证分别覆盖 futures::try_join 错误返回时 owned 与 borrowed Future 的析构边界、select 的 default/complete 与 Fuse/集合重激活、async-std 句柄分离后仍完成及显式取消后的资源析构。select 测试前提中发现的 Pending 终止语义与新空集合首次轮询行为已依据源码和实测修正，最终全量无失败。原跳跃/取书/取音乐等立即就绪占位不当作挂起或性能证据；无限流循环只保留教学假设。HTTP 示例仅处理写入错误，未运行监听或声称完整协议/网络验证；握手测试不依赖墙钟竞速。不查询、监听或等待 CI。

## 第六轮最终本地复核（2026-10-27）

第 31—36 篇已由 Luna 完整翻译正文及所引源码注释，Sol 技术校订，父代理实际阅读全文与源码验收；每篇独立提交并通过用户现有 GitHub API 快进上传，确认上传后再开始下一篇。最后一篇两处语义措辞经 Sol 追加审读通过，六个主提交索引见 README。

- mdBook 0.5.4：`mdbook build --dest-dir .tools/round6-final-book` 成功。
- `python scripts/check_chapters.py --output .tools/round6-final-book`：默认全部 37 篇通过未完成标记、空链接、本地文件、锚点及重复 HTML id 检查，新增六篇已纳入默认范围。
- 用户已有 Rust 1.96.0：`cargo +1.96.0 test --offline --workspace --manifest-path examples/Cargo.toml --target-dir target`，旧工作区 17 测试通过、0 失败，保留上游既有警告。相较上轮增加 Timer 2 项及 Executor 2 项回归；原计时器与执行器集成测试也实际重跑。
- `cargo +1.96.0 test --offline --locked --manifest-path examples/guide-tests/Cargo.toml --target-dir target/guide-tests`，独立 crate 106 测试全部通过、0 失败、无编译警告。真实本机 TCP 测试获所需权限执行；此 crate 随后未再修改，无需重复运行。
- `cargo +1.96.0 fmt --manifest-path examples/Cargo.toml -p example_02_03_timer -p example_02_04_executor --check`、独立 crate 的格式检查及 `git diff --check` 通过。保持 `CARGO_INCREMENTAL=0`，不安装新工具链或新增依赖。
- Rust 1.96.0 实际拒绝 3 个编译负例（局部栈借用、move 引用与 static、构造捕获 Rc 与 Send），另有 1 个正例编译并运行。正例验证编译约束，不冒充其立即 Ready Future 的调度实测；这 4 项不混入单元测试计数。
- 对照本轮基线 `d340f2a` 及上轮生成的 HTML，六篇原有 10 个标题锚点、20 个 include、mio 外链及原章节引用全部保留，原文无脚注。书籍源码仅改本轮六篇及 SUMMARY，既有 CI/Pages 工作流未变。

本轮累计 123 个本地测试通过。Timer 回归使用确定性自定义 Waker 验证锁外 clone/drop/wake 与注册间隙完成；Executor 回归覆盖一次 poll 内 10,001 次唤醒、Pending 恢复、完成后重复消息及关闭后的迟到唤醒。教学执行器仍无背压、唤醒去重或完整关闭协议。Socket 与 IoBlocker 仍明确为示意，没有实现或声称实测 Linux、Windows IOCP 或 Fuchsia 后端，也不把行为测试当性能或任意线程交错证明。不查询、监听或等待 CI。

## 第五轮最终本地复核（2026-10-26）

第 25—30 篇均经 Luna 翻译、Sol 补全与技术校审、父代理实际全文及源码验收后独立提交，再通过用户现有 GitHub API 快进上传；确认本篇上传成功才开始下一篇。提交索引见 README。参考篇现已完整覆盖，旧版仅继续本轮指定的前四篇，没有扩展后续章节。

- mdBook 0.5.4：`mdbook build --dest-dir .tools/round5-final-book` 成功。
- `python scripts/check_chapters.py --output .tools/round5-final-book`：全部 31 篇通过未完成标记、空链接、本地文件、锚点和重复 HTML id 检查。已将本轮六篇纳入默认范围。
- 用户已有 Rust 1.96.0：`cargo +1.96.0 test --offline --workspace --manifest-path examples/Cargo.toml --target-dir target`，原工作区 13 个测试通过、0 失败，保留上游既有警告。旧版下载/primer 的原有 2/3 项测试包含在这 13 项内，没有重复计作新增。
- `cargo +1.96.0 test --offline --locked --manifest-path examples/guide-tests/Cargo.toml --target-dir target/guide-tests`，106 个测试通过、0 失败、无编译警告。本轮新增 5 个（rustc 状态与析构 2、监督者 2、Pending 事件偏序 1），其他三篇纯文档不增加凑数测试。
- 独立 crate 的 `cargo +1.96.0 fmt --check`，以及 `cargo +1.96.0 fmt --manifest-path examples/Cargo.toml -p example_01_02_why_async -p example_01_04_async_await_primer --check`、`git diff --check` 均通过。使用现有工具链、禁用增量缓存；真实本机 TCP 测试获所需权限执行，没有跳过。
- 本机稳定 rustc 的 `--emit=mir` 已观察两个挂起状态及完成路径，不启用 nightly、`-Z` 或自行编译 rustc。父代理补充了首次复现需先创建被忽略的 `.tools` 输出目录这一前提，已包含在最终书籍构建中；此追加文档修正不改变已测源码。
- 对照本轮基线 `719310b`，五篇既有原文的标题锚点、外链与脚注均保留；结构化并发 12 个旧标题 id、10 处外链和 3 条脚注另独立比对通过。六篇主提交均为 HEAD 的祖先，书籍源码仅变更本轮六篇及 SUMMARY，既有 CI/Pages 工作流未改。

累计 119 个本地测试通过。旧版网页下载仍是明确标注的控制结构占位，未访问示意站点；原学习/唱歌/跳舞桩不当成交错证据，新增独立示例才验证真实 Pending。未运行 moro/async-nursery、实验 AsyncDrop、真实服务器或性能/实时性基准；MIR 文本不作为稳定 ABI 或所有版本布局的保证。不查询、监听或等待 CI。

## 第四轮最终本地复核（2026-10-25）

第 19—24 章已按顺序独立提交并通过用户现有 GitHub API 快进上传，提交索引见 README。每章先由 Luna 翻译已有内容，再由 Sol 实质补写、核查官方资料与验证，父代理实际阅读全文及共享源码后提交；确认上传成功才开始下一章。Pinning 长文完整保留，并校正原有技术误述，没有改成摘要。

- mdBook 0.5.4：`mdbook build --dest-dir .tools/round4-final-book` 成功。
- `python scripts/check_chapters.py --output .tools/round4-final-book`：默认检查全部 25 篇已完成章节，通过未完成标记、空链接、本地文件、锚点和重复 HTML id 检查。
- 用户已有 Rust 1.96.0：`cargo +1.96.0 test --offline --workspace --manifest-path examples/Cargo.toml --target-dir target`，13 个原有测试通过、0 失败，保留上游既有警告。
- `cargo +1.96.0 test --offline --locked --manifest-path examples/guide-tests/Cargo.toml --target-dir target/guide-tests`，101 个测试通过、0 失败、无编译警告。本轮五章各新增 3 个行为测试，共 15 个；跨语言比较复用已有已测 Rust 示例，不增加凑数测试。测试后最后一章没有改动示例目录，已比对其提交树一致。
- `cargo +1.96.0 fmt --manifest-path examples/guide-tests/Cargo.toml --check`、`git diff --check` 通过；仍禁用增量缓存，使用已有工具链，真实本机 TCP 测试获所需权限执行，没有跳过。
- 对照本轮基线 `39dd444`，Pinning 原有 21 个标题锚点、46 处外链（含重复）和 5 个脚注均保留，原长文主题逐段审读。既有锁包 pin-project-lite 0.2.17 提升为直接依赖，没有新增包版本。
- 六个章节提交均是 HEAD 的祖先；书籍源码仅改变本轮六篇和 SUMMARY。rustc/structured、旧版、书前导言、其他导航及既有 CI/Pages 工作流未修改。

累计 114 个本地测试通过。FFI 仅验证安全 Rust 注册/回调生命周期模型，未链接真实 C/C++ 库或验证原生 ABI；跨语言章节未运行其他语言或性能测试。未运行 Miri、实验语言提案或其他平台路径，也不把行为测试称作任意 unsafe 抽象的健全性证明。不查询、监听或等待 CI。

## 按指定章节补译：书前导言（2026-10-24）

- 用户截图指向 `src/intro.md` 的 Introduction 及三个小节；本次完整翻译为“导言”“什么是异步编程，为什么使用它？”“你好，世界！”和“Rust 异步功能的发展”，同步 SUMMARY，保留四个原英文显式锚点、示例 include 和脚注。
- GPT-6 Luna 完成翻译，GPT-6.1 Sol 技术校审：修正 `.await`/Pending、惰性 Future/轮询、用户态任务调度与操作系统的关系、性能和取消的表述；更新 trait async fn 稳定状态、AsyncIterator/AsyncDrop 实验状态，并区分上游重写说明与中文版进度。两个脚注 TODO 链接接入已完成的 I/O 章。
- 实际核查 Rust Reference await、Future、AsyncIterator、AsyncDrop、Rust 1.75 异步 trait 公告、Tokio task，以及保留的 Async Working Group 路线图和项目目标，正文附相关官方链接。
- mdBook 0.5.4：`mdbook build --dest-dir .tools/intro-zh-final-book` 成功；`python scripts/check_chapters.py --output .tools/intro-zh-final-book` 默认检查全部 19 篇通过。另核对生成 HTML 的中文标题、四个旧英文锚点及实际展开的示例。
- 用户已有 Rust 1.96.0：`cargo +1.96.0 run --offline --locked --manifest-path examples/hello-world/Cargo.toml --target-dir target` 成功，输出 `hello, world!`。未更改示例、依赖或新增测试；先前 99 项测试结果保留为第三轮历史证据，本次没有将其记作重新执行。
- `git diff --check` 通过。本次书籍源码只修改书前导言和 SUMMARY；不查询、监听或等待 CI。

## 第三轮最终本地复核（2026-10-24）

本轮六章主提交及第 14 章 Waker 锁边界追加修正均保留独立历史，提交索引见 README。主代理逐章阅读成品和新增共享源码，对照原提纲验收，参考篇原空目录则按主题完整补写。第 18 章完成后统一执行：

- mdBook 0.5.4：`mdbook build --dest-dir .tools/round3-final-book` 成功。
- `python scripts/check_chapters.py --output .tools/round3-final-book`：18 篇已完成章节通过未完成标记、空链接、本地文件、锚点和重复 HTML id 检查；默认检查范围已扩展至本轮六章。
- 用户已有 Rust 1.96.0：`cargo +1.96.0 test --offline --workspace --manifest-path examples/Cargo.toml --target-dir target`，13 个原有测试通过、0 失败，保留上游既有警告。
- `cargo +1.96.0 test --offline --locked --manifest-path examples/guide-tests/Cargo.toml --target-dir target/guide-tests`，86 个测试通过、0 失败、无编译警告。本轮新增 25 个（4/5/3/4/5/4），含第 14 章追加回归测试；真实本机 TCP 测试获得所需权限执行，没有跳过。
- `cargo +1.96.0 fmt --manifest-path examples/guide-tests/Cargo.toml --check`、`git diff --check` 通过；禁用增量缓存以避免 OneDrive 缓存干扰。
- 六章提交和追加修正均为 HEAD 祖先。对比第三轮开始前的 `8ce59b4`，书籍源码只改变本轮六篇与 SUMMARY；Design patterns 及后续参考主题、旧版章节、书前导言保持原样，既有 Pages 工作流保留。

累计 99 个测试通过。本轮使用 Windows，Linux epoll/io_uring、BSD/macOS kqueue、原生 IOCP/CancelIoEx、其他平台运行时和 Tokio 实验文件后端未编译运行；内存生命周期模型和文档核查不算内核验证。各章具体边界见下文。不查询、监听或等待 CI。

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
## 第三轮（2026-10-24）：第 15 章，替代运行时

- 上游只有英文目录标题，Luna翻译标题，Sol原创完整中文正文并接通alternate-runtimes.md。解释Future协议与运行时环境的区别、执行器/驱动器/库/应用生命周期，以及平台/调度/内存所有权/生态适配与维护状态选择。
- 核查futures executor、smol及LocalExecutor/Task、async-io、Glommio、Monoio官方仓库、tokio-uring、Embassy、async-compat与async-std官方README。明确async-std已停止维护；Monoio不能概括为仅Linux但Windows仍实验；io_uring运行时不等于普通Tokio开关。
- 新增smol =2.0.2，离线缓存依赖可用，锁文件新增22包。新增alternate.rs并include，3测试在Windows通过：LocalExecutor非Send任务与真实计时器；真实Pending任务丢句柄取消后析构；同一独立Future由smol与futures执行器分别驱动。累计73测试，本章过滤新增3个。
- 没有运行Glommio/Monoio/tokio-uring/Embassy，也没有运行适配器或网络性能基准；Linux/macOS/嵌入式边界正文明确。
- Rust1.96，cargo fmt --check、mdbook `.tools/round3-chapter-15`、前15章HTML/链接/锚点/未完成标记与git diff --check通过。不监听CI。
## 第三轮（2026-10-24）：第 16 章，实现你自己的运行时

- 上游只有目录标题，Luna翻译标题，Sol原创完整中文正文，新增implementing-runtime.md并接通目录。覆盖任务表/就绪队列/安全Wake/Pin、跨线程通知、Condvar检查等待协议、时间driver、取消/关闭/迟到通知以及生产IO/公平/工作窃取/panic隔离的差距。
- 新增纯安全Rust mini_runtime.rs并在正文include：当前线程支持非Send任务，Weak队列Waker，任务ID不复用，空闲Condvar等待，最近期限wait_timeout，Weak计时登记与锁外通知，显式cancel/shutdown，关闭后外部Sleep返回TimerClosed。没有busy-poll或每个计时器一个线程；没有伪称生产I/O runtime。
- 4测试通过：真实计时器与非Send任务；poll内wake保留；先真实Pending再跨线程wake且总共仅两次poll；取消任务、陈旧ID与已真实Pending注册的外部计时器关闭通知。累计77测试，本章仅新增4个过滤执行，无新依赖。
- 实际打开标准库Wake/park/Condvar与mio Poll官方文档。Windows运行教学执行器与时间driver；未实现或运行网络内核driver、多线程窃取/IOCP/io_uring。
- Rust1.96，cargo fmt --check、mdbook `.tools/round3-chapter-16`、前16章HTML/链接/锚点/未完成标记与git diff --check通过。不监听CI。
## 第三轮（2026-10-24）：第 17 章，同步中的异步，异步中的同步

- 上游只有目录标题，Luna翻译标题，Sol原创完整中文正文，新增sync-async.md并接通目录。涵盖同步Runtime拥有/root/current-thread驱动与Handle边界、专用运行时线程、双向通道/背压/关闭死锁、spawn_blocking/许可真实生命周期、长期线程、Rayon结果通道、block_in_place及join分支限制。
- 新增bridging.rs，正文include服务、阻塞限流与Rayon源码。5测试Windows实际通过：专用线程服务同步/异步调用并关闭；已开始阻塞工作在外层abort后仍占许可；Rayon异步回复；满通道/跨线程发送关闭与排空；多线程block_in_place重入等待计时器。累计82测试，本章过滤新增5个。
- 新增rayon=1.12.0及5个锁包，离线缓存编译。实际打开Tokio bridging、Handle、block_in_place、spawn_blocking、mpsc Sender/Receiver、oneshot及Rayon ThreadPool官方资料。
- 不宣称取消已开始阻塞工作或CPU计算；未使用真实数据库/GUI/外部服务，未制造不可恢复的死锁。
- Rust1.96，cargo fmt --check、mdbook `.tools/round3-chapter-17`、前17章HTML/链接/锚点/未完成标记与git diff --check通过。不监听CI。
## 第三轮（2026-10-24）：第 18 章，异步 I/O：就绪与完成，以及 io_uring

- 上游只有目录标题，Luna翻译标题，Sol原创完整中文正文，新增io-models.md并接通目录。完整解释readiness/completion、epoll/kqueue、IOCP与Mio/AFD桥接、io_uring SQ/CQ与multishot、buffer稳定地址/所有权/初始化、Future取消与内核完成/资源回收、平台及运行时差异。
- 特别核查Tokio1.54.1：Windows以IOCP后端暴露就绪API；默认普通文件操作通常使用blocking池，同时Linux已有tokio_unstable与io-uring功能限制的实验文件driver。实际打开官方docs并核对本机Tokio/Mio源码，不沿用“Tokio绝无io_uring”的过时概括。
- 新增io_models.rs，正文include真实readiness辅助函数和明确标注的内存生命周期模型。4测试通过：Windows真实TCP就绪循环/分段字节/EOF，先真实Pending再取消借用read并重新读取；纯模型取消后保留操作buffer与模拟完成唤醒。真实网络测试使用已授权提权执行，没有用沙箱网络限制跳过。累计86测试，本章过滤新增4个，无新依赖。
- 正文明确累计read_until_eof取消会丢弃已读out，不承诺整函数无损重试；内存模型不是IOCP/io_uring封装，不含完整driver关闭/错误通路。Linux epoll/io_uring、BSD/macOS kqueue、原生Windows IOCP/CancelIoEx与Tokio实验文件driver均未编译运行，文档/源码核查不计作内核验证。
- 实际打开epoll/Linux io_uring/liburing取消及multishot手册、FreeBSD kqueue、Microsoft IOCP/ReadFile/CancelIoEx、Mio Poll/AFD源码、Tokio TcpStream/fs/Builder及tokio-uring IoBuf/IoBufMut官方资料。
- Rust1.96，cargo fmt --check、mdbook `.tools/round3-chapter-18-final`、全部前18章HTML/链接/锚点/未完成标记及git diff --check通过。不查询或监听CI；本轮到18章停止，Design patterns及后续未修改。
## 第四轮（2026-10-25）：第 19 章，设计模式

- 上游只有英文目录标题，Luna翻译标题，Sol原创完整中文正文并接通design-patterns.md。覆盖单一资源所有者/请求响应、排队与执行/字节背压、响应不确定性与幂等、分开关闭通道/排空/错误退出、有限并发与监督、管线/重排/连接池、锁与通知语义。
- 新增patterns.rs并在正文include。3个确定性行为测试通过：满队列真实Pending发送在关闭后返还命令且原队列排空/Sender仍持有；放弃响应不回滚状态；业务溢出保持状态且服务继续。累计89测试，本章只过滤新增3个，无新依赖。
- 实际打开Tokio通道/共享状态/关闭指南、mpsc/Sender reserve与send、JoinSet、watch及futures StreamExt官方文档。计数器是内存状态模型，不宣称数据库持久性/外部连接集成/管线或连接池实测。
- Rust1.96，cargo fmt --check、mdbook `.tools/round4-chapter-19`、前19章加导言共20篇HTML/链接/锚点/未完成标记及git diff --check通过。未查询、监听或等待CI。
## 第四轮（2026-10-25）：第 20 章，取消与取消安全

- Luna完整逐项翻译原英文提纲，Sol实质补齐全部主题：内外取消、线程/Future区别、drop与底层工作边界、poll交还才可销毁而非字面“任意await点”、abort/令牌与清理、Tokio局部定义/整体halt safety/复制思考实验、数据丢失与状态拆分、恢复及select循环。
- 新增cancellation.rs并include。3个确定性测试通过：两个取消安全recv组合仍能丢first并改组；外部PairReader保留进度/EOF半帧返还；固定同一Future在临时借用竞争失败后继续。累计92测试，本章仅执行新增3个，无新依赖。
- 实际打开标准库Future、Tokio select/JoinHandle abort/Mutex/AsyncReadExt、tokio-util CancellationToken与futures Abortable官方文档。令牌仅文档核查；旧章abort析构/超时join测试不重复计数，不宣称内核取消集成验证。
- Rust1.96，cargo fmt --check、mdbook `.tools/round4-chapter-20`、前20章加导言共21篇HTML/链接/锚点/未完成标记及git diff --check通过。不查询、监听或等待CI。
## 第四轮（2026-10-25）：第 21 章，饥饿

- 上游只有英文目录标题，Luna翻译标题，Sol原创starvation.md并接通目录。解释合作式调度/一直Ready的await与无限poll、CPU阻塞、多线程边界、Tokio预算与yield_now非保证、biased分支、锁与许可公平/取消排位、控制通路及背压观测。
- 新增starvation.rs并include预算Future。3个确定性有界测试通过：7次Ready await同poll完成；每poll两项/自唤醒/插入其他工作及溢出；biased前Ready分支令后分支5轮从未poll。累计95测试，本章仅执行新增3个，无新依赖。
- 实际打开Tokio task/coop/yield_now/select/runtime公平性及Mutex/Semaphore官方文档。手动poll模型不宣称生产调度公平性或实时期限保证，没有无限挂死或墙钟脆弱测试。
- Rust1.96，cargo fmt --check、mdbook `.tools/round4-chapter-21`、前21章加导言共22篇HTML/链接/锚点/未完成标记及git diff --check通过。不查询、监听或等待CI。
## 第四轮（2026-10-25）：第 22 章，固定（Pinning）

- Luna完整逐段翻译原长章，Sol核查并技术校订，未摘要删节。原21标题id、46处外链（含重复）、5脚注及全部替代/扩展提案与参考保留，所有未完成项补齐，异步状态机说明连既有章节并给合法例子；假想self生命周期/不完整类型及Drop示意明确ignore。
- 校正Unpin豁免与Copy区别、!Unpin不必然地址敏感且固定前可移动、NLL后续引用使用、移动非物理擦除、句柄与所指对象、Pin<&T>契约、局部pin值而非普通引用、Rc/Arc安全pin、static_ref/static_mut自1.61与const自1.84、packed/Vec/结构字段/Unpin/Drop边界、set原址先析构与原始复制风险、forget不保证析构。futures pin_mut按官方文档准确写为软弃用，区别pin-utils整包弃用。
- 新增pinning.rs并include投影源码，3测试通过：Box句柄移动后!Unpin所指值Pending/恢复/Drop同址；Pin::set原址先drop旧值再poll/drop新值；pin-project-lite结构投影真实非Unpin Future跨Pending自借用并计poll。累计98，本章仅新增3；已有锁包pin-project-lite0.2.17提升直接依赖，不新增包版本。
- 实际打开标准库Pin/pin模块/Unpin/pin宏/Rc/Arc/forget、pin-project及lite、pin-utils/futures pin_mut、RFC2349及原替代/扩展设计者博文，对照本机Rust1.96源码核实版本。历史提案不当成稳定API，未运行unsafe破坏示例/Miri或声称测试证明任意unsafe抽象健全。
- Rust1.96，cargo test --offline --locked pinning_、cargo fmt --check、mdbook `.tools/round4-chapter-22-final`、前22章加导言共23篇HTML/链接/锚点/未完成标记、原锚点外链脚注保留比对及git diff --check通过。不查询、监听或等待CI。
- 父代理全文校审后，限定普通用户结构体假想self关系与编译器async内部借用的区别，修自然译词；最终新目录`.tools/round4-chapter-22-accepted`构建无HTML警告，23篇检查与21旧id/46旧外链/5脚注保留比对通过。源码未改，不重复运行3测试。
## 第四轮（2026-10-25）：第 23 章，异步与 FFI

- 上游只有目录标题，Luna译标题，Sol原创async-ffi.md并接通目录。覆盖阻塞调用/回调/宿主Rust服务/外部事件循环、C ABI不承诺Future/trait布局、opaque句柄与输入结果所有权、上下文地址/回调并发/线程亲和、取消请求与注销和在途确认、显式shutdown/Drop保活、错误/重入/禁止不兼容unwind以及运行时和动态库卸载顺序。
- 新增async_ffi.rs并include安全Rust注册模型，无裸指针/extern实库。3测试通过：辅助线程回调结果；丢等待者后取消仍可有回调、注销拒新、辅助线程释放在途租约后shutdown确认Weak上下文已释放；错误码与重复完成只交付一次。累计101测试，本章仅新增3，无新依赖。
- 明确模型Arc租约只模拟生命周期，不证明外库已排队回调/原始userdata/真实ABI；租约Drop先锁外释放自己的上下文Arc再归零通知，保证quiescence语义。普通Drop只注销，不伪称完成异步等待；模型无poisoning恢复或extern panic隔离，真实库需独立unsafe证明。
- 实际打开Nomicon FFI异步回调/展开、Reference external blocks/布局/trait对象、标准库catch_unwind/Box into_raw及Tokio oneshot官方文档。没有编译C/C++、链接外部库、运行外部异常/非法跨ABI展开或声称集成验证。
- Rust1.96，cargo test --offline --locked async_ffi_、cargo fmt --check、mdbook `.tools/round4-chapter-23-reviewed`、前23章加导言共24篇HTML/链接/锚点/未完成标记及git diff --check通过。不查询、监听或等待CI。
## 第四轮（2026-10-25）：第 24 章，Rust 异步编程与其他语言的比较

- 上游仅目录标题，Luna翻译标题，Sol原创comparing-languages.md并接通目录。实质比较Rust Future/独立Tokio任务、JS Promise/宿主事件循环、Python coroutine/Task、Go goroutine、C# Task的启动/执行/调度、取消/错误传播/任务所有权、资源清理与阻塞桥接，包含同一双请求业务场景的迁移检查，不只列表格。
- 校正Rust async体惰性不涵盖参数求值/同步准备/已spawn句柄；JS race/all不取消落败操作；Python固定3.14文档并标TaskGroup3.11/eager factory3.12/create_task eager_start3.14；Go context非强制退出或join且抢占有版本平台边界；C#正常Task同步前缀/上下文/ConfigureAwait/合作token/WhenAll与DisposeAsync规则。
- 实际打开ECMAScript/WHATWG规范、Mozilla async function、Node事件循环/worker_threads、Python3.14 asyncio任务/开发/free-threading、Go FAQ/spec/1.14runtime/context、Microsoft TAP/场景/取消/异步返回/WhenAll/DisposeAsync/CTS Dispose及.NET ConfigureAwait FAQ官方资料。未把未运行的他语言行为或性能排名称为实测。
- 本章是文档比较，复用既有已测Rust源码章节链接，没有新增无意义测试或依赖，也未安装运行其他语言。独立crate保持101测试，父代理已实际全量通过；旧工作区13测试同过，总114。
- mdbook `.tools/round4-chapter-24-reviewed`、全部前24章加导言共25篇HTML/链接/锚点/未完成标记、git diff --check通过。正文与目录之外不扩展第25章rustc、structured/旧章节/navigation；不查询、监听或等待CI。

## 第五轮（2026-10-26）：第 25 章，rustc 中 async/await 的实现

- 上游只有英文目录标题，GPT-6 Luna翻译标题，GPT-6.1 Sol原创rustc-async-await.md并接通目录。区分语言规范与具体rustc实现，覆盖惰性/参数捕获、HIR opaque返回与协程/THIR/MIR、IntoFuture和Pending挂起、跨挂起保存与析构标志、状态依赖Drop、Pin自借用与Send、布局/优化非ABI保证及运行时职责。
- 新增共享源码rustc_async.rs并include：真实自唤醒Pending、两阶段资源、跨挂起借用和挂起前结束Rc作用域。2测试通过：构造/未poll及两个挂起点/正常完成各路径资源释放与唤醒次数；内部借用恢复及scoped_rc的Send编译约束。累计103测试，本章只过滤新增2个，无新依赖。
- 使用既有稳定Rust1.96.0在Windows实际执行`rustc --emit=mir`，观察匿名Future、Pin/Context参数、discriminant/switchInt、状态3/4与Ready完成。产物在.tools，不将编号/字段作为稳定ABI；没有nightly、-Z、RUSTC_BOOTSTRAP、工具链安装或编译rustc本身。
- 实际打开Reference函数/await/async块、RFC2394、rustc-dev-guide MIR construction/coroutine-closures、Future/Pin和官方rustc await降低及coroutine转换源码。在线编译器文档对应1.99.1，明确区别于本机1.96.0实测，不宣称所有版本/平台/优化及panic展开组合已测。
- cargo fmt --check、mdbook `.tools/round5-chapter-25`、前25章加导言共26篇HTML/链接/锚点/未完成标记及git diff --check通过。不查询、监听或等待CI。

## 第五轮（2026-10-26）：第 26 章，结构化并发

- GPT-6 Luna完整逐段翻译原153行长章，GPT-6.1 Sol技术校订并实质补全全部未完成项：正式导读/任务树图、所有权/借用与资源推理、顶层边界价值、有限任务监督者案例、三个crate、作用域任务困难。保留原12标题HTML锚点、10处旧外链及3脚注，未以摘要替代正文。
- 校正Rust并非只能无结构（thread::scope）、取消非瞬间/外部工作不自动结束、Drop可同步发信号但不能await、select与协作取消可配合、JoinSet drop仅请求abort、panic/业务Result两层、守卫/独立任务及父Future强制取消的保证边界；解释mem::forget与借用安全责任，AsyncDrop仍nightly。
- 新增共享structured.rs与正文include，2个确定性测试通过：显式关闭等待异步清理许可与真实退出；业务错误和子任务panic分别让兄弟收到停止，但在清理许可前监督者仍Pending且资源仍在用，之后确认析构与退出。累计105，本章只过滤新增2测试，无新依赖。测试中预期子任务panic由JoinError捕获，测试总体成功。
- 案例仅一次spawn有限Vec任务、仅返回首个错误，不是网络/持久化服务；supervise自身外部drop/abort时JoinSet与同步守卫不保证子任务异步清理。moro0.4/async-nursery0.6仅核查官方仓库/API未安装运行，futures-concurrency复用既有示例；未对实验AsyncDrop或任意scoped实现安全性作实测承诺。
- 实际打开Tokio JoinSet/JoinHandle/select、标准库scope/forget/AsyncDrop、Trio nursery和moro/async-nursery/futures-concurrency官方资料。cargo fmt --check、mdbook `.tools/round5-chapter-26`、前26章加导言共27篇HTML/链接/锚点/未完成标记、旧锚点外链脚注保留比对与git diff --check通过。不查询、监听或等待CI。

## 第五轮（2026-10-26）：第 27 章，入门（旧版总导读）

- GPT-6 Luna完整翻译原欢迎与书籍目标、三个学习阶段和结束引导，GPT-6.1 Sol校审；保留getting-started/what-this-book-covers两个旧标题锚点并翻译目录。
- 明示旧版入口并补新版指南/参考链接；实际打开当前Reference async函数与Future官方文档作为语言行为依据，不将旧版“及时全面”目标写成全部旧章均已更新的承诺。
- 本章纯导读，不新增源码、依赖或无意义测试；独立crate保持105测试。mdbook `.tools/round5-chapter-27`、前27章加导言共28篇HTML/链接/锚点/未完成标记与git diff --check通过。不查询、监听或等待CI。

## 第五轮（2026-10-26）：第 28 章，为什么要用异步？

- GPT-6 Luna完整翻译原正文及旧例英文注释，GPT-6.1 Sol逐段校订；6旧标题id、原std::thread外链和两处源码include保留。并发模型、Rust实现差异、线程取舍、下载结构和混用模型全部覆盖。
- 校正线程池/性能数量级绝对化、协程与async/actor/事件驱动层次并非互斥、Future惰性与参数/独立任务边界、drop不回滚外部工作、零成本非整体零分配、运行时线程/配置及二进制体积非必然增大；补官方依据。
- 旧examples/01_02_why_async/src/lib.rs只翻译注释/panic提示并把两下载桩明确标为历史占位，不发网络请求、无真实Pending，原2测试实际通过但仅证明控制结构编译运行，不算网络下载/并发吞吐实测；未访问foo.com/bar.com。无需新增重复测试，独立crate保持105，旧工作区仍原13。
- 实际打开std::thread/Future、Reference async函数/块、Tokio runtime/async教程及futures join/block_on官方资料。Rust1.96下该例cargo test --offline --locked -p通过，指定包cargo fmt --check、mdbook `.tools/round5-chapter-28`、前28章加导言共29篇HTML/链接/锚点/未完成标记、旧锚点链接include比对和git diff --check通过。不查询、监听或等待CI。

## 第五轮（2026-10-26）：第 29 章，异步 Rust 的现状

- GPT-6 Luna完整翻译原各段/8节标题和列表，GPT-6.1 Sol技术校订；8旧标题id与原Future/futures外链、旧生态章节链接完整保留。性能/维护、语言与库、编译/运行时/新故障、兼容性、单线程与延迟场景均覆盖，不作性能排名保证。
- 当前官方核查明确async trait自1.75稳定、静态与dyn分发/返回Future Send边界；AsyncIterator与AsyncDrop仍nightly-only（标feature/日期）；async-std README已宣布停止维护并建议smol。标准库在线1.99.1与本书本机1.96的区别注明，不安装实验工具链。
- 修正同步函数可调用asyncfn构造Future但不能直接await、桥接生命周期/嵌套block_on、标准Future协议不等于运行时服务兼容、Tokio依赖不意味着每API需runtime（sync原语及_timeout例外）、futures/Tokio IO trait和旧0.1协议适配，避免选定runtime后无需再检查的旧说法。
- 实际打开Rust官方1.75发布文章/Reference dyn规则、Future/AsyncIterator/AsyncDrop、async-std官方README、futures/compat、Tokio runtime/sync/spawn/LocalSet/bridging与smol LocalExecutor资料。本章纯文档无新增测试/依赖，独立crate仍105；未运行性能或实时性基准。
- mdbook `.tools/round5-chapter-29`、前29章加导言共30篇HTML/链接/锚点/未完成标记、旧锚点外链保留比对与git diff --check通过。不查询、监听或等待CI。

## 第五轮（2026-10-26）：第 30 章，async/.await 入门

- GPT-6 Luna完整翻译旧Primer正文和所引旧源码英文注释，GPT-6.1 Sol校订并补充共享Pending事件示例；原唯一标题id及4个include保留，Cargo行9:10仍准确指向dependencies/futures。
- 解释async不自动消除同步阻塞、根执行器与子Future、IntoFuture转换、子Future真正Pending才交还控制、join同任务并发非独立spawn/多核并行；保留外部逐次block_on示例，修正内部嵌套当前futures executor会被enter检查拒绝并panic的原错误，核对官方与本机0.3.34源码，不制造死锁。
- 旧examples/01_04_async_await_primer/src/lib.rs注释已完整中文并明示learn/sing/dance立即Ready桩不提供交错证据；旧3测试运行通过（原旧13中已有，不新增计数）。新增共享primer.rs复用第25章YieldOnce，1确定性测试检查两次真实Pending、学会才唱、跳舞在学习/唱歌等待中推进及最后完成。累计106独立测试，本章仅过滤新增1，无新依赖。
- 实际打开Reference await、IntoFuture、futures block_on/join和futures-executor local_pool官方源码；新例为手动poll/无操作Waker的确定性无IO演示，不宣称音乐/动画/真实计时/网络/性能基准或一般执行器调度次序。
- Rust1.96，旧包cargo test --offline --locked -p、旧包及独立crate cargo fmt --check、mdbook `.tools/round5-chapter-30`、前30章加导言共31篇HTML/链接/锚点/未完成标记、旧锚点/include保留比对与git diff --check通过。到本章停止，不修改02_execution/navigation及更后章节；不查询、监听或等待CI。
## 第六轮（2026-10-27）：第 31 篇，底层原理：执行 Future 与任务

- GPT-6 Luna 完整翻译原两段短导读，GPT-6.1 Sol 技术校审；保留原英文标题 id `under-the-hood-executing-futures-and-tasks`，同步 SUMMARY 中文标题。原文无外链、脚注、include 或未完成项；原文快照存于忽略目录 `.tools/round6-original-02_execution-01_chapter.md`。
- 补充简短协议导读：区分 Future 与执行器任务，说明 await 不自动生成独立任务、唤醒不等于完成或立即运行、poll 应快速返回；将跳读及下一节变成实际章节链接，没有扩写为教程。
- 实际打开标准库 Future 与 Waker 官方文档并附正文链接；仅涉及稳定协议，不引入在线文档较新版本的 API。
- mdBook 0.5.4 构建 `.tools/round6-chapter31-reviewed`、指定本篇的 `check_chapters.py` 与 `git diff --check` 通过。生成 HTML 保留旧英文标题锚点；原文无外链/include/脚注需要迁移。本章无代码示例，不新增或重跑代码测试，不将既有 119 测试记作本次实测。不查询、监听或等待 CI。
## 第六轮（2026-10-27）：第 32 篇，Future trait

- GPT-6 Luna 完整翻译正文及 `examples/02_02_future_trait/src/lib.rs` 英文注释，GPT-6.1 Sol 技术校订；保留原 `the-future-trait` 标题 id 和全部 5 个 include，SUMMARY 同步中文。原文无外链或脚注。
- 保留简化协议、SocketRead、Join、AndThenFut 和真实 trait 五部分，校正快速非阻塞 poll、唤醒仅提示重试/可合并、同任务并发不等于线程并行、组合步骤无额外堆分配不等于整体零分配、完成后不应再 poll、fn 指针不能捕获任务环境、最新 Waker、Pin/Unpin 与字段投影边界。
- 明示 Socket 恒真检查/空向量/noop 回调是历史教学占位，解释检查与注册之间的竞争及真实非阻塞 I/O 的 EOF/错误/重试责任；源码注释同步标注占位，没有把它当真实网络验证，也没有更改源码行为。AndThenFut 预持两个 Future 的限制保留。
- 实际打开标准库 Future、Waker、pin 官方资料并在正文链接；未使用在线较新文档新增 API。用户已有 Rust 1.96.0 对旧包执行 `cargo test --offline --locked -p example_02_02_future_trait` 编译成功，单元及文档测试均为 0，保留既有 dead_code 警告；不把编译称作网络/并发实测，不新增凑数测试。
- mdBook 0.5.4 `.tools/round6-chapter32-final`、指定本篇 HTML/链接/锚点检查和 `git diff --check` 通过。首次检查发现新增固定章节链接路径错误，已接入已完成的 `part-reference/pinning.md` 后重新构建检查通过。不查询、监听或等待 CI。
## 第六轮（2026-10-27）：第 33 篇，使用 Waker 唤醒任务

- GPT-6 Luna 完整翻译正文及 Timer 源码英文注释，GPT-6.1 Sol 校订；两个原英文标题 id 与全部 4 个 include 保留，SUMMARY 同步中文。
- Timer 做最小锁边界修复：首次检查后锁外 clone，再锁内复查/替换；旧及未使用句柄锁外 drop；完成线程锁内 set/take，锁外 wake。避免自定义 Waker 回调重入共享状态锁，复查防止克隆期间完成却丢失通知。所有 helper 都随原 timer_new include 展示，没有断开的示例。
- 新增原旧包 2 个确定性测试：RawWaker 探针在 clone/drop/wake 中 try_lock 验证锁已释放，检查只通知最新 Waker、已完成轮询不再克隆；clone 钩子精确模拟完成落在检查/注册之间，以及首次 poll 前完成。RawWaker 的 Arc 强计数/线程安全不变量有 SAFETY 注释；测试用 Weak 避免状态/句柄循环。未声称覆盖所有线程交错或运行 Miri。
- Rust 1.96.0 的该包 `cargo test --offline --locked` 实际 3 测试通过：原有 1 秒真实线程测试及新增 2 个，无警告。新增仅 2 个，总已有测试计数应为 121（独立 crate 106，旧 workspace 15）；本次未重跑这全部 121 个。指定包 cargo fmt 已执行，无新依赖。
- 解释一计时器一线程、同步 new 即启动、drop 不取消线程、没有创建失败/锁中毒恢复的教学边界；Future 不等于任务本身、Pin 不禁止转移固定句柄、唤醒可合并且不保证结果就绪。实际核查标准库 Waker/RawWakerVTable/Wake，并附 Future 协议资料。
- mdBook `.tools/round6-chapter33-final`、指定本篇 HTML/链接/锚点检查与 git diff --check 通过。没有查询、监听或等待 CI。
## 第六轮（2026-10-27）：第 34 篇，实践：构建一个执行器

- GPT-6 Luna 全文及源码注释翻译，GPT-6.1 Sol 技术校订；原标题 id、全部 6 个 include 与原参考式章节链接均保留，SUMMARY 同步中文。
- 最小修复旧 executor：有界 sync_channel/try_send expect 改无界 channel/send，唤醒忽略接收端断连；spawn 对关闭仍明确 panic。run 短锁取 Future、锁外 poll/完成析构，Pending 再存回。单消费者保证先存回再处理 poll 期间的自唤醒，重复已完成消息跳过。没有冒称 bounded try_send 会阻塞死锁。
- 校订 LocalWaker 为 Waker、ArcWake 的 waker/waker_ref API、Mutex 的跨线程共享意义和 UnsafeCell 证明责任；补计时器导入/同包 lib+main 复现方式、Future 构造外部工作边界、队列无界增长/无去重/无预算/无关闭取消协议和潜在自持 Waker 引用环。准确说明 recv 等待全部发送端释放且队列排空，外部保留 Waker 也能延长 run。
- 原旧包新增 2 个回归：poll 内 10,001 次唤醒后 Pending 能恢复，随后 Ready 的重复排队不再 poll；关闭接收端后迟到 wake_by_ref/wake 不 panic。Rust1.96.0 该包离线锁包测试实际 3 项通过，包含原两秒 Timer 集成测试，无警告；不作为性能或任意多消费者正确性证明。累计已有 123 项（独立 106、旧 workspace17），本次不称全部重跑，无新依赖。
- 实际核查标准库 channel/Receiver::recv；docs.rs ArcWake 页面工具不可访问，改读取本机已锁 futures-task0.3.34 的 arc_wake.rs/waker_ref.rs 确认 API，不把失败网页当成功浏览。
- 指定包 cargo fmt 已执行；mdBook `.tools/round6-chapter34-reviewed`、指定本篇 HTML/链接/锚点检查及 git diff --check 通过。没有查询、监听或等待 CI。
## 第六轮（2026-10-27）：第 35 篇，执行器与系统 IO

- GPT-6 Luna 完整翻译正文及两块内嵌伪代码注释，GPT-6.1 Sol 技术校订；原标题 id、SocketRead include、mio 外链与原 Future 章节参考式链接全部保留，SUMMARY 同步中文。
- 校正 readiness 与 completion，Windows IOCP 完成通知由 Mio 适配为就绪接口；保留 Fuchsia port 历史主题并引用官方接口，但当前核查的 Mio1.2.4 支持列表不含 Fuchsia，不再断言所有旧平台均被当前 Mio 支持。
- 保留兴趣注册/事件ID/分发与回调映射两块完整伪代码主题，明确不是可编译系统绑定、前文 Socket 仍为教学占位；补最新 Waker/注册复查/先发布状态与锁外通知、多个等待者/注销/ID复用责任。说明伪就绪、非阻塞 drain 到 WouldBlock/EOF/实际错误，以及单次上层读取或公平预算必须保留就绪/安排继续处理。
- 实际打开 Mio官方仓库与Poll1.2.4文档、Microsoft IOCP资料，并通过Fuchsia官方源码/系统调用搜索核查port；直接Fuchsia旧页面不可访问，不把访问失败当成功。接入已完成的 io-models/io 章，不重写操作系统驱动。
- mdBook `.tools/round6-chapter35-reviewed`、指定本篇 HTML/链接/锚点与 git diff --check 通过。本章纯文档无新增测试/依赖，没有执行Linux/IOCP/Fuchsia系统验证或性能基准，没有把伪代码算实测；既有累计测试计数不作本篇重新执行。没有查询、监听或等待CI。
## 第六轮（2026-10-27）：第 36 篇，async/.await

- GPT-6 Luna 完整翻译正文及 `examples/03_01_async_await/src/lib.rs` 注释，GPT-6.1 Sol 技术校订；原 4 标题 id 与 4 个 include、原 Primer 参考式链接保留，SUMMARY 标题统一为正文的 `.await` 写法。
- 保留语法、生命周期、所有权/move、多线程四主题，补 IntoFuture/立即Ready不必让出/普通await不spawn，参数与外部工作不受体惰性保证；good内部局部所有权与bad栈借用区别、static不等于存活到进程退出、Copy/Arc共享、move引用仍受借用限制、构造捕获和跨挂起状态共同决定Send，以及LocalSet本地非Send任务。
- 校订标准Mutex并非一概不可用：短且不跨await临界区可用，跨await阻塞竞争/Guard非Send与逻辑死锁需区分；异步锁也不保证无死锁。实际打开Reference await/async块、Tokio共享状态及LocalSet官方文档，接入已有同步/Primer真实Pending/rustc Send章节。
- 实际使用Rust1.96.0执行3个最小编译负例：提取原bad拒绝（E0597）、async move借用不能成为static、async fn捕获Rc即使await前drop仍非Send；另1个正例编译并运行，验证拥有内部局部数据与await前独立作用域Rc满足Send/static。产物/诊断在 `.tools/round6-36-compile`，不纳入单元测试计数。
- 旧包离线锁包 cargo test 编译成功，单元/文档测试均0；保留既有 never_compiled unexpected_cfgs 警告，不把包成功当作负例验证。无新增测试/依赖，不重跑已有完整测试，不将立即Ready示例称作调度实测。
- mdBook `.tools/round6-chapter36-final`、指定本篇HTML/链接/锚点与git diff --check通过。保留父拥有的check_chapters脚本改动；本轮到此止，没有查询、监听或等待CI。

## 第七轮（2026-10-28）：第 37 篇，Stream trait

- GPT-6 Luna 完整翻译正文及 `examples/05_01_streams/src/lib.rs` 英文注释，GPT-6.1 Sol 技术校订；原标题 id `the-stream-trait` 与全部 2 个 include 保留，SUMMARY 同步中文。原文无外链、脚注、参考式章节引用或未完成项。
- 补充 poll_next 的 Pending/Waker、逐项就绪与结束三态协议，结束后不应继续轮询及 fuse 适配；明确所示为省略 size_hint 的核心接口。修正通道为全部 Sender（含克隆）释放且缓冲排空后结束，并说明 close 后排空、next 的 Future/Unpin 要求、有界通道背压与同任务先发后收的容量限制。
- 实际打开 futures-core 0.3.31 官方 Stream 文档并附链接；futures-channel/StreamExt 网页工具无法访问，改查本机锁定 futures-channel/futures-util 0.3.34 源码确认 close、poll_next 与 next 要求，不把失败网页当成功浏览。接入已完成 Stream 与 Pinning 章节。
- Rust 1.96.0 对旧包执行离线锁包 cargo test，既有通道测试 1 项通过、文档测试 0；保留教学 Stream trait 未使用的 dead_code 警告。无行为修改、新测试或新依赖，不把该立即就绪示例称作调度/挂起实测，也未重跑既有全部 123 项测试。
- mdBook `.tools/round7-chapter37-reviewed`、指定本篇 HTML/链接/锚点检查、原两处 include 比对与 git diff --check 通过。仅完成本篇，等待父验收、commit 与 API 上传成功后才开始下一篇；不查询、监听或等待 CI。

## 第七轮（2026-10-28）：第 38 篇，迭代与并发

- GPT-6 Luna 完整翻译正文与 `examples/05_02_iteration_and_concurrency/src/lib.rs` 原三处英文注释，GPT-6.1 Sol 技术校订；原标题 id `iteration-and-concurrency` 与全部 2 个 include 保留，SUMMARY 同步中文。原文无外链、脚注、参考式章节引用或未完成项。
- 保留组合器、命令式迭代与并发处理三个主题，将不存在的 TryStreamExt::try_map 校正为 map_ok/and_then，并显式说明校订原因；区分错误元素转发和消费操作提前返回。补 next/try_next 的结果形状、? 与固定可变引用的 Unpin 边界。
- 核查同任务并发而非 spawn/线程并行、limit 1/0/Some(0)/None 语义、有界背压及无限并发风险、单元素内两步顺序与跨元素顺序不保证；说明源/子 Future 错误快速返回并丢弃内部未完成操作，借用的源流仍属调用方，但已消费元素不回放、外部副作用不回滚。源码与正文都明示 jump/report 是立即成功的教学占位，不伪造真实交错。
- 实际打开 futures 0.3.34 的 StreamExt/TryStreamExt 官方文档并附正文链接，同时读取本机锁定 try_for_each_concurrent/for_each_concurrent 源码核查 limit 转换与错误清空行为。接入已完成 Stream 与取消安全章节。
- Rust 1.96.0 该旧包离线锁包 cargo test：既有 2 项求和测试通过、文档测试 0，无警告；无代码行为修改、新测试或依赖。guide-tests 已有 streams_buffer_order_and_concurrency_limit，父本轮已完整跑过其 106 项，本篇不重复执行，也不把两项求和测试冒称并发/取消实测。
- mdBook `.tools/round7-chapter38-reviewed`、指定本篇 HTML/链接/锚点检查、原两处 include 比对及 git diff --check 通过。保留父拥有的 check_chapters.py 改动；仅到本篇，等待父验收、commit 与 API 上传成功后再继续；不查询、监听或等待 CI。

## 第七轮（2026-10-28）：第 39 篇，同时执行多个 Future

- GPT-6 Luna 完整翻译原两段与四项导读，GPT-6.1 Sol 技术校订；保留原标题 id `executing-multiple-futures-at-a-time`，SUMMARY 同步中文。原文无源码、include、外链、脚注、参考式章节引用或未完成项。
- 校正 await 尚未就绪时挂起任务而非阻塞线程/自动 spawn，join/select 在当前任务并发轮询；spawn 的完成/取消/关闭取决运行时与 API；FuturesUnordered 必须由流消费驱动且加入 Future 不自动 spawn。保留四项主题并接有效章节链接，用简短段落说明并发与线程并行、观察顺序与严格物理完成顺序的区别，没有扩成完整教程。
- 实际打开 futures 0.3.34 的 join/select/FuturesUnordered 官方文档并附链接；固定版本 URL 工具失败后改 latest 成功，返回版本确为 0.3.34。另查本机锁定 FuturesUnordered 源码说明验证 push 不 poll 与需要 poll_next 驱动。
- mdBook `.tools/round7-chapter39-reviewed`、指定本篇 HTML/链接/锚点检查与 git diff --check 通过。本篇纯导读，无新增测试/依赖，也未重跑代码测试；保留父拥有的 check_chapters.py 改动。到此篇停止，等待父验收、commit 与 API 上传成功后继续；不查询、监听或等待 CI。

## 第七轮（2026-10-28）：第 40 篇，join!

- GPT-6 Luna 完整翻译正文及 `examples/06_02_join/src/lib.rs` 原英文注释、占位注释与错误字符串，GPT-6.1 Sol 技术校订；原 3 个标题 id（join、join-1、try_join）和全部 5 个 include 保留。SUMMARY 原有宏名标题已与正文一致，无需更改；原文无外链、脚注、参考式章节引用或未完成项。
- 保留顺序等待、其他语言对照、join、try_join 与统一错误类型全部主题；修正顺序不必然慢、async 函数体等待轮询而非只能 await 驱动、已启动后台工作例外。同任务 join 不 spawn/不自动线程并行、输出按参数顺序、Err 不使 join 提前结束；try_join 观察到错误即返回，owned 未完成 Future 被丢弃，borrowed 只释放借用，不回滚副作用/不保证终止后台任务。
- 明示 get_book/music 均为立即就绪占位、Result 版本只返回 Ok，不能作为耗时或交错验证；正文接已完成组合与取消章节。实际打开 futures 0.3.34 join/try_join 官方资料与 Rust Reference，核查本机锁定 futures-macro 的 join.rs，没有混用 Tokio 宏调度语义。
- 旧包新增 1 项确定性测试：显式先将探针 poll 至 Pending，再分别验证 futures::try_join 错误返回时 owned 探针析构、borrowed 探针仍可继续 poll 且调用方 drop 才析构；不依赖宏分支轮询次序，无性能/任意线程交错断言。Rust 1.96.0 离线锁包 cargo test 实际 1 项通过、文档测试 0；保留原教学函数未使用的 13 条 dead_code 警告。累计已有 124 项（独立 106、旧 workspace 18），本篇未重跑全部；无新依赖。
- 指定包 cargo fmt 已执行；mdBook `.tools/round7-chapter40-reviewed`、指定本篇 HTML/链接/锚点、原 5 include 比对与 git diff --check 通过。保留父拥有的 check_chapters.py 改动；到此篇停止，等待父验收、commit 与 API 上传成功后继续；不查询、监听或等待 CI。

## 第七轮（2026-10-28）：第 41 篇，select!

- GPT-6 Luna 全文及 `examples/06_03_select/src/lib.rs` 原英文注释/输出翻译，GPT-6.1 Sol 技术校订；保留原 4 标题 id 与全部 5 include，默认/完成、固定与融合、流、terminated Fuse 替换、Unordered 主题完整。SUMMARY 原宏名已一致，无需更改；原文无外链、脚注、参考式章节引用或未完成项。
- 校订 futures::select 同任务轮询/多就绪伪随机选择，输出改为“被选中”避免物理最先完成误解；局部丢弃与外部后台任务/副作用区别。default 选择不挂起不保证 handler/poll 非阻塞，忙等风险与 complete 优先明确；路径借用要求 Unpin、直接表达式栈固定例外、Pin句柄而非内部Future变Unpin、循环外fuse与流next状态保留完整解释。
- 说明 FusedFuture 终止不一定曾返回值，pending 自带恒终止而 pending().fuse 可表示活跃等待；select_next_some 产生 Item、终止分支排除、误 poll 终止流会 panic，以及首次发现None先Pending/wake、下一次选择才排除。解释 Pin::set 原地析构旧值/同类型替换，Unordered 需要驱动、暂空后可push再激活、无并发上限、退出与取消责任。无限interval是输入假设、常量与空函数均教学占位，非真实timer/系统实测。
- 实际打开 futures 0.3.34 select/Fuse/StreamExt 与标准库 Pin::set 官方资料并附链接，读取本机锁定 select宏、pending、Fuse、select_next_some、FuturesUnordered 源码核查细节；标准库在线较新页面只引用既有稳定 set 语义，未引入新API。
- 新增 1 项确定性状态测试，验证 default、complete优先、terminated Fuse set 后重激活、直接async表达式固定，以及 Unordered 初空首次default/次轮complete、排空后push恢复；不依赖随机分支顺序。初跑先后发现 pending 恒终止与新空集合尚未报告终止两处测试前提错误，已依据源码和实测修正测试及正文，没有改库语义。
- Rust1.96 离线锁包最终 2 项通过（原count+新增1），文档测试0，保留原10条dead_code警告；累计已有125项（独立106、旧workspace19），本篇未重跑全部，无新依赖。指定包fmt/check已执行；mdBook `.tools/round7-chapter41-final`、单篇HTML/链接/4锚点、原5include比对及git diff --check通过。
- 保留父拥有的 check_chapters.py 改动；到此篇停止，等待父验收、commit 与 API 上传成功后继续，不查询、监听或等待CI。

## 第七轮（2026-10-28）：第 42 篇，生成任务（Spawning）

- GPT-6 Luna 完整翻译正文及 `examples/06_04_spawning/src/lib.rs` 原英文注释/输出，GPT-6.1 Sol 技术校订；保留原标题 id `spawning` 与全部 2 include，SUMMARY 同步中文。原文无外链、脚注、参考式章节引用或未完成项。HTTP正文保留原Hello World字节，避免翻译引入无关协议数据变化。
- 实质纠正原文“外层函数结束会取消任务”：async-std JoinHandle 丢弃是 detach，独立调度与是否await区分、await挂起任务非线程阻塞、Send/static及输出约束、任务结果错误观察、函数返回/运行时停止/进程退出不同、显式cancel().await与取消非抢占/副作用不回滚均说明。join_all 等待已spawn任务并按输入顺序收集，不把组合器当任务生成来源。
- 实际打开 async-std 1.13.2 spawn/JoinHandle 官方文档与仓库README；截至本次校订官方已明确discontinued并建议smol，正文标为历史示例，未新增/迁移依赖。读取锁定 async-std JoinHandle drop/cancel 与 async-task cancel 源码核实 detach及等待停止行为。
- 保留连接处理教学结构，仅将丢弃句柄下会静默丢失的process_request错误改为在子任务内eprintln观察；指出仅写固定HTTP字节，不解析请求/非完整HTTP服务器，普通async main需运行时入口、无限接受与无关闭/背压策略未被包装成生产实现。本次未运行该监听示例或声称网络实测。
- 新增1项确定性oneshot握手测试：任务已启动后丢句柄，再放行并收到完成结果；另在资源guard已构造的任务中显式cancel，验证None与析构通知。无墙钟竞速、无新依赖。Rust1.96离线锁包2项通过（原join_all三任务+新增1）、文档测试0，无警告；原3秒等待仅保留原测试，不作为性能比较。累计已有126项（独立106、旧workspace20），本篇未重跑全部。
- 指定包cargo fmt已执行；mdBook `.tools/round7-chapter42-reviewed`、指定本篇HTML/链接/锚点、原2include比对与git diff --check通过。保留父拥有的check_chapters.py改动；本轮六篇到此完成，未延伸后续TODO导航，等待父验收/commit/API上传与最终汇总；不查询、监听或等待CI。

## 第八轮（2026-10-29）：第 43 篇，取消与超时

- 上游仅有 SUMMARY 中 Cancellation and Timeouts 空目录项，无英文正文、旧锚点、include 或外链。GPT-6 Luna 仅翻译标题，GPT-6.1 Sol 原创完整教程与示例；正文明确归因，新增 cancellation-and-timeouts 锚点及两处源码 include，SUMMARY 仅替换当前一项，未开始下一项。
- 教程解释 owned/borrowed Future 生命周期、取消不回滚与取消安全、timeout 的两层 Result/总预算、运行时定时器前提、协作调度与非硬实时边界、Tokio JoinHandle 超时分离/保留后 abort+await、关闭通知与异步清理、排空队列及已取得发送许可边界。公开借用示例同时处理首轮 Ok/Err，不把一秒期限误写成真实调度必然超时的契约。
- 实际打开 Tokio 1.54.1 timeout、JoinHandle、select 官方文档，并读取本机锁定 timeout.rs 核实先 poll 工作再检查期限；接入既有取消安全、定时器与清理章节，无新依赖、无既有参考模块行为变更。
- 新增 examples/guide-tests/src/legacy_timeouts.rs 四项行为测试：拥有的工作超时析构且保留副作用、借用超时后恢复、拥有句柄超时后任务继续以及保留句柄中止后析构、关闭后缓冲排空并 join。超时测试使用暂停时间，握手保证任务启动；没有墙钟竞速或真实 I/O。首轮默认工具链实际为 1.94.1，后明确使用 cargo +1.96.0，最终四项通过、106 项过滤、无警告，累计 130 项（独立 110、旧 workspace 20）；未声称运行全量。
- 最终 cargo +1.96.0 fmt --check、mdBook `.tools/round8-chapter43-final2`、指定单篇 HTML/本地链接/锚点检查与 git diff --check 通过。保留父拥有的 scripts/check_chapters.py 改动；到此停止，等待父验收、commit 与 API 上传成功后才处理第 44 篇；不查询、监听或等待 CI。

## 第八轮（2026-10-29）：第 44 篇，无序 Futures 集合

- 父确认第 43 篇 b510e4d7f173832375aaf44340607716d95d6db7 已 API 上传成功后开始本篇。上游仅有 FuturesUnordered 空目录项，无正文或旧锚点/include/外链；GPT-6 Luna 翻译中文说明标题，GPT-6.1 Sol 原创完整教程与示例并明确归因。新增 futuresunordered 锚点与两处源码 include，SUMMARY 只替换当前项。
- 教程覆盖同类型/异构装箱及 Send/生命周期选择、push 不 poll/不 spawn、next 消费驱动与唤醒、无序不代表严格物理先后、内部固定与集合 Unpin、空后重激活及 fuse 永久结束陷阱、Result 继续/短路策略与 owned/borrowed 取消、JoinHandle 分离、buffer_unordered 正数容量与已提前 spawn 的边界。给出完整可运行普通消费与限额消费函数，没有用链接代替教程。
- 实际打开 futures 0.3.34 FuturesUnordered 与 StreamExt 官方文档，读取本机锁定 futures-util 集合源码确认 push 重置终止状态、poll_next 驱动行为；无新增依赖，也未修改已有参考模块行为。
- 新增 legacy_unordered 四项确定性测试：push 尚未执行、首次消费真实 Pending、oneshot 控制后加入项先被观察完成、空后重新 push 与 fuse 记住 None；暂停时间下实际调用 collect_jobs 与 limited_jobs 并核对完整输出；buffer_unordered 首轮仅推进两项且消费一项后才接纳第三项；try_collect 错误返回销毁已明确 Pending 的 owned 工作。测试不依赖多个同时就绪分支的顺序，不冒称真实网络/调度实测。
- Rust 1.96.0 显式 cargo +1.96.0 离线锁包最终四项通过、110 项过滤，无警告；累计 134 项（独立 114、旧 workspace 20），未重跑全量。cargo +1.96.0 fmt --check、mdBook `.tools/round8-chapter44-final`、指定单篇 HTML/链接/锚点及 git diff --check 通过。
- 父审后将枚举的分配说明收窄为“避免统一异构类型的额外装箱”，明确集合节点仍可能分配；重建 `.tools/round8-chapter44-parent-reviewed` 并通过指定单篇检查。保留父拥有的 scripts/check_chapters.py 改动；完成本篇即停止，等待父验收、commit 与 API 上传成功才处理第 45 篇；不查询、监听或等待 CI。

## 第八轮（2026-10-29）：第 45 篇，值得了解并善用的变通方案

- 父确认第 44 篇 22a3a42a6077a25fe3678e578a05b94fc7cef429 已 API 上传成功后开始本篇。GPT-6 Luna 完整翻译原标题与原段落，GPT-6.1 Sol 校订历史时态；保留 workarounds-to-know-and-love 旧英文锚点，SUMMARY 同步中文。原文无源码、include、外链、脚注或参考式引用。
- 保留功能开发与诊断不足的历史背景及介绍常见问题/变通方法的主题，明确 async/await 自 Rust 1.39（2019 年）已稳定，不再把“仍相对较新”当现状。实际打开 Rust 官方稳定公告并附链接；简短接入 Send、递归、trait async 三节，没有扩成教程。
- 本篇仅导读文字调整，无新增测试/依赖、未运行代码测试。mdBook `.tools/round8-chapter45-reviewed`、指定单篇 HTML/链接/旧锚点检查及 git diff --check 通过。保留父拥有的检查脚本改动。
- 到此停止，等待父验收、commit 与 API 上传成功才处理第 46 篇；不查询、监听或等待 CI。

## 第八轮（2026-10-29）：第 46 篇，Send 的近似判定

- 父确认第 45 篇 13daedd590d8faa5387fdd3e61c6cfce8212eeeb 已 API 上传成功后开始本篇。GPT-6 Luna 全文翻译原标题与所有段落，原代码无英文注释；GPT-6.1 Sol 技术校订。保留 send-approximation 旧锚点，SUMMARY 同步中文；原文无 include、外链、脚注或参考式引用。
- 完整保留临时值、命名局部跨 await、显式 drop 与内层作用域主题。实际 Rust 1.96.0 edition2018 编译证实本地创建的 NotSend 显式 drop 后可满足 Send，明确修正原“drop 仍不工作”旧结论；命名值存活跨 await 仍拒绝。旧 generator/GenFuture 内部类型诊断替换为当前实际诊断摘录，不把历史输出当现状。
- 补充初始捕获的非 Send 参数即使 await 前 drop，尚未轮询的 Future 也持有它，因此仍非 Send；给出 compile_fail 例并说明同步预处理/本地执行边界。区分局部生命周期、编译器分析精度、Send 与 static，并接入已有捕获/Send 章节。
- 实际打开标准库 Send 官方文档与 Rust Reference 异步函数参数捕获规则；在线标准库为较新版本，仅用于稳定语义，具体代码结论均以本机 Rust 1.96.0 为准。提取最终正文全部 6 个 Rust 块，用 rustc +1.96.0 edition2018 编译：4 个通过、2 个预期失败。源码、真实诊断及结果 JSON 存在 `.tools/round8-send-check`；没有执行这些 Future、没有新增单元测试或依赖，不增加累计测试数。
- mdBook `.tools/round8-chapter46-reviewed`、指定单篇 HTML/链接/旧锚点及 git diff --check 通过。保留父拥有的检查脚本改动；到此停止，等待父验收、commit 与 API 上传成功才处理第 47 篇；不查询、监听或等待 CI。

## 第八轮（2026-10-29）：第 47 篇，递归

- 父确认第 46 篇 f26c9f554da80015190a4f73cc8a58367ceab130 已 API 上传成功后开始本篇。GPT-6 Luna 全文翻译及概念代码英文注释翻译，GPT-6.1 Sol 技术校订；保留 recursion 旧锚点、原 2 个 include、Rust 1.77 公告与 PR117703 两个原外链及参考式定义，修正译后中文标签到原定义的显式映射。SUMMARY 同步中文；原文无脚注。
- 保留状态机/无限大小、历史 BoxFuture 返回 async 块与现代 Box::pin 递归全部主题。明确手写枚举只是有意不可编译的概念伪码，用 ignore 标识；另列仅无间接递归的 compile_fail 例，本机 rustc +1.96.0 edition2018 实际拒绝并生成 E0733，源码/诊断保存 `.tools/round8-recursion-check`。
- 实际打开 Rust 1.77 官方公告确认间接递归稳定；旧 html 链接重定向后成功，GitHub PR117703 网页工具两次 cache miss，保留链接但未声称阅读成功。说明 BoxFuture 的 Send/static 是具体别名附加条件，非异步递归所需；Box::pin 不消除深层 poll/析构调用栈风险。
- 原 recursive/recursive_pinned 均无基本终止条件，只编译不运行，正文明确第二次调用无法到达与栈/内存风险。新增 count_levels 有界递归、第三处 include 及 1 项测试，验证深度 0/8 的基本分支与计数。Rust1.96 显式 cargo +1.96.0 离线锁包该旧包 1 项通过、文档测试 0，无警告/新依赖；累计 135 项（独立 114、旧 workspace 21），未重跑全量。
- cargo +1.96.0 fmt --check、mdBook `.tools/round8-chapter47-reviewed`、指定单篇 HTML/链接/旧锚点及 git diff --check 通过。保留父拥有的检查脚本改动；到此停止，等待父验收、commit 与 API 上传成功才处理第 48 篇；不查询、监听或等待 CI。

## 第八轮（2026-10-29）：第 48 篇，Trait 中的 async

- 父确认第 47 篇 a293116f47964c8292eb4c251e5309a5b6b19281 已 API 上传成功后开始本篇。GPT-6 Luna 全文翻译原三段及最后更新链接，GPT-6.1 Sol 技术校订与示例补写；保留 async-in-traits 旧锚点、2022 nightly/async-trait 仓库/2023 稳定公告原三 URL，SUMMARY 同步中文。原文无源码、include、脚注或参考式定义；本次新增三处源码 include。
- 更新原稳定 Rust 不支持 async trait 的过时说法为 1.75 已稳定；保留 nightly 与宏变通历史。解释原生静态分发不要求每次额外装箱、方法本身仍可分配；泛型接口的返回 Future Send 保证与实现类型 Send 不同、借用生命周期与任务 static 条件、原生 dyn 兼容限制及 Self:Sized 不能让该方法动态调用；展示手动 BoxFuture 类型擦除与局部借用。
- 完整保留原开销主题，明确装箱/动态分派针对具体方案，取消“绝大多数应用不显著”及每秒调用次数阈值式普遍结论，建议实际测量。实际打开官方 2023 稳定公告、2022 nightly 文（旧 html 跳转后成功）、Rust Reference 当前 dyn 规则及 async-trait 仓库；解释宏常见改写、Send/?Send与约束，未新增宏依赖或声称运行宏。
- 新增 legacy_async_traits 模块与 1 项测试，验证原生泛型调用、显式 Future+Send 契约后 spawn、借用局部 String 的手动 boxed dyn 调用均返回相同结果；它们是立即就绪接口示例，不是 I/O、线程迁移或性能实测。私有教学接口普通库构建不调用，限定模块允许 dead_code，并非隐藏行为测试失败。
- 显式 rustc +1.96.0 edition2021 两个负例分别实测 E0277（仅实现类型 Send 不保证返回 Future Send）、E0038（原生异步方法不支持 dyn），源码与真实诊断保存在 `.tools/round8-traits-check`。cargo +1.96.0 离线锁包最终该模块 1 项通过、114 项过滤、无警告；累计 136 项（独立 115、旧 workspace 21），本篇未运行全量。
- cargo +1.96.0 fmt --check、mdBook `.tools/round8-chapter48-reviewed`、指定单篇 HTML/链接/旧锚点及 git diff --check 通过。保留父拥有的检查脚本改动；本轮六篇到此停止，等待父验收、commit/API 上传及最终全量汇总，不延伸 ecosystem 等后续页；不查询、监听或等待 CI。

## 第九轮（2026-10-30）：第 49 篇，异步生态系统

- 以第八轮最终提交 ffc5b97e474ec310918fc131207b81d73bb448ab 为起点。GPT-6 Luna 完整翻译原所有标题、段落及列表，GPT-6.1 Sol 技术校订；保留原七个英文锚点与全部原 URL，SUMMARY 同步中文。原文无源码、include、脚注或参考式定义。
- 实际打开 async-std 官方文档，确认 1.13.2 文档明确宣布停止维护并建议迁移 smol；保留旧版教学上下文。实际打开 smol::Async 文档，纠正其为结构体而非 trait，并标明受支持 I/O 类型与平台边界。
- 实际打开 async-io、async-compat、tokio-util::compat、Tokio runtime、async-executor Executor/LocalExecutor 官方文档。纠正 futures executor 不能执行任何 I/O Future 的绝对说法，区分 trait 适配、上下文与持续驱动；不承诺任意兼容层可支持全部运行时 API。说明组合器/流/同步类型是否独立取决于内部依赖，执行器不自动轮询挂起任务；区分并发/并行、Send 接口契约、本地任务和专用阻塞线程。接入已有运行时指南与替代运行时参考。
- 本篇纯文档，无新增测试、代码、依赖，未运行 Rust 测试；累计测试数保持 136（独立 115、旧 workspace 21）。mdBook `.tools/round9-chapter49-reviewed`、指定单篇 HTML/本地链接/七旧锚点检查与 git diff --check 通过；保留父拥有的检查脚本改动。
- 本篇完成即停止，等待父验收、commit 与 API 上传成功后才处理第 50 篇；不查询、监听或等待 CI。

## 第九轮（2026-10-30）：第 50 篇，最终项目：HTTP 服务器

- 父确认第 49 篇 28f774e4d0ba9952f82868a752108efe517ea2cf 已 API 上传成功后开始本篇。GPT-6 Luna 完整翻译原所有标题与段落、同步包 main.rs 英文注释和两份 HTML 可见文字；GPT-6.1 Sol 校订。保留原两个英文锚点、三个 include 及 Rust Book 原 URL，并附实际打开的重定向目标；SUMMARY 同步中文。
- 保留单线程同步服务器作为后续改造对照，未改为异步。源码最小修复为保存 read 返回长度、EOF 直接返回、仅检查实际读取字节；纠正单次读必得 1024 字节、flush 保证对端收到的注释。实际打开标准库 Read/TcpStream 文档及 tcp.rs 源码；正文明确 package cwd 相对 HTML 路径、阻塞行为、TCP 短读可能误判、无完整 HTTP 解析/超时/错误恢复、write_all 与对端确认不同及连接关闭定界。
- 显式 Rust 1.96.0、指定 RUSTUP_HOME/CARGO_HOME、CARGO_INCREMENTAL=0：最初 offline --locked cargo check 成功，最终 offline 指定 sync_tcp_server 构建至根 target 成功，无新增依赖；cargo +1.96.0 fmt --check、mdBook `.tools/round9-chapter50-reviewed`、指定单篇 HTML/本地链接/旧锚点检查通过。无新增单元测试，累计仍为 136（独立 115、旧 workspace 21）。
- 父实际执行 `.tools/round9_sync_smoke.py`，以本包目录启动本次二进制，在有界本机 TCP 检查中验证 GET / 返回 200 且正文与 hello.html 完整字节一致；空连接 EOF 后仍能处理 GET /missing，返回 404 且正文与 404.html 一致。脚本预查 7878 端口，使用 5 秒边界，并在 finally 终止及等待本次监听子进程；这不证明并发或分片 HTTP 正确性，不增加单元测试计数。
- 本篇到此停止，等待父验收、commit 与 API 上传成功后才开始第 51 篇；保留父检查脚本及用户未跟踪目录，不查询、监听或等待 CI。

## 第九轮（2026-10-30）：第 51 篇，运行异步代码

- 父确认第 50 篇 1943ec7311019594922bf5b6a3a6603a68284a1a 已 API 上传成功后开始本篇。GPT-6 Luna 全文翻译原所有标题、段落和源码英文注释，GPT-6.1 Sol 技术校订；保留两个旧英文锚点、三个 include、原两个 Rust Book 外链及生态章节链接。原控制台警告保留为明确标注的历史输出，并非当前包构建结果；SUMMARY 同步中文。
- 纠正同步代码不能 poll/必需完整运行时、遗漏 await 必然 connection refused、async/await 自动获得并发/非阻塞等过时或绝对说法。实际打开 Rust Reference 异步函数规则与 async-std main 属性、sleep 文档；标明 Future 构造与轮询执行的区别、宏 attributes 功能、当前清单 1.12 的 semver 含义及锁文件 1.13.2、async-std 停止维护背景。最初尝试单独 async-block-expr Reference 页面不可访问，改读 functions 页面成功。
- 明确 09_02 是没有 HTTP 处理的签名空桩，仅将未使用的 mut stream 改为 _stream 并写明阶段；09_03 保留 std listener/read/fs/write 的同步阻塞及顺序 await，只最小修复 read n、EOF 返回、有效字节判断与 write_all。说明 task::sleep 仅在等待阶段不阻塞线程、系统连接队列不等于已被应用处理、不能靠浏览器时间差证明并发，重申相对 HTML cwd、短读/HTTP/flush 的教学边界。未修改后续包或新增依赖、测试。
- 显式 Rust 1.96.0、指定 RUSTUP_HOME/CARGO_HOME、CARGO_INCREMENTAL=0：offline --locked 定向 cargo check async_tcp_server 与 slow_request 成功，无警告；定向 cargo +1.96.0 fmt --check、mdBook `.tools/round9-chapter51-reviewed`、指定单篇 HTML/本地链接/旧锚点检查通过。本篇未启动 TCP 或进行计时实验，不增加测试数，累计仍为 136（独立 115、旧 workspace 21）。
- 本篇到此停止，等待父验收、commit 与 API 上传成功后才处理第 52 篇；保留父检查脚本及用户未跟踪目录，不查询、监听或等待 CI。

## 第九轮（2026-10-30）：第 52 篇，并发处理连接

- 父确认第 51 篇 e534f12688ed66d2edf4c8fc04a6853a41c5dde4 已 API 上传成功后开始本篇。GPT-6 Luna 全文翻译原所有标题、段落及普通源码注释，GPT-6.1 Sol 技术校订；保留两个原 H1 标题层级及英文锚点、三个 include、Stream 原链接，将原本失效的生态 single-threading-vs-multithreading 片段改为第49篇保留的 single-threaded-vs-multi-threaded-executors；SUMMARY 同步中文。
- 实际打开 Incoming、StreamExt::for_each_concurrent、async-std spawn/JoinHandle/fs 官方文档。纠正 Stream 实现者为 incoming 返回值、应导入 StreamExt、同一任务并发不等于固定 OS 线程、spawn 不等于每连接专用线程或保证并行。说明 Send/static 接口契约、None/Some(0) 无界、直接等待与生成后立即返回的限制范围不同、async-std 句柄丢弃 detach、任务结果/panic 与关闭管理的教学缺口。
- 09_04 保留只写 hello 的占位阶段并明确非 HTTP，最小修复常量命名、读取长度/EOF 与 write_all。09_05 保留 generic Read+Write+Unpin 接口，返回 io::Result，读取长度/EOF、write_all、async_std::fs.await 与 ? 传播错误，子任务内报告处理错误及主循环报告接收错误；现有测试调用加 unwrap 为签名机械适配，其他 Mock 修订留下一篇。未重造 HTTP 解析，正文明确短读误判、路径 cwd、无界资源与超时/关闭等限制。
- 显式 Rust 1.96.0、指定 RUSTUP_HOME/CARGO_HOME、CARGO_INCREMENTAL=0：offline --locked 定向 cargo check 两包成功无警告，final_tcp_server 原有 1 项测试通过；定向 cargo +1.96.0 fmt --check、mdBook `.tools/round9-chapter52-reviewed`、指定单篇 HTML/链接/旧锚点检查通过。无新增测试或依赖，未启动 TCP 或作性能/并行时序实验；累计仍为 136（独立 115、旧 workspace 21）。
- 本篇到此停止，等待父验收、commit 与 API 上传成功后才开始第 53 篇；保留父持有的第50篇命令修订和检查脚本及用户未跟踪目录，不查询、监听或等待 CI。

## 第九轮（2026-10-30）：第 53 篇，测试 TCP 服务器

- 父确认第 52 篇 f08cd2ac9a09858f331f7d48f4ab1de4538034dc 已 API 上传成功后开始本篇。GPT-6 Luna 全文翻译原标题与全部段落，GPT-6.1 Sol 技术校订与 Mock/测试补全；保留 testing-the-tcp-server 旧锚点、四个原 include 及所有原主题（原文无外链），SUMMARY 同步中文。
- 实际打开标准库 TcpListener::bind/Unpin、futures-io AsyncRead/AsyncWrite、async-std Read、Cargo test 官方文档。纠正端口0为请求OS分配临时端口而非无效UNIX端口；签名同步 Result 返回类型，区分 futures/async-std IO 与同步/Tokio trait；说明 Unpin 自动满足，将原 unpin include 改为泛型约束断言，不再手写无必要的 impl。
- Mock 读取增加游标、EOF与零长度缓冲处理；写入追加已接受字节并支持短写，不再覆盖；读写各一次真实 Pending 前改变就绪状态并 wake。提供读/写/flush 错误开关，内存 close 成功不冒称真实TCP关闭。原200测试改完整字节相等；新增404完整响应、EOF不写、直接poll验证游标/短写/空缓冲/Pending唤醒计数、handler经历Pending与3字节短写后响应完整、读写flush及WriteZero错误传播共5项，合计6项。
- 显式 Rust 1.96.0、指定 RUSTUP_HOME/CARGO_HOME、CARGO_INCREMENTAL=0：offline --locked final_tcp_server六项测试成功无警告，定向cargo check和fmt --check成功。mdBook `.tools/round9-chapter53-reviewed` 与指定单篇HTML/本地链接/旧锚点检查通过。累计141项（独立115、旧workspace26），本篇未跑其他包全量，未新增依赖或启动真实TCP。
- 测试输入提供完整请求行，未伪称当前单次read能处理分片HTTP；正文明确模拟器与真实TCP、文件系统仍按cwd读取、HTTP完整性/并行/关闭等未覆盖边界。预期HTML通过include_str编译时读取，未以网络或墙钟等待作为判据。
- 本篇到此停止，等待父验收、commit与API上传成功后才处理第54篇；保留父持有文件及用户未跟踪目录，不查询、监听或等待CI。

## 第九轮（2026-10-30）：第 54 篇，I/O：输入与输出

- 父确认第 53 篇 afd7fa26efecd2b4e2de0ba8bc9e6de961b13b0d 已 API 上传成功后开始本篇。原上游只有 SUMMARY 的 I/O 占位标题，无英文正文；GPT-6 Luna 翻译标题“I/O：输入与输出”，GPT-6.1 Sol 原创教程与可运行示例，正文明确归因。新建 src/10_io/01_chapter.md，单一 io 锚点，SUMMARY 只替换父级 I/O 占位；后续 AsyncRead/AsyncWrite 子项及其他 TODO 保留到后轮，不以空链接引用它们。
- 教程自成入门：字节流与消息边界、read有效长度/EOF/空缓冲/错误、Pending与Waker、futures/async-std与Tokio ReadBuf接口区别、短写/write_all/flush/持久化层次、缓冲预读/分帧/载荷限额、驱动上下文及文件线程池、取消不撤销已发送/消费字节。实际打开 Tokio AsyncRead、AsyncBufReadExt::fill_buf、AsyncWriteExt::write_all、fs官方文档，并连接已有IO及取消指南。没有声称完整HTTP或真实网络行为。
- 新增独立 guide-tests 的 legacy_io 模块与lib注册，无新依赖、不改旧workspace。两个源码include：有界LF帧读取函数先检查再追加，返回不含LF的字节载荷，干净EOF=None，截断EOF/超限报错；演示duplex容量2、BufReader容量2并发写读两帧。明确Vec/缓冲/连接等额外内存不受载荷上限一并约束，保留同一reader消费预读数据；解析进度在局部Vec，取消后不能无损重启。
- 新增3项确定性测试：容量2分片双帧与干净EOF；超限、截断和max=0空帧；实际duplex读取先poll得到Pending、写入后Waker计数增加、读取n=2及shutdown后EOF。显式Rust1.96.0、指定RUSTUP_HOME/CARGO_HOME、CARGO_INCREMENTAL=0，offline --locked 定向测试3项通过、115项过滤，无警告，无墙钟等待或真实网络实验。累计144项（独立118、旧workspace26）；父已另行完成旧workspace26项最终回归，本篇未重复。
- cargo +1.96.0 fmt --check、mdBook `.tools/round9-chapter54-reviewed`、指定单篇及父脚本默认全部55篇HTML/本地链接/锚点检查通过。保留父拥有的第50/53篇文档修订、README及检查脚本和用户未跟踪目录。
- 本轮六篇到此完成并停止，等待父验收、commit/API上传及最终全量汇总，不扩展后续子篇/设计模式，不查询、监听或等待CI。
