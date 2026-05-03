//! Resource contract and count validation.

use crate::ids::StepIdx;
use crate::limits::{
    MAX_ACCESSORS, MAX_CONSTANTS, MAX_EXPRESSIONS, MAX_EXPRESSION_STACK, MAX_SLOTS_PER_WORKFLOW,
    MAX_STEPS_PER_WORKFLOW,
};

use super::super::expr::ExprProgram;
use super::super::types::{ResourceContract, WorkflowError, WorkflowParts};

/// Validates the resource contract against hard limits and actual usage.
pub(crate) fn validate_resource_contract(parts: &WorkflowParts) -> Result<(), WorkflowError> {
    let contract = parts.resource_contract;
    validate_resource_counts(parts, contract)?;
    validate_expr_stack_contract(parts.expressions.as_ref(), contract.max_expr_stack)
}

fn validate_resource_counts(
    parts: &WorkflowParts,
    contract: ResourceContract,
) -> Result<(), WorkflowError> {
    validate_primary_resource_counts(parts, contract)?;
    validate_expression_resource_counts(parts, contract)
}

fn validate_primary_resource_counts(
    parts: &WorkflowParts,
    contract: ResourceContract,
) -> Result<(), WorkflowError> {
    validate_contract_limit(
        "max_steps",
        parts.nodes.len(),
        usize::from(contract.max_steps),
        MAX_STEPS_PER_WORKFLOW,
    )?;
    validate_contract_limit(
        "max_slots",
        usize::from(parts.slot_count),
        usize::from(contract.max_slots),
        MAX_SLOTS_PER_WORKFLOW,
    )?;
    validate_contract_limit(
        "max_constants",
        parts.constants.len(),
        usize::from(contract.max_constants),
        MAX_CONSTANTS,
    )
}

fn validate_expression_resource_counts(
    parts: &WorkflowParts,
    contract: ResourceContract,
) -> Result<(), WorkflowError> {
    validate_contract_limit(
        "max_accessors",
        parts.accessors.len(),
        usize::from(contract.max_accessors),
        MAX_ACCESSORS,
    )?;
    validate_contract_limit(
        "max_expressions",
        parts.expressions.len(),
        usize::from(contract.max_expressions),
        MAX_EXPRESSIONS,
    )
}

fn validate_contract_limit(
    resource: &'static str,
    actual: usize,
    declared: usize,
    hard_limit: usize,
) -> Result<(), WorkflowError> {
    if declared > hard_limit {
        return Err(WorkflowError::ResourceContractTooLarge { resource });
    }
    if actual > declared {
        Err(WorkflowError::ResourceContractExceeded { resource })
    } else {
        Ok(())
    }
}

fn validate_expr_stack_contract(
    expressions: &[ExprProgram],
    max_expr_stack: u8,
) -> Result<(), WorkflowError> {
    if max_expr_stack > MAX_EXPRESSION_STACK {
        return Err(WorkflowError::ResourceContractTooLarge {
            resource: "max_expr_stack",
        });
    }
    if expressions
        .iter()
        .any(|expression| expression.max_stack > max_expr_stack)
    {
        Err(WorkflowError::ResourceContractExceeded {
            resource: "max_expr_stack",
        })
    } else {
        Ok(())
    }
}

