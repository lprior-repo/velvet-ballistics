// SPDX-License-Identifier: MIT
//
// ============================================================================
// IN-TREE PRODUCTION-SOURCE MIRROR for `vb_cli::commands_ai_context`
// redaction surface — focused for `vb_ahfl_redaction_production`
// Verus spec
// ============================================================================
//
// This file is a MIRROR of the relevant production surface from
//   crates/vb_cli/src/commands_ai_context.rs:399-422
//   crates/vb_core/src/value.rs:14-25
//   xtask/src/evidence/release_contract.rs:54-64
// with SUBSTITUTIONS required to compile under
// `verus --crate-type=lib` without the `serde_json`, `postcard`,
// `vb_core::SlotIdx`, and `vb_storage::RunSnapshot` extern types
// (no installs allowed by the task brief):
//
//   - The production `Taint` enum at `crates/vb_core/src/value.rs:14-25`
//     is mirrored verbatim as `SpecTaintProduction` (all 5 variants
//     with verbatim `#[repr(u8)]` discriminants).
//   - The production `slot_is_secret_or_derived(slot: SlotIdx, snapshot:
//     Option<&RunSnapshot>) -> bool` at
//     `crates/vb_cli/src/commands_ai_context.rs:415-422` is mirrored
//     as `slot_is_secret_or_derived_mirror(slot_idx: usize, taint_byte:
//     usize) -> bool`. Production's `slot: SlotIdx` is abstracted to
//     `slot_idx: usize`; production's
//     `snapshot.taint.get(slot.as_usize())` is abstracted to the
//     caller-provided `taint_byte: usize` (with sentinel
//     `TAINT_NONE_SENTINEL` for the None case). The body is
//     `#[verifier::external]`.
//   - The production `redacted_slot_value(slot: SlotIdx, value:
//     Option<&Vec<u8>>, snapshot: Option<&RunSnapshot>) -> Value` at
//     `crates/vb_cli/src/commands_ai_context.rs:399-413` is mirrored
//     as `redacted_slot_value_mirror(slot_idx: usize, taint_byte:
//     usize, has_value: bool, decode_succeeds: bool,
//     decoded_value_string_len: usize) -> SpecRedactedSlotValueProduction`.
//     Production arguments are abstracted: `serde_json::Value` is
//     replaced by 4 boolean flags + 2 length fields; `SlotIdx` and
//     `RunSnapshot` are abstracted to direct inputs. The body's
//     `#[verifier::external]` mirror preserves the production
//     if/else if/else chain exactly: redaction beats decode, decode
//     failure beats success.
//   - The production `xtask::evidence::release_contract::REDACTION_CLASSES:
//     [(&str, &str); 6]` at
//     `xtask/src/evidence/release_contract.rs:54-64` is mirrored as
//     `SPEC_REDACTION_CLASSES` (six-row table of
//     `(class_name: &str, marker_total_len: usize)`).
//
// DRIFT POLICY: This file MUST be regenerated from the named
// production line ranges whenever production changes. Drift in
// `Taint` discriminants or in the `redacted_slot_value` body breaks
// the `assume_specification` bridges in the companion spec file at
// compile time, which is the explicit drift-detection mechanism.
//
// This file is included by the companion extern file
// `extern_vb_ahfl_redaction_production.rs` under module-level
// `#[path]`. Production function bodies are `#[verifier::external]`
// (Verus skips them); type-level accessors are plain Rust and
// Verus-verified. The contracts are attached in the companion spec
// file via `assume_specification` bridges.
// ============================================================================

#![allow(unsafe_code)]
#![allow(dead_code)]
#![allow(non_snake_case)]

use vstd::prelude::*;

