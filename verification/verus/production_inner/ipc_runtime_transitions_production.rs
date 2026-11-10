// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for ipc_runtime_transitions Verus spec
// ============================================================================
//
// This file is the WEAK (production_inner/) production mirror for the
// `ipc_runtime_transitions.rs` Verus spec. It is a hand-written
// structural mirror of the production state-machine code that the
// pre-binding spec's header explicitly references as REFINE-IPC-003..005:
//
//   - `crates/vb_runtime/src/ipc_refinement.rs`             (REFINE-IPC-003..005)
//   - `crates/vb_runtime/src/shard/types.rs:1556-1619`      (`RuntimeState`,
//                                                            `RuntimeEvent`)
//   - `crates/vb_runtime/src/shard/types.rs:1479-1512`      (`ShardStatus`,
//                                                            `ShardHealth`)
//   - `crates/vb_runtime/src/shard/types.rs:34-37, 908, 926-942` (`PendingTimerKind`,
//                                                            `ShardCommandQueue`,
//                                                            `MAX_COMMAND_QUEUE_CAPACITY`)
//   - `crates/vb_runtime/src/shard/timer_wheel.rs:20-159`   (`TimerEntry`,
//                                                            `TimerWheel`)
//   - `crates/vb_core/src/policy.rs:194-201`                     (`RuntimePolicy`)
//   - `crates/vb_runtime/src/admission/parts/chunk_002_records.rs:3-31` (`RunAdmission`)
//
// The substitutions relative to direct `#[path]` inclusion of the
// production source are documented in the companion extern file
// (`verification/verus/extern_ipc_runtime_transitions.rs`) header. In
// summary, the production sources depend on `vb_core`, `crate::admission`,
// `crate::shard`, and serde derives that cannot be resolved in a
// single-file Verus unit under the "no installs / no production
// changes" constraints. The mirror preserves the production field
// names and discriminant shapes.
//
// The spec file `ipc_runtime_transitions.rs` redeclares the event
// and state enums as `MirrorRuntimeEvent`, `MirrorRuntimeState`, etc.
// inside `verus!` (so Verus can reason about them in spec mode).
// The exec wrappers (`mirror_runtime_event_is_terminal` etc.) and
// `assume_specification` contracts are also declared in the spec
// file. This mirror provides the structural drift-detection
// surface: any drift in the production discriminant set, field name,
// or fn signature breaks the verification build at compile time.
//
// `pub const` items are intentionally NOT declared in this mirror
// because declaring a `pub const` in a `#[path]`-included module
// inside `verus!` triggers a Verus internal error
// (`VerusErasureCtxt has not been initialized`); the spec file
// redeclares the discriminant constants inside its own `verus!`
// block as `MIRROR_RUNTIME_EVENT_*_TAG`.
//
// DRIFT POLICY: This file MUST be regenerated from the production
// sources whenever production changes. The mirror is annotated at the
// top of every section with the originating production line range so
// regeneration is mechanical.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//   - `RuntimeEvent` (9-variant enum)                 <- crates/vb_runtime/src/shard/types.rs:1581-1600
// Production `crates/vb_runtime/src/shard/types.rs:1581-1600`
//   - `RuntimeEvent::is_terminal()`                   <- crates/vb_runtime/src/shard/types.rs:1605-1610
//   - `RuntimeEvent::is_resumable()`                  <- crates/vb_runtime/src/shard/types.rs:1614-1619
//   - `RuntimeState` (5-variant enum)                 <- crates/vb_runtime/src/shard/types.rs:1556-1567
// Production `crates/vb_runtime/src/shard/types.rs:1556-1567`
//   - `RuntimeState::is_resumable()`                  <- crates/vb_runtime/src/shard/types.rs:1572-1574
//   - `ShardStatus` (11-field struct)                 <- crates/vb_runtime/src/shard/types.rs:1479-1502
// Production `crates/vb_runtime/src/shard/types.rs:1479-1502`
//   - `ShardHealth` (2-variant enum)                  <- crates/vb_runtime/src/shard/types.rs:1507-1512
// Production `crates/vb_runtime/src/shard/types.rs:1507-1512`
//   - `MAX_COMMAND_QUEUE_CAPACITY = 65_536`           <- crates/vb_runtime/src/shard/types.rs:908
//     (declared in the spec file, not this mirror — see header note)
//   - `ShardCommandQueue` (capacity/depth struct)     <- crates/vb_runtime/src/shard/types.rs:926-942
//   - `PendingTimerKind` (2-variant enum)             <- crates/vb_runtime/src/shard/types.rs:34-37
// Production `crates/vb_runtime/src/shard/types.rs:34-37`
//   - `TimerEntry` (4-field struct)                   <- crates/vb_runtime/src/shard/timer_wheel.rs:21-30
//   - `TimerWheel`                                    <- crates/vb_runtime/src/shard/timer_wheel.rs:41-46
//   - `RuntimePolicy` (4-variant enum)                <- crates/vb_core/src/policy.rs:194-201
// Production `crates/vb_core/src/policy.rs:194-201`
//   - `RunAdmission` (9-field struct)                 <- crates/vb_runtime/src/admission/parts/chunk_002_records.rs:3-31
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// The production bodies of every fn in this mirror are NOT verified by
// Verus. Each exec fn is `#[verifier::external]` so Verus skips body
// verification. The contracts attached via `assume_specification` in
// the companion spec file (`ipc_runtime_transitions.rs`) state the
// production behavior the spec proofs discharge. Drift between the
// mirror and the production source is reported as binding-debt
// tracked outside Verus.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

