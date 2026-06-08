# Plan: Fix `$attempt.number` Scope Restriction Gap

**Bead family:** `vb-scope-attempt` (new umbrella) + reopened `vb-xi2f.25`, `vb-xi2f.31`
**Bead kind:** bug / structural-foundation
**Severity:** P0 (silent contract drift; cold AST drops body steps; test suite claims coverage it does not have)
**Source of truth:** `velvet-ballistics-MASTER.md` (reserved-root table; $attempt.number line 3322)

---

## Problem statement (Round 4 confirmation)

1. `mod restrictions;` is **not** declared in `crates/vb_compile/src/lib.rs` — the 19 tests in `restrictions/tests/attempt_number_tests.rs` are dead code (cargo test never compiles them).
2. `StepKindAst::Repeat { max_attempts: u16 }` (`crates/vb_compile/src/ast/types.rs:173`) does not carry body steps. `parse_repeat` (`crates/vb_compile/src/ast/parse.rs:381-385`) drops the `steps:` field. Therefore the cold AST has no way to enforce, or even observe, that `$attempt.number` is only valid inside a `repeat` body.
3. Production path emits `UnknownReference` for every `$attempt.number`, regardless of context. The 7 negative tests in `restrictions/tests/attempt_number_tests.rs` accept `IllegalReference | UnknownReferenceRoot` as fallback matches, which is not a contract — it is a laundered escape hatch.
4. `CompileError::InvalidVariableScope` is **not** a variant of `CompileError` (`crates/vb_compile/src/mod_compile_errors/kind.rs:1-169`). The positive-side tests assert on a non-existent variant.
5. Two closed beads have laundered proof reviews:
   - `vb-xi2f.25` (P0: lower nested repeat body steps) closed 2026-06-03 with claim that nested repeat body lowers, but the cold AST never carried the body in the first place.
   - `vb-xi2f.31` (P1: digest covers repeat semantics) closed 2026-06-03. The Kani harness targets the **lowered** `vb_yaml::ast::StepPrimitive::Repeat { max_attempts, body }` shape, not the cold `StepKindAst::Repeat`. The cold-side correctness is unproven.
6. Structural pre-condition: `StepKindAst::Repeat` must carry its body steps for any scope check to be possible.

---

## Migration order (foundation first)

```
[Foundation]  Step 1 ─┐
             Step 2 ─┴─► Step 3 (re-plumb) ──┐
[Independents] Step 4 (declare mod)          │
              Step 5 (error variant)         │
[Bookkeeping]  Step 6 (beads)                │
[Verification] Step 7 (Kani)  ◄──────────────┴─ depends on Steps 1-5
```

Steps 1+2 are the structural foundation. Everything else either fixes a match site broken by Step 1 (Step 3), activates tests that can now pass (Step 4), adds the error contract the tests need (Step 5), or proves the result (Step 7). Step 6 is bookkeeping that can run any time after the plan is approved.

---

## Per-item plan

### Step 1 — Add `body: Vec<StepAst>` to `StepKindAst::Repeat`

| | |
|---|---|
| **File** | `crates/vb_compile/src/ast/types.rs:173` |
| **Defect** | Cold `StepKindAst::Repeat { max_attempts: u16 }` carries no body. Scope checks and nested-repeat lowering are structurally impossible. |
| **Fix** | Extend the variant to `Repeat { max_attempts: u16, body: Vec<StepAst> }`. Add a doc-comment naming the structural invariant: "the body is the only syntactic context in which `$attempt.number` is well-formed." |
| **Test impact** | Compile error at every existing destructuring site (6+ in production, 1 in the dead test file). Re-plumbed in Step 3 and Step 4. |
| **Acceptance** | `cargo build -p vb_compile` fails only at the documented match sites; no other type errors. |
| **Risk** | Low (additive field, no semantics change yet). The `#[non_exhaustive]` attribute on the enum does not protect against this. |
| **Hours** | 0.5 |
| **Bead** | `vb-scope-attempt.1` |