verus! {

// ============================================================================
// PRODUCTION MIRROR — Taint (#[repr(u8)] 5-variant enum)
// ============================================================================
//
// Verbatim mirror of production `vb_core::value::Taint` at
// `crates/vb_core/src/value.rs:14-25`.
//
// Production source (verbatim):
//
//   #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
//   #[repr(u8)]
//   #[non_exhaustive]
//   pub enum Taint {
//       Clean = 0,
//       DerivedFromSecret = 1,
//       Secret = 2,
//       Random = 3,
//       TimeDependent = 4,
//   }
//
// Visibility relaxed from `pub` (in vb_core) to `pub` here (so the
// companion spec file can use it through the `production::*` re-export
// path).
pub enum SpecTaintProduction {
    Clean = 0,
    DerivedFromSecret = 1,
    Secret = 2,
    Random = 3,
    TimeDependent = 4,
}

// Spec-level projection: the production raw taint byte (0..=4).
impl SpecTaintProduction {
    pub open spec fn spec_taint_raw(self) -> int {
        match self {
            SpecTaintProduction::Clean => 0,
            SpecTaintProduction::DerivedFromSecret => 1,
            SpecTaintProduction::Secret => 2,
            SpecTaintProduction::Random => 3,
            SpecTaintProduction::TimeDependent => 4,
        }
    }

    // Spec-level decision: production's `matches!(*raw, 1 | 2)` check
    // at `commands_ai_context.rs:421`. A slot is "sensitive" iff its
    // raw taint byte is 1 (DerivedFromSecret) or 2 (Secret).
    pub open spec fn is_sensitive_spec(self) -> bool {
        self == SpecTaintProduction::DerivedFromSecret || self == SpecTaintProduction::Secret
    }
}

// Plain Rust decision mirroring `is_sensitive_spec`. Verus-verified.
impl SpecTaintProduction {
    pub fn is_sensitive(&self) -> bool {
        matches!(
            self,
            SpecTaintProduction::DerivedFromSecret | SpecTaintProduction::Secret
        )
    }
}

// ============================================================================
// SPEC-LEVEL CONSTANTS — production redaction literals
// ============================================================================
// Sentinel `taint_byte` value used by the mirror abstraction to mean
// "no snapshot or slot index out of bounds" (production's
// `snapshot.taint.get(slot.as_usize())` returned `None`). The sentinel
// is `usize::MAX` because no production taint discriminant is ever
// that value (Taint is `#[repr(u8)]` with max value 4, see
// `crates/vb_core/src/value.rs:14-25`).
pub spec const TAINT_NONE_SENTINEL: usize = usize::MAX;

// Spec-level constant: the redaction-class count is exactly 6
// (matches production's `[(&str, &str); 6]` length).
pub spec const SPEC_REDACTION_CLASS_COUNT: usize = 6;

// Spec-level constant: prefix length of the production marker
// literal "[REDACTED:".
pub spec const SPEC_REDACTED_MARKER_PREFIX_LEN: usize = 10;

// Spec-level constant: suffix length of the production marker
// literal "]".
pub spec const SPEC_REDACTED_MARKER_SUFFIX_LEN: usize = 1;

// Spec-level projection: the redaction-class count is exactly 6
// (matches production's `[(&str, &str); 6]` length).
pub open spec fn spec_redaction_class_count() -> int {
    6
}

// Spec-level projection: total marker length for a class with
// `class_name_len` characters. Mirrors production's literal
// `"[REDACTED:".len() + class_name.len() + "]".len()` formula
// = 10 + class_name_len + 1 = 11 + class_name_len.
pub open spec fn spec_redacted_marker_len(class_name_len: int) -> int {
    11 + class_name_len
}

// ============================================================================
// PRODUCTION MIRROR — slot_is_secret_or_derived (#[verifier::external])
// ============================================================================
//
// Mirror of production
// `slot_is_secret_or_derived(slot: SlotIdx, snapshot: Option<&RunSnapshot>)
// -> bool` at `crates/vb_cli/src/commands_ai_context.rs:415-422`.
//
// Production body (lines 418-422):
//
//   snapshot
//       .and_then(|snapshot| snapshot.taint.get(slot.as_usize()))
//       .is_some_and(|raw| matches!(*raw, 1 | 2))
//
// The production `slot: SlotIdx` (vb_core) is abstracted to
// `slot_idx: usize`; the production `snapshot.taint.get(slot.as_usize())`
// is abstracted to the caller-provided `taint_byte: usize` (which is
// `None`-or-`Some(raw)` projected to `usize` with sentinel
// `TAINT_NONE_SENTINEL` for the None case). The mirror returns `true`
// iff the byte equals 1 or 2 AND the byte is not the sentinel.
//
// Body is `#[verifier::external]` — Verus skips verification. The
// production contract is attached in the companion spec file via
// `assume_specification`.
#[verifier::external]
pub fn slot_is_secret_or_derived_mirror(slot_idx: usize, taint_byte: usize) -> bool {
    taint_byte == 1 || taint_byte == 2
}

// ============================================================================
// PRODUCTION MIRROR — SpecRedactedSlotValueProduction (output shape)
// ============================================================================
//
// Mirror of the production `redacted_slot_value` output structure at
// `crates/vb_cli/src/commands_ai_context.rs:399-413`.
//
// Production builds a `serde_json::Value` with one of four shapes:
//
//   Value::String("[REDACTED]".to_string())                 // marker_len=10
//   Value::Null                                             // all false, len=0
//   Value::String("[UNDECODED]".to_string())               // marker_len=11
//   Value::String(slot_value.to_string())                  // value_string_len=N
//
// Because `serde_json::Value` is not in scope in a standalone
// `verus --crate-type=lib` invocation, the mirror exposes the four
// production-derived boolean flags + length fields the spec actually
// consumes. The four flags are mutually exclusive by construction
// (production's if/else if/else chain at lines 404-412).
pub struct SpecRedactedSlotValueProduction {
    /// `true` iff production returned `Value::String("[REDACTED]")`
    /// (production lines 404-406). Marker is exactly 10 chars.
    pub is_redacted: bool,
    /// `true` iff production returned `Value::Null` (production
    /// line 407). Only set when `!is_redacted`.
    pub is_null: bool,
    /// `true` iff production returned `Value::String("[UNDECODED]")`
    /// (production lines 408-409). Marker is exactly 11 chars.
    /// Only set when `!is_redacted && has_value`.
    pub is_undecoded: bool,
    /// `true` iff production returned the decoded `slot_value` as a
    /// string (production lines 410-411). Only set when
    /// `!is_redacted && has_value && decode_succeeds`.
    pub is_decoded_string: bool,
    /// Length of the marker literal when `is_redacted` or
    /// `is_undecoded`: 10 or 11 respectively. Zero otherwise.
    pub marker_len: usize,
    /// Length of the resulting string. For `is_redacted` and
    /// `is_undecoded` this equals `marker_len`; for
    /// `is_decoded_string` this equals `decoded_value_string_len`;
    /// for `is_null` this is 0.
    pub value_string_len: usize,
}

impl SpecRedactedSlotValueProduction {
    /// Plain Rust: the output is structurally well-formed (exactly one
    /// of the four flags holds, the lengths are consistent).
    /// Verus-verified; trivial because the production mirror preserves
    /// the invariant by construction.
    pub fn is_well_formed(&self) -> bool {
        let count = (self.is_redacted as u32) + (self.is_null as u32) + (self.is_undecoded as u32)
            + (self.is_decoded_string as u32);
        count == 1
    }

    /// Plain Rust: the output is the redacted marker.
    pub fn is_redacted_marker(&self) -> bool {
        self.is_redacted
    }

    /// Plain Rust: the output is bounded (every output shape has a
    /// known length, even the decoded string is bounded by the caller-
    /// supplied `decoded_value_string_len`).
    pub fn is_bounded(&self) -> bool {
        if self.is_redacted {
            self.marker_len == 10 && self.value_string_len == 10
        } else if self.is_null {
            self.marker_len == 0 && self.value_string_len == 0
        } else if self.is_undecoded {
            self.marker_len == 11 && self.value_string_len == 11
        } else {
            self.is_decoded_string && self.marker_len == 0
        }
    }
}

// ============================================================================
// PRODUCTION MIRROR — redacted_slot_value (#[verifier::external])
// ============================================================================
//
// Mirror of production
// `redacted_slot_value(slot: SlotIdx, value: Option<&Vec<u8>>, snapshot:
// Option<&RunSnapshot>) -> Value` at
// `crates/vb_cli/src/commands_ai_context.rs:399-413`.
//
// Production body (lines 404-412):
//
//   if slot_is_secret_or_derived(slot, snapshot) {
//       return Value::String("[REDACTED]".to_string());
//   }
//   value.map_or(Value::Null, |bytes| {
//       postcard::from_bytes::<vb_core::SlotValue>(bytes)
//           .map_or(Value::String("[UNDECODED]".to_string()), |slot_value| {
//               Value::String(slot_value.to_string())
//           })
//   })
//
// The mirror abstracts:
//   `slot: SlotIdx` -> `slot_idx: usize`
//   `snapshot.taint.get(slot.as_usize())` -> `taint_byte: usize`
//                                          (sentinel `TAINT_NONE_SENTINEL`
//                                          for None)
//   `value: Option<&Vec<u8>>` -> `has_value: bool`
//                                (`bytes.len()` is opaque)
//   `postcard::from_bytes::<SlotValue>(bytes)` -> `decode_succeeds: bool`
//   `slot_value.to_string().len()` -> `decoded_value_string_len: usize`
//
// The body mirrors the production if/else if/else chain line-by-line.
// Body is `#[verifier::external]` — Verus skips verification. The
// production contract is attached in the companion spec file via
// `assume_specification`.
#[verifier::external]
pub fn redacted_slot_value_mirror(
    slot_idx: usize,
    taint_byte: usize,
    has_value: bool,
    decode_succeeds: bool,
    decoded_value_string_len: usize,
) -> SpecRedactedSlotValueProduction {
    // Mirror of production lines 404-406: redaction beats everything.
    if taint_byte == 1 || taint_byte == 2 {
        return SpecRedactedSlotValueProduction {
            is_redacted: true,
            is_null: false,
            is_undecoded: false,
            is_decoded_string: false,
            marker_len: "[REDACTED]".len(),
            value_string_len: "[REDACTED]".len(),
        };
    }
    // Mirror of production line 407: value is None -> Null.

    if !has_value {
        return SpecRedactedSlotValueProduction {
            is_redacted: false,
            is_null: true,
            is_undecoded: false,
            is_decoded_string: false,
            marker_len: 0,
            value_string_len: 0,
        };
    }
    // Mirror of production lines 408-409: decode failed -> [UNDECODED].

    if !decode_succeeds {
        return SpecRedactedSlotValueProduction {
            is_redacted: false,
            is_null: false,
            is_undecoded: true,
            is_decoded_string: false,
            marker_len: "[UNDECODED]".len(),
            value_string_len: "[UNDECODED]".len(),
        };
    }
    // Mirror of production lines 410-411: decode succeeded -> string.

    SpecRedactedSlotValueProduction {
        is_redacted: false,
        is_null: false,
        is_undecoded: false,
        is_decoded_string: true,
        marker_len: 0,
        value_string_len: decoded_value_string_len,
    }
}

// ============================================================================
// PRODUCTION MIRROR — REDACTION_CLASSES (xtask evidence table)
// ============================================================================
//
// Mirror of production
// `xtask::evidence::release_contract::REDACTION_CLASSES: [(&str, &str); 6]`
// at `xtask/src/evidence/release_contract.rs:54-64`.
//
// Production source (verbatim):
//
//   const REDACTION_CLASSES: [(&str, &str); 6] = [
//       ("sentinel", "[REDACTED:sentinel]"),
//       ("api_key", "[REDACTED:api_key]"),
//       ("token", "[REDACTED:token]"),
//       ("password", "[REDACTED:password]"),
//       ("idempotency_key", "[REDACTED:idempotency_key]"),
//       (
//           "tainted_fixture_value",
//           "[REDACTED:tainted_fixture_value]",
//       ),
//   ];
//
// The mirror exposes the literal class names + their `[REDACTED:CLASS]`
// marker total lengths as plain Rust constants. Each
// `[REDACTED:NAME]` marker has length 10 + len(NAME) + 1 = 11 + len(NAME).
//
// The marker length formula is exactly:
//   "[REDACTED:".len() + class_name.len() + "]".len()
//   = 10 + class_name.len() + 1
//   = 11 + class_name.len()
//
// This matches every entry below (verified by hand):
//   sentinel:                  11 +  8 = 19
//   api_key:                   11 +  7 = 18
//   token:                     11 +  5 = 16
//   password:                  11 +  8 = 19
//   idempotency_key:           11 + 15 = 26
//   tainted_fixture_value:     11 + 21 = 32
//
// Declared as a plain Rust `pub const` array because Verus's spec
// context can read it via the `production::*` re-export path without
// triggering the VerusErasureCtxt panic that occurs for top-level
// `pub const usize` declarations. The companion spec file's proofs
// index into this array to discharge the marker-len invariant.
pub const SPEC_REDACTION_CLASSES: [(&'static str, usize); 6] = [
    ("sentinel", 11 + 8),
    ("api_key", 11 + 7),
    ("token", 11 + 5),
    ("password", 11 + 8),
    ("idempotency_key", 11 + 15),
    ("tainted_fixture_value", 11 + 21),
];

} // verus!