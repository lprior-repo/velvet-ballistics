# ARCH-DRIFT REPORT: `together_digest_kani.rs`

**File:** `crates/vb_compile/src/together_digest_kani.rs`
**Line count:** 310 (LIMIT: <300)
**Status:** VIOLATION — SPLIT REQUIRED

---

## VIOLATION 1: Hard Size Limit (310 > 300)

The file is 10 lines over the mandatory 300-line ceiling. This is not a rounding error — it is a structural failure. The entire file (lines 1–310) must be decomposed into multiple focused harness modules.

---

## VIOLATION 2: Primitive Obsession — Unwrapped `u8` for Domain Values

| Location | Code | Problem |
|---|---|---|
| Line 58 | `let depth: u8 = kani::any();` | `u8` is a raw integer. Should be `Depth` (bounded 0..=8) |
| Line 142 | `let branch_count: u8 = kani::any();` | Raw `u8`. Should be `BranchCount` (bounded 1..=4) |
| Line 147 | `let label_c: u8 = kani::any();` | Raw byte. Should be `LabelChar` (prevalidated a-z) |
| Line 154 | `let sub_label_c: u8 = kani::any();` | Same raw byte issue |
| Line 227 | `let count_a: u8 = kani::any();` | Raw u8 again |
| Line 228 | `let count_b: u8 = kani::any();` | Raw u8 again |

**Scott Wlaschin**: *"Make illegal states unrepresentable."* `u8` represents all 256 values. Only 0–8, 1–4, and 97–122 (a–z) are valid here. These domains are strictly smaller than their runtime type.

**Refactor**: Create `Depth`, `BranchCount`, `LabelChar` newtypes with `impl TryFrom<u8>` constructors that validate the invariant and return `None` for out-of-range values. Replace all raw `u8` usages.

---

## VIOLATION 3: Primitive Obsession — Raw `bool` for State Flags

| Location | Code | Problem |
|---|---|---|
| Line 152 | `let has_sub_step: bool = kani::any();` | Untyped boolean. No domain meaning. |

**Scott Wlaschin**: A bare `bool` carries no semantic weight. `has_sub_step` is a *decision* about whether a sub-step exists — this is a **union variant choice**, not a boolean.

**Refactor**: Encode as an `enum SubStepChoice { None, One(StepAst) }` or `Option<StepAst>`. Eliminate the boolean entirely.

---

## VIOLATION 4: `unwrap_or_default()` — Silent Error Absorption

| Location | Code | Problem |
|---|---|---|
| Line 149 | `String::from_utf8(vec![label_c]).unwrap_or_default()` | Silently drops invalid UTF-8. `kani::assume` does not guarantee valid UTF-8 at the Rust type level. |
| Line 156 | `String::from_utf8(vec![sub_label_c]).unwrap_or_default()` | Same silent drop. |

**Holzman Rust**: No `unwrap`, `expect`, or silent fallbacks. A `unwrap_or_default()` on `from_utf8` discards the error entirely, replacing it with an empty string — an invisible corruption that Kani will NOT catch because the `assume` constrains the byte range but Rust's type system does not carry that proof into the `String` constructor.

**Refactor**: Use `try_from` with explicit error handling. If `kani::assume` guarantees validity, use `unsafe { String::from_utf8_unchecked(...) }` with a comment citing the assumption, or better: use a checked constructor that proves the assumption holds.

---

## VIOLATION 5: `char::from(label_c)` — Unchecked Cast

| Location | Code | Problem |
|---|---|---|
| Line 239 | `String::from(char::from(label_c))` | `char::from(u8)` panics if `label_c > 127`. The `kani::assume(label_c >= b'a' && label_c <= b'z')` guards this at the Kani level, but the Rust code is not safe without the assumption. |

This is a latent panic path. `char::from(u8)` is only safe for 0x00..=0x7F. For bytes 97..=122 (ASCII lowercase letters), it is safe — but the code does not document this invariant.

**Refactor**: Use `char::from_u32(label_c as u32).unwrap()` with a comment explaining why the assume guarantees validity, or use `char::try_from(label_c)` with explicit handling.

---

## VIOLATION 6: Comments as Documentation Waste (Lines 280–310)

Lines 284–310 are **26 lines of commented-out shell commands** masquerading as documentation. This is dead weight. Evidence commands belong in:
- The trusted-base-ledger (already referenced)
- A separate `docs/kani-commands.md`
- The bead's artifact ledger

They do NOT belong in a production harness file.

**Refactor**: Remove the comment block (lines 280–310 entirely). The production dependency table (lines 10–17, 48–49, 136–137, 221–222) should be collapsed into a single 5-line header.

---

## Structural Decomposition Plan

Split into THREE files:

### 1. `together_digest_harnesses.rs` — Recursion bounded (105 lines)
Contains: `together_digest_sub_step_recursion_bounded_kani`
- Extract `Depth` newtype helper (inline module)
- Build the nesting tree in a helper fn

### 2. `together_digest_determinism.rs` — Determinism + sensitivity (174 lines)
Contains:
- `together_digest_step_deterministic_kani`
- `together_branch_count_produces_different_digest_kani`
- Extract `BranchCount`, `LabelChar`, `SubStepChoice` newtypes

### 3. `together_digest_common.rs` — Shared builders (current ~30 lines of overlap)
Contains: shared `StepAst` leaf builders, `LabelChar`, `BranchCount` types, `SubStepChoice`.

---

## Summary Table

| # | Violation | Severity | Rule |
|---|---|---|---|
| 1 | File is 310 lines (>300) | CRITICAL | Hard size limit |
| 2 | Raw `u8` for depth/branch counts | HIGH | Primitive obsession |
| 3 | Raw `bool` for sub-step choice | MEDIUM | Primitive obsession |
| 4 | `unwrap_or_default()` on UTF-8 | HIGH | No silent fallbacks |
| 5 | `char::from(u8)` unchecked cast | MEDIUM | No unchecked casts |
| 6 | 26 lines of dead comments | LOW | Documentation discipline |

---

## Action Required

1. **Split into 3 files** per decomposition plan above
2. **Create `Depth`, `BranchCount`, `LabelChar` newtypes** with `TryFrom<u8>` validation
3. **Eliminate `bool` for sub-step** → `Option<StepAst>`
4. **Replace `unwrap_or_default()`** with checked construction or documented unsafe
5. **Delete lines 280–310** (the comment block)

**ESTIMATED REDUCTION**: ~40 lines removed (dead comments + newtypes consolidated), ~30 lines moved to `_common.rs`. Net: 310 → ~240 across 3 files.
