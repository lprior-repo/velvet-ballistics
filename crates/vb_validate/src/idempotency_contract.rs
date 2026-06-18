#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_strip,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables
)]
#![forbid(unsafe_code)]

//! Verifier-side idempotency contract checks for typed workflow IR.

use thiserror::Error;
use vb_core::action::{ActionContract, Idempotency, RetrySafety, SideEffect};
use vb_core::ids::ActionId;
use vb_core::workflow::{CompiledNodeKind, WorkflowParts};

/// Result type for verifier idempotency contract checks.
pub type IdempotencyContractResult<T> = Result<T, IdempotencyContractError>;

/// Accumulated idempotency contract violations in deterministic traversal order.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("IDEMPOTENCY_CONTRACT_VIOLATIONS")]
pub struct IdempotencyContractErrors(pub Box<[IdempotencyContractViolation]>);

/// Workflow-level idempotency contract failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum IdempotencyContractError {
    /// A Do node references an action absent from the workflow registry.
    #[error("ACTION_CONTRACT_MISSING")]
    ActionContractMissing {
        /// Missing action identifier.
        action_id: ActionId,
        /// Referencing node index.
        node_index: usize,
    },
    /// A workflow-specific registry entry is unused by the workflow.
    #[error("ACTION_CONTRACT_ORPHAN")]
    ActionContractOrphan {
        /// Orphan action identifier.
        action_id: ActionId,
    },
    /// One or more side-effecting idempotency declarations are invalid.
    #[error(transparent)]
    IdempotencyViolations(IdempotencyContractErrors),
}

/// Single statically detectable idempotency contract violation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum IdempotencyContractViolation {
    /// Side-effecting actions may not declare retry-unsafe behavior.
    #[error("IDEMPOTENCY_RETRY_UNSAFE")]
    SideEffectingRetryUnsafe {
        /// Violating action identifier.
        action: ActionId,
        /// Declared side-effect class.
        side_effect: SideEffect,
        /// Declared idempotency class.
        idempotency: Idempotency,
        /// Declared retry-safety class.
        retry_safety: RetrySafety,
    },
    /// Side-effecting actions may not declare at-least-once external behavior.
    #[error("IDEMPOTENCY_AT_LEAST_ONCE_EXTERNAL")]
    SideEffectingAtLeastOnceExternal {
        /// Violating action identifier.
        action: ActionId,
        /// Declared side-effect class.
        side_effect: SideEffect,
        /// Declared idempotency class.
        idempotency: Idempotency,
        /// Declared retry-safety class.
        retry_safety: RetrySafety,
    },
    /// Side-effecting actions may not declare deterministic-pure semantics.
    #[error("IDEMPOTENCY_SIDE_EFFECTING_DETERMINISTIC_PURE")]
    SideEffectingDeterministicPure {
        /// Violating action identifier.
        action: ActionId,
        /// Declared side-effect class.
        side_effect: SideEffect,
        /// Declared idempotency class.
        idempotency: Idempotency,
        /// Declared retry-safety class.
        retry_safety: RetrySafety,
    },
    /// Contract has an unrecognized or future-side-effect/retry-safety/idempotency
    /// combination that cannot be statically analysed.
    #[error("IDEMPOTENCY_INVALID_CONTRACT")]
    InvalidContract {
        /// Violating action identifier.
        action: ActionId,
        /// Declared side-effect class.
        side_effect: SideEffect,
        /// Declared idempotency class.
        idempotency: Idempotency,
        /// Declared retry-safety class.
        retry_safety: RetrySafety,
    },
}

impl IdempotencyContractViolation {
    /// Stable machine-readable diagnostic category.
    #[must_use]
    pub const fn reason_category(&self) -> &'static str {
        match self {
            Self::SideEffectingRetryUnsafe { .. } => "IDEMPOTENCY_RETRY_UNSAFE",
            Self::SideEffectingAtLeastOnceExternal { .. } => "IDEMPOTENCY_AT_LEAST_ONCE_EXTERNAL",
            Self::SideEffectingDeterministicPure { .. } => {
                "IDEMPOTENCY_SIDE_EFFECTING_DETERMINISTIC_PURE"
            }
            Self::InvalidContract { .. } => "IDEMPOTENCY_INVALID_CONTRACT",
        }
    }
}

/// Validates workflow-specific contract completeness, then idempotency legality.
pub fn validate_workflow_idempotency_contracts(
    parts: &WorkflowParts,
    action_contracts: &[ActionContract],
) -> IdempotencyContractResult<()> {
    ensure_contract_completeness(parts, action_contracts)?;
    collect_workflow_idempotency_violations(parts, action_contracts)
        .map_err(IdempotencyContractError::IdempotencyViolations)
}

