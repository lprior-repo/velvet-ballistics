// Verification artifact: kani_resource_contract_entry_point.rs
// PO: PO-K07
// Bead: vb-xi2f.35
// Verifier: Kani
// Command: cargo kani --harness prove_contract_survives_compilation --unwind 3
// Workdir: crates/vb_compile
//
// Proof obligation: For a non-DEFAULT ResourceContract, compile_source(source, contract)
// produces a CompiledWorkflow whose resource_contract() equals the input contract.
// Contract is not replaced, dropped, or overridden during compilation.
//
// GOD RULE 1: Uses non-DEFAULT contract with distinguishable fields.
// GOD RULE 2: Calls actual production compile_source and canonical_digest.

#![cfg(kani)]
#![forbid(unsafe_code)]

use vb_core::contract_encoding::encode_contract_bytes;
use vb_core::workflow::ResourceContract;

fn representative_source() -> crate::ast::WorkflowSource {
    let yaml = "version: velvet-ballastics/v1\nname: entry_point_test\nwhen: { manual: {} }\nsteps:\n  - id: step_one\n    set:\n      output: x\n      value: \"42\"\n";
    crate::parse_workflow_source(yaml).expect("valid representative YAML source for Kani")
}

/// PO-K07: Prove that a non-DEFAULT ResourceContract survives compilation.
///
/// This harness calls the actual production compile_source with a non-DEFAULT
/// contract and verifies the resulting CompiledWorkflow carries the correct contract.
#[kani::proof]
#[kani::unwind(3)]
fn prove_contract_survives_compilation() {
    // Create a non-DEFAULT contract that is clearly distinguishable
    let contract = ResourceContract {
        max_steps: 50,
        max_slots: 32,
        max_constants: 16,
        max_accessors: 16,
        max_expressions: 16,
        max_expr_stack: 8,
        max_step_budget_per_tick: 16,
        max_transitions_per_tick: 16,
        max_input_bytes: 256,
        max_output_bytes: 256,
        max_blob_bytes: 16,
        max_ipc_payload_bytes: 256,
        max_retry_attempts: 3,
        max_fanout: 8,
        max_collect_items: 32,
        max_queue_depth: 32,
        max_journal_batch_bytes: 256,
        allows_secret_results: true,
    };

    // Verify it's actually different from DEFAULT
    let default = ResourceContract::DEFAULT;
    assert_ne!(
        contract, default,
        "Non-DEFAULT contract must differ from DEFAULT"
    );

    // Compile with the non-DEFAULT contract
    let source = representative_source();
    let workflow = crate::mod_compile_lowering::compile_source(&source, contract)
        .expect("valid source must compile successfully");

    // Verify contract identity survived compilation
    assert_eq!(
        workflow.resource_contract(),
        contract,
        "CompiledWorkflow.resource_contract() must equal the input contract after compilation"
    );

    // Verify digest changed (contract is part of digest)
    let workflow_default = crate::mod_compile_lowering::compile_source(&source, default)
        .expect("valid source must compile successfully");

    assert_ne!(
        workflow.digest(),
        workflow_default.digest(),
        "CompiledWorkflow digest must differ when contracts differ"
    );

    kani::cover!(workflow.resource_contract() == contract);
}

/// PO-K07 H2: Prove that a non-DEFAULT contract's encoding differs from DEFAULT.
#[kani::proof]
#[kani::unwind(1)]
fn prove_non_default_contract_encoding_differs() {
    let default = ResourceContract::DEFAULT;

    let mut modified = default;
    modified.max_steps = 50;
    modified.allows_secret_results = true;

    assert_ne!(default, modified);

    let enc_default = encode_contract_bytes(&default);
    let enc_modified = encode_contract_bytes(&modified);

    assert_ne!(
        enc_default, enc_modified,
        "Non-DEFAULT contract encoding must differ from DEFAULT contract encoding"
    );

    kani::cover!(enc_default != enc_modified);
}
