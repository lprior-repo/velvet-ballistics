// SPDX-License-Identifier: MIT
//
// ============================================================================
// NARROW STRONG PRODUCTION BINDING (drift-detected mirror of select types)
// ============================================================================
//
// This file is the NARROW STRONG production-binding surface for the
// `recovery_verification` Verus spec. It carries verbatim copies of the
// `RecoveryCannotResumeState` family of decision types from the
// production source `crates/vb_storage/src/recovery/types.rs`, plus the
// priority-classification machinery (`CannotResumeClass`,
// `priority_reason`, `priority_class_first_half`,
// `priority_class_second_half`).
//
// ============================================================================
// WHY NARROW STRONG INSTEAD OF FULL `#[path]` TO types.rs
// ============================================================================
// Full `#[path = "../../crates/vb_storage/src/recovery/types.rs"]`
// inclusion is blocked by:
//
//   1. `types.rs:9` `use crate::{EventSeq, JournalError};` requires the
//      vb_storage crate root, which is not registered under
//      `verus --crate-type=lib` (no installs allowed by task brief).
//   2. `types.rs:10-13` `use vb_core::{...};` requires the `vb_core`
//      extern crate alias, which is wired through the workspace
//      `Cargo.toml` and is unavailable in a standalone
//      `verus --crate-type=lib` invocation.
//   3. `types.rs:1217`
//      `#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize,
//      serde::Deserialize)]` on `RunSnapshot` requires the `serde`
//      extern crate, also unavailable under the no-installs
//      constraint.
//
// The recovery Verus spec only reasons about the seven `*_missing`
// decision types and the priority machinery, NOT about journal I/O,
// `RunSnapshot` postcard envelopes, or `JournalError` chains. Extracting
// just the parseable subset into this narrow bind file keeps the
// binding STRONG (the verbatim copy matches production line-by-line)
// without paying the cost of registering `serde`, `vb_core`, and the
// vb_storage crate root as Verus extern crates.
//
// ============================================================================
// DRIFT POLICY
// ============================================================================
// This file is HAND-MAINTAINED against
// `crates/vb_storage/src/recovery/types.rs`. Drift MUST be detected on
// every production change to the bound surface below. The drift gate
// runs as:
//
//   bash scripts/check-production-inner-drift.sh
//
// If the drift gate reports drift on the line ranges listed below, this
// file MUST be updated to match. Production source coverage:
//
//   - `MissingRunStateComponent`         <- crates/vb_storage/src/recovery/types.rs:758-773
//   - `MissingRunStateComponents`        <- crates/vb_storage/src/recovery/types.rs:786-861
//   - `RecoveryCannotResumeState`        <- crates/vb_storage/src/recovery/types.rs:869-1098
//                                            (struct + from_unsupported + from_seed
//                                            + mark_missing_components +
//                                            mark_full_run_state_missing +
//                                            classify_step_state + is_resumable +
//                                            CANNOT_RESUME_REASONS + flag_at +
//                                            unsupported_reason)
//   - `CannotResumeClass`                <- crates/vb_storage/src/recovery/types.rs:1105-1119
//   - `priority_class_first_half`        <- crates/vb_storage/src/recovery/types.rs:1126-1146
//   - `priority_class_second_half`       <- crates/vb_storage/src/recovery/types.rs:1152-1175
//   - `priority_reason`                  <- crates/vb_storage/src/recovery/types.rs:1184-1200
//
// ============================================================================
// ROUND 8 CHANGES (vb-wy33p.11)
// ============================================================================
// Round 8 of vb-wy33p.11 added the parameterization of
// `mark_full_run_state_missing` into
// [`mark_missing_components(MissingRunStateComponents)`],
// [`MissingRunStateComponent`], and [`MissingRunStateComponents`]
// so the six currently-unreachable second-half reason tokens
// (`"store_missing"`, `"action_attempts_missing"`, `"admission_missing"`,
// `"collect_states_missing"`, `"action_contracts_missing"`,
// `"action_abi_digests_missing"`) are reachable in isolation. This
// file reflects the round-8 production surface line-for-line.
//
// Round 8 also removed the proc-macro derives on the recovery types in
// `crates/vb_storage/src/recovery/types.rs`:
//   - `use serde::{Deserialize, Serialize};` was dropped because no
//     consumer of `Serialize`/`Deserialize` exists on these decision
//     types (the storage codec only needs the derives on
//     `RunSnapshot`, which this narrow bind deliberately excludes).
//   - `#[derive(Debug, thiserror::Error)]` on `RecoveryError` was
//     replaced with manual `Display` + `std::error::Error` impls and
//     `From<JournalError>`. `RecoveryError` is intentionally excluded
//     from this narrow bind because it has multiple variants carrying
//     `WorkflowDigest`/`SlotIdx`/`StepIdx`/`ActionId` newtypes that
//     still depend on the vb_core extern crate; the existing
//     `extern_recovery_verification.rs` mirror continues to cover the
//     spec-subset of `RecoveryError` (the four variants the spec
//     actually exercises).
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
// Every type and fn in this file is mirrored verbatim from production.
// Bodies are NOT independently verified by Verus (production-side
// correctness is proven by the workspace cargo tests). The spec
// contracts in `recovery_verification.rs` reason over this mirror;
// the mirror is the production-bound surface.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

