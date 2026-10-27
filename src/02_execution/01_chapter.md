# 底层原理：执行 `Future` 与任务 {#under-the-hood-executing-futures-and-tasks}

本节将介绍 `Future` 和异步任务的底层结构，以及它们是如何被调度的。如果你只想学习如何编写使用现有 `Future` 类型的高层代码，而不关心 `Future` 类型的工作原理，可以直接跳到 [`async`/`await` 一章](../03_async_await/01_chapter.md)。不过，本章讨论的几个主题有助于理解 `async`/`await` 代码的工作方式、运行时特性和性能表现，也有助于构建新的异步原语。如果你现在决定跳过本节，不妨先将它加入书签，以后再回来阅读。

这里先区分两个层次：`Future` 描述可被轮询的异步计算；执行器调度任务，并轮询任务持有的 Future。普通的 `.await` 不会自动创建独立任务。等待中的 Future 通过 `Waker` 通知执行器重新轮询；唤醒不表示计算已经完成，也不保证立刻获得执行。这些机制以标准库的 [`Future`](https://doc.rust-lang.org/std/future/trait.Future.html) 和 [`Waker`](https://doc.rust-lang.org/std/task/struct.Waker.html) 协议为基础。`poll` 应尽快返回，不能用阻塞等待代替返回 `Pending`。

好了，铺垫到此为止。现在让我们来谈谈 [`Future` trait](02_future.md)。
