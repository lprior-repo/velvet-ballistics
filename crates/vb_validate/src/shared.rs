//! Shared validation pipeline for compiled workflow IR.
//!
//! Provides a single entry point that runs all cold-path validation gates
//! against a [`WorkflowParts`] value.  Both `vb_compile` and the standalone
//! `vb_validate` crate call this pipeline so that structural validation
//! lives in exactly one place.

use crate::ValidationResult;
use crate::gates;
use vb_core::action::ActionContract;
use vb_core::workflow::WorkflowParts;

#[cfg(test)]
use crate::ValidationError;

// Re-export gate functions so external callers can import from this module.
pub use gates::validate_gate_07_expression_stack_depth;
pub use gates::validate_gate_08_accessor_path_segments;
pub use gates::validate_gate_09_slot_references;
pub use gates::validate_gate_10_node_kind_specific;
pub use gates::validate_gate_11_loop_body_graph;
pub use gates::validate_gate_12_action_contract_completeness;
pub use gates::validate_gate_13_no_slot_cycles;
pub use gates::validate_gate_14_slot_type_consistency;
pub use gates::validate_gate_15_determinism_proof;

/// Validation configuration controlling which gates are active.
///
/// The default configuration enables all gates. Callers may selectively
/// disable individual gates when running a partial pipeline (for example
/// during incremental recompilation where only changed gates need to run).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationPipeline {
    /// Run Gate 7: expression stack depth bounded.
    pub gate_07_expression_stack: bool,
    /// Run Gate 8: accessor path segments valid.
    pub gate_08_accessor_paths: bool,
    /// Run Gate 9: slot references within bounds.
    pub gate_09_slot_references: bool,
    /// Run Gate 10: node-kind-specific constraints.
    pub gate_10_node_kind_specific: bool,
    /// Run Gate 11: loop body graph well-formed.
    pub gate_11_loop_body_graph: bool,
    /// Run Gate 12: action contract completeness.
    pub gate_12_action_contracts: bool,
    /// Run Gate 13: no slot dependency cycles.
    pub gate_13_no_slot_cycles: bool,
    /// Run Gate 14: slot type consistency.
    pub gate_14_slot_type_consistency: bool,
    /// Run Gate 15: determinism proof.
    pub gate_15_determinism_proof: bool,
}

impl Default for ValidationPipeline {
    fn default() -> Self {
        Self::all_gates()
    }
}

impl ValidationPipeline {
    /// Creates a pipeline with all validation gates enabled.
    #[must_use]
    pub const fn all_gates() -> Self {
        Self {
            gate_07_expression_stack: true,
            gate_08_accessor_paths: true,
            gate_09_slot_references: true,
            gate_10_node_kind_specific: true,
            gate_11_loop_body_graph: true,
            gate_12_action_contracts: true,
            gate_13_no_slot_cycles: true,
            gate_14_slot_type_consistency: true,
            gate_15_determinism_proof: true,
        }
    }

    /// Creates a pipeline with all validation gates disabled.
    ///
    /// Useful as a starting point for selectively enabling specific gates.
    #[must_use]
    pub const fn no_gates() -> Self {
        Self {
            gate_07_expression_stack: false,
            gate_08_accessor_paths: false,
            gate_09_slot_references: false,
            gate_10_node_kind_specific: false,
            gate_11_loop_body_graph: false,
            gate_12_action_contracts: false,
            gate_13_no_slot_cycles: false,
            gate_14_slot_type_consistency: false,
            gate_15_determinism_proof: false,
        }
    }

    /// Runs all enabled validation gates against the supplied workflow parts.
    ///
    /// Gates execute in ascending order (7, 8, 9, 10, 11, 13, 14, 15).
    /// Gate 12 (action contract completeness) requires external action contract
    /// data and is skipped by this method; use [`validate_with_contracts`]
    /// instead to include gate 12.
    ///
    /// The first failing gate short-circuits the pipeline and returns its error.
    pub fn validate(&self, parts: &WorkflowParts) -> ValidationResult<()> {
        if self.gate_07_expression_stack {
            gates::validate_gate_07_expression_stack_depth(parts)?;
        }
        if self.gate_08_accessor_paths {
            gates::validate_gate_08_accessor_path_segments(parts)?;
        }
        if self.gate_09_slot_references {
            gates::validate_gate_09_slot_references(parts)?;
        }
        if self.gate_10_node_kind_specific {
            gates::validate_gate_10_node_kind_specific(parts)?;
        }
        if self.gate_11_loop_body_graph {
            gates::validate_gate_11_loop_body_graph(parts)?;
        }
        if self.gate_13_no_slot_cycles {
            gates::validate_gate_13_no_slot_cycles(parts)?;
        }
        if self.gate_14_slot_type_consistency {
            gates::validate_gate_14_slot_type_consistency(parts)?;
        }
        if self.gate_15_determinism_proof {
            gates::validate_gate_15_determinism_proof(parts)?;
        }
        Ok(())
    }

