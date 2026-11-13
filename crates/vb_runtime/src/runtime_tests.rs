#![forbid(unsafe_code)]
//! Tests for the multi-shard runtime.

use crate::journal::{RuntimeJournal, RuntimeJournalEvent, VolatileRuntimeJournal};
use crate::runtime::Runtime;
use crate::shard::ShardConfig;
use std::sync::{Arc, Mutex};
use vb_core::action::{
    ActionContract, ActionFailureCode, ActionName, ActionOutputReady, ActionTicket, Idempotency,
    RetrySafety, SideEffect,
};
use vb_core::capability::{Capability, CapabilitySet};
use vb_core::ids::{ActionId, ConstIdx, SeqNo, SlotIdx, StepIdx, WorkflowDigest};
use vb_core::policy::RuntimePolicy;
use vb_core::value::{ConstValue, SlotValue, Taint};
use vb_core::workflow::{CompiledNodeKind, ResourceContract, WorkflowParts};

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::RuntimeError;
    use crate::engine::action::compute_idempotency_key;
    use crate::runtime::RecoveryPrefixEvidence;
    use crate::shard::{
        AskAnswer, InspectResponse, PendingTimerKind, RuntimeState, ShardDirective,
    };
    use crate::trace::TraceEvent;
    use std::num::NonZeroUsize;
    use vb_core::ids::RunId;
    use vb_core::manual_gate::{
        ApproverId, ApproverRole, BoundGateOutcomeRecord, BoundManualGate, GateBinding,
        GateDecision, GateDecisionInput, GateDigest, GateId, GateOutcome, GateOutcomeRecord,
        ManualGate, OperatorTrust, RiskClass, SeparationOfDuties,
    };

    use vb_core::workflow::{CompiledInputConstraint, CompiledInputSlot, InputSlotKind};

    include!("runtime_tests/helpers_and_recovery_tests.rs");

    include!("runtime_tests/lifecycle_and_routing.rs");

    include!("runtime_tests/quota_recovery_tests.rs");

    include!("runtime_tests/gate_decisions_and_migration.rs");

    include!("runtime_tests/migrated_run_methods.rs");

    include!("runtime_tests/counters_scheduler_and_queues.rs");

    include!("runtime_tests/resource_depth_regressions.rs");

    include!("runtime_tests/scheduler_timers_and_snapshots.rs");
}
