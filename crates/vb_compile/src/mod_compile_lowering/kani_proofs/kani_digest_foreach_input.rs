// Verification artifact: kani_digest_foreach_input.rs
// PO: PO-K-FE-01 | Command: cargo kani --harness kani_foreach_input_reaches_hasher -p vb_compile
// Bead: vb-xi2f.28 | State: 5 (proof-writer)
// Model bounds: 1-character strings for Kani tractability.
// Larger strings covered by proptest PO-P-FE-01.

#![cfg(kani)]
#![allow(unused_must_use)]

use vb_yaml::ast::{StepAst, StepPrimitive};

/// PO-K-FE-01: Prove ForEach.input reaches hasher.
/// Uses single-character strings to keep Kani state space minimal.
/// Full-length random strings covered by proptest PO-P-FE-01.
#[kani::proof]
#[kani::unwind(4)]
fn kani_foreach_input_reaches_hasher() {
    // Single-char strings generated as constrained chars
    let vc: char = kani::any();
    kani::assume(vc.is_ascii_alphanumeric() || vc == '_');
    let variable = vc.to_string();

    let ca: char = kani::any();
    kani::assume(ca.is_ascii_alphanumeric() || ca == '_');
    let input_a = ca.to_string();

    let cb: char = kani::any();
    kani::assume(cb.is_ascii_alphanumeric() || cb == '_');
    let input_b = cb.to_string();
    kani::assume(input_a != input_b);

    let at_once: Option<u32> = kani::any();

    let foreach_a = StepPrimitive::ForEach {
        variable: variable.clone(),
        input: input_a,
        at_once,
        body: vec![],
    };
    let foreach_b = StepPrimitive::ForEach {
        variable,
        input: input_b,
        at_once,
        body: vec![],
    };

    let mut ha = blake3::Hasher::new();
    let mut hb = blake3::Hasher::new();
    super::super::digest_step_primitive(&mut ha, &foreach_a);
    super::super::digest_step_primitive(&mut hb, &foreach_b);

    kani::assert(ha.finalize().as_bytes() != hb.finalize().as_bytes());
}
