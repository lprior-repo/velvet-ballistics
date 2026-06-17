// Verification artifact: kani_digest_foreach_body.rs
// PO: PO-K-FE-04 | Command: cargo kani --harness kani_foreach_body_set_content_reaches_hasher -p vb_compile
// Bead: vb-xi2f.28 | State: 5 (proof-writer)
// Note: blake3::Hasher triggers InlineAsm in Kani (BLOCKED_TOOLING).
// Harness compiles but verification fails at runtime due to Kani limitation.

#![cfg(kani)]
#![allow(unused_must_use)]

use vb_yaml::ast::{ScalarValue, StepAst, StepPrimitive};

/// H1: Changing Set body step output changes digest (1-char strings for Kani tractability).
#[kani::proof]
#[kani::unwind(6)]
fn kani_foreach_body_set_content_reaches_hasher() {
    let vc: char = kani::any();
    kani::assume(vc.is_ascii_alphanumeric() || vc == '_');
    let variable = vc.to_string();
    let ic: char = kani::any();
    kani::assume(ic.is_ascii_alphanumeric() || ic == '_');
    let input = ic.to_string();
    let at_once: Option<u32> = kani::any();

    let oc: char = kani::any();
    kani::assume(oc.is_ascii_alphanumeric() || oc == '_');
    let output_a = oc.to_string();
    kani::assume(!output_a.is_empty());
    let dc: char = kani::any();
    kani::assume(dc.is_ascii_alphanumeric() || dc == '_');
    let output_b = dc.to_string();
    kani::assume(!output_b.is_empty());
    kani::assume(output_a != output_b);

    let vsc: char = kani::any();
    kani::assume(vsc.is_ascii_alphanumeric() || vsc == '_');
    let value_s = vsc.to_string();

    let body_a = vec![StepAst {
        id: "s".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: output_a,
            value: value_s.clone(),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];
    let body_b = vec![StepAst {
        id: "s".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Set {
            output: output_b,
            value: value_s,
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];

    let foreach_a = StepPrimitive::ForEach {
        variable: variable.clone(),
        input: input.clone(),
        at_once,
        body: body_a,
    };
    let foreach_b = StepPrimitive::ForEach {
        variable,
        input,
        at_once,
        body: body_b,
    };

    let mut ha = blake3::Hasher::new();
    let mut hb = blake3::Hasher::new();
    super::super::digest_step_primitive(&mut ha, &foreach_a);
    super::super::digest_step_primitive(&mut hb, &foreach_b);
    kani::assert(ha.finalize().as_bytes() != hb.finalize().as_bytes());
}

/// H2: Changing Finish body step value changes digest.
#[kani::proof]
#[kani::unwind(6)]
fn kani_foreach_body_finish_content_reaches_hasher() {
    let vc: char = kani::any();
    kani::assume(vc.is_ascii_alphanumeric() || vc == '_');
    let variable = vc.to_string();
    let ic: char = kani::any();
    kani::assume(ic.is_ascii_alphanumeric() || ic == '_');
    let input = ic.to_string();
    let at_once: Option<u32> = kani::any();
    let result_a: i64 = kani::any();
    let result_b: i64 = kani::any();
    kani::assume(result_a != result_b);

    let body_a = vec![StepAst {
        id: "f".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Finish {
            result: ScalarValue::Integer(result_a),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];
    let body_b = vec![StepAst {
        id: "f".to_string(),
        name: None,
        condition: None,
        primitive: StepPrimitive::Finish {
            result: ScalarValue::Integer(result_b),
        },
        with: None,
        retry: None,
        on_error: None,
        then: None,
    }];

    let foreach_a = StepPrimitive::ForEach {
        variable: variable.clone(),
        input: input.clone(),
        at_once,
        body: body_a,
    };
    let foreach_b = StepPrimitive::ForEach {
        variable,
        input,
        at_once,
        body: body_b,
    };

    let mut ha = blake3::Hasher::new();
    let mut hb = blake3::Hasher::new();
    super::super::digest_step_primitive(&mut ha, &foreach_a);
    super::super::digest_step_primitive(&mut hb, &foreach_b);
    kani::assert(ha.finalize().as_bytes() != hb.finalize().as_bytes());
}

/// H3: Empty body vs one-step body changes digest.
#[kani::proof]
#[kani::unwind(6)]
fn kani_foreach_body_count_reaches_hasher() {
    let vc: char = kani::any();
    kani::assume(vc.is_ascii_alphanumeric() || vc == '_');
    let variable = vc.to_string();
    let ic: char = kani::any();
    kani::assume(ic.is_ascii_alphanumeric() || ic == '_');
    let input = ic.to_string();
    let at_once: Option<u32> = kani::any();

    let sc: char = kani::any();
    kani::assume(sc.is_ascii_alphanumeric() || sc == '_');
    let value_s = sc.to_string();

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

    let foreach_empty = StepPrimitive::ForEach {
        variable: variable.clone(),
        input: input.clone(),
        at_once,
        body: vec![],
    };
    let foreach_one = StepPrimitive::ForEach {
        variable,
        input,
        at_once,
        body: vec![step],
    };

    let mut ha = blake3::Hasher::new();
    let mut hb = blake3::Hasher::new();
    super::super::digest_step_primitive(&mut ha, &foreach_empty);
    super::super::digest_step_primitive(&mut hb, &foreach_one);
    kani::assert(ha.finalize().as_bytes() != hb.finalize().as_bytes());
}