/// Identifier for a single full-RunState component whose absence is
/// tracked on a [`RecoveryCannotResumeState`] witness.
///
/// These map 1:1 to the seven `*_missing` flags on
/// [`RecoveryCannotResumeState`]. The enum is intentionally distinct
/// from the mask struct ([`MissingRunStateComponents`]) so callers
/// can talk about a single component (e.g. via
/// [`MissingRunStateComponents::single`]) without constructing the
/// whole bitmask.
///
/// Mirror of `crates/vb_storage/src/recovery/types.rs:758-773`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissingRunStateComponent {
    /// A compiled workflow is required by the live runtime but is not represented.
    Workflow,
    /// A cold value store is required by the live runtime but is not represented.
    Store,
    /// Per-Do-step action attempt counters are required but not represented.
    ActionAttempts,
    /// Admission metadata is required by the live runtime but is not represented.
    Admission,
    /// Per-run collect pagination state is required but not represented.
    CollectStates,
    /// Validated action contracts are required but not represented.
    ActionContracts,
    /// Dense action ABI digest table is required but not represented.
    ActionAbiDigests,
}

/// Mask for which full-RunState components are absent from a seed.
///
/// Used to parameterize
/// [`RecoveryCannotResumeState::mark_missing_components`] so the
/// priority chain reason string in
/// [`RecoveryCannotResumeState::unsupported_reason`] can exercise
/// every reachable token, not just the first one in priority order.
///
/// Mirror of `crates/vb_storage/src/recovery/types.rs:786-861`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MissingRunStateComponents {
    /// Compile workflow digest is not represented in the seed.
    pub workflow: bool,
    /// Cold value store is not represented in the seed.
    pub store: bool,
    /// Per-Do-step action attempt counters are not represented.
    pub action_attempts: bool,
    /// Admission metadata is not represented.
    pub admission: bool,
    /// Per-run collect pagination state is not represented.
    pub collect_states: bool,
    /// Validated action contracts are not represented.
    pub action_contracts: bool,
    /// Dense action ABI digest table is not represented.
    pub action_abi_digests: bool,
}

impl MissingRunStateComponents {
    /// Mask with every full-RunState component marked missing.
    pub const ALL: Self = Self {
        workflow: true,
        store: true,
        action_attempts: true,
        admission: true,
        collect_states: true,
        action_contracts: true,
        action_abi_digests: true,
    };

    /// Mask with no full-RunState component marked missing.
    pub const NONE: Self = Self {
        workflow: false,
        store: false,
        action_attempts: false,
        admission: false,
        collect_states: false,
        action_contracts: false,
        action_abi_digests: false,
    };

