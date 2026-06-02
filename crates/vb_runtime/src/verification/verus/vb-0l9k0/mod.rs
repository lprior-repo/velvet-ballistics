//! Verus extern_spec proof artifacts for vb-0l9k0: Numeric Timer Seam
//!
//! Production-bound #[extern_spec] declarations that bind to actual production code:
//! - `crates/vb_runtime/src/shard/timer_wheel.rs` - TimerWheel implementation
//! - `crates/vb_runtime/src/shard/types.rs` - TimerDeadline, TimerTick, PendingTimer types
//!
//! This artifact covers 27 proof obligations across 7 contract clauses:
//! - C-001: Generation never wraps (R-004)
//! - C-002: Generation starts at one (R-004)
//! - C-003: Generation increments on replacement (R-004)
//! - C-004: Invalid authority cannot mutate state (R-002)
//! - C-005: Deadline overflow returns typed error (R-001)
//! - C-006: Duplicate key idempotency (R-005)
//! - C-007: Monotonic clock, deterministic fire (R-006)
//! - C-008: Fire expired returns all expired (R-006)
//! - C-009: Fire expired returns in deadline order (R-006)
//! - C-010: Next deadline returns earliest (R-006)
//! - C-011: Cancel removes entry (R-005)
//! - C-012: Len and is_empty reflect state (R-005)
//! - C-013: Zero-duration timer fires at exact deadline (R-008)
//! - C-014: Replacement preserves correct entry (R-005)
//! - C-015: Numeric timer arithmetic safety (R-001)

pub mod helpers;
pub mod numeric_timer;
pub mod pending_timer;
pub mod timer_wheel;
