# Rust 异步编程之书（简体中文版）

本项目基于 [rust-lang/async-book](https://github.com/rust-lang/async-book)，逐章翻译为简体中文并补全未完成内容，保留上游历史、作者和许可。上游当前同时包含新版指南、进阶参考和旧版章节，本项目按新版指南的目录顺序推进。

## 本轮范围与工作流

2026 年 10 月 22 日已完成新版指南的前六章。其余章节保留上游内容，留待后续翻译。

| 章节 | 逐章验收提交 |
| --- | --- |
| [指南导读](src/part-guide/intro.md) | [b58dd03](https://github.com/Asuka01124/async-book-zh-cn/commit/b58dd038e605a56bd581b910c3132cb6c59434b5) |
| [并发编程](src/part-guide/concurrency.md) | [52f86ad](https://github.com/Asuka01124/async-book-zh-cn/commit/52f86adf340fb5b197079694b57b60d86d70d9ee) |
| [Async 与 Await](src/part-guide/async-await.md) | [34af0a9](https://github.com/Asuka01124/async-book-zh-cn/commit/34af0a98ef914996e6f06b9f95c82abf87e822c7) |
| [更多 async/await 主题](src/part-guide/more-async-await.md) | [b5f964a](https://github.com/Asuka01124/async-book-zh-cn/commit/b5f964afcd841e2b63171ff19642b1d93afb9bde) |
| [I/O 与阻塞](src/part-guide/io.md) | [a1f302d](https://github.com/Asuka01124/async-book-zh-cn/commit/a1f302d20c2ca13ec6ee779a3e7fb0550849cc73) |
| [并发组合 Future](src/part-guide/concurrency-primitives.md) | [bf7673a](https://github.com/Asuka01124/async-book-zh-cn/commit/bf7673af4b51475519029fb68d0e0618bd108aeb) |

每章按顺序进行：GPT-6 Luna 子代理翻译 → GPT-6.1 Sol 补全 TODO 并验收 → 构建及相关代码检查 → 单独提交并推送。验收包含原文完整性、术语、技术准确性、示例和内部链接，技术补充优先查证官方文档。实际完成情况和检查证据见 [逐章记录](TRANSLATION_PROGRESS.md)。

本轮最终本地复核通过：书籍构建、六章链接与锚点检查、13 个原有测试和 29 个新增验证测试，以及新增示例的格式检查。CI 留给仓库所有者自行查看，不作为本轮上传的等待条件。

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