pub(crate) fn validate_entry(entry: StepIdx, node_count: usize) -> Result<(), WorkflowError> {
    if entry.as_usize() < node_count {
        Ok(())
    } else {
        Err(WorkflowError::EntryOutOfBounds { entry })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{SlotIdx, StepIdx, ConstIdx, ExprIdx};
    use crate::workflow::{CompiledNode, CompiledNodeKind, ResourceContract, WorkflowDigest, WorkflowParts};
    use crate::value::ConstValue;

    fn default_contract() -> ResourceContract {
        ResourceContract {
            max_steps: 10,
            max_slots: 10,
            max_constants: 10,
            max_accessors: 10,
            max_expressions: 10,
            max_expr_stack: 64,
            max_step_budget_per_tick: 10_000,
            max_input_bytes: 1_048_576,
            max_output_bytes: 1_048_576,
            max_blob_bytes: 16_777_216,
            max_ipc_payload_bytes: 1_048_576,
            max_retry_attempts: 3,
            max_fanout: 64,
            max_collect_items: 1_024,
            max_queue_depth: 1_024,
            max_journal_batch_bytes: 1_048_576,
        }
    }

    fn make_parts(
        node_count: usize,
        slot_count: u16,
        const_count: usize,
        accessor_count: usize,
        expr_count: usize,
        contract: ResourceContract,
    ) -> WorkflowParts {
        let nodes: Vec<CompiledNode> = (0..node_count)
            .map(|i| CompiledNode {
                id: StepIdx::new(i as u16),
                output: None,
                next: None,
                on_error: None,
                error_slot: None,
                kind: CompiledNodeKind::Finish { result: SlotIdx::new(0) },
            })
            .collect();
        let constants: Vec<ConstValue> = (0..const_count).map(|_| ConstValue::Null).collect();
        let expressions: Vec<crate::workflow::ExprProgram> = (0..expr_count)
            .map(|_| crate::workflow::ExprProgram { ops: Box::new([]), max_stack: 0 })
            .collect();
        let accessors: Vec<crate::workflow::AccessorProgram> = (0..accessor_count)
            .map(|_| crate::workflow::AccessorProgram {
                root: SlotIdx::new(0),
                path: Box::new([]),
            })
            .collect();
        WorkflowParts {
            name: Box::<str>::from("test"),
            digest: WorkflowDigest::from_bytes([0; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: expressions.into_boxed_slice(),
            accessors: accessors.into_boxed_slice(),
            constants: constants.into_boxed_slice(),
            slot_count,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: contract,
            step_names: Box::new([]),
        }
    }

    // -- validate_entry --

    #[test]
    fn validate_entry_accepts_zero_with_nodes() {
        assert_eq!(validate_entry(StepIdx::new(0), 5), Ok(()));
    }

    #[test]
    fn validate_entry_accepts_last_valid_index() {
        assert_eq!(validate_entry(StepIdx::new(4), 5), Ok(()));
    }

    #[test]
    fn validate_entry_rejects_at_node_count() {
        let result = validate_entry(StepIdx::new(5), 5);
        assert!(matches!(result, Err(WorkflowError::EntryOutOfBounds { entry }) if entry == StepIdx::new(5)));
    }

    #[test]
    fn validate_entry_rejects_past_node_count() {
        let result = validate_entry(StepIdx::new(100), 5);
        assert!(matches!(result, Err(WorkflowError::EntryOutOfBounds { entry }) if entry == StepIdx::new(100)));
    }

    #[test]
    fn validate_entry_rejects_zero_with_zero_nodes() {
        let result = validate_entry(StepIdx::new(0), 0);
        assert!(matches!(result, Err(WorkflowError::EntryOutOfBounds { .. })));
    }

    // -- validate_resource_contract --

    #[test]
    fn resource_contract_accepts_exact_match() {
        let contract = default_contract();
        let parts = make_parts(10, 10, 10, 10, 10, contract);
        assert_eq!(validate_resource_contract(&parts), Ok(()));
    }

    #[test]
    fn resource_contract_accepts_below_limits() {
        let contract = default_contract();
        let parts = make_parts(3, 3, 3, 3, 3, contract);
        assert_eq!(validate_resource_contract(&parts), Ok(()));
    }

    #[test]
    fn resource_contract_rejects_nodes_exceeding_max_steps() {
        let contract = ResourceContract { max_steps: 2, ..default_contract() };
        let parts = make_parts(5, 1, 0, 0, 0, contract);
        let result = validate_resource_contract(&parts);
        assert!(matches!(result, Err(WorkflowError::ResourceContractExceeded { resource }) if resource == "max_steps"));
    }

    #[test]
    fn resource_contract_rejects_slots_exceeding_max_slots() {
        let contract = ResourceContract { max_slots: 2, ..default_contract() };
        let parts = make_parts(1, 5, 0, 0, 0, contract);
        let result = validate_resource_contract(&parts);
        assert!(matches!(result, Err(WorkflowError::ResourceContractExceeded { resource }) if resource == "max_slots"));
    }

    #[test]
    fn resource_contract_rejects_constants_exceeding_max_constants() {
        let contract = ResourceContract { max_constants: 2, ..default_contract() };
        let parts = make_parts(1, 0, 5, 0, 0, contract);
        let result = validate_resource_contract(&parts);
        assert!(matches!(result, Err(WorkflowError::ResourceContractExceeded { resource }) if resource == "max_constants"));
    }

    #[test]
    fn resource_contract_rejects_accessors_exceeding_max_accessors() {
        let contract = ResourceContract { max_accessors: 2, ..default_contract() };
        let parts = make_parts(1, 1, 0, 5, 0, contract);
        let result = validate_resource_contract(&parts);
        assert!(matches!(result, Err(WorkflowError::ResourceContractExceeded { resource }) if resource == "max_accessors"));
    }

    #[test]
    fn resource_contract_rejects_expressions_exceeding_max_expressions() {
        let contract = ResourceContract { max_expressions: 2, ..default_contract() };
        let parts = make_parts(1, 0, 0, 0, 5, contract);
        let result = validate_resource_contract(&parts);
        assert!(matches!(result, Err(WorkflowError::ResourceContractExceeded { resource }) if resource == "max_expressions"));
    }

    // -- Hard limit checks --

    #[test]
    fn resource_contract_rejects_max_steps_over_hard_limit() {
        let contract = ResourceContract {
            max_steps: u16::try_from(MAX_STEPS_PER_WORKFLOW).map_or(u16::MAX, |v| v.saturating_add(1)),
            ..default_contract()
        };
        // max_steps > MAX_STEPS_PER_WORKFLOW triggers too-large
        let parts = make_parts(1, 0, 0, 0, 0, contract);
        let result = validate_resource_contract(&parts);
        assert!(matches!(result, Err(WorkflowError::ResourceContractTooLarge { resource }) if resource == "max_steps"));
    }

    #[test]
    fn resource_contract_rejects_max_slots_over_hard_limit() {
        let contract = ResourceContract {
            max_slots: u16::try_from(MAX_SLOTS_PER_WORKFLOW).map_or(u16::MAX, |v| v.saturating_add(1)),
            ..default_contract()
        };
        let parts = make_parts(1, 0, 0, 0, 0, contract);
        let result = validate_resource_contract(&parts);
        assert!(matches!(result, Err(WorkflowError::ResourceContractTooLarge { resource }) if resource == "max_slots"));
    }

    // -- Expression stack contract --

    #[test]
    fn resource_contract_accepts_expr_stack_within_bounds() {
        let contract = default_contract();
        let mut parts = make_parts(1, 0, 0, 0, 1, contract);
        parts.expressions = vec![crate::workflow::ExprProgram {
            ops: Box::new([]),
            max_stack: 10,
        }].into_boxed_slice();
        // max_expr_stack in contract is 64, max_stack is 10 -> ok
        assert_eq!(validate_resource_contract(&parts), Ok(()));
    }

    #[test]
    fn resource_contract_rejects_expr_stack_over_hard_limit() {
        let contract = ResourceContract {
            max_expr_stack: u8::MAX,
            ..default_contract()
        };
        let parts = make_parts(1, 0, 0, 0, 0, contract);
        let result = validate_resource_contract(&parts);
        assert!(matches!(result, Err(WorkflowError::ResourceContractTooLarge { resource }) if resource == "max_expr_stack"));
    }

    #[test]
    fn resource_contract_rejects_expr_stack_exceeded() {
        let contract = ResourceContract {
            max_expr_stack: 2,
            ..default_contract()
        };
        let mut parts = make_parts(1, 0, 0, 0, 1, contract);
        parts.expressions = vec![crate::workflow::ExprProgram {
            ops: Box::new([]),
            max_stack: 5,
        }].into_boxed_slice();
        let result = validate_resource_contract(&parts);
        assert!(matches!(result, Err(WorkflowError::ResourceContractExceeded { resource }) if resource == "max_expr_stack"));
    }
}
