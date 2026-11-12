use super::{validate_accessor_paths, validate_gate_08_accessor_path_segments};
use proptest::prelude::*;
use vb_core::workflow::WorkflowParts;
#[path = "gate_08_projection_tests/strategies.rs"]
mod strategies;
use strategies::{accessors, unrelated_parts};

proptest! {
    #![proptest_config(ProptestConfig {
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    #[test]
    fn gate_08_public_projection_parity_and_noninterference(
        slot_count in any::<u16>(), symbols_count in any::<u32>(), accessors in accessors(),
        left in unrelated_parts(), right in unrelated_parts(),
    ) {
        let mut left = left.map_err(|e| TestCaseError::fail(e.to_string()))?;
        let right = right.map_err(|e| TestCaseError::fail(e.to_string()))?;
        left.accessors = accessors.into_boxed_slice();
        left.slot_count = slot_count;
        left.symbols_count = symbols_count;
        let expected = validate_accessor_paths(&left.accessors, slot_count, symbols_count);
        prop_assert_eq!(validate_gate_08_accessor_path_segments(&left), expected.clone());
        let variants = [
            WorkflowParts { name: right.name, ..left.clone() },
            WorkflowParts { digest: right.digest, ..left.clone() },
            WorkflowParts { semantic_source_identity: right.semantic_source_identity, ..left.clone() },
            WorkflowParts { source_metadata: right.source_metadata, ..left.clone() },
            WorkflowParts { nodes: right.nodes, ..left.clone() },
            WorkflowParts { expressions: right.expressions, ..left.clone() },
            WorkflowParts { constants: right.constants, ..left.clone() },
            WorkflowParts { entry: right.entry, ..left.clone() },
            WorkflowParts { resource_contract: right.resource_contract, ..left.clone() },
            WorkflowParts { step_names: right.step_names, ..left.clone() },
            WorkflowParts { input_slots: right.input_slots, ..left.clone() },
        ];
        for variant in variants {
            prop_assert_eq!(validate_gate_08_accessor_path_segments(&variant), expected.clone());
        }
    }
}
