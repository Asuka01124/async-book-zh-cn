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
