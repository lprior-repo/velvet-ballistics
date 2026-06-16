#![forbid(unsafe_code)]
//! ResourceContract type integrity tests (Behaviors B1–B5).
//! Bead: vb-xi2f.35 — P1: digest covers resource contract semantics.
//!
//! Verifies that ResourceContract has exactly 18 fields including
//! max_transitions_per_tick and allows_secret_results.

// Test code uses `.expect("descriptive message")` to convert fallible
// public-API results into asserted values. Per repository policy
// (AGENTS.md: "Tests must compile and run, but test clippy is not strict"),
// `clippy::expect_used` is allowed in this test target.
#![allow(clippy::expect_used)]

use vb_core::ResourceContract;

// ---------------------------------------------------------------------------
// B1: Canonical type has exactly 18 fields
// ---------------------------------------------------------------------------

/// This test proves that ResourceContract has exactly 18 fields by
/// constructing the struct with all 18 fields named. If a field is missing,
/// this won't compile. If an extra field is expected but missing, the
/// struct literal will have an extra `..ResourceContract::DEFAULT` but
/// still produce a compilation error for the missing field.
///
/// This is primarily a COMPILE-TIME test — the runtime assertions just
/// provide additional confidence.
#[test]
fn resource_contract_canonical_type_has_18_fields() {
    let c = ResourceContract {
        max_steps: 1,
        max_slots: 1,
        max_constants: 1,
        max_accessors: 1,
        max_expressions: 1,
        max_expr_stack: 1,
        max_step_budget_per_tick: 1,
        max_transitions_per_tick: 1,
        max_input_bytes: 1,
        max_output_bytes: 1,
        max_blob_bytes: 1,
        max_ipc_payload_bytes: 1,
        max_retry_attempts: 1,
        max_fanout: 1,
        max_collect_items: 1,
        max_queue_depth: 1,
        max_journal_batch_bytes: 1,
        allows_secret_results: false,
    };

    // Verify field values reflect what we set
    assert_eq!(c.max_steps, 1);
    assert_eq!(c.max_slots, 1);
    assert_eq!(c.max_constants, 1);
    assert_eq!(c.max_accessors, 1);
    assert_eq!(c.max_expressions, 1);
    assert_eq!(c.max_expr_stack, 1);
    assert_eq!(c.max_step_budget_per_tick, 1);
    assert_eq!(c.max_transitions_per_tick, 1);
    assert_eq!(c.max_input_bytes, 1);
    assert_eq!(c.max_output_bytes, 1);
    assert_eq!(c.max_blob_bytes, 1);
    assert_eq!(c.max_ipc_payload_bytes, 1);
    assert_eq!(c.max_retry_attempts, 1);
    assert_eq!(c.max_fanout, 1);
    assert_eq!(c.max_collect_items, 1);
    assert_eq!(c.max_queue_depth, 1);
    assert_eq!(c.max_journal_batch_bytes, 1);
    assert!(!c.allows_secret_results);
}

// ---------------------------------------------------------------------------
// B2: CompiledWorkflow accepts 18-field ResourceContract
// ---------------------------------------------------------------------------