/// Validates one action contract against the static idempotency decision table.
pub fn validate_action_idempotency_contract(
    contract: &ActionContract,
) -> Result<(), IdempotencyContractViolation> {
    is_statically_idempotent_contract(contract)
}

/// Collects all idempotency violations in input contract traversal order.
pub fn collect_idempotency_contract_violations(
    action_contracts: &[ActionContract],
) -> Result<(), IdempotencyContractErrors> {
    errors_from_violations(
        action_contracts
            .iter()
            .filter_map(|contract| is_statically_idempotent_contract(contract).err()),
    )
}

/// Returns whether a single action contract is statically idempotent.
pub fn is_statically_idempotent_contract(
    contract: &ActionContract,
) -> Result<(), IdempotencyContractViolation> {
    match (
        contract.side_effect,
        contract.retry_safety,
        contract.idempotency,
    ) {
        (SideEffect::Pure, _, _) => Ok(()),
        (side_effect, RetrySafety::NotRetrySafe, idempotency) => {
            Err(IdempotencyContractViolation::SideEffectingRetryUnsafe {
                action: contract.id,
                side_effect,
                idempotency,
                retry_safety: RetrySafety::NotRetrySafe,
            })
        }
        (side_effect, RetrySafety::Unknown, idempotency) => {
            Err(IdempotencyContractViolation::SideEffectingRetryUnsafe {
                action: contract.id,
                side_effect,
                idempotency,
                retry_safety: RetrySafety::Unknown,
            })
        }
        (side_effect, retry_safety, Idempotency::AtLeastOnceExternal) => Err(
            IdempotencyContractViolation::SideEffectingAtLeastOnceExternal {
                action: contract.id,
                side_effect,
                idempotency: Idempotency::AtLeastOnceExternal,
                retry_safety,
            },
        ),
        (side_effect, retry_safety, Idempotency::DeterministicPure) => Err(
            IdempotencyContractViolation::SideEffectingDeterministicPure {
                action: contract.id,
                side_effect,
                idempotency: Idempotency::DeterministicPure,
                retry_safety,
            },
        ),
        (
            _,
            RetrySafety::Idempotent | RetrySafety::RequiresIdempotencyKey,
            Idempotency::IdempotentExternal,
        ) => Ok(()),
        // `SideEffect`, `RetrySafety`, and `Idempotency` are all `#[non_exhaustive]`.
        // Any unrecognized combination is treated as an invalid contract.
        (side_effect, retry_safety, idempotency) => {
            Err(IdempotencyContractViolation::InvalidContract {
                action: contract.id,
                side_effect,
                retry_safety,
                idempotency,
            })
        }
    }
}

fn ensure_contract_completeness(
    parts: &WorkflowParts,
    action_contracts: &[ActionContract],
) -> IdempotencyContractResult<()> {
    match first_missing_contract(parts, action_contracts) {
        Some(error) => Err(error),
        None => match first_orphan_contract(parts, action_contracts) {
            Some(error) => Err(error),
            None => Ok(()),
        },
    }
}

fn first_missing_contract(
    parts: &WorkflowParts,
    action_contracts: &[ActionContract],
) -> Option<IdempotencyContractError> {
    parts
        .nodes
        .iter()
        .enumerate()
        .find_map(|(node_index, node)| {
            do_action(&node.kind).and_then(|action_id| {
                has_contract(action_contracts, action_id)
                    .then_some(())
                    .map_or_else(
                        || {
                            Some(IdempotencyContractError::ActionContractMissing {
                                action_id,
                                node_index,
                            })
                        },
                        |_| None,
                    )
            })
        })
}

fn first_orphan_contract(
    parts: &WorkflowParts,
    action_contracts: &[ActionContract],
) -> Option<IdempotencyContractError> {
    action_contracts.iter().find_map(|contract| {
        has_do_action(parts, contract.id).then_some(()).map_or_else(
            || {
                Some(IdempotencyContractError::ActionContractOrphan {
                    action_id: contract.id,
                })
            },
            |_| None,
        )
    })
}

fn collect_workflow_idempotency_violations(
    parts: &WorkflowParts,
    action_contracts: &[ActionContract],
) -> Result<(), IdempotencyContractErrors> {
    errors_from_violations(parts.nodes.iter().filter_map(|node| {
        do_action(&node.kind)
            .and_then(|action_id| find_contract(action_contracts, action_id))
            .and_then(|contract| is_statically_idempotent_contract(contract).err())
    }))
}

