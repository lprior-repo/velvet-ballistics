//! Kani proof harnesses for Section 16 diagnostic code verification.
//!
//! This module is gated behind `#[cfg(all(kani, feature = "kani-diagnostic-codes"))]`
//! so unrelated Kani lanes can list/codegen without compiling diagnostic-code
//! harnesses first.
//!
//! Obligations covered: PO-001, PO-002, PO-004, PO-005, PO-007, PO-008,
//! PO-009, PO-010, PO-011, PO-012, PO-013, PO-014

#![forbid(unsafe_code)]

pub mod kani_determinism;
pub mod kani_diagnostic_constructor;
pub mod kani_from_str_compat;
pub mod kani_is_supported_code;
pub mod kani_registry_bijection;
pub mod kani_registry_category;
pub mod kani_reverse_lookup;
pub mod kani_serde_roundtrip;
pub mod kani_symbolic_code_validation;
pub mod kani_zero_alloc;
