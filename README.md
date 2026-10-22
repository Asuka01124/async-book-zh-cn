# Rust 异步编程之书（简体中文版）

本项目基于 [rust-lang/async-book](https://github.com/rust-lang/async-book)，逐章翻译为简体中文并补全未完成内容，保留上游历史、作者和许可。上游当前同时包含新版指南、进阶参考和旧版章节，本项目按新版指南的目录顺序推进。

## 本轮范围与工作流

2026 年 10 月 22 日计划完成新版指南的前六章：指南导言、并发编程、async 与 await、更多 async/await 主题、I/O 与阻塞、并发组合 Future。其余章节保留上游内容，留待后续翻译。

每章按顺序进行：GPT-6 Luna 子代理翻译 → GPT-6.1 Sol 补全 TODO 并验收 → 构建及相关代码检查 → 单独提交并推送。验收包含原文完整性、术语、技术准确性、示例和内部链接，技术补充优先查证官方文档。实际完成情况和检查证据见 [逐章记录](TRANSLATION_PROGRESS.md)。

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

构建后，使用 Python 3 标准库检查已译章节的未完成标记、空链接及生成 HTML 中的本地文件和锚点：

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
