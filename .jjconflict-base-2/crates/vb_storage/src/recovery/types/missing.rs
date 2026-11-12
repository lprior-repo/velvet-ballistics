#![forbid(unsafe_code)]

/// Identifier for a single full-RunState component whose absence is
/// tracked on a [`super::RecoveryCannotResumeState`] witness.
///
/// These map 1:1 to the seven `*_missing` flags on
/// [`super::RecoveryCannotResumeState`]. The enum is intentionally distinct
/// from the mask struct ([`MissingRunStateComponents`]) so callers
/// can talk about a single component (e.g. via
/// [`MissingRunStateComponents::single`]) without constructing the
/// whole bitmask.
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
/// [`super::RecoveryCannotResumeState::mark_missing_components`] so the
/// priority chain reason string in
/// [`super::RecoveryCannotResumeState::unsupported_reason`] can exercise
/// every reachable token (e.g. `"store_missing"`,
/// `"action_attempts_missing"`, `"admission_missing"`,
/// `"collect_states_missing"`, `"action_contracts_missing"`,
/// `"action_abi_digests_missing"`), not just the first one in priority
/// order (`"workflow_missing"`).
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
            MissingRunStateComponent::Workflow => Self {
                workflow: true,
                ..Self::NONE
            },
            MissingRunStateComponent::Store => Self {
                store: true,
                ..Self::NONE
            },
            MissingRunStateComponent::ActionAttempts => Self {
                action_attempts: true,
                ..Self::NONE
            },
            MissingRunStateComponent::Admission => Self {
                admission: true,
                ..Self::NONE
            },
            MissingRunStateComponent::CollectStates => Self {
                collect_states: true,
                ..Self::NONE
            },
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
