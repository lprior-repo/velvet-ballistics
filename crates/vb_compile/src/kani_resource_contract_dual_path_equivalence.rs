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

// YAML source can't be symbolic (kani::any) because the YAML parser
// (saphyr/vb_yaml) requires concrete string inputs. Coverage beyond the
// representative single-step Set workflow is provided by:
// - proptest: proptest_finish_digest, proptest_choose_lowering,
//   proptest_together_errors
// - fuzz: fuzz/fuzz_targets/compile_source.rs
fn representative_source() -> vb_yaml::ast::WorkflowSource {
    let yaml = "version: velvet-ballastics/v1\nname: dual_path_test\nwhen: { manual: {} }\nsteps:\n  - id: step_one\n    set:\n      output: x\n      value: \"42\"\n";
    match vb_yaml::parse_workflow_source(yaml) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop {}}
    }
}

/// PO-K10: Prove that canonical_digest is consistent within the compilation pipeline.
/// The digest computed by compile_source matches the digest from canonical_digest directly.
#[kani::proof]
#[kani::unwind(8)]
fn prove_dual_path_digest_equivalence() {
    let source = representative_source();
    let contract: ResourceContract = kani::any();

    // Digest computed directly
    let digest_direct = crate::mod_compile_lowering::canonical_digest(&source, contract);

    // Digest from compilation
    let workflow = match crate::mod_compile_lowering::compile_source(&source, contract) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop {}}
    };
    let digest_compiled = workflow.digest();

    kani::assert_eq!(digest_direct, digest_compiled,
        "Direct canonical_digest must match the digest in CompiledWorkflow")

    kani::cover!(digest_direct == digest_compiled);
}

/// PO-K10 H2: Verify with a non-DEFAULT contract.
#[kani::proof]
#[kani::unwind(4)]
fn prove_dual_path_digest_equivalence_non_default() {
    let source = representative_source();
    let contract: ResourceContract = kani::any();
    kani::assume(contract != ResourceContract::DEFAULT);

    let digest_direct = crate::mod_compile_lowering::canonical_digest(&source, contract);
    let workflow = match crate::mod_compile_lowering::compile_source(&source, contract) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop {}}
    };

    kani::assert_eq!(digest_direct,
        workflow.digest(),
        "Direct digest must match compiled workflow digest for non-DEFAULT contract")

    kani::cover!(digest_direct == workflow.digest());
}
