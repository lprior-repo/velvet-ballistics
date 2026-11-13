// Group control types only. No GroupControl implementation.
// Actual admission/completion/drain/abort logic lives in group.rs.

use super::{GroupId, LaneError, LaneToken};
use crate::value::{SlotValue, Taint};

/// How branch lanes execute relative to each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupMode {
    ForEachOrdered,
    TogetherOrdered,
    FirstSuccess,
}

/// Outcome observed by control logic; actual cause is journaled separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaneCompletion {
    Succeeded { value: SlotValue, taint: Taint },
    Failed,
}

/// Per-ordinal result slot. Owned by group control, not copied from runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultCell {
    Pending,
    Value { value: SlotValue, taint: Taint },
    Failed,
}

/// Immutable winner identity and value, set once during drain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Winner {
    pub token: LaneToken,
    pub value: SlotValue,
    pub taint: Taint,
}

/// Reasons a group reaches terminal failure state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupFailure {
    AllBranchesFailed,
    BranchFailed { ordinal: u32 },
    Canceled,
    ParentFailed,
}

/// Why a group terminates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Termination {
    Ordered,
    Winner(Winner),
    Failed(GroupFailure),
}

/// Explicit lifecycle phase; transitions only via committed events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupPhase {
    Running,
    Draining(Termination),
    Terminal(Termination),
}

/// Reasons to abort remaining work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbortReason {
    Canceled,
    ParentFailed,
}

/// Event kinds the journal adapter receives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupEventKind {
    Admission(LaneToken),
    Completion {
        token: LaneToken,
        outcome: LaneCompletion,
    },
    Drained(LaneToken),
    Aborted(AbortReason),
}

/// Immutable event record, including the next revision after commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupEvent {
    pub group: GroupId,
    pub revision: u64,
    pub kind: GroupEventKind,
}

/// Receipt asserting the adapter durably journaled the event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroupCommitReceipt {
    pub(super) event: GroupEvent,
}

impl GroupCommitReceipt {
    /// Construct receipt after adapter durably appends the exact event.
    /// The caller guarantees journal success; pure control logic cannot prove it.
    pub fn from_committed_event(event: GroupEvent) -> Self {
        Self { event }
    }
}

/// Current observable outcome. Pending until phase Terminal, then final.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupOutcome<'a> {
    Pending,
    Ordered(&'a [ResultCell]),
    Winner(Winner),
    Failed(GroupFailure),
}

/// Typed error for all group control operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupError {
    Lane(LaneError),
    EmptyGroup,
    InsufficientCapacity,
    InvalidTransition,
    InvalidReceipt,
    RevisionOverflow,
    RepresentationOverflow,
    Allocate,
}

impl From<LaneError> for GroupError {
    fn from(e: LaneError) -> Self {
        GroupError::Lane(e)
    }
}

/// Summary returned by each committed event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupTransition {
    Admitted(LaneToken),
    Changed(GroupPhase),
}

/// Per-slot state managed by group control (internal).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LaneRecord {
    Vacant,
    Live(LaneToken),
    CancelRequested(LaneToken),
}
