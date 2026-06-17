// Verification artifact: kani_digest_determinism.rs
// PO: PO-K-FE-05 | Command: cargo kani --harness kani_foreach_digest_step_deterministic -p vb_compile
// Bead: vb-xi2f.28 | State: 5 (proof-writer)
// Model bounds: max_string_len=16, tool: --unwind 5

#![cfg(kani)]
#![allow(unused_must_use)]

use vb_yaml::ast::{StepAst, StepPrimitive};

fn bounded_string(max_len: usize) -> String {
    let mut buf: Vec<u8> = Vec::with_capacity(max_len);
    let len: u8 = kani::any();
    let actual_len = (len as usize).min(max_len);
    for _i in 0..actual_len {
        let byte: u8 = kani::any();
        kani::assume(byte.is_ascii_alphanumeric() || byte == b'_');
        buf.push(byte);
    }
    String::from_utf8(buf).unwrap_or_default()
}

/// PO-K-FE-05 H1: digest_step_primitive is deterministic for ForEach.
#[kani::proof]
#[kani::unwind(5)]
fn kani_foreach_digest_step_deterministic() {
    let variable = bounded_string(16);
    kani::assume(!variable.is_empty());
    let input = bounded_string(16);
    kani::assume(!input.is_empty());
    let at_once: Option<u32> = kani::any();

    let foreach = StepPrimitive::ForEach {
        variable,
        input,
        at_once,
        body: vec![],
    };

    let mut h1 = blake3::Hasher::new();
    let mut h2 = blake3::Hasher::new();
    super::super::digest_step_primitive(&mut h1, &foreach);
    super::super::digest_step_primitive(&mut h2, &foreach);
    kani::assert(h1.finalize().as_bytes() == h2.finalize().as_bytes(), "assertion failed");
}

/// PO-K-FE-05 H2: digest_step_primitive is deterministic for Set.
#[kani::proof]
#[kani::unwind(8)]
fn kani_set_digest_step_deterministic() {
    let output = bounded_string(16);
    kani::assume(!output.is_empty());
    let value = bounded_string(16);
    kani::assume(!value.is_empty());

    let set_prim = StepPrimitive::Set { output, value };

    let mut h1 = blake3::Hasher::new();
    let mut h2 = blake3::Hasher::new();
    super::super::digest_step_primitive(&mut h1, &set_prim);
    super::super::digest_step_primitive(&mut h2, &set_prim);
    kani::assert(h1.finalize().as_bytes() == h2.finalize().as_bytes(), "assertion failed");
}

// NOTE: H3 (kani_canonical_digest_deterministic) removed — GOD RULE 1 violation
// (hardcoded YAML document). Determinism coverage for canonical_digest is provided
// by H1 (foreach determinism) and H2 (set determinism), plus proptest PO-P-FE-05.
