use std::{
    marker::PhantomData,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
};

pub const CANCEL_PANIC_MSG: &'static str = "requested cancellation";

static FLAG: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Copy, Clone, Default)]
pub struct CancellationToken;

impl CancellationToken {
    pub fn new() -> Self {
        Self
    }

    pub fn is_cancelled(&self) -> bool {
        FLAG.load(Ordering::Relaxed)
    }

    pub fn panic_if_cancelled(&self) {
        panic!("{CANCEL_PANIC_MSG}")
    }

    pub fn cancel(&self) {
        FLAG.store(true, Ordering::Relaxed)
    }

    pub fn drop_guard(&self) -> TokenDropGuard {
        TokenDropGuard::new(self.clone())
    }

    pub fn cancel_and_panic(&self) -> ! {
        self.cancel();
        self.panic_if_cancelled();
        unreachable!()
    }
}

#[derive(Debug)]
pub struct TokenDropGuard {
    flag: bool,
    token: CancellationToken,
    /// I don't want that someone have ability to send token to other thread or save it to use somewhere else.
    /// So this marker prevents user from doing so by making type !Send & !Sync
    _marker: PhantomData<Rc<()>>,
}

impl TokenDropGuard {
    fn new(token: CancellationToken) -> Self {
        Self {
            flag: true,
            token,
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
            self.token.cancel()
        }
    }
}