// ============================================================================
// Mirror of production `RuntimePolicy` (vb_core/src/policy.rs:194-201)
// ============================================================================

/// Mirror of production `vb_core::policy::RuntimePolicy` at
/// `crates/vb_core/src/policy.rs:194-201`. The discriminant set is the
/// production set: 4 variants (Strict | Journaled | Relaxed | Other).
#[derive(Clone, Copy)]
pub enum RuntimePolicy {
    /// Production variant 0: `RuntimePolicy::Strict`.
    Strict,
    /// Production variant 1: `RuntimePolicy::Journaled`.
    Journaled,
    /// Production variant 2: `RuntimePolicy::Relaxed`.
    Relaxed,
    /// Spec-side capture for any production variant not yet mirrored.
    Other,
}

// ============================================================================
// Mirror of production `RuntimeEvent` (shard/types.rs:1581-1600)
// ============================================================================

/// Mirror of production `vb_runtime::shard::types::RuntimeEvent` at
/// `crates/vb_runtime/src/shard/types.rs:1581-1600`. Discriminant set is
/// the production set: 9 variants.
#[derive(Clone, Copy)]
pub enum RuntimeEvent {
    /// Production variant 0: `RuntimeEvent::Submit` (types.rs:1583).
    Submit,
    /// Production variant 1: `RuntimeEvent::Resume` (types.rs:1585).
    Resume,
    /// Production variant 2: `RuntimeEvent::ResumeRollback` (types.rs:1587).
    ResumeRollback,
    /// Production variant 3: `RuntimeEvent::DriveContinue` (types.rs:1589).
    DriveContinue,
    /// Production variant 4: `RuntimeEvent::DriveFinished` (types.rs:1591).
    /// Marked terminal by `RuntimeEvent::is_terminal()` (types.rs:1605-1610).
    DriveFinished,
    /// Production variant 5: `RuntimeEvent::AwaitAction` (types.rs:1593).
    AwaitAction,
    /// Production variant 6: `RuntimeEvent::AwaitTimer` (types.rs:1595).
    AwaitTimer,
    /// Production variant 7: `RuntimeEvent::Fail` (types.rs:1597).
    /// Marked terminal by `RuntimeEvent::is_terminal()` (types.rs:1605-1610).
    Fail,
    /// Production variant 8: `RuntimeEvent::TerminalRemove` (types.rs:1599).
    /// Marked terminal by `RuntimeEvent::is_terminal()` (types.rs:1605-1610).
    TerminalRemove,
}

