<a id="workarounds-to-know-and-love"></a>
# 值得了解并善用的变通方案

本章原文写于 Rust 异步支持较早的阶段，当时一些呼声很高的特性仍在开发，诊断信息也有待改进。如今 `async`/`.await` 已自 [Rust 1.39（2019 年）起稳定](https://blog.rust-lang.org/2019/11/07/Async-await-stable/)，不应再笼统地称为“相对较新”；特性支持和诊断表现需要结合所用编译器版本判断。

本章介绍一些常见的棘手问题及应对方法，包括 [`Send` 推断](03_send_approximation.md)、[递归](04_recursion.md)与 [trait 中的 `async`](05_async_in_traits.md)。后续各节会区分仍有必要的变通方案与旧版编译器的限制，避免把历史经验当成当前语言规则。
