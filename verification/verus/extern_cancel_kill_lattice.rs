// SPDX-License-Identifier: MIT
//
// Extern surface for `cancel_kill_lattice.rs`.
//
// This file binds the Verus proof to the in-tree production mirror at
// `production_inner/cancel_kill_lattice_production.rs`. The mirror captures the
// production cancel/kill lifecycle surface from
// `crates/vb_runtime/src/shard/lifecycle/chunk_002.rs:127-174`.

#![forbid(unsafe_code)]
#![allow(dead_code)]

use vstd::prelude::*;

verus! {

#[path = "production_inner/cancel_kill_lattice_production.rs"]
pub mod production_inner;

pub use production_inner::Counters;
pub use production_inner::Frame;
pub use production_inner::RunId;
pub use production_inner::RunState;
pub use production_inner::RuntimeError;
pub use production_inner::RuntimeJournalEvent;
pub use production_inner::RuntimeResult;
pub use production_inner::Shard;
pub use production_inner::String;
pub use production_inner::TraceEvent;
pub use production_inner::TraceRing;

} // verus!
