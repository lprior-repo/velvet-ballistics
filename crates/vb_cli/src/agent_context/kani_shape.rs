#![forbid(unsafe_code)]

use super::constants::{
    AGENT_CONTEXT_KIND, CAPABILITY_BLOCKER_COUNT, CLI_NAME, COMMAND_COUNT, ENUM_COUNT,
    EXIT_CODE_COUNT, LANGUAGE_VERSION, POLICY_BLOCKER_COUNT, RESOURCE_BLOCKER_COUNT,
    STATIC_SERIALIZED_UPPER_BOUND, VOCABULARY_ARRAY_COUNT,
};

pub(crate) const ACTIVE_GATE_COUNT: usize = 5;
pub(crate) const BOOL_CONTRACT_COUNT: usize = 5;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct AgentContextShape {
    version_len: usize,
}

pub(crate) const fn build_shape(version_len: usize) -> AgentContextShape {
    AgentContextShape { version_len }
}

impl AgentContextShape {
    pub(crate) const fn has_required_fields(self) -> bool {
        !super::constants::SCHEMA_VERSION.is_empty()
            && !AGENT_CONTEXT_KIND.is_empty()
            && !CLI_NAME.is_empty()
            && !LANGUAGE_VERSION.is_empty()
    }

    pub(crate) const fn has_runtime_policy_fields(self) -> bool {
        ACTIVE_GATE_COUNT == 5 && POLICY_BLOCKER_COUNT == 8
    }

    pub(crate) const fn includes_agent_context_command(self) -> bool {
        COMMAND_COUNT >= 1
    }

    pub(crate) const fn exit_code_count(self) -> usize {
        EXIT_CODE_COUNT
    }

    pub(crate) const fn blocker_category_count(self) -> usize {
        3
    }

    pub(crate) const fn serialized_size_upper_bound(self) -> usize {
        match STATIC_SERIALIZED_UPPER_BOUND.checked_add(self.version_len) {
            Some(total) => total,
            None => usize::MAX,
        }
    }

    pub(crate) const fn deterministic_fingerprint(self) -> usize {
        STATIC_SERIALIZED_UPPER_BOUND
            ^ ACTIVE_GATE_COUNT
            ^ EXIT_CODE_COUNT
            ^ ENUM_COUNT
            ^ COMMAND_COUNT
            ^ POLICY_BLOCKER_COUNT
            ^ RESOURCE_BLOCKER_COUNT
            ^ BOOL_CONTRACT_COUNT
            ^ VOCABULARY_ARRAY_COUNT
            ^ self.version_len
    }

    pub(crate) const fn structural_fingerprint(self) -> usize {
        ACTIVE_GATE_COUNT
            ^ EXIT_CODE_COUNT
            ^ ENUM_COUNT
            ^ COMMAND_COUNT
            ^ POLICY_BLOCKER_COUNT
            ^ RESOURCE_BLOCKER_COUNT
            ^ BOOL_CONTRACT_COUNT
            ^ VOCABULARY_ARRAY_COUNT
    }

    pub(crate) const fn policy_blocker_count(self) -> usize {
        POLICY_BLOCKER_COUNT
    }

    pub(crate) const fn resource_blocker_count(self) -> usize {
        RESOURCE_BLOCKER_COUNT
    }

    pub(crate) const fn capability_blocker_count(self) -> usize {
        CAPABILITY_BLOCKER_COUNT
    }

    pub(crate) const fn command_count(self) -> usize {
        COMMAND_COUNT
    }

    pub(crate) const fn output_is_object(self) -> bool {
        true
    }

    pub(crate) const fn enum_count(self) -> usize {
        ENUM_COUNT
    }

    pub(crate) const fn bool_contract_count(self) -> usize {
        BOOL_CONTRACT_COUNT
    }

    pub(crate) const fn vocabulary_array_count(self) -> usize {
        VOCABULARY_ARRAY_COUNT
    }
}
