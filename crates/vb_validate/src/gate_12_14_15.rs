//! Gates 12, 14, 15: Action contract, slot type consistency, determinism proof.

#![allow(unreachable_pub)]
#![allow(clippy::collapsible_if)]

use crate::{ValidationError, ValidationResult};
use vb_core::action::ActionContract;
use vb_core::workflow::{CompiledNodeKind, WorkflowParts};

pub fn validate_gate_12_action_contract_completeness(
    parts: &WorkflowParts,
    action_contracts: &[ActionContract],
) -> ValidationResult<()> {
    let mut do_action_ids: Vec<u16> = Vec::new();
    for (node_index, node) in parts.nodes.iter().enumerate() {
        if let CompiledNodeKind::Do { action, .. } = &node.kind {
            let action_val = action.get();
            let mut found = false;
            for contract in action_contracts {
                if contract.id.get() == action_val {
                    found = true;
                    break;
                }
            }
            if !found {
                return Err(ValidationError::ActionContractMissing {
                    action_id: usize::from(action_val),
                    node_index,
                });
            }
            if !do_action_ids.contains(&action_val) {
                do_action_ids.push(action_val);
            }
        }
    }
    for contract in action_contracts {
        let cid = contract.id.get();
        let mut found = false;
        for do_id in &do_action_ids {
            if *do_id == cid {
                found = true;
                break;
            }
        }
        if !found {
            return Err(ValidationError::ActionContractOrphan {
                action_id: usize::from(cid),
            });
        }
    }
    Ok(())
}

