// ANCHOR: smol-local
pub fn local_executor() -> u32 {
    let executor = smol::LocalExecutor::new();
    let state = std::rc::Rc::new(std::cell::Cell::new(0));
    let copy = state.clone();
    smol::block_on(executor.run(async {
        let task = executor.spawn(async move {
            smol::Timer::after(std::time::Duration::from_millis(1)).await;
            copy.set(21);
        });
        task.await;
        state.get() * 2
    }))
}
// ANCHOR_END: smol-local

#[cfg(test)]
mod tests {
    use std::{future::Future, pin::Pin, sync::{Arc, atomic::{AtomicBool, Ordering}}, task::{Context, Poll}};
    #[test]
    fn alternate_local_non_send_task_and_timer() {
        assert_eq!(super::local_executor(), 42);
    }
    #[test]
    fn alternate_dropping_task_cancels_after_first_poll() {
        struct Work(Arc<AtomicBool>);
        impl Future for Work {
            type Output = ();
            fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<()> { Poll::Pending }
        }
        impl Drop for Work { fn drop(&mut self) { self.0.store(true, Ordering::SeqCst); } }
        let executor = smol::LocalExecutor::new();
        let dropped = Arc::new(AtomicBool::new(false));
        let task = executor.spawn(Work(dropped.clone()));
        assert!(executor.try_tick());
        drop(task);
        while executor.try_tick() {}
        assert!(dropped.load(Ordering::SeqCst));
    }
    #[test]
    fn alternate_runtime_independent_future_on_two_executors() {
        async fn portable() -> u32 {
            let (tx, rx) = crate::implementing::channel();
            std::thread::spawn(move || tx.send(12).unwrap());
            rx.await.unwrap()
        }
        assert_eq!(smol::block_on(portable()), 12);
        assert_eq!(futures::executor::block_on(portable()), 12);
    }
}
