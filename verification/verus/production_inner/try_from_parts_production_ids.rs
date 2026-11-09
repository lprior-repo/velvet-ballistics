// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR: numeric-id and scalar types for
// `CompiledWorkflow::try_from_parts`
// ============================================================================
//
// Sibling of `try_from_parts_production.rs`: the `StepIdx` / `SlotIdx` /
// `ConstIdx` / `ActionId` / `ExprIdx` / `AccessorIdx` / `SymbolId` /
// `WorkflowDigest` / `FiniteF64` / `ConstValue` mirrors, verbatim copies
// of the production declarations in `crates/vb_core/src/ids/mod.rs` and
// `crates/vb_core/src/value.rs`, with minimal substitutions required to
// compile under `verus --crate-type=lib` without proc-macro crate
// registration:
//
//   1. The `#[derive(Debug, Clone, PartialEq, Eq, Serialize,
//      Deserialize)]` derive attributes are dropped from every
//      production struct / enum declaration. `serde::{Serialize,
//      Deserialize}` and `thiserror::Error` are proc-macro attributes
//      that Verus single-file mode cannot expand without extern crate
//      registration.
//
//   2. The newtype mirrors expose the inner field as `pub` so spec
//      proofs can name `.0`. Field NAME and TYPE match production
//      exactly.
//
// This file exists so the companion `extern_try_from_parts.rs` can
// include it under `#[path =
// "production_inner/try_from_parts_production_ids.rs"]` and the
// `production_try_from_parts` module can re-export the types. Any drift
// in the production field names or variant sets breaks this mirror at
// compile time, which is the explicit drift-detection mechanism for the
// try_from_parts binding.
//
// DRIFT POLICY: This file MUST be regenerated from
// `crates/vb_core/src/value.rs:72-180` and `crates/vb_core/src/ids/mod.rs`
// whenever production changes. The mirror is annotated at the top of
// every section with the originating production line range so
// regeneration is mechanical.
//
// This file is included by the companion extern file under module-
// level `#[verifier::external]` so every body is opaque to Verus. It
// compiles as plain Rust (no `verus!` block, no `vstd` import) and is
// checked by the Verus invocation purely for structural resolution
// and type well-formedness — Verus never reasons about the bodies.
#![forbid(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(unused_imports)]

// ---------------------------------------------------------------------------
// Local stub: `vb_core::ids::*` newtypes
// ---------------------------------------------------------------------------
//
// Production `crates/vb_core/src/ids/mod.rs` generates
// `pub struct $name(u16);` (private inner field) via the
// `numeric_id!(StepIdx, u16, get)` macro for each newtype. Mirrors
// below expose the inner field as `pub` so spec proofs can name `.0`.

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StepIdx(pub u16);

impl StepIdx {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u16 {
        self.0
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotIdx(pub u16);

impl SlotIdx {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u16 {
        self.0
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstIdx(pub u16);

impl ConstIdx {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActionId(pub u16);

impl ActionId {
    pub const fn new(value: u16) -> Self {
        Self(value)
    }
    pub const fn get(self) -> u16 {
        self.0
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExprIdx(pub u32);

impl ExprIdx {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccessorIdx(pub u32);

impl AccessorIdx {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolId(pub u32);

impl SymbolId {
    pub const fn new(value: u32) -> Self {
        Self(value)
    }
}

// ---------------------------------------------------------------------------
// Local stub: `vb_core::WorkflowDigest`
// ---------------------------------------------------------------------------
//
// Production `crates/vb_core/src/ids/mod.rs` declares `WorkflowDigest`
// as a `[u8; 32]` byte array via a macro. The mirror retains the
// same shape.

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkflowDigest(pub [u8; 32]);

impl WorkflowDigest {
    pub const fn new(value: [u8; 32]) -> Self {
        Self(value)
    }
}

// ---------------------------------------------------------------------------
// Local stub: `vb_core::value::FiniteF64`
// ---------------------------------------------------------------------------
//
// Production `FiniteF64` at `crates/vb_core/src/value.rs:72-104` is a
// `#[repr(transparent)]` newtype over `f64` that rejects NaN and
// infinities at construction. The mirror carries the same payload type
// and the manual `Eq` impl; the construction-time finiteness invariant
// lives in production `FiniteF64::new` and is not modelled here.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct FiniteF64(pub f64);

impl Eq for FiniteF64 {}

// ---------------------------------------------------------------------------
// Local stub: `vb_core::value::ConstValue`
// ---------------------------------------------------------------------------
//
// Production `crates/vb_core/src/value.rs:169-180` declares `ConstValue`
// with five scalar variants. The mirror carries all five in production
// order, plus a mirror-only `String` variant retained from the earlier
// subset (production has no `String` variant).

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstValue {
    Null,
    Bool(bool),
    I64(i64),
    F64(FiniteF64),
    Symbol(SymbolId),
    String(Box<str>),
}