pub fn validate_gate_14_slot_type_consistency(parts: &WorkflowParts) -> ValidationResult<()> {
    let slot_count = usize::from(parts.slot_count);
    if slot_count == 0 {
        return Ok(());
    }
    let mut slot_const_kind: Vec<u8> = vec![0; slot_count];
    for node in parts.nodes.iter() {
        if let CompiledNodeKind::SetConst { value } = &node.kind {
            let cidx = value.as_usize();
            if cidx >= parts.constants.len() {
                continue;
            }
            if let Some(constant) = parts.constants.get(cidx) {
                let kind = const_value_discriminant(constant);
                if let Some(slot) = node.output {
                    let su = slot.as_usize();
                    if su < slot_count {
                        let existing = slot_const_kind
                            .get(su)
                            .copied()
                            .ok_or(ValidationError::SlotTypeInconsistency { slot: su })?;
                        if existing == 0 {
                            if let Some(e) = slot_const_kind.get_mut(su) {
                                *e = kind;
                            }
                        } else if existing != kind {
                            return Err(ValidationError::SlotTypeInconsistency { slot: su });
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn const_value_discriminant(value: &vb_core::value::ConstValue) -> u8 {
    match value {
        vb_core::value::ConstValue::Null => 1,
        vb_core::value::ConstValue::Bool(_) => 2,
        vb_core::value::ConstValue::I64(_) => 3,
        vb_core::value::ConstValue::F64(_) => 4,
        vb_core::value::ConstValue::Symbol(_) => 5,
    }
}

pub fn validate_gate_15_determinism_proof(parts: &WorkflowParts) -> ValidationResult<()> {
    let node_count = parts.nodes.len();
    for (node_index, node) in parts.nodes.iter().enumerate() {
        if !is_non_deterministic(&node.kind) {
            continue;
        }
        if let Some(next_step) = node.next {
            let nu = next_step.as_usize();
            if nu < node_count {
                if let Some(next_node) = parts.nodes.get(nu) {
                    if is_non_deterministic(&next_node.kind) {
                        return Err(ValidationError::NonDeterministicPath {
                            from_node: node_index,
                            to_node: nu,
                        });
                    }
                }
            }
        }
    }
    Ok(())
}

fn is_non_deterministic(kind: &CompiledNodeKind) -> bool {
    matches!(
        kind,
        CompiledNodeKind::Do { .. } | CompiledNodeKind::Ask { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::action::{ActionContract, Idempotency, RetrySafety, SideEffect};
    use vb_core::ids::{ActionId, ConstIdx, SlotIdx, StepIdx};
    use vb_core::value::ConstValue;
    use vb_core::workflow::{CompiledNode, ResourceContract};

    fn make_parts(nodes: Vec<CompiledNode>, slot_count: u16) -> WorkflowParts {
        WorkflowParts {
            name: Box::from("test"),
            digest: vb_core::ids::WorkflowDigest::from_bytes([0u8; 32]),
            nodes: nodes.into_boxed_slice(),
            expressions: Box::new([]),
            accessors: Box::new([]),
            constants: Box::new([]),
            slot_count,
            symbols_count: 0,
            entry: StepIdx::new(0),
            resource_contract: ResourceContract::DEFAULT,
            step_names: Box::new([]),
        }
    }

    fn finish_node(index: u16, result_slot: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(index),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Finish {
                result: SlotIdx::new(result_slot),
            },
        }
    }

    fn nop_node(index: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(index),
            output: None,
            next: Some(StepIdx::new(index.saturating_add(1))),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }
    }

    fn do_node(index: u16, action: u16, input: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(index),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(index.saturating_add(1))),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(action),
                input: SlotIdx::new(input),
            },
        }
    }

    fn make_contract(action_id: u16) -> ActionContract {
        ActionContract {
            id: ActionId::new(action_id),
            input_slot_count: 1,
            output_slot_count: 1,
            max_input_bytes: 1024,
            max_output_bytes: 1024,
            timeout_ms: 5000,
            idempotency: Idempotency::DeterministicPure,
            side_effect: SideEffect::None,
            retry_safety: RetrySafety::Safe,
            required_capabilities: Box::new([]),
        }
    }

    fn set_const_node(index: u16, const_idx: u16, output_slot: u16) -> CompiledNode {
        CompiledNode {
            id: StepIdx::new(index),
            output: Some(SlotIdx::new(output_slot)),
            next: Some(StepIdx::new(index.saturating_add(1))),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::SetConst {
                value: ConstIdx::new(const_idx),
            },
        }
    }

    // ===== Gate 12 tests =====

    #[test]
    fn gate_12_accepts_no_do_nodes_no_contracts() {
        let parts = make_parts(vec![finish_node(0, 0)], 1);
        assert_eq!(
            validate_gate_12_action_contract_completeness(&parts, &[]),
            Ok(())
        );
    }

    #[test]
    fn gate_12_accepts_do_node_with_matching_contract() {
        let parts = make_parts(vec![do_node(0, 1, 0), finish_node(1, 0)], 1);
        let contracts = [make_contract(1)];
        assert_eq!(
            validate_gate_12_action_contract_completeness(&parts, &contracts),
            Ok(())
        );
    }

    #[test]
    fn gate_12_accepts_multiple_do_nodes_with_contracts() {
        let parts = make_parts(
            vec![do_node(0, 1, 0), do_node(1, 2, 0), finish_node(2, 0)],
            1,
        );
        let contracts = [make_contract(1), make_contract(2)];
        assert_eq!(
            validate_gate_12_action_contract_completeness(&parts, &contracts),
            Ok(())
        );
    }

    #[test]
    fn gate_12_rejects_missing_contract() {
        let parts = make_parts(vec![do_node(0, 5, 0), finish_node(1, 0)], 1);
        assert!(matches!(
            validate_gate_12_action_contract_completeness(&parts, &[]),
            Err(ValidationError::ActionContractMissing { .. })
        ));
    }

    #[test]
    fn gate_12_rejects_orphan_contract() {
        let parts = make_parts(vec![finish_node(0, 0)], 1);
        let contracts = [make_contract(99)];
        assert!(matches!(
            validate_gate_12_action_contract_completeness(&parts, &contracts),
            Err(ValidationError::ActionContractOrphan { .. })
        ));
    }

    #[test]
    fn gate_12_rejects_partial_mismatch() {
        let parts = make_parts(vec![do_node(0, 1, 0), finish_node(1, 0)], 1);
        let contracts = [make_contract(1), make_contract(2)];
        // contract 2 has no matching Do node -> orphan
        assert!(matches!(
            validate_gate_12_action_contract_completeness(&parts, &contracts),
            Err(ValidationError::ActionContractOrphan { .. })
        ));
    }

    #[test]
    fn gate_12_accepts_two_do_nodes_one_contract() {
        // Two Do nodes using the same action_id, one contract covers both.
        let parts = make_parts(
            vec![do_node(0, 1, 0), do_node(1, 1, 0), finish_node(2, 0)],
            1,
        );
        let contracts = [make_contract(1)];
        assert_eq!(
            validate_gate_12_action_contract_completeness(&parts, &contracts),
            Ok(())
        );
    }

    // ===== Gate 14 tests =====

    #[test]
    fn gate_14_accepts_empty_slots() {
        let parts = make_parts(vec![nop_node(0)], 0);
        assert_eq!(validate_gate_14_slot_type_consistency(&parts), Ok(()));
    }

    #[test]
    fn gate_14_accepts_consistent_types() {
        let mut parts = make_parts(
            vec![
                set_const_node(0, 0, 0),
                set_const_node(1, 1, 0),
                finish_node(2, 0),
            ],
            1,
        );
        parts.constants = Box::new([ConstValue::I64(1), ConstValue::I64(2)]);
        assert_eq!(validate_gate_14_slot_type_consistency(&parts), Ok(()));
    }

    #[test]
    fn gate_14_rejects_inconsistent_types() {
        let mut parts = make_parts(
            vec![
                set_const_node(0, 0, 0),
                set_const_node(1, 1, 0),
                finish_node(2, 0),
            ],
            1,
        );
        parts.constants = Box::new([ConstValue::I64(1), ConstValue::Bool(true)]);
        assert!(matches!(
            validate_gate_14_slot_type_consistency(&parts),
            Err(ValidationError::SlotTypeInconsistency { .. })
        ));
    }

    #[test]
    fn gate_14_accepts_single_set_const() {
        let mut parts = make_parts(vec![set_const_node(0, 0, 0)], 1);
        parts.constants = Box::new([ConstValue::I64(42)]);
        assert_eq!(validate_gate_14_slot_type_consistency(&parts), Ok(()));
    }

    #[test]
    fn gate_14_accepts_different_slots_with_different_types() {
        let mut parts = make_parts(
            vec![
                set_const_node(0, 0, 0),
                set_const_node(1, 1, 1),
                finish_node(2, 0),
            ],
            2,
        );
        parts.constants = Box::new([ConstValue::I64(1), ConstValue::Bool(false)]);
        assert_eq!(validate_gate_14_slot_type_consistency(&parts), Ok(()));
    }

    // ===== Gate 15 tests =====

    #[test]
    fn gate_15_accepts_deterministic_workflow() {
        let parts = make_parts(vec![finish_node(0, 0)], 1);
        assert_eq!(validate_gate_15_determinism_proof(&parts), Ok(()));
    }

    #[test]
    fn gate_15_accepts_do_followed_by_deterministic() {
        let parts = make_parts(vec![do_node(0, 1, 0), finish_node(1, 0)], 1);
        assert_eq!(validate_gate_15_determinism_proof(&parts), Ok(()));
    }

    #[test]
    fn gate_15_rejects_do_followed_by_do() {
        let parts = make_parts(
            vec![do_node(0, 1, 0), do_node(1, 2, 0), finish_node(2, 0)],
            1,
        );
        assert!(matches!(
            validate_gate_15_determinism_proof(&parts),
            Err(ValidationError::NonDeterministicPath { .. })
        ));
    }

    #[test]
    fn gate_15_rejects_ask_followed_by_do() {
        let ask_node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: Some(StepIdx::new(1)),
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Ask {
                prompt: SlotIdx::new(0),
                timeout_slot: None,
            },
        };
        let parts = make_parts(vec![ask_node, do_node(1, 1, 0), finish_node(2, 0)], 1);
        assert!(matches!(
            validate_gate_15_determinism_proof(&parts),
            Err(ValidationError::NonDeterministicPath { .. })
        ));
    }

    #[test]
    fn gate_15_accepts_do_with_no_next() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(0)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Do {
                action: ActionId::new(1),
                input: SlotIdx::new(0),
            },
        };
        let parts = make_parts(vec![node], 1);
        assert_eq!(validate_gate_15_determinism_proof(&parts), Ok(()));
    }

    #[test]
    fn gate_15_accepts_nop_workflow() {
        let parts = make_parts(vec![nop_node(0)], 0);
        assert_eq!(validate_gate_15_determinism_proof(&parts), Ok(()));
    }

    #[test]
    fn gate_15_accepts_do_followed_by_nop_then_do() {
        // Do -> Nop -> Do is OK because the Nop separates them
        let parts = make_parts(
            vec![
                do_node(0, 1, 0),
                nop_node(1),
                do_node(2, 2, 0),
                finish_node(3, 0),
            ],
            1,
        );
        assert_eq!(validate_gate_15_determinism_proof(&parts), Ok(()));
    }
}
