// Verification artifact: kani_digest_foreach_variable.rs
// PO: PO-K-FE-03 | Command: cargo kani --harness kani_foreach_variable_reaches_hasher -p vb_compile
// Bead: vb-xi2f.28 | State: 5 (proof-writer)
// Note: blake3-dependent harness — blocked by InlineAsm (BLOCKED-TOOL-01).

#![cfg(kani)]
#![allow(unused_must_use)]

use vb_yaml::ast::{StepAst, StepPrimitive};

#[kani::proof]
#[kani::unwind(4)]
fn kani_foreach_variable_reaches_hasher() {
    let ca: char = kani::any();
    kani::assume(ca.is_ascii_alphanumeric() || ca == '_');
    let variable_a = ca.to_string();
    let cb: char = kani::any();
    kani::assume(cb.is_ascii_alphanumeric() || cb == '_');
    let variable_b = cb.to_string();
    kani::assume(variable_a != variable_b);

    let ic: char = kani::any();
    kani::assume(ic.is_ascii_alphanumeric() || ic == '_');
    let input = ic.to_string();
    let at_once: Option<u32> = kani::any();

    let foreach_a = StepPrimitive::ForEach {
        variable: variable_a,
        input: input.clone(),
        at_once,
        body: vec![],
    };
    let foreach_b = StepPrimitive::ForEach {
        variable: variable_b,
        input,
        at_once,
        body: vec![],
    };

    let mut ha = blake3::Hasher::new();
    let mut hb = blake3::Hasher::new();
    super::super::digest_step_primitive(&mut ha, &foreach_a);
    super::super::digest_step_primitive(&mut hb, &foreach_b);
    kani::assert(ha.finalize().as_bytes() != hb.finalize().as_bytes());
}
