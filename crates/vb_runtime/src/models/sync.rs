//! Sync indirection layer for loom/testing vs production.
//! When `loom` cfg is set, re-exports loom sync types.
//! Otherwise, re-exports std sync types.

#[cfg(loom)]
pub mod sync {
    pub use loom::sync::{Arc, Mutex, MutexGuard};
    pub use loom::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    pub use loom::thread;
}

#[cfg(not(loom))]
pub mod sync {
    pub use std::sync::{Arc, Mutex, MutexGuard};
    pub use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    pub use std::thread;
}