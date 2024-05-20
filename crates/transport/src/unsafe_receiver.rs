use std::{ops::Deref, sync::mpsc::Receiver};

pub(crate) struct UnsafeReceiver<T>(pub Receiver<T>);

unsafe impl<T> Sync for UnsafeReceiver<T> {}

impl<T> Deref for UnsafeReceiver<T> {
    type Target = Receiver<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
