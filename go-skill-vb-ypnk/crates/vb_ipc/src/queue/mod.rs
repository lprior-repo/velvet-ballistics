//! Queue module — ArrayQueue vs crossbeam_channel benchmark tests.
//!
//! These tests validate the `MemoryIngress` SPSC queue contract against the
//! ArrayQueue migration requirements in MAJOR-1.  The tests use the current
//! `crossbeam_channel`-based implementation as the reference; once
//! `ArrayQueue<T, RingFlagged>` replaces it, the same assertions must hold.
//!
//! ## Test file location
//!
//! Integration tests for this module live in `tests/array_queue_tests.rs` and are
//! compiled as separate test binaries by Cargo's integration test discovery.
