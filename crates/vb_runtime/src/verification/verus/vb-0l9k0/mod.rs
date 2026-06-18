//! Verus proof artifacts for vb-0l9k0: Numeric Timer Seam
//!
//! Production-bound specs that bind to actual production code:
//! - `crates/vb_runtime/src/shard/timer_wheel.rs` - TimerWheel implementation
//! - `crates/vb_runtime/src/shard/types.rs` - TimerDeadline, TimerTick, PendingTimer types

use vstd::prelude::*;

mod helpers;
mod numeric_timer;
mod pending_timer;
mod timer_wheel;
