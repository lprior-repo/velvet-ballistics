#![forbid(unsafe_code)]

use super::{GateKind, ParseGateKindError};

const VALID_GATE_KINDS: [(GateKind, &str); 9] = [
    (
        GateKind::Gate07ExpressionStackDepth,
        "gate_07_expression_stack_depth",
    ),
    (
        GateKind::Gate08AccessorPathSegments,
        "gate_08_accessor_path_segments",
    ),
    (GateKind::Gate09SlotReferences, "gate_09_slot_references"),
    (
        GateKind::Gate10NodeKindSpecific,
        "gate_10_node_kind_specific",
    ),
    (GateKind::Gate11LoopBodyGraph, "gate_11_loop_body_graph"),
    (
        GateKind::Gate12ActionContractCompleteness,
        "gate_12_action_contract_completeness",
    ),
    (GateKind::Gate13NoSlotCycles, "gate_13_no_slot_cycles"),
    (
        GateKind::Gate14SlotTypeConsistency,
        "gate_14_slot_type_consistency",
    ),
    (
        GateKind::Gate15DeterminismProof,
        "gate_15_determinism_proof",
    ),
];

#[test]
fn gate_kind_try_from_accepts_existing_wire_names() {
    VALID_GATE_KINDS
        .iter()
        .copied()
        .for_each(|(expected, wire_name)| {
            assert_eq!(GateKind::try_from(wire_name), Ok(expected));
        });
}

#[test]
fn gate_kind_as_str_preserves_existing_wire_names() {
    VALID_GATE_KINDS
        .iter()
        .copied()
        .for_each(|(kind, wire_name)| {
            assert_eq!(kind.as_str(), wire_name);
        });
}

#[test]
fn gate_kind_try_from_rejects_unknown_wire_names() {
    assert_eq!(
        GateKind::try_from("gate_99_future_gate"),
        Err(ParseGateKindError)
    );
}
