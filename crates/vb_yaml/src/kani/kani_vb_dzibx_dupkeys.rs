#![forbid(unsafe_code)]

//! `RPO-YAML-004`: bounded symbolic Kani proof for duplicate-key rejection.
//!
//! The harness avoids the previous symbolic `String` blowup by selecting keys
//! from a finite static alphabet with symbolic `u8` values.  It still calls the
//! production `reject_duplicate_keys` implementation directly and varies both
//! sequence length and key choices symbolically.

use crate::{YamlError, YamlResult, profile::reject_duplicate_keys};

fn symbolic_len_up_to_six() -> u8 {
    let raw: u8 = kani::any();
    match raw {
        0 => 0,
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 4,
        5 => 5,
        _ => 6,
    }
}

fn symbolic_key_symbol() -> u8 {
    let raw: u8 = kani::any();
    raw & 0b0000_0011
}

fn key_for_symbol(symbol: u8) -> &'static str {
    match symbol {
        0 => "a",
        1 => "b",
        2 => "c",
        _ => "d",
    }
}

fn has_duplicate_2(s0: u8, s1: u8) -> bool {
    s0 == s1
}

fn has_duplicate_3(s0: u8, s1: u8, s2: u8) -> bool {
    has_duplicate_2(s0, s1) || has_duplicate_2(s0, s2) || has_duplicate_2(s1, s2)
}

fn has_duplicate_4(s0: u8, s1: u8, s2: u8, s3: u8) -> bool {
    has_duplicate_3(s0, s1, s2)
        || has_duplicate_2(s0, s3)
        || has_duplicate_2(s1, s3)
        || has_duplicate_2(s2, s3)
}

fn has_duplicate_5(s0: u8, s1: u8, s2: u8, s3: u8, s4: u8) -> bool {
    has_duplicate_4(s0, s1, s2, s3)
        || has_duplicate_2(s0, s4)
        || has_duplicate_2(s1, s4)
        || has_duplicate_2(s2, s4)
        || has_duplicate_2(s3, s4)
}

fn has_duplicate_6(s0: u8, s1: u8, s2: u8, s3: u8, s4: u8, s5: u8) -> bool {
    has_duplicate_5(s0, s1, s2, s3, s4)
        || has_duplicate_2(s0, s5)
        || has_duplicate_2(s1, s5)
        || has_duplicate_2(s2, s5)
        || has_duplicate_2(s3, s5)
        || has_duplicate_2(s4, s5)
}

fn duplicate_exists_up_to_len(len: u8, s0: u8, s1: u8, s2: u8, s3: u8, s4: u8, s5: u8) -> bool {
    match len {
        0 | 1 => false,
        2 => has_duplicate_2(s0, s1),
        3 => has_duplicate_3(s0, s1, s2),
        4 => has_duplicate_4(s0, s1, s2, s3),
        5 => has_duplicate_5(s0, s1, s2, s3, s4),
        _ => has_duplicate_6(s0, s1, s2, s3, s4, s5),
    }
}

fn reject_for_len(
    len: u8,
    k0: &'static str,
    k1: &'static str,
    k2: &'static str,
    k3: &'static str,
    k4: &'static str,
    k5: &'static str,
) -> YamlResult<()> {
    match len {
        0 => reject_duplicate_keys(&[]),
        1 => {
            let keys = [k0];
            reject_duplicate_keys(&keys)
        }
        2 => {
            let keys = [k0, k1];
            reject_duplicate_keys(&keys)
        }
        3 => {
            let keys = [k0, k1, k2];
            reject_duplicate_keys(&keys)
        }
        4 => {
            let keys = [k0, k1, k2, k3];
            reject_duplicate_keys(&keys)
        }
        5 => {
            let keys = [k0, k1, k2, k3, k4];
            reject_duplicate_keys(&keys)
        }
        _ => {
            let keys = [k0, k1, k2, k3, k4, k5];
            reject_duplicate_keys(&keys)
        }
    }
}

/// `RPO-YAML-004`: for symbolic length `0..=6` and symbolic key choices from
/// a four-symbol static alphabet, production `reject_duplicate_keys` returns
/// `Err(DuplicateKey)` iff any duplicate symbol appears in the selected prefix.
#[kani::proof]
#[kani::unwind(8)]
fn vb_dzibx_yaml_duplicate_keys_bounded_symbols() {
    let len = symbolic_len_up_to_six();
    let s0 = symbolic_key_symbol();
    let s1 = symbolic_key_symbol();
    let s2 = symbolic_key_symbol();
    let s3 = symbolic_key_symbol();
    let s4 = symbolic_key_symbol();
    let s5 = symbolic_key_symbol();

    let duplicate_exists = duplicate_exists_up_to_len(len, s0, s1, s2, s3, s4, s5);

    kani::cover!(len == 0, "RPO-YAML-004 covers empty key sequence");
    kani::cover!(len == 6, "RPO-YAML-004 covers maximum bounded key sequence");
    kani::cover!(
        duplicate_exists,
        "RPO-YAML-004 covers duplicate key sequence"
    );
    kani::cover!(
        !duplicate_exists,
        "RPO-YAML-004 covers non-duplicate key sequence"
    );

    let result = reject_for_len(
        len,
        key_for_symbol(s0),
        key_for_symbol(s1),
        key_for_symbol(s2),
        key_for_symbol(s3),
        key_for_symbol(s4),
        key_for_symbol(s5),
    );

    let typed_result = matches!(&result, Ok(()) | Err(YamlError::DuplicateKey { .. }));
    kani::assert(
        typed_result,
        "RPO-YAML-004 production duplicate-key API returns only Ok or DuplicateKey",
    );

    let observed_duplicate = matches!(&result, Err(YamlError::DuplicateKey { .. }));
    kani::assert(
        observed_duplicate == duplicate_exists,
        "RPO-YAML-004 rejects iff a duplicate symbol exists in the bounded sequence",
    );
}
