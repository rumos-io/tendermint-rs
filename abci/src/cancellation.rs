use std::{
    marker::PhantomData,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

pub const CANCEL_PANIC_MSG: &'static str = "requested cancellation";

static FLAG: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Copy, Clone, Default)]
pub struct CancellationSource;

impl CancellationSource {
    pub fn is_cancelled() -> bool {
        FLAG.load(Ordering::Relaxed)
    }

    pub fn panic_if_cancelled() {
      if Self::is_cancelled()
      {
        panic!("{CANCEL_PANIC_MSG}")
      }
    }

    pub fn cancel() {
        FLAG.store(true, Ordering::Relaxed)
    }

    pub fn drop_guard() -> TokenDropGuard {
        TokenDropGuard::new()
    }

    pub fn cancel_and_panic(&self) -> ! {
        Self::cancel();
        Self::panic_if_cancelled();
        unreachable!()
    }
}

#[derive(Debug)]
pub struct TokenDropGuard {
    flag: bool,
    /// I don't want that someone have ability to send token to other thread or save it to use somewhere else.
    /// So this marker prevents user from doing so by making type !Send & !Sync
    _marker: PhantomData<Rc<()>>,
}

impl TokenDropGuard {
    fn new() -> Self {
        Self {
            flag: true,
            _marker: PhantomData,
        }
    }

    pub fn disarm(mut self) {
        self.flag = false;
    }
}

impl Drop for TokenDropGuard {
    fn drop(&mut self) {
        // Other way is to catch panic and cancel in such case,
        // but I think it slightly unclear but cleaner
        if self.flag {
            CancellationSource::cancel()
        }
    }
}
