// Verification artifact: kani_digest_save_prefix.rs
// Bead: vb-pkif2 | State: 5 (proof-writer)
// PO: obl-vb-pkif2-kani-002 — digest_step_primitive(Save{v}) starts with b"set" prefix
// Command: cargo kani --harness kani_digest_step_primitive_save_prefix_is_set -p vb_compile
//
// GOD RULE 1: Uses kani::any() for bounded symbolic Save{value}.
// GOD RULE 2: Binds to production digest_step_primitive (part_05.rs:374-381).
// GOD RULE 3: No hardcoded structural inputs — all fields use kani::any().
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

/// Hash a byte sequence and return the raw blake3 output bytes.
fn hash_bytes(data: &[&[u8]]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    for chunk in data {
        hasher.update(chunk);
    }
    hasher.finalize().into()
}

/// PO-KANI-002a: digest_step_primitive(Save{String}) starts with b"set" tag.
///
/// Constructs a Save with bounded symbolic string value, hashes it,
/// and verifies the initial tag prefix matches blake3(b"set" + value).
#[kani::proof]
#[kani::unwind(4)]
fn kani_digest_step_primitive_save_prefix_is_set() {
    let value_str = bounded_string(256);
    kani::assume(!value_str.is_empty());

    let save_primitive = StepPrimitive::Save {
        value: ScalarValue::String(value_str.clone()),
    };

    // Compute the expected digest: blake3(b"set" + value_str_bytes)
    let expected = hash_bytes(&[b"set", value_str.as_bytes()]);

    // Compute actual digest via production function
    let mut h1 = blake3::Hasher::new();
    crate::mod_compile_lowering::digest_step_primitive(&mut h1, &save_primitive).unwrap_or(());
    let actual = h1.finalize().into();

    // Verification artifact: kani_digest_save_prefix.rs
// Bead: vb-pkif2 | State: 5 (proof-writer)
// PO: obl-vb-pkif2-kani-002 — digest_step_primitive(Save{v}) starts with b"set" prefix
// Command: cargo kani --harness kani_digest_step_primitive_save_prefix_is_set -p vb_compile
//
// GOD RULE 1: Uses kani::any() for bounded symbolic Save{value}.
// GOD RULE 2: Binds to production digest_step_primitive (part_05.rs:374-381).
// GOD RULE 3: No hardcoded structural inputs — all fields use kani::any().
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

/// Hash a byte sequence and return the raw blake3 output bytes.
fn hash_bytes(data: &[&[u8]]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    for chunk in data {
        hasher.update(chunk);
    }
    hasher.finalize().into()
}

/// PO-KANI-002a: digest_step_primitive(Save{String}) starts with b"set" tag.
///
/// Constructs a Save with bounded symbolic string value, hashes it,
/// and verifies the initial tag prefix matches blake3(b"set" + value).
#[kani::proof]
#[kani::unwind(4)]
fn kani_digest_step_primitive_save_prefix_is_set() {
    let value_str = bounded_string(256);
    kani::assume(!value_str.is_empty());

    let save_primitive = StepPrimitive::Save {
        value: ScalarValue::String(value_str.clone()),
    };

    // Compute the expected digest: blake3(b"set" + value_str_bytes)
    let expected = hash_bytes(&[b"set", value_str.as_bytes()]);

    // Compute actual digest via production function
    let mut h1 = blake3::Hasher::new();
    crate::mod_compile_lowering::digest_step_primitive(&mut h1, &save_primitive).unwrap_or(());
    let actual = h1.finalize().into();

    kani::assert(actual == expected, "digest_step_primitive(Save{{String}}) must produce blake3(b\"set\" + value_bytes)");
}

/// PO-KANI-002b: digest_step_primitive(Save{Integer}) starts with b"set" tag.
#[kani::proof]
#[kani::unwind(4)]
fn kani_digest_step_primitive_save_integer_prefix_is_set() {
    let value: i64 = kani::any();

    let save_primitive = StepPrimitive::Save {
        value: ScalarValue::Integer(value),
    };

    // Expected: blake3(b"set" + i64::to_le_bytes())
    let expected = hash_bytes(&[b"set", &value.to_le_bytes()]);

    // Actual
    let mut h1 = blake3::Hasher::new();
    crate::mod_compile_lowering::digest_step_primitive(&mut h1, &save_primitive).unwrap_or(());
    let actual = h1.finalize().into();

     must produce blake3(b\"set\" + value_bytes)");
}

/// PO-KANI-002b: digest_step_primitive(Save{Integer}) starts with b"set" tag.
#[kani::proof]
#[kani::unwind(4)]
fn kani_digest_step_primitive_save_integer_prefix_is_set() {
    let value: i64 = kani::any();

    let save_primitive = StepPrimitive::Save {
        value: ScalarValue::Integer(value),
    };

    // Expected: blake3(b"set" + i64::to_le_bytes())
    let expected = hash_bytes(&[b"set", &value.to_le_bytes()]);

    // Actual
    let mut h1 = blake3::Hasher::new();
    crate::mod_compile_lowering::digest_step_primitive(&mut h1, &save_primitive).unwrap_or(());
    let actual = h1.finalize().into();

    kani::assert(actual == expected, "digest_step_primitive(Save{{Integer}}) must produce blake3(b\"set\" + i64_le_bytes)");
}

/// PO-KANI-002c: Save and Set digest both start with b"set" tag prefix.
///
/// Proves that both primitives use the same "set" byte prefix in their digests.
#[kani::proof]
#[kani::unwind(4)]
fn kani_digest_save_and_set_both_use_set_tag() {
    let save_val = bounded_string(256);
    let set_output = bounded_string(64);
    let set_value = bounded_string(256);
    kani::assume(!save_val.is_empty());
    kani::assume(!set_output.is_empty());
    kani::assume(!set_value.is_empty());

    // Compute expected Save digest
    let save_expected = hash_bytes(&[b"set", save_val.as_bytes()]);

    // Compute expected Set digest
    let set_expected = hash_bytes(&[b"set", set_value.as_bytes()]);

    // Compute actual Save digest
    let save_prim = StepPrimitive::Save {
        value: ScalarValue::String(save_val),
    };
    let mut h_save = blake3::Hasher::new();
    crate::mod_compile_lowering::digest_step_primitive(&mut h_save, &save_prim).unwrap_or(());
    let save_actual = h_save.finalize().into();

    // Compute actual Set digest
    let set_prim = StepPrimitive::Set {
        output: set_output,
        value: set_value,
    };
    let mut h_set = blake3::Hasher::new();
    crate::mod_compile_lowering::digest_step_primitive(&mut h_set, &set_prim).unwrap_or(());
    let set_actual = h_set.finalize().into();

    // Both must match their expected digests
     must produce blake3(b\"set\" + i64_le_bytes)");
}

/// PO-KANI-002c: Save and Set digest both start with b"set" tag prefix.
///
/// Proves that both primitives use the same "set" byte prefix in their digests.
#[kani::proof]
#[kani::unwind(4)]
fn kani_digest_save_and_set_both_use_set_tag() {
    let save_val = bounded_string(256);
    let set_output = bounded_string(64);
    let set_value = bounded_string(256);
    kani::assume(!save_val.is_empty());
    kani::assume(!set_output.is_empty());
    kani::assume(!set_value.is_empty());

    // Compute expected Save digest
    let save_expected = hash_bytes(&[b"set", save_val.as_bytes()]);

    // Compute expected Set digest
    let set_expected = hash_bytes(&[b"set", set_value.as_bytes()]);

    // Compute actual Save digest
    let save_prim = StepPrimitive::Save {
        value: ScalarValue::String(save_val),
    };
    let mut h_save = blake3::Hasher::new();
    crate::mod_compile_lowering::digest_step_primitive(&mut h_save, &save_prim).unwrap_or(());
    let save_actual = h_save.finalize().into();

    // Compute actual Set digest
    let set_prim = StepPrimitive::Set {
        output: set_output,
        value: set_value,
    };
    let mut h_set = blake3::Hasher::new();
    crate::mod_compile_lowering::digest_step_primitive(&mut h_set, &set_prim).unwrap_or(());
    let set_actual = h_set.finalize().into();

    // Both must match their expected digests
    kani::assert(save_actual == save_expected, "Save digest must be blake3(b\"set\" + value_bytes)");
    ");
    kani::assert(set_actual == set_expected, "Set digest must be blake3(b\"set\" + value_bytes)");
}

/// PO-KANI-002d: Save digest is deterministic (same input → same output).
#[kani::proof]
#[kani::unwind(4)]
fn kani_digest_save_deterministic() {
    let value: i64 = kani::any();

    let save_prim = StepPrimitive::Save {
        value: ScalarValue::Integer(value),
    };

    let mut h1 = blake3::Hasher::new();
    let mut h2 = blake3::Hasher::new();
    crate::mod_compile_lowering::digest_step_primitive(&mut h1, &save_prim).unwrap_or(());
    crate::mod_compile_lowering::digest_step_primitive(&mut h2, &save_prim).unwrap_or(());

    kani::assert(h1.finalize(, "assertion failed").as_bytes() == h2.finalize().as_bytes(), "Save digest must be deterministic");
}