### Step 2 — Update `parse_repeat` to call `parse_body_steps`

| | |
|---|---|
| **File** | `crates/vb_compile/src/ast/parse.rs:381-385` |
| **Defect** | `parse_repeat` only reads `max_attempts` and discards the `steps:` mapping. |
| **Fix** | Extract `steps` via the existing `required_sequence` helper (line 52) and parse with a new `parse_body_steps` helper that returns `Vec<StepAst>`. The helper must thread the `AstMarks` and `index` for source-mark preservation. Mark shape: `expected: "a sequence of step mappings"`. |
| **Test impact** | Existing parser tests under `crates/vb_compile/src/ast/parse.rs:670-720` (u16 boundary tests) are unaffected. New positive tests added in Step 4. |
| **Acceptance** | A YAML doc with `repeat: { max_attempts: 3, steps: [...] }` parses into a `StepKindAst::Repeat` whose `body` length equals the source sequence length. Empty body (`steps: []`) is permitted. |
| **Risk** | Medium — `parse_step` calls back into `parse_step_kind` which calls back into `parse_repeat`. Recursion is bounded by `DepthLimit` (u16) so the call stack is safe. |
| **Hours** | 1.5 |
| **Bead** | `vb-scope-attempt.2` |

### Step 3 — Re-plumb all match sites that treat `StepKindAst::Repeat { .. }` as a leaf

| | |
|---|---|
| **Files** | `crates/vb_compile/src/compile/type_taint/steps.rs:78`<br>`crates/vb_compile/src/references.rs:118`<br>`crates/vb_compile/src/type_taint.rs:226`<br>`crates/vb_compile/src/control_flow.rs:125` |
| **Defect** | Each of these sites matches `StepKindAst::Repeat { .. }` and performs a leaf action, ignoring the body. Body steps are invisible to taint, reference collection, and control-flow reachability analysis. |
| **Fix** | At each site, after the leaf action, iterate `body` and recurse: `for body_step in body { process(body_step); }`. Where the helper signature is `(steps: &[StepAst], …)`, call it with `&body` for the new body. For `control_flow.rs:push_successors`, the outer repeat is a linear successor (next step) AND each body step is reachable from it — push body indices in declaration order. |
| **Test impact** | Existing 13 unit tests in `kani_digest_repeat.rs` use the lowered `vb_yaml::StepPrimitive` and are unaffected. The 19 attempt_number tests in `restrictions/tests/` (when activated in Step 4) now have bodies to walk. `references.rs` may surface new `UnknownReferenceName` / `IllegalReference` errors in body steps that were previously invisible — those must be diagnosed. |
| **Acceptance** | `cargo test -p vb_compile` passes with body steps walked in all four match sites. A new unit test under `compile/type_taint/steps.rs` proves that an `attempt.number` reference in a body step is recognized by the taint pass as a body-internal reference. |
| **Risk** | High. Recursion into body steps can re-introduce the very `UnknownReference` floods the production path already has. The fix must be paired with Step 5 (`InvalidVariableScope` variant) so the error taxonomy is correct for body-internal references. |
| **Hours** | 6.0 (1.5 per file + 1.5 integration test authoring) |
| **Bead** | `vb-scope-attempt.3` |

### Step 4 — Declare `mod restrictions;` in `vb_compile/src/lib.rs`

