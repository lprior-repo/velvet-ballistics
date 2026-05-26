// Verification artifact: kani_digest_foreach_exhaustive.rs
// PO: PO-K-FE-09 | Command: cargo kani --harness kani_foreach_all_fields_hashed -p vb_compile
// Bead: vb-xi2f.28 | State: 5 (proof-writer)
// Model bounds: variable_max_len=16, input_max_len=32, body_max_steps=2, tool: --unwind 8

#![cfg(kani)]

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

/// H1: Varying all 4 fields simultaneously changes digest.
#[kani::proof]
#[kani::unwind(8)]
fn kani_foreach_all_fields_hashed() {
    let var_a = bounded_string(16);
    kani::assume(!var_a.is_empty());
    let var_b = bounded_string(16);
    kani::assume(!var_b.is_empty());
    kani::assume(var_a != var_b);

    let input_a = bounded_string(32);
    kani::assume(!input_a.is_empty());
    let input_b = bounded_string(32);
    kani::assume(!input_b.is_empty());
    kani::assume(input_a != input_b);

    let ao_a: Option<u32> = kani::any();
    let ao_b: Option<u32> = kani::any();
    kani::assume(!((ao_a.is_none() && ao_b == Some(1)) || (ao_b.is_none() && ao_a == Some(1))));
    kani::assume(ao_a != ao_b);

    let value_s = bounded_string(16);
    let step = StepAst {
        id: "s".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: "x".to_string(),
            value: value_s,
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    };

    let foreach_a = StepPrimitive::ForEach {
        variable: var_a,
        input: input_a,
        at_once: ao_a,
        body: vec![],
    };
    let foreach_b = StepPrimitive::ForEach {
        variable: var_b,
        input: input_b,
        at_once: ao_b,
        body: vec![step],
    };

    let mut ha = blake3::Hasher::new();
    let mut hb = blake3::Hasher::new();
    super::super::digest_step_primitive(&mut ha, &foreach_a);
    super::super::digest_step_primitive(&mut hb, &foreach_b);
    assert_ne!(ha.finalize().as_bytes(), hb.finalize().as_bytes());
}

/// H2: ForEach arm does not fall through to the catch-all.
#[kani::proof]
#[kani::unwind(3)]
fn kani_foreach_arm_not_fallthrough() {
    let foreach = StepPrimitive::ForEach {
        variable: "x".to_string(),
        input: "items".to_string(),
        at_once: Some(1),
        body: vec![],
    };

    let mut hasher = blake3::Hasher::new();
    super::super::digest_step_primitive(&mut hasher, &foreach);

    let mut name_hasher = blake3::Hasher::new();
    name_hasher.update(b"for_each");

    // After the ForEach fix, this should be assert_ne! because
    // the ForEach arm adds field hashing beyond just the name.
    assert_ne!(
        hasher.finalize().as_bytes(),
        name_hasher.finalize().as_bytes()
    );
}
