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
//! and is included as a `#[cfg(test)]` submodule below so Cargo compiles it
//! as part of the `vb_ipc` library test binary (otherwise Cargo's integration
//! test discovery would not pick it up under `src/queue/tests/`).

#[cfg(test)]
#[path = "tests/array_queue_tests.rs"]
mod tests;
