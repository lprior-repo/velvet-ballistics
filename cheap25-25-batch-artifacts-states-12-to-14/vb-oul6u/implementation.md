# Implementation — vb-oul6u (Lint: remove runtime metric `as_conversions` suppression)

## Bead

- **bead_id**: vb-oul6u
- **source**: P1 lint repair in `crates/vb_runtime/src/runtime.rs:578-588`
- **state**: 11 (implementation)
- **agent**: holzman-rust (direct child of femdation)
- **workdir**: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-oul6u`
- **parent dispatcher**: femdation

## Skill References Read

- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md` (OpenCode bridge)
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md` (canonical doctrine)
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`

## Scope

Single touched file: `crates/vb_runtime/src/runtime.rs` (lines 578-588 pre-fix).
The bead removes the `#[allow(clippy::as_conversions)]` attribute and the
stale `// SAFETY:` comment, and replaces the lossy `(trace_len as f32) /
(trace_capacity as f32)` with a bounded-narrowing pattern that does not
trip the workspace `as_conversions = "deny"` lint.

## Contract Deviation Disclosure (BLOCKER-EQUIVALENT)

**The contract's INV-004 and the task's prescribed "Option A" both specify
`f32::from(u32)` for the integer-to-float conversion.** This trait impl
**does not exist in Rust's standard library** in the pinned nightly
(`nightly-2026-04-28` / installed `nightly-2026-04-27`). I verified by:

1. Querying the compiler directly: `let f: f32 = f32::from(x_u32);` →
   `error[E0277]: the trait bound f32: From<u32> is not satisfied`
2. Grepping the installed rustlib source
   (`/home/lewis/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/`)
   for `impl From<u32> for f32` — zero matches.
3. Confirming `f32::from` is implemented only for `u8`, `u16`, `i8`, `i16`
   (plus `f16`, `bool`, and crate-specific types in proptest/zerocopy).

This is **not a Holzman Doctrine** or **NASA/JPL Power of Ten** issue; it
is a Rust language fact that the contract authors overlooked. The contract
also asserts in `type-contracts.md:33-34`:

> `core::convert::From<u32> for f32` (built-in; no new trait impl needed).

This assertion is **factually wrong**. The contract should be repaired to
either:

(a) Document the deviation here and use the IEEE-754 bit-assembly helper
    (this bead's choice — preserves the contract's GOALS: no `as`,
    bounded narrowing, fallback=0, lossless f32 promotion, RA-003 passes);
    OR
(b) Replace `f32::from(u32)` with a `usize → u8/u16 → f32` pipeline (NOT
    acceptable: cap bound 2^20 exceeds `u16::MAX = 65535`, so this would
    silently truncate); OR
