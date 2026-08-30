#![forbid(unsafe_code)]
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

/// Sealed gate-status type for validation pipeline configuration.
///
/// Replaces the raw `bool` fields previously stored in `ValidationPipeline`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum GateStatus {
    /// The gate is active and will be executed.
    Enabled,
    /// The gate is skipped.
    Disabled,
}

impl GateStatus {
    /// Returns true when this gate should execute.
    #[must_use]
    pub const fn should_run(self) -> bool {
        matches!(self, Self::Enabled)
    }
}

impl core::ops::Not for GateStatus {
    type Output = Self;

    fn not(self) -> Self {
        match self {
            Self::Enabled => Self::Disabled,
            Self::Disabled => Self::Enabled,
        }
    }
}

impl From<bool> for GateStatus {
    fn from(value: bool) -> Self {
        if value {
            Self::Enabled
        } else {
            Self::Disabled
        }
    }
}

impl From<GateStatus> for bool {
    fn from(status: GateStatus) -> Self {
        status.should_run()
    }
}

/// Validation configuration controlling which gates are active.
///
/// The default configuration enables all gates. Callers may selectively
/// disable individual gates when running a partial pipeline (for example
/// during incremental recompilation where only changed gates need to run).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationPipeline {
    /// Run Gate 7: expression stack depth bounded.
    pub gate_07_expression_stack: GateStatus,
    /// Run Gate 8: accessor path segments valid.
    pub gate_08_accessor_paths: GateStatus,
    /// Run Gate 9: slot references within bounds.
    pub gate_09_slot_references: GateStatus,
    /// Run Gate 10: node-kind-specific constraints.
    pub gate_10_node_kind_specific: GateStatus,
    /// Run Gate 11: loop body graph well-formed.
    pub gate_11_loop_body_graph: GateStatus,
    /// Run Gate 12: action contract completeness.
    pub gate_12_action_contracts: GateStatus,
    /// Run Gate 13: no slot dependency cycles.
    pub gate_13_no_slot_cycles: GateStatus,
    /// Run Gate 14: slot type consistency.
    pub gate_14_slot_type_consistency: GateStatus,
    /// Run Gate 15: determinism proof.
    pub gate_15_determinism_proof: GateStatus,
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
            gate_07_expression_stack: GateStatus::Enabled,
            gate_08_accessor_paths: GateStatus::Enabled,
            gate_09_slot_references: GateStatus::Enabled,
            gate_10_node_kind_specific: GateStatus::Enabled,
            gate_11_loop_body_graph: GateStatus::Enabled,
            gate_12_action_contracts: GateStatus::Enabled,
            gate_13_no_slot_cycles: GateStatus::Enabled,
            gate_14_slot_type_consistency: GateStatus::Enabled,
            gate_15_determinism_proof: GateStatus::Enabled,
        }
    }

    /// Creates a pipeline with all validation gates disabled.
    ///
    /// Useful as a starting point for selectively enabling specific gates.
    #[must_use]
    pub const fn no_gates() -> Self {
        Self {
            gate_07_expression_stack: GateStatus::Disabled,
            gate_08_accessor_paths: GateStatus::Disabled,
            gate_09_slot_references: GateStatus::Disabled,
            gate_10_node_kind_specific: GateStatus::Disabled,
            gate_11_loop_body_graph: GateStatus::Disabled,
            gate_12_action_contracts: GateStatus::Disabled,
            gate_13_no_slot_cycles: GateStatus::Disabled,
            gate_14_slot_type_consistency: GateStatus::Disabled,
            gate_15_determinism_proof: GateStatus::Disabled,
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
        if self.gate_07_expression_stack.should_run() {
            gates::validate_gate_07_expression_stack_depth(parts)?;
        }
        if self.gate_08_accessor_paths.should_run() {
            gates::validate_gate_08_accessor_path_segments(parts)?;
        }
        if self.gate_09_slot_references.should_run() {
            gates::validate_gate_09_slot_references(parts)?;
        }
        if self.gate_10_node_kind_specific.should_run() {
            gates::validate_gate_10_node_kind_specific(parts)?;
        }
        if self.gate_11_loop_body_graph.should_run() {
            gates::validate_gate_11_loop_body_graph(parts)?;
        }
        if self.gate_13_no_slot_cycles.should_run() {
            gates::validate_gate_13_no_slot_cycles(parts)?;
        }
        if self.gate_14_slot_type_consistency.should_run() {
            gates::validate_gate_14_slot_type_consistency(parts)?;
        }
        if self.gate_15_determinism_proof.should_run() {
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
        if self.gate_12_action_contracts.should_run() {
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
#[path = "shared/tests.rs"]
mod tests;
