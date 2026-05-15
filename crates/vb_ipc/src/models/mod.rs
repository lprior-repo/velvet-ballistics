//! vb_ipc debug/test models.
//!
//! This module is only compiled under `#[cfg(loom)]` for concurrency testing.

#[cfg(loom)]
pub mod loom;
