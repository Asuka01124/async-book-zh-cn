# 同时执行多个 Future {#executing-multiple-futures-at-a-time}

到目前为止，我们主要通过 `.await` 来等待 `Future` 完成。如果被等待的 Future 尚未就绪，
当前任务便会挂起等待唤醒，让执行器有机会推进其他任务；`.await` 本身不会阻塞线程，
也不会自动创建新任务。实际的异步应用程序通常需要并发执行多个不同的操作。

本章将介绍几种同时推进多个异步操作的方法：

- [`join!`](02_join.md)：在当前任务中并发轮询多个 Future，等待它们全部完成。
- [`select!`](03_select.md)：在当前任务中等待多个 Future，从本次轮询发现就绪的分支中选择一个执行。
- [生成任务（Spawning）](04_spawning.md)：把 Future 交给执行器作为独立任务调度；其运行、取消和运行时关闭时的行为由所用运行时及 API 决定，并不保证一定执行到完成。
- [`FuturesUnordered`](../part-guide/streams.md)：管理一组 Future，以 Stream 的形式逐个产生结果；调用方必须持续轮询或消费该流来驱动它，加入 Future 本身不会自动 `spawn` 任务。

这些方式提供的是并发推进；`join!`、`select!` 与 `FuturesUnordered` 本身不会引入线程并行。
`FuturesUnordered` 不按加入顺序交付结果，`select!` 也可能在多个已就绪的分支中作出选择，
因此结果的观察顺序不能视为外部操作严格的物理完成先后。
接口细节可查阅 [`join!`](https://docs.rs/futures/latest/futures/macro.join.html)、
[`select!`](https://docs.rs/futures/latest/futures/macro.select.html) 和
[`FuturesUnordered`](https://docs.rs/futures/latest/futures/stream/struct.FuturesUnordered.html) 文档。