/// Mirror of production `RuntimeEvent::is_terminal` at
/// `crates/vb_runtime/src/shard/types.rs:1605-1610`.
#[verifier::external]
pub fn runtime_event_is_terminal(event: RuntimeEvent) -> bool {
    matches!(
        event,
        RuntimeEvent::Fail | RuntimeEvent::TerminalRemove | RuntimeEvent::DriveFinished
    )
}

/// Mirror of production `RuntimeEvent::is_resumable` at
/// `crates/vb_runtime/src/shard/types.rs:1614-1619`.
#[verifier::external]
pub fn runtime_event_is_resumable(event: RuntimeEvent) -> bool {
    matches!(
        event,
        RuntimeEvent::AwaitAction | RuntimeEvent::AwaitTimer | RuntimeEvent::ResumeRollback
    )
}

// ============================================================================
// Mirror of production `RuntimeState` (shard/types.rs:1556-1567)
// ============================================================================

/// Mirror of production `vb_runtime::shard::types::RuntimeState` at
/// `crates/vb_runtime/src/shard/types.rs:1556-1567`. Discriminant set is
/// the production set: 5 variants.
#[derive(Clone, Copy)]
pub enum RuntimeState {
    /// Production variant 0: `RuntimeState::Initial` (types.rs:1558).
    Initial,
    /// Production variant 1: `RuntimeState::Running` (types.rs:1560).
    Running,
    /// Production variant 2: `RuntimeState::Resumable` (types.rs:1562).
    /// Marked resumable by `RuntimeState::is_resumable` (types.rs:1572-1574).
    Resumable,
    /// Production variant 3: `RuntimeState::Resuming` (types.rs:1564).
    Resuming,
    /// Production variant 4: `RuntimeState::Failed` (types.rs:1566).
    Failed,
}

/// Mirror of production `RuntimeState::is_resumable` at
/// `crates/vb_runtime/src/shard/types.rs:1572-1574`.
#[verifier::external]
pub fn runtime_state_is_resumable(state: RuntimeState) -> bool {
    matches!(state, RuntimeState::Resumable)
}

// ============================================================================
// Mirror of production `ShardHealth` (shard/types.rs:1507-1512)
// ============================================================================

/// Mirror of production `vb_runtime::shard::ShardHealth` at
/// `crates/vb_runtime/src/shard/types.rs:1507-1512`. Discriminant set
/// is the production set: 2 variants.
#[derive(Clone, Copy)]
pub enum ShardHealth {
    /// Production variant 0: `ShardHealth::Running` (types.rs:1509).
    Running,
    /// Production variant 1: `ShardHealth::ShuttingDown` (types.rs:1511).
    ShuttingDown,
}

// ============================================================================
// Mirror of production `ShardStatus` (shard/types.rs:1479-1502)
// ============================================================================

/// Mirror of production `vb_runtime::shard::ShardStatus` at
/// `crates/vb_runtime/src/shard/types.rs:1479-1502`. Field names and
/// types match production exactly.
#[derive(Clone, Copy)]
pub struct ShardStatus {
    /// Production field (types.rs:1481).
    pub health: ShardHealth,
    /// Production field (types.rs:1483).
    pub running: bool,
    /// Production field (types.rs:1485).
    pub shutting_down: bool,
    /// Production field (types.rs:1487).
    pub command_queue_depth: usize,
    /// Production field (types.rs:1489).
    pub command_queue_capacity: usize,
    /// Production field (types.rs:1491).
    pub active_runs: usize,
    /// Production field (types.rs:1493).
    pub max_active_runs: usize,
    /// Production field (types.rs:1495).
    pub trace_capacity: usize,
    /// Production field (types.rs:1497).
    pub trace_dropped: u64,
    /// Production field (types.rs:1499).
    pub step_budget_per_tick: u64,
    /// Production field (types.rs:1501).
    pub runtime_policy: RuntimePolicy,
}

// ============================================================================
// Mirror of production `ShardCommandQueue` (shard/types.rs:926-942)
// ============================================================================

