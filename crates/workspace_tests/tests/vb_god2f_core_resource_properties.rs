#![forbid(unsafe_code)]

//! HVR-PO-CORE-006: generated behavior pressure for StepBudget and ResourceContract.

use proptest::prelude::*;
use proptest::strategy::Strategy;
use vb_core::limits::{
    MAX_ACCESSORS, MAX_CONSTANTS, MAX_EXPRESSION_STACK, MAX_EXPRESSIONS, MAX_SLOTS_PER_WORKFLOW,
    MAX_STEP_BUDGET, MAX_STEPS_PER_WORKFLOW,
};
use vb_core::{
    CompiledNode, CompiledNodeKind, ResourceContract, SlotIdx, StepBudget, StepIdx, WorkflowDigest,
    WorkflowParts, validate_resource_contract,
};

fn contract_strategy() -> impl Strategy<Value = ResourceContract> {
    (
        any::<u16>(),
        any::<u16>(),
        any::<u16>(),
        any::<u16>(),
        any::<u16>(),
        any::<u8>(),
        any::<u64>(),
    )
        .prop_map(
            |(
                max_steps,
                max_slots,
                max_constants,
                max_accessors,
                max_expressions,
                max_expr_stack,
                max_transitions_per_tick,
            )| ResourceContract {
                max_steps,
                max_slots,
                max_constants,
                max_accessors,
                max_expressions,
                max_expr_stack,
                max_transitions_per_tick,
                ..ResourceContract::DEFAULT
            },
        )
}

fn generated_parts(contract: ResourceContract, node_count: u16, slot_count: u16) -> WorkflowParts {
    let mut nodes = Vec::new();
    let mut index: u16 = 0;
    while index < node_count {
        nodes.push(CompiledNode {
            id: StepIdx::new(index),
            output: Some(SlotIdx::ZERO),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        });
        index = match index.checked_add(1) {
            Some(value) => value,
            None => node_count,
        };
    }
    WorkflowParts {
        name: Box::from("hvr_po_core_resource"),
        digest: WorkflowDigest::from_bytes([0x44; 32]),
        nodes: nodes.into_boxed_slice(),
        expressions: Box::from([]),
        accessors: Box::from([]),
        constants: Box::from([]),
        slot_count,
        symbols_count: 0,
        entry: StepIdx::ZERO,
        resource_contract: contract,
        step_names: Box::from([]),
    }
}

fn expected_resource_contract_accepts(parts: &WorkflowParts) -> bool {
    let contract = parts.resource_contract;
    parts.nodes.len() <= usize::from(contract.max_steps)
        && usize::from(contract.max_steps) <= MAX_STEPS_PER_WORKFLOW
        && usize::from(parts.slot_count) <= usize::from(contract.max_slots)
        && usize::from(contract.max_slots) <= MAX_SLOTS_PER_WORKFLOW
        && parts.constants.len() <= usize::from(contract.max_constants)
        && usize::from(contract.max_constants) <= MAX_CONSTANTS
        && parts.accessors.len() <= usize::from(contract.max_accessors)
        && usize::from(contract.max_accessors) <= MAX_ACCESSORS
        && parts.expressions.len() <= usize::from(contract.max_expressions)
        && usize::from(contract.max_expressions) <= MAX_EXPRESSIONS
        && contract.max_expr_stack <= MAX_EXPRESSION_STACK
        && contract.max_transitions_per_tick != 0
        && contract.max_transitions_per_tick <= MAX_STEP_BUDGET
}

proptest! {
    #[test]
    fn vb_god2f_core_resource_properties(
        raw_budget in any::<u64>(),
        contract in contract_strategy(),
        node_count in 0u16..=32,
        slot_count in 0u16..=64,
    ) {
        let mut budget = StepBudget::new(raw_budget);
        let expected_remaining = if raw_budget > MAX_STEP_BUDGET {
            MAX_STEP_BUDGET
        } else {
            raw_budget
        };
        prop_assert_eq!(budget.remaining(), expected_remaining);
        let before = budget.remaining();
        let take = budget.try_take();
        if before == 0 {
            prop_assert!(matches!(take, Ok(false)), "zero StepBudget should be exhausted, got {take:?}");
            prop_assert_eq!(budget.remaining(), 0);
        } else {
            prop_assert!(matches!(take, Ok(true)), "positive StepBudget should be consumed, got {take:?}");
            prop_assert_eq!(budget.remaining(), before.saturating_sub(1));
        }

        let parts = generated_parts(contract, node_count, slot_count);
        prop_assert_eq!(validate_resource_contract(&parts).is_ok(), expected_resource_contract_accepts(&parts));
    }
}

#[test]
fn vb_god2f_core_resource_budget_boundaries_match_contract_text() {
    let above_max = match MAX_STEP_BUDGET.checked_add(1) {
        Some(value) => value,
        None => MAX_STEP_BUDGET,
    };
    let cases = [0, 1, MAX_STEP_BUDGET, above_max, u64::MAX];
    for raw in cases {
        let budget = StepBudget::new(raw);
        let expected = if raw > MAX_STEP_BUDGET {
            MAX_STEP_BUDGET
        } else {
            raw
        };
        assert_eq!(budget.remaining(), expected);
    }
}
