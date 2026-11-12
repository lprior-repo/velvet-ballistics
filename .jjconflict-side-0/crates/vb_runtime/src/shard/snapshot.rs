#![forbid(unsafe_code)]
//! Read-only shard boundary snapshots for scheduler and observability callers.
//!
//! The implementation is split across focused chunks under `snapshot_chunks/`.
//! All chunks share this module's `use` declarations and are `include!`-d
//! into this shell to keep the public surface and tests unchanged.
//! Splitting by domain responsibility:
//!
//! - `chunk_001_boundary_types` - the four boundary snapshot value types
//!   (`PendingTimerBoundarySnapshot`, `PendingActionBoundarySnapshot`,
//!   `PendingAskTimeoutBoundarySnapshot`, `PendingAskBoundarySnapshot`),
//!   their accessors, the private `PendingAskSnapshotSet` collection type,
//!   and the `ShardPendingBoundarySnapshot` aggregate and its accessors.
//! - `chunk_002_shard_impl` - the `Shard::pending_boundary_snapshot`
//!   method, the per-collection snapshot builders (`active_run_snapshots`,
//!   `pending_timer_snapshots`, `pending_action_snapshots`,
//!   `pending_ask_snapshots`), the ask-timeout resolver, and the free
//!   `asking_step_count`, `frame_step_is_asking`, and `snapshot_capacity`
//!   helpers.

use std::time::Instant;

use vb_core::action::ActionTicket;
use vb_core::frame::StepState;
use vb_core::ids::{RunId, StepIdx};

use crate::shard::types::{PendingTimerKind, RunState, Shard};

include!("snapshot_chunks/chunk_001_boundary_types.rs");
include!("snapshot_chunks/chunk_002_shard_impl.rs");
