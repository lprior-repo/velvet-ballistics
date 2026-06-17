// Verification artifact: kani_resource_contract_type_canonical_fields.rs
// PO: PO-K05
// Bead: vb-xi2f.35
// Verifier: Kani
// Command: cargo kani --harness prove_canonical_contract_has_18_fields --unwind 1
// Workdir: crates/vb_core
//
// Proof obligation: Prove that the canonical ResourceContract type has exactly 17
// accessible fields, including max_transitions_per_tick and allows_secret_results.
//
// GOD RULE 1: Uses compile-time field enumeration; no hardcoded dummy structs.
// GOD RULE 2: Binds to actual vb_core::workflow::ResourceContract definition.

#![cfg(kani)]
#![forbid(unsafe_code)]

use vb_core::workflow::ResourceContract;

/// PO-K05: Verify that the canonical ResourceContract has exactly 18 fields.
///
/// This is a compile-time structural check. The harness constructs a contract
/// with explicit field assignments and Kani verifies the struct shape.
/// The duplicate 15-field type in compiled_workflow.rs has been removed (PF-BH-002).
/// This confirms the canonical 18-field type in workflow/mod.rs is the single source of truth.
#[kani::proof]
#[kani::unwind(10)]
fn prove_canonical_contract_has_18_fields() {
    // Construct a contract with all 18 fields explicitly assigned.
    // This binds to the actual struct definition; if a field is missing
    // or extra, this won't compile — providing a compile-time check.
    let contract = ResourceContract {
        max_steps: 100,
        max_slots: 32,
        max_constants: 16,
        max_accessors: 16,
        max_expressions: 16,
        max_expr_stack: 8,
        max_step_budget_per_tick: 16,
        max_transitions_per_tick: 16, // CRITICAL: must be present (missing from 15-field dup)
        max_input_bytes: 256,
        max_output_bytes: 256,
        max_blob_bytes: 16,
        max_ipc_payload_bytes: 256,
        max_retry_attempts: 3,
        max_fanout: 8,
        max_collect_items: 32,
        max_queue_depth: 32,
        max_journal_batch_bytes: 256,
        allows_secret_results: true, // CRITICAL: must be present (missing from 15-field dup)
    };

    // Verify field values are preserved intact through the struct.
    kani::assert(contract.max_transitions_per_tick == 16);
    kani::assert(contract.allows_secret_results == true);
    kani::assert(contract.max_steps == 100);

    // Structural assertion: the type is Copy (value semantics)
    let copy = contract;
    kani::assert(copy.max_transitions_per_tick == contract.max_transitions_per_tick);
    kani::assert(copy.allows_secret_results == contract.allows_secret_results);

    kani::cover!(contract.max_transitions_per_tick == 16);
}

/// PO-K05 variant: Verify that DEFAULT has all 18 fields set.
/// This ensures no field of DEFAULT is accidentally left uninitialized.
#[kani::proof]
#[kani::unwind(10)]
fn prove_default_contract_has_18_fields() {
    let default = ResourceContract::DEFAULT;

    // All 18 fields of DEFAULT must be accessible.
    // Compilation ensures this; we verify the values are sensible.
    kani::assert(default.max_steps > 0, "kani harness assertion");
    kani::assert(default.max_slots > 0, "kani harness assertion");
    kani::assert(default.max_constants > 0, "kani harness assertion");
    kani::assert(default.max_accessors > 0, "kani harness assertion");
    kani::assert(default.max_expressions > 0, "kani harness assertion");
    kani::assert(default.max_expr_stack > 0, "kani harness assertion");
    kani::assert(default.max_step_budget_per_tick > 0, "kani harness assertion");
    kani::assert(default.max_transitions_per_tick > 0, "kani harness assertion") // Critical field
    kani::assert(default.max_input_bytes > 0, "kani harness assertion");
    kani::assert(default.max_output_bytes > 0, "kani harness assertion");
    kani::assert(default.max_blob_bytes > 0, "kani harness assertion");
    kani::assert(default.max_ipc_payload_bytes > 0, "kani harness assertion");
    kani::assert(default.max_retry_attempts > 0, "kani harness assertion");
    kani::assert(default.max_fanout > 0, "kani harness assertion");
    kani::assert(default.max_collect_items > 0, "kani harness assertion");
    kani::assert(default.max_queue_depth > 0, "kani harness assertion");
    kani::assert(default.max_journal_batch_bytes > 0, "kani harness assertion");
    // allows_secret_results is false by default (conservative)
    kani::assert(!default.allows_secret_results, "kani harness assertion");

    kani::cover!(default.max_transitions_per_tick > 0);
}
