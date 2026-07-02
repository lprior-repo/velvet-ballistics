// Verification artifact: kani_digest_foreach_at_once_equiv.rs
// PO: PO-K-FE-07 | Command: cargo kani --harness kani_foreach_at_once_none_some1_equivalence -p vb_compile
// Bead: vb-xi2f.28 | State: 5 (proof-writer)
// Model bounds: at_once=[None, Some(0), Some(1)], tool: --unwind 3
//
// GOD RULE 1: Uses kani::any() for variable and input fields instead of hardcoded
// strings. Only at_once is varied for the equivalence test; other fields are
// generated via kani::any() with valid YAML identifier constraints.

#![cfg(kani)]
#![allow(unused_must_use)]


/// Generate a bounded YAML-identifier string using kani::any().
fn any_yaml_identifier(max_len: usize) -> String {
    let len: u8 = kani::any();
    let actual_len = (len as usize) % (max_len + 1);
    let mut s = String::with_capacity(actual_len);
    for _i in 0..actual_len {
        let byte: u8 = kani::any();
        kani::assume(byte.is_ascii_alphanumeric() || byte == b'_');
        s.push(byte as char);
    }
    kani::assume(!s.is_empty());
    s
}

/// H1: None and Some(1) produce identical digest contributions.
/// variable/input generated via kani::any() — GOD RULE 1 compliant.
#[kani::proof]
#[kani::unwind(4)]
fn kani_foreach_at_once_none_some1_equivalence() {
    let variable = any_yaml_identifier(8);
    let input = any_yaml_identifier(8);

    let foreach_none = StepPrimitiveAst::ForEach {
        variable: variable.clone(),
        input: input.clone(),
        at_once: None,
        body: vec![],
    };
    let foreach_some1 = StepPrimitiveAst::ForEach {
        variable,
        input,
        at_once: Some(1),
        body: vec![],
    };

    let mut hn = blake3::Hasher::new();
    let mut h1 = blake3::Hasher::new();
    super::super::digest_step_primitive(&mut hn, &foreach_none);
    super::super::digest_step_primitive(&mut h1, &foreach_some1);
    assert_eq!(hn.finalize().as_bytes(), h1.finalize().as_bytes());
}

/// H2: None and Some(0) produce DIFFERENT digest contributions.
/// variable/input generated via kani::any() — GOD RULE 1 compliant.
#[kani::proof]
#[kani::unwind(4)]
fn kani_foreach_at_once_none_some0_inequivalence() {
    let variable = any_yaml_identifier(8);
    let input = any_yaml_identifier(8);

    let foreach_none = StepPrimitiveAst::ForEach {
        variable: variable.clone(),
        input: input.clone(),
        at_once: None,
        body: vec![],
    };
    let foreach_some0 = StepPrimitiveAst::ForEach {
        variable,
        input,
        at_once: Some(0),
        body: vec![],
    };

    let mut hn = blake3::Hasher::new();
    let mut h0 = blake3::Hasher::new();
    super::super::digest_step_primitive(&mut hn, &foreach_none);
    super::super::digest_step_primitive(&mut h0, &foreach_some0);
    // After fix, None hashes 1u32, Some(0) hashes 0u32 — should differ
    assert_ne!(hn.finalize().as_bytes(), h0.finalize().as_bytes());
}
