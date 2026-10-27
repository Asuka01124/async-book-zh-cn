#![allow(unused)]
#![cfg(test)]

mod async_fn_and_block_examples {
use std::future::Future;
// ANCHOR: async_fn_and_block_examples

// `foo()` 返回一个实现了 `Future<Output = u8>` 的类型。
// `foo().await` 的结果是 `u8` 类型的值。
async fn foo() -> u8 { 5 }

fn bar() -> impl Future<Output = u8> {
    // 这个 `async` 块会生成一个实现了 `Future<Output = u8>` 的类型。
    async {
        let x: u8 = foo().await;
        x + 5
    }
}
// ANCHOR_END: async_fn_and_block_examples
}

mod async_lifetimes_examples {
use std::future::Future;
// ANCHOR: lifetimes_expanded
// 这个函数：
async fn foo(x: &u8) -> u8 { *x }

// 等价于这个函数：
fn foo_expanded<'a>(x: &'a u8) -> impl Future<Output = u8> + 'a {
    async move { *x }
}
// ANCHOR_END: lifetimes_expanded

async fn borrow_x(x: &u8) -> u8 { *x }

#[cfg(feature = "never_compiled")]
// ANCHOR: static_future_with_borrow
fn bad() -> impl Future<Output = u8> {
    let x = 5;
    borrow_x(&x) // 错误：`x` 的生命周期不够长
}

fn good() -> impl Future<Output = u8> {
    async {
        let x = 5;
        borrow_x(&x).await
    }
}
// ANCHOR_END: static_future_with_borrow
}

mod async_move_examples {
use std::future::Future;
// ANCHOR: async_move_examples
/// `async` 块：
///
/// 只要在该变量的作用域内执行，多个不同的 `async` 块就可以访问同一个局部变量。
async fn blocks() {
    let my_string = "foo".to_string();

    let future_one = async {
        // ...
        println!("{my_string}");
    };

    let future_two = async {
        // ...
        println!("{my_string}");
    };

    // 运行两个 Future 直至完成，并打印两次 "foo"：
    let ((), ()) = futures::join!(future_one, future_two);
}

/// `async move` 块：
///
/// 由于捕获的变量会被移入 `async move` 块生成的 `Future`，
/// 因此这个非 Copy 的 String 被移动后，原绑定不能再被其他块使用。
/// 它拥有 String，所以可以离开原作用域；捕获引用则仍受被引用数据的生命周期限制。
fn move_block() -> impl Future<Output = ()> {
    let my_string = "foo".to_string();
    async move {
        // ...
        println!("{my_string}");
    }
}
// ANCHOR_END: async_move_examples
}
