# Rust 异步编程之书（简体中文版）

本项目基于 [rust-lang/async-book](https://github.com/rust-lang/async-book)，逐章翻译为简体中文并补全未完成内容，保留上游历史、作者和许可。上游当前同时包含新版指南、进阶参考和旧版章节，本项目按新版指南的目录顺序推进。

## 本轮范围与工作流

已完成新版指南全部 13 篇、参考篇全部 13 篇、旧版入门前四篇及[书前导言](src/intro.md)，累计 31 篇。第一、二轮（2026 年 10 月 22—23 日）完成指南前 12 篇；第三轮（10 月 24 日）完成指南最后一篇和参考篇前五篇，同日追加书前导言；第四轮（10 月 25 日）完成参考篇接下来的六篇；第五轮（10 月 26 日）完成参考篇最后两篇和旧版入门四篇。其余旧版章节和其他导航内容仍保留上游内容。

| 章节 | 逐章验收提交 |
| --- | --- |
| [指南导读](src/part-guide/intro.md) | [b58dd03](https://github.com/Asuka01124/async-book-zh-cn/commit/b58dd038e605a56bd581b910c3132cb6c59434b5) |
| [并发编程](src/part-guide/concurrency.md) | [52f86ad](https://github.com/Asuka01124/async-book-zh-cn/commit/52f86adf340fb5b197079694b57b60d86d70d9ee) |
| [Async 与 Await](src/part-guide/async-await.md) | [34af0a9](https://github.com/Asuka01124/async-book-zh-cn/commit/34af0a98ef914996e6f06b9f95c82abf87e822c7) |
| [更多 async/await 主题](src/part-guide/more-async-await.md) | [b5f964a](https://github.com/Asuka01124/async-book-zh-cn/commit/b5f964afcd841e2b63171ff19642b1d93afb9bde) |
| [I/O 与阻塞](src/part-guide/io.md) | [a1f302d](https://github.com/Asuka01124/async-book-zh-cn/commit/a1f302d20c2ca13ec6ee779a3e7fb0550849cc73) |
| [并发组合 Future](src/part-guide/concurrency-primitives.md) | [bf7673a](https://github.com/Asuka01124/async-book-zh-cn/commit/bf7673af4b51475519029fb68d0e0618bd108aeb) |

第二轮已按以下顺序逐章验收并上传：

| 章节 | 逐章验收提交 |
| --- | --- |
| [通道、锁与同步](src/part-guide/sync.md) | [66bc1f2](https://github.com/Asuka01124/async-book-zh-cn/commit/66bc1f224eee4a3a9f2c02c3f632533cb87f6983) |
| [异步编程工具](src/part-guide/tools.md) | [597e2a5](https://github.com/Asuka01124/async-book-zh-cn/commit/597e2a58453256e74e1a6ed70cf7e1d0549eb980) |
| [析构与清理](src/part-guide/dtors.md) | [1c84318](https://github.com/Asuka01124/async-book-zh-cn/commit/1c8431867299c839badf9c28f8a58b6f138f16e0) |
| [Future 详解](src/part-guide/futures.md) | [6da4123](https://github.com/Asuka01124/async-book-zh-cn/commit/6da41236f05c2f755cc7017250b4bf21e08daeff) |
| [运行时及其使用边界](src/part-guide/runtimes.md) | [02fbb27](https://github.com/Asuka01124/async-book-zh-cn/commit/02fbb27149bee998ea5cf51d6b1ec09f25a128e0) |
| [定时器与信号处理](src/part-guide/timers-signals.md) | [9ba46e1](https://github.com/Asuka01124/async-book-zh-cn/commit/9ba46e17dbab52454dd0f329d102b9e587cfa4c6) |

第 10 章另有[措辞复核提交 f24d6e5](https://github.com/Asuka01124/async-book-zh-cn/commit/f24d6e5d9cf5357cb4576fc4c70a8f46b0080d9a)，用于校准唤醒和重复轮询的契约描述。

第三轮已按目录顺序逐章验收并上传以下六项，编号沿用项目的累计完成顺序：

| 章节 | 逐章验收提交 |
| --- | --- |
| [异步迭代器](src/part-guide/streams.md) | [e25ab4c](https://github.com/Asuka01124/async-book-zh-cn/commit/e25ab4cf07c68fa45be33d58b8aaea39b2c497e9) |
| [实现 Future 与 Stream](src/part-reference/implementing-futures-streams.md) | [076fc07](https://github.com/Asuka01124/async-book-zh-cn/commit/076fc076abe868369d3a75d2dfd5fb50a9a00308) |
| [替代运行时](src/part-reference/alternate-runtimes.md) | [caf55fa](https://github.com/Asuka01124/async-book-zh-cn/commit/caf55fa5d4ed9c76b69310ff7846742412e8e26a) |
| [实现自己的运行时](src/part-reference/implementing-runtime.md) | [d16ea8a](https://github.com/Asuka01124/async-book-zh-cn/commit/d16ea8afd28967a680f3aef2106386ad73322ffd) |
| [同步与异步互调](src/part-reference/sync-async.md) | [434e5a2](https://github.com/Asuka01124/async-book-zh-cn/commit/434e5a2b885ca45ac07b974d3c6d7fadfbd861ee) |
| [异步 I/O：就绪与完成，以及 io_uring](src/part-reference/io-models.md) | [cd9a1d9](https://github.com/Asuka01124/async-book-zh-cn/commit/cd9a1d9e20700c36796d1c8d80f5555cde2a1222) |

参考篇这五项原本只有空目录标题，没有英文正文。Luna 翻译已有标题，Sol 补写中文正文；不将新写内容标为已有原文的翻译。

第 14 章另有[Waker 回调锁边界复核提交 34ebd75](https://github.com/Asuka01124/async-book-zh-cn/commit/34ebd75bd0ff8d808d839c762a975c7397f39057)，并新增析构重入的回归测试。

第四轮（2026 年 10 月 25 日）已按目录顺序逐章验收并上传以下六章，编号沿用前三轮累计顺序，书前导言单独计数：

| 章节 | 逐章验收提交 |
| --- | --- |
| [设计模式](src/part-reference/design-patterns.md) | [0ee2bc1](https://github.com/Asuka01124/async-book-zh-cn/commit/0ee2bc143e3102c0b28874e0b886364c218c591d) |
| [取消与取消安全](src/part-reference/cancellation.md) | [339fe34](https://github.com/Asuka01124/async-book-zh-cn/commit/339fe346e0355d86d245f13b7c468023197bdf3c) |
| [饥饿](src/part-reference/starvation.md) | [8f72485](https://github.com/Asuka01124/async-book-zh-cn/commit/8f72485e24239694a3620e95358dfd04ae2f7b9c) |
| [固定（Pinning）](src/part-reference/pinning.md) | [2b026f8](https://github.com/Asuka01124/async-book-zh-cn/commit/2b026f8d59f8ffaf25729320ada1b3dcd5e1e991) |
| [异步与 FFI](src/part-reference/async-ffi.md) | [d78c2d3](https://github.com/Asuka01124/async-book-zh-cn/commit/d78c2d39495cad50b93ddca2e8e15ea3639c85dd) |
| [Rust 异步编程与其他语言的比较](src/part-reference/comparing-languages.md) | [90df556](https://github.com/Asuka01124/async-book-zh-cn/commit/90df556aa36cc245bdfd4bea96fe7f66bc86c669) |

第四轮的设计模式、饥饿、异步与 FFI、跨语言比较原本仅有空目录标题，Luna 翻译标题，Sol 原创补写正文；取消章按原提纲补全，Pinning 章完整翻译已有长文并技术校订。

第五轮（2026 年 10 月 26 日）已按目录页面顺序逐篇验收并独立提交上传以下六篇，书前导言仍单独计数：

| 章节 | 逐章验收提交 |
| --- | --- |
| [rustc 中 async/await 的实现](src/part-reference/rustc-async-await.md) | [11ff7a9](https://github.com/Asuka01124/async-book-zh-cn/commit/11ff7a912bcec8dfc391b5148e046f039949537c) |
| [结构化并发](src/part-reference/structured.md) | [b4dd105](https://github.com/Asuka01124/async-book-zh-cn/commit/b4dd10589b6b90d3f1eb49517755152cdbc10ee5) |
| [旧版：入门](src/01_getting_started/01_chapter.md) | [abf3711](https://github.com/Asuka01124/async-book-zh-cn/commit/abf371186abde2844e0f27739db4deea729a6ed0) |
| [旧版：为什么使用异步？](src/01_getting_started/02_why_async.md) | [1d09671](https://github.com/Asuka01124/async-book-zh-cn/commit/1d09671b304ef01add1567136ff2013ccca903b6) |
| [旧版：Rust 异步编程的现状](src/01_getting_started/03_state_of_async_rust.md) | [3c719ba](https://github.com/Asuka01124/async-book-zh-cn/commit/3c719ba6e77fe6bcc53f1a82bf82141d33ba850c) |
| [旧版：async/await 入门](src/01_getting_started/04_async_await_primer.md) | [056a256](https://github.com/Asuka01124/async-book-zh-cn/commit/056a256124d59b128a6bcfad0ae4e3e5f6a0fb2d) |

rustc 实现条目原本仅有标题，Luna 翻译标题、Sol 原创正文；结构化并发和旧版页面完整翻译已有原文，并补齐提纲或校订过时表述。

每章按顺序进行：GPT-6 Luna 子代理翻译 → GPT-6.1 Sol 补全 TODO 并验收 → 构建及相关代码检查 → 单独提交并推送。验收包含原文完整性、术语、技术准确性、示例和内部链接，技术补充优先查证官方文档。实际完成情况和检查证据见 [逐章记录](TRANSLATION_PROGRESS.md)。

第五轮最终本地复核通过：书籍构建、全部 31 篇已完成章节的链接与锚点检查、13 个原有测试和 106 个独立验证测试（累计 119 个），以及独立验证 crate 和本轮修改的两个旧示例的格式检查。本轮新增 5 个测试，覆盖挂起状态的析构、内部借用、监督者关闭与错误、学习和跳舞的真实 Pending 交错。稳定 Rust 1.96 的 MIR 已实际观察；五篇已有原文的标题锚点、外链和脚注均保留。旧版下载函数仍为明确标注的历史占位，没有发出 HTTP 请求；未运行第三方作用域库或实验 AsyncDrop，也未做性能基准。其他平台及实验功能的验证边界见逐篇记录。CI 留给仓库所有者自行查看，不作为上传的等待条件。

书前导言也已单独完成翻译与技术校订，包含三个小节和全部脚注，保留原英文锚点并补齐阻塞 I/O 链接；引用的 Hello World 示例已用现有 Rust 工具链实际运行并正常输出。

## 构建要求

需要可用的 Rust 工具链和 [`mdbook`]（[文档](https://rust-lang.github.io/mdBook/index.html)）。默认只构建 HTML，不强制安装外部网页链接检查插件。

```
cargo install mdbook --version 0.5.4 --locked
```

[`mdbook`]: https://github.com/rust-lang/mdBook

## 本地构建

执行以下命令，在 `book/` 目录生成书籍：

```
mdbook build
```

检查原有示例工作区：

```sh
cargo test --workspace --manifest-path examples/Cargo.toml
```

新版指南和参考篇的补全示例使用独立验证 crate，正文通过 mdBook include 引用其中的源码：

```sh
cargo test --locked --manifest-path examples/guide-tests/Cargo.toml
cargo fmt --manifest-path examples/guide-tests/Cargo.toml --check
```

构建后，使用 Python 3 标准库检查已译章节的未完成标记、空链接、重复 HTML id，以及生成 HTML 中的本地文件和锚点：

```sh
python scripts/check_chapters.py
```

也可只检查指定章节：`python scripts/check_chapters.py part-guide/intro.md`。

## 本地预览

`mdbook serve` 启动本地预览服务器：

```
mdbook serve
```

## 来源与许可

本轮基线是上游提交 [`840327a2e8ea30644b2a6c8940d467183a17c43d`](https://github.com/rust-lang/async-book/commit/840327a2e8ea30644b2a6c8940d467183a17c43d)。原作者、贡献者和许可文件均予保留。本仓库是独立的中文翻译与补全项目。
