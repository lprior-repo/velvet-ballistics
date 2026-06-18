#![forbid(unsafe_code)]

//! `P-EMPTY-BODY` / `RPO-YAML-001` replacement Kani harnesses for retired
//! `vb_yaml` Verus mirror specs.
//!
//! These harnesses call production APIs directly:
//! - `crate::reject_yaml_1_1_ambiguous_scalars`
//! `RPO-YAML-004` duplicate-key production behavior is covered by
//! `kani_vb_dzibx_dupkeys.rs` using a finite static key alphabet after the
//! prior arbitrary symbolic `String` attempt exceeded the local Kani budget.
//!
//! Bounds and assumptions are recorded in
//! `.beads/vb-dzibx/proof-evidence.md`.

use crate::{reject_yaml_1_1_ambiguous_scalars, YamlError};

const MAX_ASCII_SCALAR_BYTES: usize = 3;
const MAX_ASCII_BYTE: u8 = 0x7f;

fn push_symbolic_ascii_case(target: &mut String, lower: char, upper: char) {
    if kani::any::<bool>() {
        target.push(upper);
    } else {
        target.push(lower);
    }
}

fn symbolic_case_y() -> String {
    let mut word = String::new();
    push_symbolic_ascii_case(&mut word, 'y', 'Y');
    word
}

fn symbolic_case_n() -> String {
    let mut word = String::new();
    push_symbolic_ascii_case(&mut word, 'n', 'N');
    word
}

fn symbolic_case_yes() -> String {
    let mut word = String::new();
    push_symbolic_ascii_case(&mut word, 'y', 'Y');
    push_symbolic_ascii_case(&mut word, 'e', 'E');
    push_symbolic_ascii_case(&mut word, 's', 'S');
    word
}

fn symbolic_case_no() -> String {
    let mut word = String::new();
    push_symbolic_ascii_case(&mut word, 'n', 'N');
    push_symbolic_ascii_case(&mut word, 'o', 'O');
    word
}

fn symbolic_case_on() -> String {
    let mut word = String::new();
    push_symbolic_ascii_case(&mut word, 'o', 'O');
    push_symbolic_ascii_case(&mut word, 'n', 'N');
    word
}

fn symbolic_case_off() -> String {
    let mut word = String::new();
    push_symbolic_ascii_case(&mut word, 'o', 'O');
    push_symbolic_ascii_case(&mut word, 'f', 'F');
    push_symbolic_ascii_case(&mut word, 'f', 'F');
    word
}

fn assert_ambiguous_rejected(input: &str) {
    let result = reject_yaml_1_1_ambiguous_scalars(&[input]);
    kani::assert(
        matches!(result, Err(YamlError::AmbiguousScalar { .. })),
        "YAML 1.1 ambiguous scalar must be rejected through production API",
    );
}

fn bounded_ascii_string<const MAX_LEN: usize>() -> String {
    let len: usize = kani::any();
    kani::assume(len <= MAX_LEN);

    let mut value = String::new();
    for _ in 0..len {
        let byte: u8 = kani::any();
        kani::assume(byte <= MAX_ASCII_BYTE);
        value.push(char::from(byte));
    }
    value
}

/// `P-EMPTY-BODY`: symbolic finite proof over every ASCII case permutation of
/// the complete YAML 1.1 ambiguous set used by production.  The prior retired
/// mirror missed this mixed-case behavior; this harness calls production.
#[kani::proof]
fn p_empty_body_yaml_ambiguous_case_permutations_rejected() {
    assert_ambiguous_rejected(&symbolic_case_y());
    assert_ambiguous_rejected(&symbolic_case_n());
    assert_ambiguous_rejected(&symbolic_case_yes());
    assert_ambiguous_rejected(&symbolic_case_no());
    assert_ambiguous_rejected(&symbolic_case_on());
    assert_ambiguous_rejected(&symbolic_case_off());
}

/// `P-EMPTY-BODY`: direct production API is total and returns only the typed
/// ambiguity error or `Ok` for arbitrary bounded ASCII bytes of lengths covering
/// the ambiguous scalar vocabulary (`y`, `n`, `no`, `on`, `yes`, `off`).
#[kani::proof]
#[kani::unwind(4)]
fn p_empty_body_yaml_ambiguous_api_typed_for_bounded_ascii() {
    let scalar = bounded_ascii_string::<MAX_ASCII_SCALAR_BYTES>();
    let result = reject_yaml_1_1_ambiguous_scalars(&[scalar.as_str()]);

    kani::assert(
        matches!(result, Ok(()) | Err(YamlError::AmbiguousScalar { .. })),
        "bounded ASCII scalar classification returns only typed outcomes",
    );
}