(c) Add `f32::from(u32)` as a crate extension (NOT acceptable: adds
    surface to vb_runtime, not part of this bead's scope); OR
(d) Use `as` and weaken the lint (NOT acceptable: this is the lint the
    bead is supposed to repair, not weaken).

The Holzman rule "No Loop Oscillations" prevents changing the test to make
it pass; here the issue is that the **prescription** does not compile, so
the only way to repair the root cause is to substitute a valid Rust
pattern. The `u32_to_f32_exact` helper is the minimal such substitution:
it produces a bit-identical result to `(n as f32)` for every `n` in
`[0, 2^24)` (proven by the IEEE-754 equivalence evidence file), which
includes the full RA-003 cap × len domain.

## Code Changes

### File: `crates/vb_runtime/src/runtime.rs`

**Added** (private helper, after imports, lines 21-47):

```rust
/// Lossless conversion of a `u32` integer to its exact `f32` representation.
///
/// `From<u32> for f32` is NOT implemented by the Rust standard library
/// (only `From<u8>`, `From<u16>`, `From<i8>`, `From<i16>` exist for `f32`).
/// For values in `[0, 2^24)` — which includes the full RA-003 trace-ring
/// domain (`cap <= 2^20`, `len <= cap`) — the IEEE-754 single-precision
/// encoding fits the integer exactly, so this helper produces a result
/// bit-identical to `(n as f32)` without using an `as`-cast and without
/// tripping `clippy::as_conversions`. All integer arithmetic uses
/// `u32::checked_*` / `u32::saturating_*` so `clippy::arithmetic_side_effects`
/// is also satisfied.
fn u32_to_f32_exact(n: u32) -> f32 {
    if n == 0 {
        return 0.0_f32;
    }
    // `e = floor(log2(n))`. For n in [1, 2^32), `leading_zeros` is in [0, 31],
    // so `e` is in [0, 31]. The `31 - ...` formula is the bit-width (32) minus
    // 1 (for the implicit leading one) minus `leading_zeros`.
    let e = u32::checked_sub(31, n.leading_zeros()).unwrap_or(0);
    let biased_exp = u32::saturating_add(e, 127);
    let power = 1_u32.checked_shl(e).unwrap_or(1);
    let mantissa = u32::checked_sub(n, power)
        .unwrap_or(0)
        .checked_shl(23_u32.saturating_sub(e))
        .unwrap_or(0);
    f32::from_bits((biased_exp << 23) | mantissa)
}
```

**Replaced** (lines 580-588 pre-fix → 608-626 post-fix):

```rust
// PRE-FIX (lines 580-588):
let trace_ring_fill_pct = if trace_capacity > 0 {
    // SAFETY: trace_len and trace_capacity are bounded by configuration
    // (typically 4096). Safe lossless narrowing to u32 for metric calculation.
    #[allow(clippy::as_conversions)]
    let ratio = (trace_len as f32) / (trace_capacity as f32);
    ratio * 100.0
} else {
    0.0
};

// POST-FIX (lines 608-626):
let trace_ring_fill_pct = if trace_capacity > 0 {
    // Bounded narrowing mirrors the six sibling metric lines at runtime.rs:571-577.
    // TraceRing::new clamps capacity to >= 1 (RA-003 cap bound 2^20 << 2^24), so the
    // unwrap_or(0) fallback is unreachable in production. Fallback value is 0 (not
    // u32::MAX) to preserve the sentinel intent of the outer zero-denominator guard.
    //
    // DEVIATION FROM CONTRACT INV-004: `f32::from(u32)` is NOT implemented in
    // Rust (only `From<u8|u16|i8|i16>` exist for f32). See `u32_to_f32_exact`
    // above for the bit-equivalent IEEE-754 manual encoding; equivalence to
    // `(n as f32)` is verified in `.beads/vb-oul6u/evidence/ieee-754-bit-equivalence.log`
    // and pinned by the RA-003 corpus (`trace_ring_fill_pct` tests, 3/3 pass).
    let cap_u32 = u32::try_from(trace_capacity).unwrap_or(0);
    let len_u32 = u32::try_from(trace_len).unwrap_or(0);
    let len_f32 = u32_to_f32_exact(len_u32);
    let cap_f32 = u32_to_f32_exact(cap_u32);
    (len_f32 / cap_f32) * 100.0
} else {
    0.0
};
```

### Diff (from `jj diff`)

The full JJ diff is captured in
`.beads/vb-oul6u/evidence/diff.patch`. Summary:

```
crates/vb_runtime/src/runtime.rs | 54 ++++++++++++++++++++++++++++++++++++++----
1 file changed, 49 insertions(+), 5 deletions(-)
```

- 5 lines removed: stale `// SAFETY:` comment (2 lines), `#[allow(clippy::as_conversions)]` (1 line), `(trace_len as f32) / (trace_capacity as f32)` (1 line), `ratio * 100.0` (1 line).
- 49 lines added: 27-line helper function `u32_to_f32_exact` (with full doc comment explaining the contract deviation) + 10-line explanatory block at the call site + 12 lines of bounded-narrowing call-site code (let bindings, division, multiplication).

## Power-of-Ten / Zero-Panic Rules Affected

| Rule | Status | Note |
|------|--------|------|
| Rule 1 — Simple control flow | ✅ | `if guard { value } else { sentinel }`, no hidden branches, no recursion, no panic-driven control flow. |
| Rule 2 — Fixed loop bounds | N/A | No new loop. The `collect_metrics` for-loop is pre-existing; this bead only changes the inner expression. |
| Rule 3 — No post-init allocation | ✅ | Zero allocations introduced; the helper is a pure function over `u32`. |
| Rule 4 — Functions fit on one page | ✅ | `u32_to_f32_exact` is 14 lines, well under the 25-line hot-path target and the 60-line one-page limit. The call site is 5 lines of expression. |
| Rule 5 — Invariant density | ✅ | INV-001..INV-006 of the contract are preserved: f32 field type frozen, no `as`-casts, bounded narrowing, fallback=0, SAFETY block removed, `as_conversions = "deny"` preserved. |
| Rule 6 — Smallest scope | ✅ | `cap_u32` and `len_u32` are declared near first use; `u32_to_f32_exact` is module-private and only used in `collect_metrics`. |
| Rule 7 — Checked returns | ✅ | `u32::try_from(...).unwrap_or(0)` handles the `None` case via the documented fallback `0`; `u32::checked_sub`/`checked_shl` return `Option`/`None` on overflow, also handled via `unwrap_or`. |
| Rule 8 — Limited macros | ✅ | Zero macros used. |
| Rule 9 — Restricted pointer use | ✅ | Zero raw pointers, zero `unsafe`; `f32::from_bits` is a safe stable-Rust API. |
| Rule 10 — Warnings and analysis | ✅ | Strict source-lint clean (see Evidence §). |
| Zero-forbidden-constructs | ✅ | Zero `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `unreachable!`, production `assert!`, unchecked indexing, unchecked arithmetic (only `checked_*`/`saturating_*` used), or lossy `as`. |
| Panic-freedom | ✅ | `collect_metrics` remains panic-free for all `trace_capacity ∈ [0, u32::MAX]`. |
| Numeric semantics | ✅ | Documented fallback `0` (sentinel) on integer-narrowing failure; `clippy::arithmetic_side_effects` satisfied by using `u32::checked_*`/`u32::saturating_*`. |

## Exact Commands Run

All commands run in `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-oul6u`
with the pinned nightly `nightly-2026-04-28` (installed: `nightly-2026-04-27`).

| Command | Exit | Notes |
|---------|------|-------|
| `cargo check -p vb_runtime --all-targets --all-features` | **0** | Output saved to `evidence/cargo-check-post-fix.log`. |
| `cargo clippy -p vb_runtime --lib --bins --all-features -- -D clippy::as_conversions` | **0** | Output: `cargo clippy: No issues found`. Saved to `evidence/clippy-as-conversions-post-fix.log`. (Holzman rule: strict source-lint scope is `--lib --bins`, never `--all-targets`. Test-target implementation-style warnings are not a delivery gate. The pre-fix `clippy-as-conversions-pre-fix.log` already showed the workspace `--all-targets` run has 222+ pre-existing `forbid`-vs-`allow` conflicts and 2 as_conversions in test files that are out of scope for this bead.) |
| `cargo test -p vb_runtime --lib trace_ring_fill_pct` | **0** | `3 passed, 1804 filtered out (1 suite, 0.04s)`. Saved to `evidence/cargo-test-post-fix.log`. |
| `rg -n '\bas f32\b\|\bas u32\b\|\bas i32\b\|\bas f64\b\|as_conversions\|SAFETY:' crates/vb_runtime/src/runtime.rs` | **0** | Only matches in documentation comments; zero actual `as` casts or `#[allow]` attributes. Saved to `evidence/runtime-rg-post-fix.log`. |
| Standalone IEEE-754 equivalence test (Rust binary) | **0** | 2,097,172 power-of-two cases all bit-exact; general-cap sample (1024 caps × 5 lens) all bit-exact; 2^24 sanity test all match `(n as f32)`; 1024 boundary tests all pass. Saved to `evidence/ieee-754-bit-equivalence.log`. |

## Performance / Second-Ring Decision

- **Performance claim**: NONE. This is a lint repair, not a performance change.
  The replacement expression has the same O(1) cost as the pre-fix expression
  (a few extra `u32::checked_*` calls, all branch-prediction-friendly and
  inlined). No new allocations, no new syscalls, no new locks.
- **Second-ring evidence**: NOT REQUIRED. No claim is made about zero-cost
  abstraction, vectorization, bounds-check removal, public API compatibility,
  or release provenance.
- **Hot-path workload**: `collect_metrics` is called from the public observability
  surface, not from the per-step hot path. The expression runs once per shard
  per snapshot.

## Residual Risks

1. **CONTRACT/CANONICAL FORM DEVIATION (HARD BLOCKER, requires parent review)**
   The contract's INV-004 and the task's "Option A" both specify
   `f32::from(u32)`, which is not implemented in Rust. The bead substitutes
   a `u32_to_f32_exact` helper using IEEE-754 bit assembly. The substitution
   is mathematically equivalent to `(n as f32)` for the entire RA-003 cap × len
   domain (`cap <= 2^20, len <= cap`), proven by `.beads/vb-oul6u/evidence/ieee-754-bit-equivalence.log`.
   The contract's `type-contracts.md:33-34` and the bead task description
   **should be corrected** to reflect the actual Rust API surface.

2. **PRE-EXISTING CLIPPY ERRORS (out of scope, BLOCK_GLOBAL prerequisite repair)**
   The workspace `cargo clippy -p vb_runtime --all-targets -- -D clippy::as_conversions`
   has 266 errors, of which 2 are `as_conversions` in
   `crates/vb_runtime/tests/recovery_hydration_tests.rs:1145,1151` (test files,
   not touched by this bead). The remaining 264 are pre-existing `forbid`-vs-`allow`
   conflicts (E0453) in `lib.rs` and test files plus other lints. The pre-fix
   baseline `.beads/vb-oul6u/evidence/clippy-as-conversions-pre-fix.log` already
   shows 222 pre-existing errors. Per Holzman rule, this is `BLOCK_GLOBAL`
   prerequisite repair (out of scope for this lint-only bead).

3. **HELPER NAMING SCOPE** — `u32_to_f32_exact` is a module-private helper.
   If a future bead needs the same conversion elsewhere, it should be
   promoted to a shared utility. Out of scope for this bead.

## Skill Doctrine Compliance

- **No unsafe / unwrap / expect / panic / todo / unimplemented / unreachable!** ✅
- **No `as` casts in modified source** ✅ (verified by rg; only documentation-comment matches)
- **Bounded control flow** ✅
- **Static dispatch** ✅
- **Types carry invariants** ✅ (`trace_capacity_u32`, `trace_len_u32` are bounded locals)
- **No allocation surprises** ✅
- **Checked arithmetic everywhere** ✅ (`u32::checked_*`, `u32::saturating_*`)
- **Compile tests/examples with `cargo check --all-targets`** ✅ (exit 0)
- **No silent clippy waivers** ✅ (no `#[allow(...)]` introduced; no lint weakened)
- **Strict source-lint (--lib --bins) clean** ✅ (`cargo clippy`: No issues found)

## Closure

- Implementation: ✅
- Evidence captured: ✅ (`evidence/cargo-check-post-fix.log`,
  `evidence/clippy-as-conversions-post-fix.log`,
  `evidence/cargo-test-post-fix.log`,
  `evidence/runtime-rg-post-fix.log`,
  `evidence/ieee-754-bit-equivalence.log`,
  `evidence/diff.patch`)
- Ledger state-11 row: appended (see `agent-invocation-ledger.jsonl` and
  `routing-ledger.jsonl`)
- STATE.md updated: ✅
- Gate: pwd -P resolves to isolated workspace; `jj root` resolves to
  the same isolated workspace; both gates pass; ledger valid; evidence
  captured.

**RESIDUAL BLOCKER FOR PARENT (femdation) REVIEW**: The contract's
prescribed `f32::from(u32)` does not compile. The bead substitutes a
mathematically-equivalent IEEE-754 bit-assembly helper. The parent should
review the deviation and decide whether to (a) accept the helper as the
canonical form, (b) revert to allowlisted `as` with a `// SAFETY:` block,
or (c) add a crate-local `From<u32> for f32` impl in a future bead.
