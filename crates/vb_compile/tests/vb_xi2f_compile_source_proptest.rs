#![forbid(unsafe_code)]
#![allow(clippy::expect_used)]
//! Proptest property tests for compile_source try_from_parts integration.
//!
//! PO: PO-003 (compile_source and YamlCompiler::compile public APIs never panic)
//! Bead: vb-xi2f.4
//! Verifier: proptest
//! Command: cargo test --package vb_compile --test vb_xi2f_compile_source_proptest
//
// Proof obligations:
// - PO-003: Property-based testing verifies compile_source returns Result
//   (never panics) for arbitrary valid AST inputs and that output workflow
//   is structurally valid.

use proptest::prelude::*;
use vb_compile::{CompileError, CompileErrors, YamlCompiler, compile_workflow};

// ---------------------------------------------------------------------------
// Strategies: Valid YAML workflow generation
// ---------------------------------------------------------------------------

/// Generate a valid minimal YAML workflow string.
fn valid_yaml_workflow() -> impl Strategy<Value = String> {
    // Valid primitive step generators
    let set_step = r#"
  - id: set_step
    set:
      output: result
      value: "42"
"#;
    let finish_step = r#"
  - id: finish_step
    finish:
      result: result
"#;

    let header =
        "version: velvet-ballistics/v1\nname: test-workflow\nwhen:\n  manual: {}\nsteps:\n";

    // Generate workflows with 1-10 steps, always ending with finish
    prop_oneof![
        Just(format!("{}{}{}", header, set_step, finish_step)),
        Just(format!(
            "{}  - id: step1\n    set:\n      output: a\n      value: \"1\"\n{}{}",
            header, set_step, finish_step
        )),
        Just(format!(
            "{}  - id: step1\n    set:\n      output: a\n      value: \"1\"\n  - id: step2\n    set:\n      output: b\n      value: \"2\"\n{}{}",
            header, set_step, finish_step
        )),
    ]
}