| | |
|---|---|
| **File** | `crates/vb_compile/src/lib.rs:14-26` (module block) |
| **Defect** | `restrictions.rs` is on disk but unreachable. The `mod tests { mod attempt_number_tests; }` inner block is dead. |
| **Fix** | Add `pub mod restrictions;` next to `mod references;` (line 23). Move the `#[cfg(test)] mod tests { mod attempt_number_tests; }` to the top of the module so it is the public `restrictions::tests::attempt_number_tests::*` surface. |
| **Test impact** | 19 tests in `restrictions/tests/attempt_number_tests.rs` begin compiling. The 12 positive-path tests will start to exercise the new `StepKindAst::Repeat { body, .. }` shape. The 7 negative-path tests will start to assert against `CompileError::InvalidVariableScope` (which does not exist yet — they will fail at compile time until Step 5 lands). |
| **Acceptance** | `cargo test -p vb_compile --lib restrictions::` runs 19 tests, of which 4 positive tests pass (the `B1` group, after Steps 1+2+3) and the 15 negative tests fail with `non-existent variant` compile error until Step 5 lands. |
| **Risk** | Low. Pure module-attribute edit. |
| **Hours** | 0.25 (edit) + 0.5 (run) = 0.75 |
| **Bead** | `vb-scope-attempt.4` |

### Step 5 — Add `InvalidVariableScope { reference, valid_contexts }` variant to `CompileError`

| | |
|---|---|
| **File** | `crates/vb_compile/src/mod_compile_errors/kind.rs:1-169` |
| **Defect** | The variant does not exist. The attempt_number tests assert against it. The production path needs a precise diagnostic that names the construct (`$attempt.number`) and the contexts in which it is legal (`repeat body` only). |
| **Fix** | Insert: <br>`#[error("reference {reference} is not valid in this scope; legal contexts: {valid_contexts:?}")]`<br>`InvalidVariableScope { reference: Box<str>, valid_contexts: Vec<&'static str> }` <br>Place it next to `IllegalReference` (line 132) for grouping. Add a `Debug`-derived helper `pub(crate) fn valid_contexts_to_str(…) -> String` if the `{:?}` format produces noisy output in user-facing messages. |
| **Test impact** | The 7 negative tests in `restrictions/tests/attempt_number_tests.rs` will start matching on this new variant instead of the `IllegalReference | UnknownReferenceRoot` fallback. Update test helpers (line 28-39 `parse_error`) to also unwrap the new variant. |
| **Acceptance** | `cargo test -p vb_compile --lib restrictions::` passes all 7 negative tests. The error message contains both the reference name and the valid context list. |
| **Risk** | Medium. Touches the public error enum. Existing `match` statements that destructure `CompileError` may need `..` to remain exhaustive — verify with `cargo build --all-features`. |
| **Hours** | 1.0 (variant + tests + exhaustive-match audit) |
| **Bead** | `vb-scope-attempt.5` |

### Step 6 — Open a tracking bead; reopen `vb-xi2f.25` and `vb-xi2f.31`

| | |
|---|---|
| **File** | n/a (bead ops) |
| **Defect** | Closed beads claim coverage that the code does not have. Round-4 audit is not surfaced in the bead graph. |
| **Fix** | (a) `bd create --title "P0: fix $attempt.number scope restriction gap" --priority 0 --label "compiler,restrictions,scope,master-doc" vb-scope-attempt`. Block it on Steps 1-5. (b) `bd reopen vb-xi2f.25 --reason "cold AST StepKindAst::Repeat dropped body steps; nested-repeat lowering was unproven. Blocked by vb-scope-attempt.3"`. (c) `bd reopen vb-xi2f.31 --reason "Kani digest harness targets vb_yaml lowered shape, not cold StepKindAst. Blocked by vb-scope-attempt.7"`. |
| **Test impact** | None (bead graph only). |
| **Acceptance** | `bd ready` shows the new tracking bead. `bd show vb-xi2f.25` and `bd show vb-xi2f.31` show status `open` with reopen reason. |
| **Risk** | Low. Bookkeeping only. Do not auto-close the reopened beads — they re-close when their narrow contracts are re-proven under the new foundation. |
| **Hours** | 0.5 |
| **Bead** | `vb-scope-attempt.6` (this step itself) |

### Step 7 — Add a Kani harness with `kani::any()` for the scope check