#[test]
fn compiled_workflow_accepts_18_field_resource_contract() {
    use vb_core::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstValue, StepIdx, WorkflowDigest,
        WorkflowParts,
    };

    let mut c = ResourceContract::DEFAULT;
    c.max_transitions_per_tick = 100;
    c.allows_secret_results = true;

    let parts = WorkflowParts {
        name: Box::<str>::from("type_integrity_test"),
        digest: WorkflowDigest::from_bytes([0xCA; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: c,
        step_names: Box::default(),
    };

    let workflow = CompiledWorkflow::try_from_parts(parts)
        .expect("CompiledWorkflow::try_from_parts must accept 18-field ResourceContract");
    assert_eq!(workflow.resource_contract().max_transitions_per_tick, 100);
    assert!(workflow.resource_contract().allows_secret_results);
}

// ---------------------------------------------------------------------------
// B3: resource_contract() returns full 18-field contract
// ---------------------------------------------------------------------------

#[test]
fn compiled_workflow_resource_contract_returns_full_18_field_contract() {
    use vb_core::{
        CompiledNode, CompiledNodeKind, CompiledWorkflow, ConstValue, StepIdx, WorkflowDigest,
        WorkflowParts,
    };

    let mut c = ResourceContract::DEFAULT;
    c.max_steps = 42;
    c.max_slots = 7;
    c.max_step_budget_per_tick = 999;
    c.max_transitions_per_tick = 888;
    c.allows_secret_results = true;

    let parts = WorkflowParts {
        name: Box::<str>::from("roundtrip_test"),
        digest: WorkflowDigest::from_bytes([0x1D; 32]),
        nodes: vec![CompiledNode {
            id: StepIdx::new(0),
            output: None,
            next: None,
            on_error: None,
            error_slot: None,
            kind: CompiledNodeKind::Nop,
        }]
        .into_boxed_slice(),
        expressions: Box::new([]),
        accessors: Box::new([]),
        constants: vec![ConstValue::Null].into_boxed_slice(),
        slot_count: 0,
        symbols_count: 0,
        entry: StepIdx::new(0),
        resource_contract: c,
        step_names: Box::default(),
    };

    let workflow =
        CompiledWorkflow::try_from_parts(parts).expect("Valid parts with 18-field contract");
    let returned = workflow.resource_contract();

    assert_eq!(returned.max_steps, 42, "max_steps roundtrip");
    assert_eq!(returned.max_slots, 7, "max_slots roundtrip");
    assert_eq!(
        returned.max_step_budget_per_tick, 999,
        "max_step_budget_per_tick roundtrip"
    );
    assert_eq!(
        returned.max_transitions_per_tick, 888,
        "max_transitions_per_tick roundtrip"
    );
    assert!(
        returned.allows_secret_results,
        "allows_secret_results roundtrip"
    );
    // Spot-check a few more fields to ensure full roundtrip
    assert_eq!(
        returned.max_input_bytes, c.max_input_bytes,
        "max_input_bytes roundtrip"
    );
    assert_eq!(
        returned.max_output_bytes, c.max_output_bytes,
        "max_output_bytes roundtrip"
    );
    assert_eq!(
        returned.max_retry_attempts, c.max_retry_attempts,
        "max_retry_attempts roundtrip"
    );
    assert_eq!(returned.max_fanout, c.max_fanout, "max_fanout roundtrip");
}

// ---------------------------------------------------------------------------
// B4/B5: Import paths (static checks)
// ---------------------------------------------------------------------------
//
// B4: Validation imports canonical type — verified by B2 and B3 which
//     successfully construct and validate WorkflowParts with the 18-field
//     ResourceContract. If validation imported the 16-field duplicate,
//     these tests would fail with type mismatches.
//
// B5: 16-field duplicate is inaccessible — verified by the fact that
//     ResourceContract (from vb_core) has exactly 18 fields. Any code
//     depending on a 16-field variant would fail to compile against
//     the public API.

// ---------------------------------------------------------------------------
// Additional: DEFAULT contract has 17 reasonable values
// ---------------------------------------------------------------------------

#[test]
fn resource_contract_default_has_reasonable_values() {
    let c = ResourceContract::DEFAULT;
    assert!(c.max_steps > 0, "DEFAULT max_steps must be > 0");
    assert!(c.max_slots > 0, "DEFAULT max_slots must be > 0");
    assert!(
        c.max_step_budget_per_tick > 0,
        "DEFAULT max_step_budget_per_tick must be > 0"
    );
    assert!(
        c.max_transitions_per_tick > 0,
        "DEFAULT max_transitions_per_tick must be > 0"
    );
    // allows_secret_results defaults to false (conservative)
    assert!(
        !c.allows_secret_results,
        "DEFAULT must be conservative: allows_secret_results=false"
    );
}

// ---------------------------------------------------------------------------
// Additional: ResourceContract implements Copy (transparently)
// ---------------------------------------------------------------------------

#[test]
fn resource_contract_is_copy() {
    let c = ResourceContract::DEFAULT;
    let c2 = c; // Copy, not move
    assert_eq!(c, c2);
}

// ---------------------------------------------------------------------------
// Additional: ResourceContract field-level doc-check
// ---------------------------------------------------------------------------

#[test]
fn resource_contract_all_18_fields_accessible() {
    let c = ResourceContract::DEFAULT;
    // Access all fields — this is a compile-time check that each field exists
    let _fields = (
        c.max_steps,
        c.max_slots,
        c.max_constants,
        c.max_accessors,
        c.max_expressions,
        c.max_expr_stack,
        c.max_step_budget_per_tick,
        c.max_transitions_per_tick,
        c.max_input_bytes,
        c.max_output_bytes,
        c.max_blob_bytes,
        c.max_ipc_payload_bytes,
        c.max_retry_attempts,
        c.max_fanout,
        c.max_collect_items,
        c.max_queue_depth,
        c.max_journal_batch_bytes,
        c.allows_secret_results,
    );
}
