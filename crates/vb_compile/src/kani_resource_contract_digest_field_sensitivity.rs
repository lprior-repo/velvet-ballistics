// Verification artifact: kani_resource_contract_digest_field_sensitivity.rs
// PO: PO-K02, PO-K08
// Bead: vb-xi2f.35
// Verifier: Kani
// Commands:
//   PO-K02: cargo kani --harness prove_single_field_changes_digest --unwind 3
//   PO-K08: cargo kani --harness prove_secret_results_changes_digest --unwind 2
// Workdir: crates/vb_compile
//
// Proof obligations:
// - PO-K02: Changing any single contract field changes the canonical digest
// - PO-K08: Changing allows_secret_results changes the canonical digest
//
// GOD RULE 1: Uses kani::any() for bounded contract pairs.
// GOD RULE 2: Calls actual production encode_contract_bytes and canonical_digest.

#![cfg(kani)]
#![forbid(unsafe_code)]

use vb_core::contract_encoding::encode_contract_bytes;
use vb_core::workflow::ResourceContract;

fn base_contract() -> ResourceContract {
    ResourceContract {
        max_steps: 50,
        max_slots: 16,
        max_constants: 8,
        max_accessors: 8,
        max_expressions: 8,
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
        allows_secret_results: false,
    }
}

/// Helper: Get a representative WorkflowSource for Kani verification.
fn representative_source() -> vb_yaml::ast::WorkflowSource {
    let yaml = "version: velvet-ballastics/v1\nname: field_sensitivity_test\nwhen: { manual: {} }\nsteps:\n  - id: step_one\n    set:\n      output: x\n      value: \"42\"\n";
    vb_yaml::parse_workflow_source(yaml).expect("valid representative YAML source for Kani")
}

/// PO-K02: Prove that changing a single field changes the digest.
///
/// We verify this at two levels:
/// 1. The encoding level (encode_contract_bytes): field change ⇒ encoding change
/// 2. The digest level (canonical_digest): field change ⇒ digest change
///
/// Kani nondeterministically selects a field index to mutate.
#[kani::proof]
#[kani::unwind(3)]
fn prove_single_field_changes_digest() {
    let field_idx: u8 = kani::any();
    kani::assume(field_idx < 17);

    let base = base_contract();
    let mut modified = base;

    // Mutate exactly one field based on the index
    match field_idx {
        0 => {
            let v: u16 = kani::any();
            kani::assume(v != base.max_steps && v >= 1 && v <= 100);
            modified.max_steps = v;
        }
        1 => {
            let v: u16 = kani::any();
            kani::assume(v != base.max_slots && v >= 1 && v <= 32);
            modified.max_slots = v;
        }
        2 => {
            let v: u16 = kani::any();
            kani::assume(v != base.max_constants && v >= 1 && v <= 32);
            modified.max_constants = v;
        }
        3 => {
            let v: u16 = kani::any();
            kani::assume(v != base.max_accessors && v >= 1 && v <= 32);
            modified.max_accessors = v;
        }
        4 => {
            let v: u16 = kani::any();
            kani::assume(v != base.max_expressions && v >= 1 && v <= 32);
            modified.max_expressions = v;
        }
        5 => {
            let v: u8 = kani::any();
            kani::assume(v != base.max_expr_stack && v >= 1 && v <= 16);
            modified.max_expr_stack = v;
        }
        6 => {
            let v: u64 = kani::any();
            kani::assume(v != base.max_step_budget_per_tick && v >= 1 && v <= 16);
            modified.max_step_budget_per_tick = v;
        }
        7 => {
            let v: u64 = kani::any();
            kani::assume(v != base.max_transitions_per_tick && v >= 1 && v <= 16);
            modified.max_transitions_per_tick = v;
        }
        8 => {
            let v: u32 = kani::any();
            kani::assume(v != base.max_input_bytes && v >= 1 && v <= 256);
            modified.max_input_bytes = v;
        }
        9 => {
            let v: u32 = kani::any();
            kani::assume(v != base.max_output_bytes && v >= 1 && v <= 256);
            modified.max_output_bytes = v;
        }
        10 => {
            let v: u64 = kani::any();
            kani::assume(v != base.max_blob_bytes && v >= 1 && v <= 16);
            modified.max_blob_bytes = v;
        }
        11 => {
            let v: u32 = kani::any();
            kani::assume(v != base.max_ipc_payload_bytes && v >= 1 && v <= 256);
            modified.max_ipc_payload_bytes = v;
        }
        12 => {
            let v: u16 = kani::any();
            kani::assume(v != base.max_retry_attempts && v >= 1 && v <= 32);
            modified.max_retry_attempts = v;
        }
        13 => {
            let v: u16 = kani::any();
            kani::assume(v != base.max_fanout && v >= 1 && v <= 32);
            modified.max_fanout = v;
        }
        14 => {
            let v: u32 = kani::any();
            kani::assume(v != base.max_collect_items && v >= 1 && v <= 256);
            modified.max_collect_items = v;
        }
        15 => {
            let v: u32 = kani::any();
            kani::assume(v != base.max_queue_depth && v >= 1 && v <= 256);
            modified.max_queue_depth = v;
        }
        16 => {
            let v: u32 = kani::any();
            kani::assume(v != base.max_journal_batch_bytes && v >= 1 && v <= 256);
            modified.max_journal_batch_bytes = v;
        }
        _ => {}
    }

    // Ensure the field actually changed
    kani::assume(base != modified);

    // Level 1: Encoding must differ
    let encoded_base = encode_contract_bytes(&base);
    let encoded_modified = encode_contract_bytes(&modified);

    assert_ne!(
        encoded_base, encoded_modified,
        "Changing any single field must change the encoding (field_idx={})",
        field_idx
    );

    // Level 2: Digest must differ
    let source = representative_source();
    let digest_base = crate::mod_compile_lowering::canonical_digest(&source, base);
    let digest_modified = crate::mod_compile_lowering::canonical_digest(&source, modified);
    assert_ne!(
        digest_base, digest_modified,
        "canonical_digest must be field-sensitive (field_idx={})",
        field_idx
    );

    kani::cover!(field_idx < 17);
}

/// PO-K08: Prove that allows_secret_results changes the canonical digest.
#[kani::proof]
#[kani::unwind(2)]
fn prove_secret_results_changes_digest() {
    let mut contract_true = base_contract();
    contract_true.allows_secret_results = true;

    let mut contract_false = base_contract();
    contract_false.allows_secret_results = false;

    assert_ne!(
        contract_true.allows_secret_results, contract_false.allows_secret_results,
        "Precondition: contracts must differ in allows_secret_results"
    );

    // Encoding must differ
    let encoded_true = encode_contract_bytes(&contract_true);
    let encoded_false = encode_contract_bytes(&contract_false);
    assert_ne!(
        encoded_true, encoded_false,
        "allows_secret_results: true vs false must produce different encodings"
    );

    // Digest must differ
    let source = representative_source();
    let digest_true = crate::mod_compile_lowering::canonical_digest(&source, contract_true);
    let digest_false = crate::mod_compile_lowering::canonical_digest(&source, contract_false);
    assert_ne!(
        digest_true, digest_false,
        "canonical_digest must change when allows_secret_results changes"
    );

    kani::cover!(digest_true != digest_false);
}
