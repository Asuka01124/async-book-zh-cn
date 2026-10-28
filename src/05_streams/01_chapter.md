# `Stream` trait {#the-stream-trait}

`Stream` trait 与 `Future` 类似，但它可以在完成之前产生多个值，类似于标准库中的
`Iterator` trait：

```rust,ignore
{{#include ../../examples/05_01_streams/src/lib.rs:stream_trait}}
```

这里展示的是 `futures` 的 `Stream` trait 的核心接口，省略了有默认实现的 `size_hint` 方法。`poll_next` 应快速返回：`Pending` 表示下一项暂未就绪，实现必须安排在可能取得进展时通过 `cx.waker()` 唤醒当前任务；`Ready(Some(item))` 表示取得一项，之后还可以继续轮询；`Ready(None)` 才表示整个流结束。通用代码不应在流结束后再次轮询；如果需要结束后仍安全地重复轮询，可以使用 `StreamExt::fuse`。详见 [Stream 协议文档](https://docs.rs/futures-core/0.3.31/futures_core/stream/trait.Stream.html)。

`Stream` 的一个常见例子是 `futures` crate 中通道类型的 `Receiver`。每当从
`Sender` 端发送一个值时，它就会产生 `Some(val)`；当所有 `Sender`（包括克隆的发送端）都被丢弃，且所有待处理消息
都已接收后，它会产生 `None`：

```rust,edition2018,ignore
{{#include ../../examples/05_01_streams/src/lib.rs:channels}}
```

示例通过 `futures::prelude::*` 引入 `SinkExt` 和 `StreamExt`，分别提供 `send` 和 `next`。`next()` 返回等待下一项的 Future，`.await` 才取得 `Option<Item>`；它要求接收者实现 `Unpin`，这里的 `Receiver` 满足该条件。对于不满足该条件的流，可先固定流，再通过固定后的可变引用调用 `next`，参见[固定](../part-reference/pinning.md)。

这里先发送两个值再接收，通道容量足以容纳它们，因此演示的是有序接收和缓冲排空后的结束；并没有创建独立发送任务，也不保证这些 `.await` 实际发生挂起。有界通道发送较多数据时可能等待容量，不能无条件照搬“同一任务先发送全部数据，再开始接收”的顺序。接收端也可以调用 `close()` 阻止后续发送，然后继续接收至缓冲排空；暂时没有消息但通道仍开放时，流应等待，而不是返回 `None`。更多用法见[异步迭代器（Stream）](../part-guide/streams.md)。