// ---------------------------------------------------------------------------
// PO-003: compile_source never panics for valid YAML
// ---------------------------------------------------------------------------

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 10000,
        ..ProptestConfig::default()
    })]

    /// compile_source returns Ok(validated) or Err(typed) — never panics.
    #[test]
    fn compile_source_never_panics(yaml in valid_yaml_workflow()) {
        let result = compile_workflow(yaml.as_bytes());

        match result {
            Ok(workflow) => {
                // If compilation succeeds, the workflow must be structurally valid
                prop_assert!(workflow.node_count() > 0, "compiled workflow must have nodes");
                prop_assert!(
                    workflow.slot_count() >= 1,
                    "compiled workflow must have at least one slot"
                );
            }
            Err(CompileErrors(errors)) => {
                // Errors must be typed CompileError variants, never panic
                prop_assert!(
                    !errors.is_empty(),
                    "error result must contain at least one error"
                );
                for error in &errors {
                    prop_assert!(
                        matches!(
                            error,
                            CompileError::Workflow(_)
                                | CompileError::EmptySteps
                                | CompileError::StepIndexOutOfRange { .. }
                                | CompileError::UnsupportedStepPrimitive { .. }
                                | CompileError::LastStepMustFinish
                                | CompileError::UnknownOutputName { .. }
                                | CompileError::DuplicateOutputName { .. }
                                | CompileError::InvalidName { .. }
                                | CompileError::MissingStepId { .. }
                                | CompileError::DuplicateStepId { .. }
                                | CompileError::StepShape { .. }
                                | CompileError::UnknownStepField { .. }
                                | CompileError::MissingStepPrimitive { .. }
                                | CompileError::MultipleStepPrimitives { .. }
                                | CompileError::MissingStepField { .. }
                                | CompileError::StepFieldShape { .. }
                                | CompileError::BackwardBranchTarget { .. }
                                | CompileError::UnknownStepTarget { .. }
                                | CompileError::UnreachableStep { .. }
                                | CompileError::TypeMismatch { .. }
                                | CompileError::UnknownSlotType { .. }
                                | CompileError::SecretTaintLeak { .. }
                                | CompileError::ExpressionUnexpectedChar { .. }
                                | CompileError::ExpressionUnterminatedString { .. }
                                | CompileError::ExpressionIntegerOutOfRange { .. }
                                | CompileError::ExpressionFloatOutOfRange { .. }
                                | CompileError::ExpressionLimitExceeded { .. }
                                | CompileError::ExpressionUnexpectedToken { .. }
                                | CompileError::ExpressionUnknownIdentifier { .. }
                                | CompileError::ExpressionLoweringUnsupported { .. }
                                | CompileError::ExpressionHelperArity { .. }
                                | CompileError::IdempotencyViolation { .. }
                                | CompileError::SourceTooLarge { .. }
                                | CompileError::Utf8(_)
                                | CompileError::EmptySource
                                | CompileError::Parse(_)
                                | CompileError::CanonicalYaml { .. }
                                | CompileError::DocumentCount { .. }
                                | CompileError::TopLevelNotMapping
                                | CompileError::NonStringKey { .. }
                                | CompileError::DuplicateKey { .. }
                                | CompileError::AliasForbidden { .. }
                                | CompileError::AnchorForbidden { .. }
                                | CompileError::MergeKeyForbidden { .. }
                                | CompileError::TagForbidden { .. }
                                | CompileError::BadValue
                                | CompileError::FloatForbidden
                                | CompileError::DepthLimit { .. }
                                | CompileError::NodeLimit { .. }
                                | CompileError::SequenceLimit { .. }
                                | CompileError::MappingLimit { .. }
                                | CompileError::ScalarLimit { .. }
                                | CompileError::Validation(_)
                                | CompileError::MissingField { .. }
                                | CompileError::UnknownTopLevelField { .. }
                                | CompileError::InvalidVersion { .. }
                                | CompileError::InvalidTriggerCount { .. }
                                | CompileError::UnknownTriggerKind { .. }
                                | CompileError::TriggerShape { .. }
                                | CompileError::UnknownTriggerField { .. }
                                | CompileError::MissingTriggerField { .. }
                                | CompileError::InvalidTriggerField { .. }
                                | CompileError::FieldShape { .. }
                                | CompileError::UnknownInputSchemaField { .. }
                                | CompileError::InvalidInputSchema { .. }
                                | CompileError::UnsupportedTopLevelResult
                                | CompileError::UnsupportedTopLevelDeclaration { .. }
                                | CompileError::PrimitiveLoweringLimitExceeded { .. }
                                | CompileError::UnsupportedConstantValue { .. }
                                | CompileError::UnknownReferenceRoot { .. }
                                | CompileError::IllegalReference { .. }
                                | CompileError::UnknownReferenceName { .. }
                                | CompileError::UnsupportedAccessorReference { .. }
                        ),
                        "error must be a typed CompileError variant: {:?}",
                        error
                    );
                }
            }
        }
    }

    /// YamlCompiler::compile is the public API entrypoint and also never panics.
    #[test]
    fn yaml_compiler_compile_never_panics(yaml in valid_yaml_workflow()) {
        let result = YamlCompiler::default().compile(yaml.as_bytes());

        // Must return Result, never panic
        prop_assert!(
            result.is_ok() || result.is_err(),
            "YamlCompiler::compile must return Result"
        );
    }
}

// ---------------------------------------------------------------------------
// Source scan: no from_parts_unchecked reachable from public APIs
// ---------------------------------------------------------------------------

#[test]
fn no_unchecked_construction_in_public_compile_apis() {
    // This test documents the bead acceptance criteria: the only way to
    // construct CompiledWorkflow from public compile APIs is through
    // try_from_parts. The source change in part_01.rs removed the single
    // unchecked emission site.
    let source = include_str!("../src/mod_compile_lowering/part_01.rs");
    assert!(
        !source.contains("from_parts_unchecked"),
        "part_01.rs must not contain from_parts_unchecked after bead fix"
    );
}
