// SPDX-License-Identifier: MIT
//
// ============================================================================
// Extern surface for step_offset Verus spec.
// ============================================================================
//
// STRONG PRODUCTION BINDING (GOD RULE 2 compliance)
//
// This file is the production-binding surface for the
// `step_offset.rs` Verus spec. It binds the spec to the production
// arithmetic primitive `StepIdx::checked_add` at
// `crates/vb_core/src/ids/mod.rs:303-308` and to the production
// `checked_step_offset` wrapper at
// `crates/vb_compile/src/mod_compile_lowering/part_12.rs:199-212`,
// via `#[path]` inclusion of the in-tree production mirror at
// `verification/verus/production_inner/step_offset_production.rs`.
//
// The pre-binding spec defined a shadow `SpecStepOffsetError` enum
// containing only a `StepIndexOutOfRange` variant, and proved
// arithmetic lemmas against that shadow type. That is a VACUUM
// proof: production never constructs `SpecStepOffsetError`.
//
// This binding replaces the shadow type with the production
// `CompileError::PrimitiveLoweringLimitExceeded` variant from
// `crates/vb_compile/src/mod_compile_errors/kind.rs:124` (which is
// what `checked_step_offset` actually constructs on overflow — see
// part_12.rs:206-211), and grounds the spec lemmas in the production
// `StepIdx::checked_add` contract via `assume_specification` bridges.
//
// ============================================================================
// BINDING LEDGER
// ============================================================================
//
//   - `StepIdx` (u16 newtype)         <- production_inner/step_offset_production.rs
//                                        (verbatim mirror of
//                                         crates/vb_core/src/ids/mod.rs:55)
//   - `StepIdx::new`                  <- production_inner/step_offset_production.rs
//                                        (mirror of
//                                         crates/vb_core/src/ids/mod.rs:21)
//   - `StepIdx::get`                  <- production_inner/step_offset_production.rs
//                                        (mirror of
//                                         crates/vb_core/src/ids/mod.rs:27)
//   - `StepIdx::checked_add`          <- production_inner/step_offset_production.rs
//                                        (mirror of
//                                         crates/vb_core/src/ids/mod.rs:303-308)
//   - `SpecCompileError`              <- production_inner/step_offset_production.rs
//                                        (mirror of
//                                         crates/vb_compile/src/mod_compile_errors/kind.rs:124)
//   - `checked_step_offset`           <- production_inner/step_offset_production.rs
//                                        (verbatim mirror of
//                                         crates/vb_compile/src/mod_compile_lowering/part_12.rs:199-212)
//
// ============================================================================
// TRUST BOUNDARY
// ============================================================================
//
// The production bodies of every fn in this file are NOT verified by
// Verus. Each exec fn in `production_inner/step_offset_production.rs`
// is `#[verifier::external]`, the contracts are attached via
// `assume_specification` in the companion spec file (`step_offset.rs`)
// state the production behavior the spec proofs discharge. Drift
// between the production mirror and the production source is reported
// as binding-debt item tracked outside Verus.
// ============================================================================

#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ---------------------------------------------------------------------------
// PRODUCTION INCLUSION via #[path]
// ---------------------------------------------------------------------------
//
// Direct `#[path]` inclusion of the verbatim production mirror at
// `production_inner/step_offset_production.rs`. The mirror declares
// `StepIdx`, `SpecCompileError`, and `checked_step_offset` exactly as
// the production source does (modulo the documented substitutions in
// the production_inner header). All production method bodies are
// marked `#[verifier::external]` inside the production_inner file so
// Verus skips body verification; the inclusion still validates Rust
// resolution (type names, field names, fn signatures, discriminant
// sets) at compile time. Any drift in the production impl surface
// breaks this Verus build.
#[path = "production_inner/step_offset_production.rs"]
pub mod production;

// Re-export the production types and functions so the companion spec
// file can reference them as `production::StepIdx`,
// `production::checked_step_offset`, etc.
pub use production::{checked_step_offset, SpecCompileError, StepIdx};

} // verus!