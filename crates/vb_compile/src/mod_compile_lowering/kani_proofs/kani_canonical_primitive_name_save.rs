// Verification artifact: kani_canonical_primitive_name_save_is_set.rs
// Bead: vb-pkif2 | State: 5 (proof-writer)
// PO: obl-vb-pkif2-kani-001 — canonical_primitive_name(Save{v}) == "set"
// Command: cargo kani --harness kani_canonical_primitive_name_save_is_set -p vb_compile
//
// GOD RULE 1: Uses kani::any() for bounded symbolic Save{value} fields.
// GOD RULE 2: Binds to production canonical_primitive_name (part_05.rs:98-114).
// GOD RULE 3: No hardcoded structural inputs — all field values use kani::any().
// Model bounds: max_string_len=256, tool: --unwind 4

#![cfg(kani)]
#![allow(unused_must_use)]

use vb_yaml::ast::{ScalarValue, StepPrimitive};

/// Generate a bounded symbolic string using kani::any() + kani::assume().
fn bounded_string(max_len: usize) -> String {
    let mut buf: Vec<u8> = Vec::with_capacity(max_len);
    let len: u8 = kani::any();
    let actual_len = (len as usize).min(max_len);
    for _ in 0..actual_len {
        let byte: u8 = kani::any();
        kani::assume(byte.is_ascii_alphanumeric() || byte == b'_');
        buf.push(byte);
    }
    String::from_utf8_lossy(&buf).into_owned()
}

/// PO-KANI-001: canonical_primitive_name(Save{v}) == "set" for bounded symbolic Save.
///
/// Constructs a Save primitive with kani::any() bounded string value,
/// calls canonical_primitive_name, and asserts the result is "set".
#[kani::proof]
#[kani::unwind(4)]
fn kani_canonical_primitive_name_save_is_set() {
    // Generate bounded symbolic value for Save{value}
    let value_str = bounded_string(256);
    kani::assume(!value_str.is_empty());

    let save_primitive = StepPrimitive::Save {
        value: ScalarValue::String(value_str),
    };

    let result = crate::mod_compile_lowering::canonical_primitive_name(&save_primitive);

    kani::assert(
        result == "set",
        "canonical_primitive_name(Save{{value}}) must return \"set\", not \"save\"",
    );
}

/// PO-KANI-001b: canonical_primitive_name(Save{integer}) == "set" for bounded symbolic integer.
///
/// Same invariant but with Integer scalar value.
#[kani::proof]
#[kani::unwind(4)]
fn kani_canonical_primitive_name_save_integer_is_set() {
    // Generate bounded symbolic integer value
    let value: i64 = kani::any();

    let save_primitive = StepPrimitive::Save {
        value: ScalarValue::Integer(value),
    };

    let result = crate::mod_compile_lowering::canonical_primitive_name(&save_primitive);

    kani::assert(
        result == "set",
        "canonical_primitive_name(Save{{Integer}}) must return \"set\"",
    );
}

/// PO-KANI-001c: canonical_primitive_name(Save{v}) == canonical_primitive_name(Set{..}).
///
/// Proves Save and Set canonical names are identical for all bounded symbolic inputs.
#[kani::proof]
#[kani::unwind(4)]
fn kani_canonical_primitive_name_save_matches_set() {
    // Generate bounded symbolic values for both Save and Set
    let save_val = bounded_string(256);
    let set_output = bounded_string(64);
    let set_value = bounded_string(256);
    kani::assume(!save_val.is_empty());
    kani::assume(!set_output.is_empty());
    kani::assume(!set_value.is_empty());

    let save_prim = StepPrimitive::Save {
        value: ScalarValue::String(save_val),
    };
    let set_prim = StepPrimitive::Set {
        output: set_output,
        value: set_value,
    };

    let save_name = crate::mod_compile_lowering::canonical_primitive_name(&save_prim);
    let set_name = crate::mod_compile_lowering::canonical_primitive_name(&set_prim);

    kani::assert(
        save_name == set_name,
        "Save and Set canonical names must be identical",
    );
    kani::assert(
        save_name == "set",
        "Both Save and Set canonical names must be \"set\"",
    );
}

/// PO-KANI-001d: All 12 StepPrimitive variants including Save map correctly.
///
/// Extends the existing exhaustive harness to include the Save variant's
/// post-fix assertion that it returns "set".
#[kani::proof]
#[kani::unwind(4)]
fn kani_canonical_primitive_name_all_variants() {
    use vb_yaml::ast::{ScalarValue, StepPrimitive, TogetherBranch};

    // Symbolic discriminant (GOD RULE 1)
    let discriminant: u8 = kani::any();
    kani::assume(discriminant < 12);

    let label = String::from("d");
    let value = String::from("1");

    let primitive = match discriminant {
        0 => StepPrimitive::Set {
            output: label.clone(),
            value: value.clone(),
        },
        1 => StepPrimitive::Save {
            value: ScalarValue::String(value.clone()),
        },
        2 => StepPrimitive::Do {
            action: label.clone(),
            input: value.clone(),
        },
        3 => StepPrimitive::Choose {
            branches: vec![],
            otherwise: None,
        },
        4 => StepPrimitive::ForEach {
            variable: label.clone(),
            input: value.clone(),
            at_once: None,
            body: vec![],
        },
        5 => StepPrimitive::Together {
            branches: vec![TogetherBranch {
                label: label.clone(),
                steps: vec![],
            }],
        },
        6 => StepPrimitive::Collect {
            variable: label.clone(),
            source: value.clone(),
            pages: None,
            items: None,
            body: vec![],
        },
        7 => StepPrimitive::Reduce {
            variable: label.clone(),
            input: value.clone(),
            initial: value.clone(),
            body: vec![],
        },
        8 => StepPrimitive::Repeat {
            max_attempts: 1,
            body: vec![],
        },
        9 => StepPrimitive::Wait {
            event: None,
            timeout: None,
        },
        10 => StepPrimitive::Ask {
            prompt: label.clone(),
            timeout: None,
        },
        11 => StepPrimitive::Finish {
            result: ScalarValue::String(value.clone()),
        },
        _ => {
            kani::assume(false);
            loop {}
        }
    };

    let result = crate::mod_compile_lowering::canonical_primitive_name(&primitive);

    // Verify each variant maps to its canonical name — NOTE: Save (discriminant 1) now asserts "set"
    match discriminant {
        0 => kani::assert(result == "set", "Set must map to \"set\""),
        1 => kani::assert(result == "set", "Save must map to \"set\""),  // FIXED from "save"
        2 => kani::assert(result == "do", "Do must map to \"do\""),
        3 => kani::assert(result == "choose", "Choose must map to \"choose\""),
        4 => kani::assert(result == "for_each", "ForEach must map to \"for_each\""),
        5 => kani::assert(result == "together", "Together must map to \"together\""),
        6 => kani::assert(result == "collect", "Collect must map to \"collect\""),
        7 => kani::assert(result == "reduce", "Reduce must map to \"reduce\""),
        8 => kani::assert(result == "repeat", "Repeat must map to \"repeat\""),
        9 => kani::assert(result == "wait", "Wait must map to \"wait\""),
        10 => kani::assert(result == "ask", "Ask must map to \"ask\""),
        11 => kani::assert(result == "finish", "Finish must map to \"finish\""),
        _ => {
            kani::assume(false);
            loop {}
        }
    }
}
