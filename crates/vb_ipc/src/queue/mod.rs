//! Queue module — ArrayQueue vs crossbeam_channel benchmark tests.
//!
//! These tests validate the `MemoryIngress` SPSC queue contract against the
//! ArrayQueue migration requirements in MAJOR-1.  The tests use the current
//! `crossbeam_channel`-based implementation as the reference; once
//! `ArrayQueue<T, RingFlagged>` replaces it, the same assertions must hold.
//!
//! ## Test file location
//!
//! The 931-line `array_queue_tests.rs` lives at `src/queue/tests/array_queue_tests.rs`
//! and is wired into the test binary from `crates/vb_ipc/src/lib.rs` via
//! `#[path = "queue/tests/array_queue_tests.rs"] mod array_queue_tests;`.