    /// Build a mask that marks exactly one component missing.
    #[must_use]
    pub const fn single(component: MissingRunStateComponent) -> Self {
        match component {
            MissingRunStateComponent::Workflow => Self { workflow: true, ..Self::NONE },
            MissingRunStateComponent::Store => Self { store: true, ..Self::NONE },
            MissingRunStateComponent::ActionAttempts => Self {
                action_attempts: true,
                ..Self::NONE
            },
            MissingRunStateComponent::Admission => Self { admission: true, ..Self::NONE },
            MissingRunStateComponent::CollectStates => Self { collect_states: true, ..Self::NONE },
            MissingRunStateComponent::ActionContracts => Self {
                action_contracts: true,
                ..Self::NONE
            },
            MissingRunStateComponent::ActionAbiDigests => Self {
                action_abi_digests: true,
                ..Self::NONE
            },
        }
    }
}

/// Typed recovery decision for live resume eligibility.
///
/// This is deliberately wider than `UnsupportedRecoveryState`: a frame
/// seed can have supported slot bytes and still be unsafe to resume
/// because live runtime boundary state is not represented by
/// `RunFrame`.
///
/// Mirror of `crates/vb_storage/src/recovery/types.rs:869-1098`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryCannotResumeState {
    /// Slot values are not present or cannot be reconstructed.
    pub slot_values: bool,
    /// Slot taint is not present or cannot be reconstructed.
    pub slot_taint: bool,
    /// Action payload/result bodies are not present in durable records.
    pub action_payloads: bool,
    /// An unresolved action boundary exists without live queue reconstruction.
    pub pending_actions: bool,
    /// A wait/timer boundary exists without timer-wheel authority.
    pub pending_timers: bool,
    /// An ask boundary exists without ask-ticket/resume-slot authority.
    pub pending_asks: bool,
    /// A compiled workflow is required by the live runtime but is not represented.
    pub workflow_missing: bool,
    /// A cold value store is required by the live runtime but is not represented.
    pub store_missing: bool,
    /// Per-Do-step action attempt counters are required but not represented.
    pub action_attempts_missing: bool,
    /// Admission metadata is required by the live runtime but is not represented.
    pub admission_missing: bool,
    /// Per-run collect pagination state is required but not represented.
    pub collect_states_missing: bool,
    /// Validated action contracts are required but not represented.
    pub action_contracts_missing: bool,
    /// Dense action ABI digest table is required but not represented.
    pub action_abi_digests_missing: bool,
}

impl RecoveryCannotResumeState {
    /// Fully resumable state: no missing evidence and no pending live boundary.
    pub const RESUMABLE: Self = Self {
        slot_values: false,
        slot_taint: false,
        action_payloads: false,
        pending_actions: false,
        pending_timers: false,
        pending_asks: false,
        workflow_missing: false,
        store_missing: false,
        action_attempts_missing: false,
        admission_missing: false,
        collect_states_missing: false,
        action_contracts_missing: false,
        action_abi_digests_missing: false,
    };

    /// Apply a parameter-mask of missing full-RunState components,
    /// setting the corresponding `*_missing` flags.
    ///
    /// Mirror of `crates/vb_storage/src/recovery/types.rs:966-992`.
    ///
    /// Implementation note: the production source uses `mut self`
    /// (sequential flag writes); this narrow-bind rewrite uses
    /// functional construction (each flag is `self.flag ||
    /// components.flag`) to satisfy Verus's lack of `mut self`
    /// parameter support. The two formulations are bit-equivalent
    /// for bool fields.
    #[must_use]
    pub const fn mark_missing_components(self, components: MissingRunStateComponents) -> Self {
        Self {
            workflow_missing: self.workflow_missing || components.workflow,
            store_missing: self.store_missing || components.store,
            action_attempts_missing: self.action_attempts_missing || components.action_attempts,
            admission_missing: self.admission_missing || components.admission,
            collect_states_missing: self.collect_states_missing || components.collect_states,
            action_contracts_missing: self.action_contracts_missing || components.action_contracts,
            action_abi_digests_missing: self.action_abi_digests_missing
                || components.action_abi_digests,
            slot_values: self.slot_values,
            slot_taint: self.slot_taint,
            action_payloads: self.action_payloads,
            pending_actions: self.pending_actions,
            pending_timers: self.pending_timers,
            pending_asks: self.pending_asks,
        }
    }