    /// Runs all enabled validation gates including gate 12 (action contract
    /// completeness).
    ///
    /// This is the same as [`validate`] but also runs gate 12 against the
    /// provided action contracts. Gate 12 verifies that every Do node's
    /// `action_id` has a matching contract and every contract has a matching
    /// Do node.
    pub fn validate_with_contracts(
        &self,
        parts: &WorkflowParts,
        action_contracts: &[ActionContract],
    ) -> ValidationResult<()> {
        // Run all non-contract gates first.
        self.validate(parts)?;

        // Then run the contract gate if enabled.
        if self.gate_12_action_contracts {
            gates::validate_gate_12_action_contract_completeness(parts, action_contracts)?;
        }
        Ok(())
    }
}

/// Convenience function: runs all validation gates with default configuration.
///
/// This is the primary entry point for callers that want the full cold-path
/// validation pipeline without customising gate selection.
pub fn validate(parts: &WorkflowParts) -> ValidationResult<()> {
    ValidationPipeline::default().validate(parts)
}

/// Convenience function: runs all validation gates including gate 12 (action
/// contract completeness) with default configuration.
///
/// This is the primary entry point for callers that want the full cold-path
/// validation pipeline with action contract verification.
pub fn validate_with_contracts(
    parts: &WorkflowParts,
    action_contracts: &[ActionContract],
) -> ValidationResult<()> {
    ValidationPipeline::default().validate_with_contracts(parts, action_contracts)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use vb_core::ids::{SlotIdx, StepIdx};
    use vb_core::workflow::{CompiledNode, CompiledNodeKind, ResourceContract};

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

    #[test]
    fn pipeline_default_is_all_gates() {
        let pipeline = ValidationPipeline::default();
        assert!(pipeline.gate_07_expression_stack);
        assert!(pipeline.gate_08_accessor_paths);
        assert!(pipeline.gate_09_slot_references);
        assert!(pipeline.gate_10_node_kind_specific);
        assert!(pipeline.gate_11_loop_body_graph);
        assert!(pipeline.gate_12_action_contracts);
        assert!(pipeline.gate_13_no_slot_cycles);
        assert!(pipeline.gate_14_slot_type_consistency);
        assert!(pipeline.gate_15_determinism_proof);
    }

    #[test]
    fn pipeline_no_gates_disables_all() {
        let pipeline = ValidationPipeline::no_gates();
        assert!(!pipeline.gate_07_expression_stack);
        assert!(!pipeline.gate_08_accessor_paths);
        assert!(!pipeline.gate_09_slot_references);
        assert!(!pipeline.gate_10_node_kind_specific);
        assert!(!pipeline.gate_11_loop_body_graph);
        assert!(!pipeline.gate_12_action_contracts);
        assert!(!pipeline.gate_13_no_slot_cycles);
        assert!(!pipeline.gate_14_slot_type_consistency);
        assert!(!pipeline.gate_15_determinism_proof);
    }

    #[test]
    fn validate_convenience_passes_valid_parts() {
        let parts = make_parts(vec![finish_node(0, 0)], 1);
        assert_eq!(validate(&parts), Ok(()));
    }

    #[test]
    fn validate_convenience_catches_bad_slot_reference() {
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(99)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let parts = make_parts(vec![node], 1);
        assert!(matches!(
            validate(&parts),
            Err(ValidationError::SlotReferenceOutOfRange { .. })
        ));
    }

    #[test]
    fn selective_gates_skip_disabled() {
        // Gate 9 catches out-of-range slot refs; disable it and the same
        // parts should pass.
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(99)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let parts = make_parts(vec![node], 1);
        let pipeline = ValidationPipeline {
            gate_09_slot_references: false,
            ..ValidationPipeline::no_gates()
        };
        assert_eq!(pipeline.validate(&parts), Ok(()));
    }

    #[test]
    fn pipeline_short_circuits_on_first_error() {
        // Construct parts that fail gate 9 (slot out of range). Gate 7 would
        // also fail if the stack depth is wrong, but we set up a case where
        // only gate 9 fails.
        let node = CompiledNode {
            id: StepIdx::new(0),
            output: Some(SlotIdx::new(99)),
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        };
        let parts = make_parts(vec![node], 1);
        let result = ValidationPipeline::default().validate(&parts);
        assert!(matches!(
            result,
            Err(ValidationError::SlotReferenceOutOfRange { .. })
        ));
    }
}
