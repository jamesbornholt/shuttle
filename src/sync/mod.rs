//! Shuttle's implementation of [`std::sync`].

pub mod atomic;
mod barrier;
mod condvar;
pub mod mpsc;
mod mutex;
mod rwlock;

pub use barrier::{Barrier, BarrierWaitResult};
pub use condvar::{Condvar, WaitTimeoutResult};

pub use mutex::Mutex;
pub use mutex::MutexGuard;

pub use rwlock::RwLock;
pub use rwlock::RwLockReadGuard;
pub use rwlock::RwLockWriteGuard;

// TODO implement true support for `Arc`
pub use std::sync::Arc;

pub struct Notify {
    waiter: Mutex<bool>,
    condvar: Condvar,
    notified: atomic::AtomicBool,
}

impl Notify {
    pub fn new() -> Self {
        Self {
            waiter: Mutex::new(false),
            condvar: Condvar::new(),
            notified: atomic::AtomicBool::new(false),
        }
    }

    pub fn notify(&self) {
        let mut _lock = self.waiter.lock().unwrap();
        self.notified.store(true, atomic::Ordering::SeqCst);
        self.condvar.notify_one();
    }

    pub fn wait(&self) {
        let mut lock = self.waiter.lock().unwrap();
        assert!(!*lock, "only a single thread may wait on `Notify`");
        *lock = true;

        while !self.notified.load(atomic::Ordering::SeqCst) {
            lock = self.condvar.wait(lock).unwrap();
        }
        *lock = false;
    }
}