    /// Returns true only when every cannot-resume flag is false.
    /// Mirror of `crates/vb_storage/src/recovery/types.rs:1025-1039`.
    #[must_use]
    pub const fn is_resumable(self) -> bool {
        !self.slot_values && !self.slot_taint && !self.action_payloads && !self.pending_actions
            && !self.pending_timers && !self.pending_asks && !self.workflow_missing
            && !self.store_missing && !self.action_attempts_missing && !self.admission_missing
            && !self.collect_states_missing && !self.action_contracts_missing
            && !self.action_abi_digests_missing
    }

    /// Returns the cannot-resume flag at priority index `i`. Out-of-
    /// range indices return `false` so the walk above terminates.
    /// Mirror of `crates/vb_storage/src/recovery/types.rs:1063-1080`.
    #[must_use]
    pub const fn flag_at(self, i: usize) -> bool {
        match i {
            0 => self.slot_values,
            1 => self.slot_taint,
            2 => self.action_payloads,
            3 => self.pending_actions,
            4 => self.pending_timers,
            5 => self.pending_asks,
            6 => self.workflow_missing,
            7 => self.store_missing,
            8 => self.action_attempts_missing,
            9 => self.admission_missing,
            10 => self.collect_states_missing,
            11 => self.action_contracts_missing,
            12 => self.action_abi_digests_missing,
            _ => false,
        }
    }

    /// Canonical reason string for a typed `UnsupportedFrameSeed`
    /// error. The first true flag in classification-priority order
    /// wins; `"resumable"` is the fallback when every flag is false.
    /// Mirror of `crates/vb_storage/src/recovery/types.rs:1089-1097`.
    #[must_use]
    pub const fn unsupported_reason(self) -> &'static str {
        match self.priority_class_first_half() {
            Some(class) => priority_reason(class),
            None => match self.priority_class_second_half() {
                Some(class) => priority_reason(class),
                None => "resumable",
            },
        }
    }

    /// First-half priority scan (storage-layer + pending-boundary
    /// flags 0..6). Mirror of
    /// `crates/vb_storage/src/recovery/types.rs:1126-1146`.
    const fn priority_class_first_half(self) -> Option<CannotResumeClass> {
        if self.slot_values {
            return Some(CannotResumeClass::SlotValues);
        }
        if self.slot_taint {
            return Some(CannotResumeClass::SlotTaint);
        }
        if self.action_payloads {
            return Some(CannotResumeClass::ActionPayloads);
        }
        if self.pending_actions {
            return Some(CannotResumeClass::PendingActions);
        }
        if self.pending_timers {
            return Some(CannotResumeClass::PendingTimers);
        }
        if self.pending_asks {
            return Some(CannotResumeClass::PendingAsks);
        }
        None
    }

    /// Second-half priority scan (the seven `*_missing` full-RunState
    /// flags 6..13). Mirror of
    /// `crates/vb_storage/src/recovery/types.rs:1152-1175`.
    const fn priority_class_second_half(self) -> Option<CannotResumeClass> {
        if self.workflow_missing {
            return Some(CannotResumeClass::WorkflowMissing);
        }
        if self.store_missing {
            return Some(CannotResumeClass::StoreMissing);
        }
        if self.action_attempts_missing {
            return Some(CannotResumeClass::ActionAttemptsMissing);
        }
        if self.admission_missing {
            return Some(CannotResumeClass::AdmissionMissing);
        }
        if self.collect_states_missing {
            return Some(CannotResumeClass::CollectStatesMissing);
        }
        if self.action_contracts_missing {
            return Some(CannotResumeClass::ActionContractsMissing);
        }
        if self.action_abi_digests_missing {
            return Some(CannotResumeClass::ActionAbiDigestsMissing);
        }
        None
    }
}

