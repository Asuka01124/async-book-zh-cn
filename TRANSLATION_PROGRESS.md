# 翻译与补全进度

本次范围：新版指南前六章。翻译作者为 GPT-6 Luna；技术补全、校审与验收作者为 GPT-6.1 Sol。上游基线：`840327a`。各章逐章翻译、校审、验证、提交并推送后，再开始下一章。书前导言暂保留上游英文，基础设施由主代理负责。

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
