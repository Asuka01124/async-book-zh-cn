use std::{future::Future, pin::Pin, task::{Context, Poll}};

// ANCHOR: projection
pin_project_lite::pin_project! {
    pub struct CountPolls<F> {
        #[pin]
        inner: F,
        polls: usize,
    }
}
impl<F> CountPolls<F> {
    pub fn new(inner: F) -> Self { Self { inner, polls: 0 } }
    pub fn polls(&self) -> usize { self.polls }
}
impl<F: Future> Future for CountPolls<F> {
    type Output = F::Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        *this.polls += 1; // 非结构固定字段：&mut usize。
        this.inner.poll(cx) // 结构固定字段：Pin<&mut F>，不要求F: Unpin。
    }
}
// ANCHOR_END: projection

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::{Cell, RefCell}, marker::PhantomPinned, rc::Rc};
    type Events = Rc<RefCell<Vec<(u8, usize)>>>;
    struct AddressProbe {
        id: u8,
        seen: Cell<Option<usize>>,
        waiting: Cell<bool>,
        events: Events,
        _pin: PhantomPinned,
    }
    impl AddressProbe {
        fn new(id: u8, events: Events) -> Self {
            Self { id, seen: Cell::new(None), waiting: Cell::new(true), events, _pin: PhantomPinned }
        }
    }
    impl Future for AddressProbe {
        type Output = u8;
        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u8> {
            let this = self.as_ref().get_ref();
            let address = this as *const Self as usize;
            if let Some(old) = this.seen.replace(Some(address)) { assert_eq!(old, address); }
            if this.waiting.replace(false) {
                cx.waker().wake_by_ref();
                Poll::Pending
            } else { Poll::Ready(this.id) }
        }
    }
    impl Drop for AddressProbe {
        fn drop(&mut self) {
            let address = self as *const Self as usize;
            if let Some(old) = self.seen.get() { assert_eq!(old, address); }
            self.events.borrow_mut().push((self.id, address));
        }
    }

    #[test]
    fn pinning_moving_box_handle_keeps_non_unpin_pointee_address() {
        let events = Events::default();
        let mut pinned = Box::pin(AddressProbe::new(1, events.clone()));
        let mut cx = Context::from_waker(std::task::Waker::noop());
        assert!(pinned.as_mut().poll(&mut cx).is_pending());
        let address = pinned.seen.get().unwrap();
        let mut moved = pinned;
        assert_eq!(moved.as_mut().poll(&mut cx), Poll::Ready(1));
        drop(moved);
        assert_eq!(*events.borrow(), vec![(1, address)]);
    }

    #[test]
    fn pinning_set_drops_old_value_in_place_before_polling_replacement() {
        let events = Events::default();
        let mut pinned = Box::pin(AddressProbe::new(1, events.clone()));
        let mut cx = Context::from_waker(std::task::Waker::noop());
        assert!(pinned.as_mut().poll(&mut cx).is_pending());
        let address = pinned.seen.get().unwrap();
        pinned.as_mut().set(AddressProbe::new(2, events.clone()));
        assert_eq!(*events.borrow(), vec![(1, address)]);
        assert!(pinned.as_mut().poll(&mut cx).is_pending());
        assert_eq!(pinned.seen.get(), Some(address));
        assert_eq!(pinned.as_mut().poll(&mut cx), Poll::Ready(2));
        drop(pinned);
        assert_eq!(*events.borrow(), vec![(1, address), (2, address)]);
    }

    #[tokio::test]
    async fn pinning_projection_preserves_borrow_across_pending() {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let future = async {
            let text = String::from("固定");
            let borrowed = &text;
            rx.await.unwrap();
            borrowed.chars().count()
        };
        let mut wrapper = std::pin::pin!(CountPolls::new(future));
        assert!(futures::poll!(wrapper.as_mut()).is_pending());
        assert_eq!(wrapper.polls(), 1);
        tx.send(()).unwrap();
        assert_eq!(wrapper.as_mut().await, 2);
        assert_eq!(wrapper.polls(), 2);
    }
}