| | |
|---|---|
| **File** | new file `crates/vb_compile/src/kani_attempt_scope.rs` |
| **Defect** | No bounded model-checked proof that the scope check is panic-free for arbitrary body shapes. The Kani harness at `kani_digest_repeat.rs` only covers the lowered shape's digest, not the cold-AST scope decision. |
| **Fix** | Author `#[kani::proof] fn kani_attempt_scope_in_repeat_body_is_accepted` and `kani_attempt_scope_outside_repeat_body_is_rejected`. Use `kani::any::<StepKindAst>()` to construct a symbolic cold AST node, then assert: (a) the function-under-test returns `Ok` when the symbolic step is a `Repeat` whose body contains an `$attempt.number` reference; (b) returns `Err(InvalidVariableScope)` when the symbolic step is anything other than `Repeat` (or `Repeat` with an empty body that uses the reference outside the body). Bind to the production entry point (`crate::restrictions::check_step_scope` or the new public function introduced by Step 5). The harness MUST NOT hardcode `StepKindAst::Repeat { max_attempts: 0 }` per GOD RULE 1. |
| **Test impact** | New harness file. Requires `vb_compile` to have a public `restrictions::check_step_scope(&StepAst) -> Result<(), CompileError>` entry point. If absent, add it as part of Step 5. |
| **Acceptance** | `bash scripts/kani-list.sh vb_compile` enumerates the new harness under feature `kani-attempt-scope`. `cargo kani -p vb_compile --features kani-attempt-scope` produces a SUCCESS verdict for both harnesses. Raw output goes to `.evidence/kani/attempt-scope-*.log`. |
| **Risk** | High. Kani on `saphyr::Yaml` input is fragile; use a `StepKindAst` constructor that does not require YAML decoding. Bind to a thin scope-check function, not the whole compile pipeline. |
| **Hours** | 6.0 (harness + binding function + Kani run + evidence packaging) |
| **Bead** | `vb-scope-attempt.7` |

---

## Per-item summary table

| Step | Bead | Defect | Fix | Hours | Risk |
|---|---|---|---|---|---|
| 1 | `vb-scope-attempt.1` | `Repeat { max_attempts: u16 }` has no body | Add `body: Vec<StepAst>` | 0.5 | L |
| 2 | `vb-scope-attempt.2` | Parser drops `steps:` field | Call `parse_body_steps` in `parse_repeat` | 1.5 | M |
| 3 | `vb-scope-attempt.3` | 4 match sites treat Repeat as leaf | Recurse into body at each site | 6.0 | H |
| 4 | `vb-scope-attempt.4` | `mod restrictions;` not declared | Declare in `lib.rs:14-26` | 0.75 | L |
| 5 | `vb-scope-attempt.5` | `InvalidVariableScope` variant absent | Add to `CompileError` enum | 1.0 | M |
| 6 | `vb-scope-attempt.6` | Closed beads have laundered reviews | Reopen + new tracking bead | 0.5 | L |
| 7 | `vb-scope-attempt.7` | No Kani proof for cold-AST scope check | Add `kani::any()` harnesses | 6.0 | H |

---

## Cross-cutting tasks (not in the 7 steps, but required to ship)

| Task | Hours | Notes |
|---|---|---|
| Re-run `cargo test -p vb_compile` after each step; expect 19 new attempts in `restrictions::` once Step 4 lands | 1.0 | Distributed across the bead |
| `moon ci` green-gate (lint + build + test) | 1.0 | Per AGENTS.md, `moon ci` is canonical |
| Black-hat review of the reopen justification (GOD RULE 4: "no blind verification mutations") | 2.0 | Refinery skill, raw evidence required |
| Truth-serum evidence pack (Step 7 Kani logs + 19 attempt_number tests + step-by-step `cargo test` transcripts) | 2.0 | Evidence-packaging skill |
| Bead closeout + dolt push + git push (per landing-skill) | 0.5 | End-of-session |

