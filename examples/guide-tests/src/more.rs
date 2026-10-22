// ANCHOR: async-closure
use std::ops::AsyncFnMut;

pub async fn visit_numbers<F>(mut visit: F)
where
    F: for<'a> AsyncFnMut(&'a u32),
{
    for number in [1, 2, 3] {
        visit(&number).await;
    }
}

pub async fn sum_with_async_closure() -> u32 {
    let mut sum = 0;
    let visit = async |number: &u32| {
        tokio::task::yield_now().await;
        sum += *number;
    };
    visit_numbers(visit).await;
    sum
}
// ANCHOR_END: async-closure

// ANCHOR: borrowing
use std::future::Future;

pub fn borrowed_len(text: &str) -> impl Future<Output = usize> + '_ {
    async move {
        tokio::task::yield_now().await;
        text.len()
    }
}
// ANCHOR_END: borrowing

// ANCHOR: local-task
pub async fn local_rc_len() -> usize {
    let local = tokio::task::LocalSet::new();
    local
        .run_until(async {
            let text = std::rc::Rc::new(String::from("Rust"));
            tokio::task::spawn_local(async move {
                tokio::task::yield_now().await;
                text.len()
            })
            .await
            .expect("本地任务正常完成")
        })
        .await
}
// ANCHOR_END: local-task

// ANCHOR: send-trait
pub struct Greeter(pub String);

pub trait SendGreeting: Sync {
    fn greet(&self) -> impl std::future::Future<Output = String> + Send + '_;
}

impl SendGreeting for Greeter {
    async fn greet(&self) -> String {
        tokio::task::yield_now().await;
        format!("你好，{}", self.0)
    }
}
// ANCHOR_END: send-trait

// ANCHOR: dyn-trait
pub trait DynGreeting: Send + Sync {
    fn greet(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = String> + Send + '_>>;
}

impl DynGreeting for Greeter {
    fn greet(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = String> + Send + '_>> {
        Box::pin(async move { format!("你好，{}", self.0) })
    }
}
// ANCHOR_END: dyn-trait

// ANCHOR: recursion
pub async fn count_down(n: u32) -> u32 {
    if n == 0 {
        0
    } else {
        1 + Box::pin(count_down(n - 1)).await
    }
}
// ANCHOR_END: recursion

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn lending_closure_mutates_captured_state() {
        assert_eq!(sum_with_async_closure().await, 6);
    }

    #[tokio::test]
    async fn borrowed_future_can_live_within_its_owner() {
        let text = String::from("Rust");
        assert_eq!(borrowed_len(&text).await, 4);
        assert_eq!(text, "Rust");
    }

    #[tokio::test(flavor = "current_thread")]
    async fn local_task_accepts_rc_across_await() {
        assert_eq!(local_rc_len().await, 4);
    }

    #[tokio::test]
    async fn trait_future_can_be_sent_and_dyn_trait_can_dispatch() {
        let greeter = Greeter(String::from("Rust"));
        fn assert_send(_: impl Send) {}
        assert_send(SendGreeting::greet(&greeter));
        let result = tokio::spawn(async move { SendGreeting::greet(&greeter).await })
            .await
            .expect("任务正常完成");
        assert_eq!(result, "你好，Rust");
        let object: Box<dyn DynGreeting> = Box::new(Greeter(String::from("异步")));
        assert_eq!(object.greet().await, "你好，异步");
    }

    #[tokio::test]
    async fn boxed_recursive_calls_have_a_finite_type() {
        assert_eq!(count_down(32).await, 32);
    }
}

// ANCHOR: cancellation-paths
pub async fn some_function(input: Option<u32>) -> Result<u32, &'static str> {
    let Some(input) = input else {
        return Ok(0); // 普通提前返回。
    };
    let x = input.checked_add(1).ok_or("整数溢出")?; // 错误提前返回。
    let y = async {
        tokio::task::yield_now().await;
        x
    }
    .await; // 若拥有该 Future 的调用方此时丢弃它，后续代码不会执行。
    Ok(y)
}
// ANCHOR_END: cancellation-paths

#[cfg(test)]
mod control_flow_tests {
    use super::*;

    #[tokio::test]
    async fn ordinary_returns_and_errors_are_preserved() {
        assert_eq!(some_function(None).await, Ok(0));
        assert_eq!(some_function(Some(41)).await, Ok(42));
        assert_eq!(some_function(Some(u32::MAX)).await, Err("整数溢出"));
    }
}
