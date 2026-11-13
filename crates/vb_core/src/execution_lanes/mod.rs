//! Execution-lane lease allocator.
//!
//! A bounded pool of reusable execution lanes. Each lane is leased for a
//! single item within a group; lanes are released back to the pool after the
//! item completes. The pool guarantees at most `capacity` lanes are leased
//! simultaneously, and lease tokens are opaque with private identity fields.
//!
//! # Runtime ownership
//!
//! A single `LanePool` instance is owned by the runtime for one execution run.
//! The caller is responsible for pairing each `admit` with exactly one
//! `release` for the returned token. The pool itself does not track
//! completion, durability, or scheduling policy.
//!
//! # Group identity
//!
//! Group identity is per-run. The caller is responsible for checking RunId
//! and journaling admission/sequencing externally. The lease allocator
//! alone does not prove scheduler correctness, durability, winner
//! determination, or recovery semantics. Tokens embed group identity with
//! private fields and checked construction, but are not cryptographic or
//! unforgeable capabilities.

mod leases;

pub use leases::{GroupId, LaneError, LanePool, LaneToken};

mod group;
mod group_commit;
mod group_prepare;
mod group_state;
mod group_types;

pub use group::GroupControl;
pub use group_prepare::PreparedGroupEvent;
pub use group_types::*;

pub use leases::storage::{ActiveGroup, GroupStorage};