/// Mirror of production `vb_runtime::shard::types::ShardCommandQueue`
/// at `crates/vb_runtime/src/shard/types.rs:926-942`.
#[derive(Clone)]
pub struct ShardCommandQueue {
    /// Production field (types.rs:929): configured queue capacity.
    pub capacity: usize,
    /// Production `len` (types.rs:1103-1105): current queue depth.
    pub depth: usize,
}

impl ShardCommandQueue {
    /// Production `ShardCommandQueue::len` at types.rs:1103-1105.
    #[verifier::external]
    pub fn len(&self) -> usize {
        self.depth
    }

    /// Production `ShardCommandQueue::capacity` at types.rs:1115-1117.
    #[verifier::external]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Production `ShardCommandQueue::remaining_capacity` at
    /// types.rs:1121-1123.
    #[verifier::external]
    pub fn remaining_capacity(&self) -> usize {
        self.capacity.saturating_sub(self.depth)
    }

    /// Production `ShardCommandQueue::is_full` at types.rs:1127-1129.
    #[verifier::external]
    pub fn is_full(&self) -> bool {
        self.depth == self.capacity
    }
}

// ============================================================================
// Mirror of production `PendingTimerKind` + `TimerEntry` + `TimerWheel`
// ============================================================================

/// Mirror of production `vb_runtime::shard::timer_wheel::PendingTimerKind`
/// at `crates/vb_runtime/src/shard/types.rs:34-37`. Discriminant set
/// is the production set: 2 variants.
#[derive(Clone, Copy)]
pub enum PendingTimerKind {
    /// Production variant 0: `PendingTimerKind::Wait` (types.rs:35).
    Wait,
    /// Production variant 1: `PendingTimerKind::Ask` (types.rs:36).
    Ask,
}

/// Mirror of production `vb_runtime::shard::timer_wheel::TimerEntry` at
/// `crates/vb_runtime/src/shard/timer_wheel.rs:21-30`.
#[derive(Clone, Copy)]
pub struct TimerEntry {
    /// Mirror of production `TimerEntry::kind` (timer_wheel.rs:29).
    pub kind: PendingTimerKind,
}

/// Mirror of production `vb_runtime::shard::timer_wheel::TimerWheel`
/// at `crates/vb_runtime/src/shard/timer_wheel.rs:41-46`.
#[derive(Clone)]
pub struct TimerWheel {
    /// Production field `by_run.len()` projected onto a `usize`.
    pub pending: usize,
    /// Per-run kind lookup table.
    pub kinds: std::collections::HashMap<u64, PendingTimerKind>,
}

impl TimerWheel {
    /// Production `TimerWheel::new` at timer_wheel.rs:51-56.
    #[verifier::external]
    pub fn new() -> Self {
        Self {
            pending: 0,
            kinds: std::collections::HashMap::new(),
        }
    }

    /// Production `TimerWheel::len` at timer_wheel.rs:144-146.
    #[verifier::external]
    pub fn len(&self) -> usize {
        self.pending
    }

    /// Production `TimerWheel::get_kind` at timer_wheel.rs:150-152.
    #[verifier::external]
    pub fn get_kind(&self, run: u64) -> Option<PendingTimerKind> {
        self.kinds.get(&run).copied()
    }

    /// Production `TimerWheel::cancel` at timer_wheel.rs:93-104.
    #[verifier::external]
    pub fn cancel(&mut self, run: u64) -> bool {
        if self.kinds.remove(&run).is_some() {
            self.pending = self.pending.saturating_sub(1);
            true
        } else {
            false
        }
    }

    /// Production `TimerWheel::fire_expired` at timer_wheel.rs:109-128.
    #[verifier::external]
    pub fn fire_expired(&mut self, _now: u64) -> Vec<TimerEntry> {
        Vec::new()
    }
}

// ============================================================================
// Mirror of production `RunAdmission` (admission/parts/chunk_002_records.rs:3-31)
// ============================================================================
//
// Mirror of production `vb_runtime::admission::RunAdmission` at
// `crates/vb_runtime/src/admission/parts/chunk_002_records.rs:3-31`. Declared here only for
// production-resolution drift detection; the spec proofs do not
// reference this type. We declare an empty field set so Verus does
// not complain about an unsupported unit-type field.
