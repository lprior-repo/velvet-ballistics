// Verification artifact: kani_resource_contract_cross_field_collision.rs
// PO: PO-K03
// Bead: vb-xi2f.35
// Verifier: Kani
// Command: cargo kani --harness prove_no_cross_field_collision --unwind 2
// Workdir: crates/vb_compile
//
// Proof obligation: Domain-tagged encoding prevents cross-field collisions.
// Swapping values between two differently-named fields produces different encodings
// because field tags are hashed as part of the encoding.
//
// GOD RULE 1: Uses kani::any() for value generation.
// GOD RULE 2: Calls actual production encode_contract_bytes and canonical_digest.

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
    let yaml = "version: velvet-ballastics/v1\nname: collision_test\nwhen: { manual: {} }\nsteps:\n  - id: step_one\n    set:\n      output: x\n      value: \"42\"\n";
    match vb_yaml::parse_workflow_source(yaml) {
        Ok(v) => v,
        Err(_) => { kani::assume(false); loop {}}
    }
}

/// PO-K03: Prove that domain-tagged encoding prevents cross-field collisions.
///
/// Strategy: Test pairs of same-type fields (where collision risk is highest).
/// For each pair (field_A, field_B), construct two contracts where the values
/// are swapped between fields and verify that the encodings differ.
#[kani::proof]
#[kani::unwind(10)]
fn prove_no_cross_field_collision() {
    // Test the most collision-prone pair: two u16 fields
    // Create contract where max_steps=A, max_slots=B
    // vs contract where max_steps=B, max_slots=A

    let field_a_val: u16 = kani::any();
    kani::assume(field_a_val >= 1 && field_a_val <= 100);
    let field_b_val: u16 = kani::any();
    kani::assume(field_b_val >= 1 && field_b_val <= 100);
    kani::assume(field_a_val != field_b_val);

    // Contract 1: max_steps = field_a_val, max_slots = field_b_val
    let mut contract_1 = ResourceContract::DEFAULT;
    contract_1.max_steps = field_a_val;
    contract_1.max_slots = field_b_val;

    // Contract 2: max_steps = field_b_val, max_slots = field_a_val (swapped)
    let mut contract_2 = ResourceContract::DEFAULT;
    contract_2.max_steps = field_b_val;
    contract_2.max_slots = field_a_val;

    assert_ne!(
        contract_1.max_steps, contract_2.max_steps,
        "Swapped pair: max_steps values must differ"
    );
    assert_ne!(
        contract_1.max_slots, contract_2.max_slots,
        "Swapped pair: max_slots values must differ"
    );

    // Encoding must differ due to field tags
    let enc_1 = encode_contract_bytes(&contract_1);
    let enc_2 = encode_contract_bytes(&contract_2);

    assert_ne!(
        enc_1, enc_2,
        "Field-tagged encoding must prevent cross-field collision: \
         swapping max_steps and max_slots values must produce different encodings"
    );

    // Digest must also differ
    let source = representative_source();
    let digest_1 = crate::mod_compile_lowering::canonical_digest(&source, contract_1);
    let digest_2 = crate::mod_compile_lowering::canonical_digest(&source, contract_2);
    assert_ne!(
        digest_1, digest_2,
        "canonical_digest must prevent cross-field collisions"
    );

    kani::cover!(enc_1 != enc_2 && digest_1 != digest_2);
}

/// PO-K03 H2: Test same-type field collision for u32 pairs (max_input_bytes vs max_output_bytes).
#[kani::proof]
#[kani::unwind(10)]
fn prove_no_cross_field_collision_u32() {
    let val_a: u32 = kani::any();
    kani::assume(val_a >= 1 && val_a <= 256);
    let val_b: u32 = kani::any();
    kani::assume(val_b >= 1 && val_b <= 256);
    kani::assume(val_a != val_b);

    let mut contract_1 = ResourceContract::DEFAULT;
    contract_1.max_input_bytes = val_a;
    contract_1.max_output_bytes = val_b;

    let mut contract_2 = ResourceContract::DEFAULT;
    contract_2.max_input_bytes = val_b;
    contract_2.max_output_bytes = val_a;

    assert_ne!(contract_1.max_input_bytes, contract_2.max_input_bytes);
    assert_ne!(contract_1.max_output_bytes, contract_2.max_output_bytes);

    let enc_1 = encode_contract_bytes(&contract_1);
    let enc_2 = encode_contract_bytes(&contract_2);

    assert_ne!(
        enc_1, enc_2,
        "Field-tagged encoding must prevent u32 cross-field collision"
    );
}

/// PO-K03 H3: Test same-type field collision for u64 pairs (max_step_budget_per_tick vs max_transitions_per_tick).
#[kani::proof]
#[kani::unwind(10)]
fn prove_no_cross_field_collision_u64() {
    let val_a: u64 = kani::any();
    kani::assume(val_a >= 1 && val_a <= 16);
    let val_b: u64 = kani::any();
    kani::assume(val_b >= 1 && val_b <= 16);
    kani::assume(val_a != val_b);

    let mut contract_1 = ResourceContract::DEFAULT;
    contract_1.max_step_budget_per_tick = val_a;
    contract_1.max_transitions_per_tick = val_b;

    let mut contract_2 = ResourceContract::DEFAULT;
    contract_2.max_step_budget_per_tick = val_b;
    contract_2.max_transitions_per_tick = val_a;

    assert_ne!(
        contract_1.max_step_budget_per_tick,
        contract_2.max_step_budget_per_tick
    );
    assert_ne!(
        contract_1.max_transitions_per_tick,
        contract_2.max_transitions_per_tick
    );

    let enc_1 = encode_contract_bytes(&contract_1);
    let enc_2 = encode_contract_bytes(&contract_2);

    assert_ne!(
        enc_1, enc_2,
        "Field-tagged encoding must prevent u64 cross-field collision"
    );
}