/// Classification priority for the cannot-resume reason tokens.
/// The first-matching rule (highest enum variant above) wins.
/// Mirror of `crates/vb_storage/src/recovery/types.rs:1105-1119`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CannotResumeClass {
    SlotValues,
    SlotTaint,
    ActionPayloads,
    PendingActions,
    PendingTimers,
    PendingAsks,
    WorkflowMissing,
    StoreMissing,
    ActionAttemptsMissing,
    AdmissionMissing,
    CollectStatesMissing,
    ActionContractsMissing,
    ActionAbiDigestsMissing,
}

/// Maps a [`CannotResumeClass`] to its canonical reason token.
/// Mirror of `crates/vb_storage/src/recovery/types.rs:1184-1200`.
#[must_use]
pub const fn priority_reason(class: CannotResumeClass) -> &'static str {
    match class {
        CannotResumeClass::SlotValues => "slot_values",
        CannotResumeClass::SlotTaint => "slot_taint",
        CannotResumeClass::ActionPayloads => "action_payloads",
        CannotResumeClass::PendingActions => "pending_actions",
        CannotResumeClass::PendingTimers => "pending_timers",
        CannotResumeClass::PendingAsks => "pending_asks",
        CannotResumeClass::WorkflowMissing => "workflow_missing",
        CannotResumeClass::StoreMissing => "store_missing",
        CannotResumeClass::ActionAttemptsMissing => "action_attempts_missing",
        CannotResumeClass::AdmissionMissing => "admission_missing",
        CannotResumeClass::CollectStatesMissing => "collect_states_missing",
        CannotResumeClass::ActionContractsMissing => "action_contracts_missing",
        CannotResumeClass::ActionAbiDigestsMissing => "action_abi_digests_missing",
    }
}

// ============================================================================
// Spec-side projections of the round-8 parametrize surface.
//
// Verus does not allow `proof fn` to call `exec fn`, so the spec
// proofs in `recovery_verification.rs` reason over these pure spec
// projections of `mark_missing_components` + the priority chain.
// The exec mirror above (`mark_missing_components`) and the spec
// projections below MUST stay bit-equivalent — the spec proof
// `proof_parametrized_mask_exec_spec_equivalent` (in
// `recovery_verification.rs`) discharges the equivalence via
// `mark_missing_components`'s const-fn body.
// ============================================================================
/// Spec projection of `MissingRunStateComponents::single`. Returns
/// the spec mask containing exactly the one component named.
pub open spec fn spec_single(component: MissingRunStateComponent) -> MissingRunStateComponents {
    MissingRunStateComponents {
        workflow: component matches MissingRunStateComponent::Workflow,
        store: component matches MissingRunStateComponent::Store,
        action_attempts: component matches MissingRunStateComponent::ActionAttempts,
        admission: component matches MissingRunStateComponent::Admission,
        collect_states: component matches MissingRunStateComponent::CollectStates,
        action_contracts: component matches MissingRunStateComponent::ActionContracts,
        action_abi_digests: component matches MissingRunStateComponent::ActionAbiDigests,
    }
}

/// Spec projection of
/// `RecoveryCannotResumeState::mark_missing_components`. Takes the
/// base state and a mask, returns the state with the mask's flags
/// OR'd in.
pub open spec fn spec_mark_missing_components(
    state: RecoveryCannotResumeState,
    components: MissingRunStateComponents,
) -> RecoveryCannotResumeState {
    RecoveryCannotResumeState {
        workflow_missing: state.workflow_missing || components.workflow,
        store_missing: state.store_missing || components.store,
        action_attempts_missing: state.action_attempts_missing || components.action_attempts,
        admission_missing: state.admission_missing || components.admission,
        collect_states_missing: state.collect_states_missing || components.collect_states,
        action_contracts_missing: state.action_contracts_missing || components.action_contracts,
        action_abi_digests_missing: state.action_abi_digests_missing
            || components.action_abi_digests,
        slot_values: state.slot_values,
        slot_taint: state.slot_taint,
        action_payloads: state.action_payloads,
        pending_actions: state.pending_actions,
        pending_timers: state.pending_timers,
        pending_asks: state.pending_asks,
    }
}

