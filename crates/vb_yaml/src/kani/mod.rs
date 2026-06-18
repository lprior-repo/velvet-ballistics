//! Kani proof harnesses for vb_yaml diagnostic code verification.
//!
//! Obligations covered:
//! - PO-006 / PO-KANI-002: kani_yaml_error_code, kani_all_variants_registered
//! - PO-KANI-001: kani_checked_add
//! - PO-KANI-004: kani_panic_freedom
//! - P-EMPTY-BODY: kani_profile_replacement
//! - RPO-YAML-004: kani_vb_dzibx_dupkeys

#![forbid(unsafe_code)]

// PO-006: Vacuum-style legacy harness. Retained alongside the
// non-vacuum rewrite in kani_yaml_error_code.rs.
pub mod kani_yaml_error_code;

// PO-KANI-002: Every YamlError variant maps to a registered SymbolicCode.
pub mod kani_all_variants_registered;

// PO-KANI-001: checked_add sites never panic on overflow.
pub mod kani_checked_add;

// PO-KANI-004: parse_yaml_events / validate_yaml_profile never panic on
// bounded UTF-8 input.
pub mod kani_panic_freedom;

// P-EMPTY-BODY: production-bound replacement harnesses for the retired
// vb_yaml Verus mirror specs. These call profile validation and duplicate-key
// production APIs directly.
pub mod kani_profile_replacement;

// RPO-YAML-004: finite-symbol duplicate-key proof over the production
// reject_duplicate_keys implementation.
pub mod kani_vb_dzibx_dupkeys;