fn errors_from_violations(
    violations: impl Iterator<Item = IdempotencyContractViolation>,
) -> Result<(), IdempotencyContractErrors> {
    let collected: Box<[_]> = violations.collect();
    if collected.is_empty() {
        Ok(())
    } else {
        Err(IdempotencyContractErrors(collected))
    }
}

fn do_action(kind: &CompiledNodeKind) -> Option<ActionId> {
    match kind {
        CompiledNodeKind::Do { action, .. } => Some(*action),
        _ => None,
    }
}

fn has_contract(action_contracts: &[ActionContract], action_id: ActionId) -> bool {
    action_contracts
        .iter()
        .any(|contract| contract.id == action_id)
}

fn find_contract(
    action_contracts: &[ActionContract],
    action_id: ActionId,
) -> Option<&ActionContract> {
    action_contracts
        .iter()
        .find(|contract| contract.id == action_id)
}

fn has_do_action(parts: &WorkflowParts, action_id: ActionId) -> bool {
    parts
        .nodes
        .iter()
        .filter_map(|node| do_action(&node.kind))
        .any(|do_action_id| do_action_id == action_id)
}

#[cfg(test)]
#[allow(clippy::doc_markdown)]
mod tests {
    use super::*;
    use crate::idempotency_contract::IdempotencyContractViolation;

    fn contract(
        id: u16,
        side_effect: SideEffect,
        idempotency: Idempotency,
        retry_safety: RetrySafety,
    ) -> ActionContract {
        ActionContract {
            id: ActionId::new(id),
            name: vb_core::action::ActionName::new(format!("action-{id}")).unwrap(),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 1000,
            idempotency,
            side_effect,
            retry_safety,
            required_capabilities: Box::from([]),
        }
    }

    /// Tier 1: `is_statically_idempotent_contract` returns
    /// `Err(SideEffectingRetryUnsafe)` for `Unknown` retry_safety with
    /// a non-Pure side-effect (the bead's primary contract addition).
    #[test]
    fn is_statically_idempotent_contract_returns_err_for_unknown_retry_safety() {
        let c = contract(
            1,
            SideEffect::LocalWrite,
            Idempotency::IdempotentExternal,
            RetrySafety::Unknown,
        );
        let result = is_statically_idempotent_contract(&c);
        assert!(matches!(
            result,
            Err(IdempotencyContractViolation::SideEffectingRetryUnsafe {
                retry_safety: RetrySafety::Unknown,
                ..
            })
        ));
    }

    /// Tier 1: `is_static_returns_unit_for_pure_contract_for_all_retry_and_idempotency_values`
    /// exercises the 4 RetrySafety × 3 Idempotency = 12 cells with SideEffect::Pure
    /// (all 12 must pass because Pure always passes per the static decision table).
    #[test]
    fn is_static_returns_unit_for_pure_contract_for_all_retry_and_idempotency_values() {
        let mut count = 0usize;
        for retry_safety in [
            RetrySafety::Idempotent,
            RetrySafety::RequiresIdempotencyKey,
            RetrySafety::NotRetrySafe,
            RetrySafety::Unknown,
        ] {
            for idempotency in [
                Idempotency::DeterministicPure,
                Idempotency::IdempotentExternal,
                Idempotency::AtLeastOnceExternal,
            ] {
                let c = contract(
                    100 + count as u16,
                    SideEffect::Pure,
                    idempotency,
                    retry_safety,
                );
                let result = is_statically_idempotent_contract(&c);
                assert_eq!(
                    result,
                    Ok(()),
                    "Pure must pass for {retry_safety:?}+{idempotency:?}"
                );
                count += 1;
            }
        }
        assert_eq!(count, 12, "4 RetrySafety × 3 Idempotency = 12 cells");
    }

    /// Tier 1: `side_effecting_retry_unknown` — side-effecting action with
    /// `Unknown` retry_safety must be rejected as `SideEffectingRetryUnsafe`.
    #[test]
    fn side_effecting_retry_unknown() {
        let c = contract(
            2,
            SideEffect::ExternalWrite,
            Idempotency::IdempotentExternal,
            RetrySafety::Unknown,
        );
        let result = is_statically_idempotent_contract(&c);
        assert!(
            matches!(
                result,
                Err(IdempotencyContractViolation::SideEffectingRetryUnsafe { .. })
            ),
            "side-effecting + Unknown retry must produce SideEffectingRetryUnsafe"
        );
    }
}
