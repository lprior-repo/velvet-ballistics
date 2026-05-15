//! Loom concurrency models for vb_ipc IPC seams.
//!
//! These models verify ordering invariants for concurrent data structures
//! used in the IPC layer. Each model is a `#[cfg(loom)]` test module that
//! exercises the production code under loom's permutation exploration.
//!
//! Run with: RUSTFLAGS="--cfg loom" cargo test -p vb_ipc --models

#[cfg(loom)]
pub mod memory_ingress;

#[cfg(loom)]
pub mod ipc_server_clients;

#[cfg(loom)]
pub mod write_buffer;