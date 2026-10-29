//! 原生静态分发与手动装箱动态分发的对照。
// 私有教学接口由本模块测试调用，普通库构建不调用它们。
#![allow(dead_code)]
// ANCHOR: native
trait Greeting {
    async fn greet(&self, name: &str) -> String;
}

struct Greeter;

impl Greeting for Greeter {
    async fn greet(&self, name: &str) -> String {
        format!("你好，{name}")
    }
}

async fn greet_native(service: &impl Greeting) -> String {
    service.greet("小林").await
}
// ANCHOR_END: native

// ANCHOR: send
trait SendGreeting {
    fn greet<'a>(&'a self, name: &'a str)
        -> impl std::future::Future<Output = String> + Send + 'a;
}

impl SendGreeting for Greeter {
    async fn greet<'a>(&'a self, name: &'a str) -> String {
        format!("你好，{name}")
    }
}

fn greet_on_task(service: impl SendGreeting + Send + 'static)
    -> tokio::task::JoinHandle<String>
{
    tokio::spawn(async move { service.greet("小林").await })
}
// ANCHOR_END: send

// ANCHOR: erased
use futures::future::{BoxFuture, FutureExt};

trait DynGreeting {
    fn greet<'a>(&'a self, name: &'a str) -> BoxFuture<'a, String>;
}

impl DynGreeting for Greeter {
    fn greet<'a>(&'a self, name: &'a str) -> BoxFuture<'a, String> {
        async move { format!("你好，{name}") }.boxed()
    }
}

async fn greet_dynamic(service: &dyn DynGreeting, name: &str) -> String {
    service.greet(name).await
}
// ANCHOR_END: erased

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn native_send_task_and_erased_borrowed_calls_agree() {
        assert_eq!(greet_native(&Greeter).await, "你好，小林");
        assert_eq!(greet_on_task(Greeter).await.unwrap(), "你好，小林");
        let service: Box<dyn DynGreeting> = Box::new(Greeter);
        let name = String::from("小林");
        assert_eq!(greet_dynamic(service.as_ref(), &name).await, "你好，小林");
    }
}