---

## Total work-hour estimate

| Category | Hours |
|---|---|
| 7 implementation steps | 16.25 |
| Cross-cutting tasks | 6.5 |
| **Total** | **22.75** |
| Round-up (buffer for unanticipated exhaustive-match fallout) | **24.0** |

**Single-agent focus run:** ~3 working days.
**With femdation-style multi-bead dispatch** (Steps 1+2+4+5+6 in parallel lanes, Steps 3+7 sequential): ~1.5 working days wall-clock.

---

## Definition of done

A bead-deliverable for `vb-scope-attempt` is **done** when **all** of the following hold:

1. `cargo test -p vb_compile --lib restrictions::` runs and passes all 19 tests in `restrictions/tests/attempt_number_tests.rs` with the **primary** diagnostic being `CompileError::InvalidVariableScope` (not the `IllegalReference | UnknownReferenceRoot` fallback that was previously a laundered pass).
2. `cargo build -p vb_compile` and `cargo build -p vb_compile --all-features` both succeed. The cold AST variant reads `StepKindAst::Repeat { max_attempts: u16, body: Vec<StepAst> }`.
3. The 4 production match sites (`compile/type_taint/steps.rs:78`, `references.rs:118`, `type_taint.rs:226`, `control_flow.rs:125`) all recurse into `body` and behave correctly for arbitrary body shapes (proven by at least one unit test per site plus the 19 attempt_number tests).
4. The Kani harness in `kani_attempt_scope.rs` is registered under a feature flag (e.g. `kani-attempt-scope`), passes `cargo kani -p vb_compile --features kani-attempt-scope`, and binds to a production function — **not** a hardcoded AST shape (GOD RULE 1).
5. `moon ci` is green. Holzman gate passes. Source lint zero-tolerance gate passes.
6. Bead graph state:
   - `vb-scope-attempt` (umbrella) is `closed` with completion reason naming Steps 1-5 as completed and Step 7 evidence path.
   - `vb-scope-attempt.1` through `vb-scope-attempt.7` are all `closed`.
   - `vb-xi2f.25` and `vb-xi2f.31` are re-closed **only** after their narrow contracts are re-proven under the new cold AST shape (Kani digest harness for `vb-xi2f.31` must be re-pointed at the cold `StepKindAst`, or a parallel cold-AST digest harness must be added).
7. Truth-serum evidence pack exists at `.evidence/vb-scope-attempt/` containing:
   - Raw `cargo test -p vb_compile --lib restrictions::` output.
   - Raw `cargo kani …` log for `kani_attempt_scope.rs`.
   - Step-by-step diff of the 4 production match sites.
   - Bead reopen notes for `vb-xi2f.25` and `vb-xi2f.31`.
8. `git push` succeeds. `bd dolt push` succeeds. `git status` is clean. (Per landing-skill session-completion mandate.)

A bead-deliverable is **not done** if any of the following is true (GOD RULES):

- The Kani harness hardcodes a `StepKindAst::Repeat { max_attempts: 0 }` or any other fixed shape.
- The 7 negative tests still pass on the `IllegalReference | UnknownReferenceRoot` fallback branch without the `InvalidVariableScope` variant being the primary match.
- `vb-xi2f.25` or `vb-xi2f.31` is closed without a fresh Kani/proptest run on the cold AST shape.
- A production match site is altered to keep using `Repeat { .. }` (the `..` pattern) without recursing into the body.

---

## Bead-prefix reservation

`vb-scope-attempt` is the umbrella; `vb-scope-attempt.1` through `vb-scope-attempt.7` are the leaves. If additional sub-steps emerge during implementation (e.g. an unanticipated exhaustive-match fallout needs its own bead), reserve `vb-scope-attempt.8` onwards.

Reopened beads: `vb-xi2f.25`, `vb-xi2f.31`. They re-close under their original IDs; do not create replacement beads.
