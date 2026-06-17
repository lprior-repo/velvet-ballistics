// Verification artifact: kani_digest_foreach_at_once.rs
// PO: PO-K-FE-02 | Command: cargo kani --harness kani_foreach_at_once_reaches_hasher -p vb_compile
// Bead: vb-xi2f.28 | State: 5 (proof-writer)

#![cfg(kani)]
#![allow(unused_must_use)]

use vb_yaml::ast::{StepAst, StepPrimitive};

fn fixed_string<const N: usize>() -> String {
    let bytes: [u8; N] = kani::any();
    let len: u8 = kani::any();
    let actual_len = (len as usize) % (N + 1);
    let mut s = String::with_capacity(N);
    for i in 0..N {
        if i < actual_len {
            kani::assume(bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_');
            s.push(bytes[i] as char);
        }
    }
    s
}

#[kani::proof]
#[kani::unwind(8)]
fn kani_foreach_at_once_reaches_hasher() {
    let variable = fixed_string::<8>();
    kani::assume(!variable.is_empty());
    let input = fixed_string::<8>();
    kani::assume(!input.is_empty());
    let at_once_a: Option<u32> = kani::any();
    let at_once_b: Option<u32> = kani::any();
    kani::assume(
        !((at_once_a.is_none() && at_once_b == Some(1))
            || (at_once_b.is_none() && at_once_a == Some(1))),
    );
    kani::assume(at_once_a != at_once_b);
    let body: Vec<StepAst> = vec![];

    let foreach_a = StepPrimitive::ForEach {
        variable: variable.clone(),
        input: input.clone(),
        at_once: at_once_a,
        body: body.clone(),
    };
    let foreach_b = StepPrimitive::ForEach {
        variable,
        input,
        at_once: at_once_b,
        body,
    };

    let mut ha = blake3::Hasher::new();
    let mut hb = blake3::Hasher::new();
    super::super::digest_step_primitive(&mut ha, &foreach_a);
    super::super::digest_step_primitive(&mut hb, &foreach_b);
    kani::assert(ha.finalize().as_bytes() != hb.finalize().as_bytes(), "assertion failed");
}
