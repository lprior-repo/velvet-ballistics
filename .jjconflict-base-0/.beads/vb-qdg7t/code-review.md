# Code Review — vb-qdg7t (Independent Re-review of Source Semantic Digest Repair)

**Bead**: vb-qdg7t
**Reviewer**: DeepSeek v4 Pro via OMP (independent; did not author)
**Workspace**: `/home/lewis/src/isoloated/velvet-ballistics-vb-qdg7t-digest-review`

## Frozen Source (reviewed)

| Role | SHA |
|------|-----|
| Frozen checkpoint (`@`) | `013189c5be8cac3dba616253db39465e0de3691b` |
| Parent (prior rejected candidate) | `1e7aeee2495f2cfca0e6c1e7719727dc7515ea0a` |
| Digest-framing origin (vb-a2l02) | `c789b1b35cb5d19a60cf2f786cbda5947109dcb1` |

Verified: `git rev-parse --show-toplevel` and `jj root` both resolve to this workspace;
working copy clean; reviewed tree == frozen SHA.

## Gate Result

**STATUS: APPROVED** (source-identity scope only).

Scope is the semantic source digest: `crates/vb_compile/src/mod_compile_lowering/part_05_digest.rs`
plus its imports/helpers/tests/canonical consumers. Runtime retry/executor paths are out of scope
(separate author, per task). The three digest blockers from prior review
`18ef1b9c2b91d5d3c8f497d07d262567daea10d9` (F-4 fail-open/saturation, F-5 defaults, F-6 size) are
resolved in the actual code, not just claimed.

---

## F-4 [HIGH] fail-open `0xFF` / length saturation — RESOLVED (verified)

**Prior finding**: `digest_author_value`/`TriggerAst`/`ScalarValue` unknown variants hashed constant
`0xFF`; `digest_string` silently saturated `len` to `u32::MAX` (dead error branch).

**Actual code now** (all confirmed by reading, not claim):

- `digest_string` (part_05_digest.rs:40-47) returns `Result<(), CompileErrors>`; length overflow
  propagates via `?` (`"string length exceeds u32 range"`, line 43). No saturation, no dead branch.
- `digest_len` (new, :50-56) fails closed on container-length overflow (`u32::try_from(...)?`).
- `digest_author_value` unknown variant → `Err` (`"unknown author value variant cannot be
  canonicalized"`, :157), no `0xFF`.
- `TriggerAst` unknown → `Err` (`"unknown trigger kind cannot be canonicalized"`, :211), no `0xFF`.
- `digest_scalar_value` (new, :344-362) unknown → `Err` (`"unknown scalar value variant cannot be
  canonicalized"`, :362), no `0xFF`.
- `digest_step_primitive` unknown → `Err` (:495), already fail-closed.

**Enum exhaustiveness verified**: `AuthorValue`, `TriggerAst`, `StepPrimitive`, `ScalarValue` are all
`#[non_exhaustive]` (`crates/vb_yaml/src/yaml_ast/types/step.rs:6,20,60,158`). The `_ => Err` arms are
therefore *required* (compiler will not force future-variant coverage) and now fail closed instead of
emitting a collision-prone sentinel. Correct.

**Length overflow**: all `u32::try_from(...).unwrap_or(u32::MAX)` sites in the prior file were
replaced by `digest_len`/`digest_string` `?`-propagation. Confirmed no remaining `unwrap_or(u32::MAX)`
in the production file (only `value.unwrap_or(1)`, a semantic default — see F-5).

## F-5 [MEDIUM] `at_once`/`pages`/`items` `None` vs `Some(1)` — RESOLVED with verified oracle

**Prior finding**: tests flipped `assert_eq!`→`assert_ne!` without an oracle; risk of breaking digest
stability if `None`/`Some(1)` lower to identical IR.

**Oracle verified against actual lowering**:

- `crates/vb_compile/src/mod_compile_lowering/part_02.rs:188` → `limit: at_once.unwrap_or(1)`.
- `crates/vb_compile/src/mod_compile_lowering/part_03.rs:218-219` →
  `limit: collect.pages.unwrap_or(1)`, `page_size: collect.items.unwrap_or(1)`.

`None` and `Some(1)` are the *same canonical IR* for all three fields, so `digest_optional_u32`
(:99-113) hashes `value.unwrap_or(1)` — `None`/`Some(1)` share a digest, `Some(0)` (and every other
explicit value) stays distinct. This matches master §59 digest stability. The `assert_eq!` flip in
`foreach_digest_tests.rs` is backed by this oracle, and the inverse boundary (`Some(0)` distinct from
both `None` and `Some(1)`) is explicitly tested
(`foreach_at_once_none_produces_different_step_digest_than_some0`,
`foreach_at_once_some1_produces_different_step_digest_than_some0`,
`foreach_at_once_zero_step_digest_differs_from_none/some1`, and collect equivalents in
`mod_compile_lowering/tests.rs`). Not a weakening.

**Distinct-meaning omission preserved**: optional *string* fields (`step_name`, `condition`, `with`,
`then`, retry delay, wait event/timeout, ask timeout) keep the `Some`/`None` discriminant
(`digest_optional_string`, :82-95), because `None` (absent) and `Some("")` (empty) are semantically
distinct authoring states. This matches the task constraint ("don't normalize semantically different
omission").

## F-6 [LOW] large cold-path functions — RESOLVED (justification added)

Bead-linked justification comment added at top of `part_05_digest.rs` (:5-13) naming `canonical_digest`
and `digest_step_primitive` as cold compile-time-only encoders whose length tracks `StepPrimitive`/
`AuthorValue` variant coverage, referencing vb-a2l02 + vb-wlnl4 and master §125. Legitimate
cold-path exception, not a fabricated blanket exemption.

## Canonical consumers (verified)

- **Compiled artifact hash derivation** — `part_01.rs:24,62`: `semantic_source_identity =
  canonical_digest(source)?` → `Some(...)`; `WorkflowParts.digest` zeroed during postcard encode, then
  set to `blake3::hash(encoded)`. IR-only paths set `None` (`part_05_ir.rs:26`, `part_08.rs:119`),
  preserved. No raw-source-hash shortcut; no fabricated compiled evidence.
- **schema3 typed historical v2 migration** — unchanged (`migration_compat.rs` delta is rustfmt +
  removal of a dead `proof_module_paths` test; no digest/migration semantic change).
- Public re-exports (`canonical_digest`, `compile_source`, `parse_workflow_source`) intact — the
  collision integration test compiled and ran against them.
- All `digest_string`/`digest_optional_string`/`digest_optional_u32`/`digest_author_value` call sites
  updated to propagate `Result` (`?`); no dangling `unused_must_use` (lib compiles clean, 496 tests).

## Forbidden-construct scan (production file)

`grep -n "unwrap\|expect\|panic\|unsafe\|dbg\|todo\|unimplemented"` → only `value.unwrap_or(1)`
(a semantic default, not an unwrap panic). No `unsafe`, no `unwrap`/`expect`/`panic`, no `dbg`/`todo`.

---

## Verdict

**APPROVED** for the source-identity scope. All three prior digest blockers are genuinely fixed in
the frozen code and consistent with the actual lowering and the non-exhaustive enum shapes. No
source-level blocker remains in this scope.

One test-suite issue (stale `digest_duplicate_parity`, 4 red tests) is out of this repair's diff and
reported separately in `test-review.md` T-1 — it does not originate from the F-4/F-5/F-6 changes.
