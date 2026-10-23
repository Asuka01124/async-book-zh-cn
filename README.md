# Rust 异步编程之书（简体中文版）

本项目基于 [rust-lang/async-book](https://github.com/rust-lang/async-book)，逐章翻译为简体中文并补全未完成内容，保留上游历史、作者和许可。上游当前同时包含新版指南、进阶参考和旧版章节，本项目按新版指南的目录顺序推进。

## 本轮范围与工作流

已完成新版指南第 1—12 章：第一轮（2026 年 10 月 22 日）完成第 1—6 章，第二轮（2026 年 10 月 23 日）完成第 7—12 章。第 13 章“异步迭代器”、参考篇、旧版章节和书前导言仍保留上游内容。

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

每章按顺序进行：GPT-6 Luna 子代理翻译 → GPT-6.1 Sol 补全 TODO 并验收 → 构建及相关代码检查 → 单独提交并推送。验收包含原文完整性、术语、技术准确性、示例和内部链接，技术补充优先查证官方文档。实际完成情况和检查证据见 [逐章记录](TRANSLATION_PROGRESS.md)。

第二轮最终本地复核通过：书籍构建、12 章链接与锚点检查、13 个原有测试和 61 个新版指南验证测试（累计 74 个），以及新增示例的格式检查。本轮新增 32 个测试。Windows 信号函数已编译，但未触发真实信号；Unix 分支及平台专用工具的验证边界详见逐章记录。CI 留给仓库所有者自行查看，不作为上传的等待条件。

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

新版指南的补全示例使用独立验证 crate，正文通过 mdBook include 引用其中的源码：

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
