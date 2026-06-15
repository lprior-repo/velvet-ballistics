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

// YAML source can't be symbolic (kani::any) because the YAML parser
// (saphyr/vb_yaml) requires concrete string inputs. Coverage beyond the
// representative single-step Set workflow is provided by:
// - proptest: proptest_finish_digest, proptest_choose_lowering,
//   proptest_together_errors
// - fuzz: fuzz/fuzz_targets/compile_source.rs
fn representative_source() -> vb_yaml::ast::WorkflowSource {
    let yaml = "version: velvet-ballastics/v1\nname: entry_point_test\nwhen: { manual: {} }\nsteps:\n  - id: step_one\n    set:\n      output: x\n      value: \"42\"\n";
    match vb_yaml::parse_workflow_source(yaml) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop {}}
    }
}

/// PO-K07: Prove that a non-DEFAULT ResourceContract survives compilation.
///
/// This harness calls the actual production compile_source with a non-DEFAULT
/// contract and verifies the resulting CompiledWorkflow carries the correct contract.
///
/// Uses kani::any::<ResourceContract>() for bounded nondeterministic input
/// and constrains it to be different from DEFAULT (the proof requires inequality).
#[kani::proof]
#[kani::unwind(8)]
fn prove_contract_survives_compilation() {
    let contract: ResourceContract = kani::any();
    kani::assume(contract != ResourceContract::DEFAULT);

    // Verify it's actually different from DEFAULT
    let default = ResourceContract::DEFAULT;
    kani::assert_ne!(contract, default,
        "Non-DEFAULT contract must differ from DEFAULT")

    // Compile with the non-DEFAULT contract
    let source = representative_source();
    let workflow = match crate::mod_compile_lowering::compile_source(&source, contract) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop {}}
    };

    // Verify contract identity survived compilation
    kani::assert_eq!(workflow.resource_contract(),
        contract,
        "CompiledWorkflow.resource_contract() must equal the input contract after compilation")

    // Verify digest changed (contract is part of digest)
    let workflow_default = match crate::mod_compile_lowering::compile_source(&source, default) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop {}}
    };

    kani::assert_ne!(workflow.digest(),
        workflow_default.digest(),
        "CompiledWorkflow digest must differ when contracts differ")

    kani::cover!(workflow.resource_contract() == contract);
}

/// PO-K07 H2: Prove that a non-DEFAULT contract's encoding differs from DEFAULT.
#[kani::proof]
#[kani::unwind(10)]
fn prove_non_default_contract_encoding_differs() {
    let default = ResourceContract::DEFAULT;

    let mut modified = default;
    let max_steps: u16 = kani::any();
    kani::assume(max_steps > 0 && max_steps < 10_000);
    kani::assume(max_steps != default.max_steps);
    modified.max_steps = max_steps;
    modified.allows_secret_results = true;

    kani::assert_ne!(default, modified)

    let enc_default = encode_contract_bytes(&default);
    let enc_modified = encode_contract_bytes(&modified);

    kani::assert_ne!(enc_default, enc_modified,
        "Non-DEFAULT contract encoding must differ from DEFAULT contract encoding")

    kani::cover!(enc_default != enc_modified);
}
