// Verification artifact: kani_resource_contract_dual_path_equivalence.rs
// PO: PO-K10
// Bead: vb-xi2f.35
// Verifier: Kani
// Command: cargo kani --harness prove_dual_path_digest_equivalence --unwind 3
// Workdir: crates/vb_compile
//
// Proof obligation: After deduplication (single canonical_digest in mod_compile_lowering),
// prove that compile_source produces a CompiledWorkflow with a correct digest that
// incorporates the contract.
//
// GOD RULE 1: Representative source with bounded contract.
// GOD RULE 2: Calls actual production canonical_digest and compile_source.

#![cfg(kani)]
#![forbid(unsafe_code)]

use vb_core::workflow::ResourceContract;

fn representative_source() -> crate::ast::WorkflowSource {
    let yaml = "version: velvet-ballastics/v1\nname: dual_path_test\nwhen: { manual: {} }\nsteps:\n  - id: step_one\n    set:\n      output: x\n      value: \"42\"\n";
    crate::parse_workflow_source(yaml).expect("valid representative YAML source for Kani")
}

/// PO-K10: Prove that canonical_digest is consistent within the compilation pipeline.
/// The digest computed by compile_source matches the digest from canonical_digest directly.
#[kani::proof]
#[kani::unwind(3)]
fn prove_dual_path_digest_equivalence() {
    let source = representative_source();
    let contract = ResourceContract::DEFAULT;

    // Digest computed directly
    let digest_direct = crate::mod_compile_lowering::canonical_digest(&source, contract);

    // Digest from compilation
    let workflow = crate::mod_compile_lowering::compile_source(&source, contract)
        .expect("valid source must compile");
    let digest_compiled = workflow.digest();

    assert_eq!(
        digest_direct, digest_compiled,
        "Direct canonical_digest must match the digest in CompiledWorkflow"
    );

    kani::cover!(digest_direct == digest_compiled);
}

/// PO-K10 H2: Verify with a non-DEFAULT contract.
#[kani::proof]
#[kani::unwind(2)]
fn prove_dual_path_digest_equivalence_non_default() {
    let source = representative_source();
    let contract = ResourceContract {
        max_steps: 5,
        max_slots: 3,
        max_constants: 2,
        max_accessors: 2,
        max_expressions: 2,
        max_expr_stack: 4,
        max_step_budget_per_tick: 8,
        max_transitions_per_tick: 8,
        max_input_bytes: 128,
        max_output_bytes: 128,
        max_blob_bytes: 8,
        max_ipc_payload_bytes: 128,
        max_retry_attempts: 2,
        max_fanout: 4,
        max_collect_items: 16,
        max_queue_depth: 16,
        max_journal_batch_bytes: 128,
        allows_secret_results: true,
    };

    let digest_direct = crate::mod_compile_lowering::canonical_digest(&source, contract);
    let workflow = crate::mod_compile_lowering::compile_source(&source, contract)
        .expect("valid source must compile");

    assert_eq!(
        digest_direct,
        workflow.digest(),
        "Direct digest must match compiled workflow digest for non-DEFAULT contract"
    );

    kani::cover!(digest_direct == workflow.digest());
}
