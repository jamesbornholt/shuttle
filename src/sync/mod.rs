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
    lock: Mutex<bool>,
    condvar: Condvar,
}

impl Notify {
    pub fn new() -> Self {
        Self {
            lock: Mutex::new(false),
            condvar: Condvar::new(),
        }
    }

    pub fn notify(&self) {
        self.condvar.notify_one();
    }

    pub fn wait(&self) {
        let mut lock = self.lock.lock().unwrap();
        assert!(!*lock, "only a single thread may wait on `Notify`");
        *lock = true;

        let mut lock = self.condvar.wait(lock).unwrap();
        *lock = false;
    }
}