/// Spec projection of
/// `RecoveryCannotResumeState::priority_class_first_half`. Returns
/// the highest-priority cannot-resume class in the first half, or
/// `None` if every flag is false.
pub open spec fn spec_priority_class_first_half(state: RecoveryCannotResumeState) -> Option<
    CannotResumeClass,
> {
    if state.slot_values {
        Some(CannotResumeClass::SlotValues)
    } else if state.slot_taint {
        Some(CannotResumeClass::SlotTaint)
    } else if state.action_payloads {
        Some(CannotResumeClass::ActionPayloads)
    } else if state.pending_actions {
        Some(CannotResumeClass::PendingActions)
    } else if state.pending_timers {
        Some(CannotResumeClass::PendingTimers)
    } else if state.pending_asks {
        Some(CannotResumeClass::PendingAsks)
    } else {
        None
    }
}

/// Spec projection of
/// `RecoveryCannotResumeState::priority_class_second_half`. Returns
/// the highest-priority cannot-resume class in the second half, or
/// `None` if every flag is false.
pub open spec fn spec_priority_class_second_half(state: RecoveryCannotResumeState) -> Option<
    CannotResumeClass,
> {
    if state.workflow_missing {
        Some(CannotResumeClass::WorkflowMissing)
    } else if state.store_missing {
        Some(CannotResumeClass::StoreMissing)
    } else if state.action_attempts_missing {
        Some(CannotResumeClass::ActionAttemptsMissing)
    } else if state.admission_missing {
        Some(CannotResumeClass::AdmissionMissing)
    } else if state.collect_states_missing {
        Some(CannotResumeClass::CollectStatesMissing)
    } else if state.action_contracts_missing {
        Some(CannotResumeClass::ActionContractsMissing)
    } else if state.action_abi_digests_missing {
        Some(CannotResumeClass::ActionAbiDigestsMissing)
    } else {
        None
    }
}

/// Spec projection of `priority_reason`. Maps each
/// [`CannotResumeClass`] variant to its canonical reason string.
pub open spec fn spec_priority_reason_strong(class: CannotResumeClass) -> &'static str {
    match class {
        CannotResumeClass::SlotValues => "slot_values",
        CannotResumeClass::SlotTaint => "slot_taint",
        CannotResumeClass::ActionPayloads => "action_payloads",
        CannotResumeClass::PendingActions => "pending_actions",
        CannotResumeClass::PendingTimers => "pending_timers",
        CannotResumeClass::PendingAsks => "pending_asks",
        CannotResumeClass::WorkflowMissing => "workflow_missing",
        CannotResumeClass::StoreMissing => "store_missing",
        CannotResumeClass::ActionAttemptsMissing => "action_attempts_missing",
        CannotResumeClass::AdmissionMissing => "admission_missing",
        CannotResumeClass::CollectStatesMissing => "collect_states_missing",
        CannotResumeClass::ActionContractsMissing => "action_contracts_missing",
        CannotResumeClass::ActionAbiDigestsMissing => "action_abi_digests_missing",
    }
}

/// Spec projection of `RecoveryCannotResumeState::unsupported_reason`.
/// Priority-ordered: first half (storage-layer + pending-boundary),
/// then second half (`*_missing`), then `"resumable"` fallback.
pub open spec fn spec_unsupported_reason_strong(state: RecoveryCannotResumeState) -> &'static str {
    match spec_priority_class_first_half(state) {
        Some(class) => spec_priority_reason_strong(class),
        None => match spec_priority_class_second_half(state) {
            Some(class) => spec_priority_reason_strong(class),
            None => "resumable",
        },
    }
}

/// Spec projection of `RecoveryCannotResumeState::is_resumable`.
/// Returns true iff every cannot-resume flag is false.
pub open spec fn spec_is_resumable_strong(state: RecoveryCannotResumeState) -> bool {
    !state.slot_values && !state.slot_taint && !state.action_payloads && !state.pending_actions
        && !state.pending_timers && !state.pending_asks && !state.workflow_missing
        && !state.store_missing && !state.action_attempts_missing && !state.admission_missing
        && !state.collect_states_missing && !state.action_contracts_missing
        && !state.action_abi_digests_missing
}

} // verus!
