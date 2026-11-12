#![forbid(unsafe_code)]
//! Single-threaded shard owning mutable run state directly.

use core::num::NonZeroU16;
use crossbeam_queue::ArrayQueue;
use indexmap::{IndexMap, IndexSet};
use std::time::Instant;
use vb_core::action::{ActionContract, ActionTicket};
use vb_core::capability::CapabilitySet;
use vb_core::frame::{RunFrame, RunFrameCheckpointV1};
use vb_core::ids::{ActionId, RunId, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::manual_gate::{ApproverId, GateDecisionInput, GateId, ManualGate};
use vb_core::value::{SlotValue, Taint};
use vb_core::value_store::ValueStore;
use vb_core::workflow::CompiledWorkflow;
use vb_storage::EventSeq;

use crate::RuntimeResult;
use crate::counters::ShardCounters;
use crate::frame_pool::FramePool;
use crate::journal::SharedRuntimeJournal;
use crate::primitives::collect::{CollectStates, CollectStatesCheckpointV1};

use super::timer_wheel::{TimerEntry, TimerWheel};
use crate::trace::TraceRing;

// Aggregate resource model touchpoints for vb-qi37.2.1:
// ShardConfig aggregate_capacity, Shard active_usage, Shard reservations,
// RunState AggregateReservation, ShardStatus active_usage aggregate_capacity.

include!("types/pending_timer.rs");

include!("types/commands.rs");

include!("types/recover.rs");

include!("types/ask.rs");

include!("types/inspect.rs");

include!("types/introspection.rs");

include!("types/command_queue.rs");

include!("types/run_aggregate.rs");

include!("types/run_aggregate_terminal.rs");

include!("types/shard.rs");

include!("types/runtime_state.rs");

include!("types/timers.rs");

include!("types/tests.rs");

include!("types/introspection_poison_regression_tests.rs");

include!("types/format_snapshot_tests.rs");
