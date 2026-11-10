#![forbid(unsafe_code)]

use super::cannot_resume_reason::priority_reason;
use super::{
    MissingRunStateComponents, RecoveredStepState, RecoveryFrameSeed, UnsupportedRecoveryState,
};

/// Typed recovery decision for live resume eligibility.
///
/// This is deliberately wider than [`UnsupportedRecoveryState`]: a frame seed
/// can have supported slot bytes and still be unsafe to resume because live
/// runtime boundary state is not represented by `RunFrame`.
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

    /// Starts classification from storage-level unsupported evidence.
    #[must_use]
    pub const fn from_unsupported(unsupported: UnsupportedRecoveryState) -> Self {
        Self {
            slot_values: unsupported.slot_values,
            slot_taint: unsupported.slot_taint,
            action_payloads: unsupported.action_payloads,
            pending_actions: unsupported.pending_actions,
            pending_timers: false,
            pending_asks: false,
            workflow_missing: false,
            store_missing: false,
            action_attempts_missing: false,
            admission_missing: false,
            collect_states_missing: false,
            action_contracts_missing: false,
            action_abi_digests_missing: false,
        }
    }

    /// Classifies a recovered frame seed as resumable or cannot-resume.
    ///
    /// The frame seed is missing every component of the live runtime
    /// `RunState` (workflow, store, action attempts, admission,
    /// collect states, action contracts, action ABI digests), so the
    /// mask is [`MissingRunStateComponents::ALL`]. Tests that want to
    /// exercise a single second-half reason token call
    /// [`Self::mark_missing_components`] directly with
    /// [`MissingRunStateComponents::single`] instead.
    #[must_use]
    pub fn from_seed(seed: &RecoveryFrameSeed) -> Self {
        let mut state = Self::from_unsupported(seed.unsupported);
        state.pending_actions = state.pending_actions || !seed.pending_actions.is_empty();
        for entry in &seed.steps {
            state.classify_step_state(entry.state);
        }
        state = state.mark_missing_components(MissingRunStateComponents::ALL);
        state
    }

    /// Apply a parameter-mask of missing full-RunState components,
    /// setting the corresponding `*_missing` flags. Replaces the
    /// previous unconditional `mark_full_run_state_missing`. Tests
    /// drive this with [`MissingRunStateComponents::single`] to
    /// exercise each second-half reason token
    /// (`"store_missing"`, `"action_attempts_missing"`, etc.) in
    /// isolation, since the priority chain in
    /// [`Self::unsupported_reason`] is dominated by `"workflow_missing"`
    /// when all seven flags are set together.
    #[must_use]
    pub const fn mark_missing_components(mut self, components: MissingRunStateComponents) -> Self {
        if components.workflow {
            self.workflow_missing = true;
        }
        if components.store {
            self.store_missing = true;
        }
        if components.action_attempts {
            self.action_attempts_missing = true;
        }
        if components.admission {
            self.admission_missing = true;
        }
        if components.collect_states {
            self.collect_states_missing = true;
        }
        if components.action_contracts {
            self.action_contracts_missing = true;
        }
        if components.action_abi_digests {
            self.action_abi_digests_missing = true;
        }
        self
    }

    /// Deprecated alias for
    /// [`Self::mark_missing_components`] with
    /// [`MissingRunStateComponents::ALL`]. Retained so any external
    /// call site (currently none — only `from_seed` invokes it) keeps
    /// compiling unchanged. New code should call
    /// [`Self::mark_missing_components`] directly.
    #[must_use]
    #[deprecated(
        since = "0.0.0",
        note = "use `mark_missing_components(MissingRunStateComponents::ALL)` instead"
    )]
    pub const fn mark_full_run_state_missing(self) -> Self {
        self.mark_missing_components(MissingRunStateComponents::ALL)
    }

    fn classify_step_state(&mut self, state: RecoveredStepState) {
        match state {
            RecoveredStepState::Waiting => {
                self.pending_timers = true;
            }
            RecoveredStepState::Asking => {
                self.pending_asks = true;
            }
            RecoveredStepState::Running
            | RecoveredStepState::Succeeded
            | RecoveredStepState::Failed => {}
        }
    }

    /// Returns true only when every cannot-resume flag is false.
    #[must_use]
    pub const fn is_resumable(self) -> bool {
        !self.slot_values
            && !self.slot_taint
            && !self.action_payloads
            && !self.pending_actions
            && !self.pending_timers
            && !self.pending_asks
            && !self.workflow_missing
            && !self.store_missing
            && !self.action_attempts_missing
            && !self.admission_missing
            && !self.collect_states_missing
            && !self.action_contracts_missing
            && !self.action_abi_digests_missing
    }

    /// Canonical reason strings ordered by classification priority
    /// (the first true flag wins). Index `i` corresponds to
    /// [`flag_at`](Self::flag_at).
    pub const CANNOT_RESUME_REASONS: [&str; 13] = [
        "slot_values",
        "slot_taint",
        "action_payloads",
        "pending_actions",
        "pending_timers",
        "pending_asks",
        "workflow_missing",
        "store_missing",
        "action_attempts_missing",
        "admission_missing",
        "collect_states_missing",
        "action_contracts_missing",
        "action_abi_digests_missing",
    ];

    /// Returns the cannot-resume flag at priority index `i`. Out-of-
    /// range indices return `false` so the walk above terminates.
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

    /// Canonical reason string for a typed `UnsupportedFrameSeed` error.
    ///
    /// Dispatches via the `priority_class_first_half` +
    /// `priority_class_second_half` priority scanners plus the
    /// `priority_reason` mapping. The first true flag in
    /// classification-priority order wins; `"resumable"` is the
    /// fallback when every flag is false.
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
}
