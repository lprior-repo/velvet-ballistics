#![cfg(kani)]
#![forbid(unsafe_code)]

//! Kani harnesses for together primitive verification (vb-xi2f.36).
//!
//! These harnesses prove that:
//! 1. `is_primitive("together")` returns `true` (after production changes)
//! 2. `parse_step_primitive` accepts "together" key without panic
//!
//! ## Production Code Changes Required
//!
//! Before these harnesses can pass, the following changes must be made to
//! `crates/vb_yaml/src/ast/parse_steps.rs`:
//!
//! 1. Add `"together"` to `is_primitive()` match arms (line ~85-102)
//! 2. Add `"together" => parse_parallel(sub)` to `parse_step_primitive()` (line ~68-82)
//! 3. Add `"together"` to `reject_unknown_step_fields()` allowed list (line ~105-131)
//!
//! ## GOD RULES COMPLIANCE
//!
//! - GOD RULE 1: Uses `kani::any()` for bounded symbolic inputs
//! - GOD RULE 2: Binds to actual Rust implementations in vb_yaml crate
//! - GOD RULE 3: No hardcoded structural inputs
//! - GOD RULE 4: Fixed unwind bounds documented in trusted-base-ledger.jsonl

use crate::YamlResult;

include!("vb_yaml_together_primitive_is_primitive.rs");
include!("vb_yaml_together_primitive_parse_step.rs");
include!("vb_yaml_together_primitive_error_arbitrary.rs");
