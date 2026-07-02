# Black Hat Review — vb-vzo9b

**Bead**: vb-vzo9b
**State**: 13 (black-hat-reviewer)
**Reviewer Skill**: black-hat-reviewer
**Reviewer Invocation**: `black-hat-reviewer-vb-vzo9b-state13-attempt1`
**Source checkout**: `/home/lewis/src/velvet-ballistics` (coord only — implementation in `velvet-ballistics-cheap25-vb-vzo9b`)
**Isolated workdir**: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b`
**Attempt**: 1
**Reviewed at**: 2026-07-01
**Touched file**: `fuzz/src/journal_target/readback.rs` (single file, lines 183-217 post-fix; pre-fix 183-203)

---

## Gate Result

**STATUS: APPROVED**

The fuzz body is a one-line replacement of a disjunctive `assert!` with a single
`assert_eq!` over a 11-field `RecoveryRuntimeSummary` derived `PartialEq + Eq +
Copy + Debug` struct. The diff is restricted to a single test file, all three
contract closure commands pass, and the source-lint gates from PO-003 are
clean. No behavior-affecting risk, no parity drift, no vacuum proof, no
production-code drift. Pre-existing concerns in non-touched files are
explicitly out of scope per `delivery-scope.jsonl` and the proof-coverage
matrix.

---

## PHASE 1: Contract & Bead Parity

| Requirement | Status | Evidence |
|-------------|--------|----------|
| **C-1** Exactness of pin (all 11 fields) | PASS | `fuzz/src/journal_target/readback.rs:196-209` constructs the expected `RecoveryRuntimeSummary` field-by-field covering all 11 fields (`run`, `first_seq`, `last_seq`, `workflow`, `steps_started`, `steps_succeeded`, `actions_scheduled`, `actions_resolved`, `suspensions`, `slots_written`, `terminal`) and asserts via `assert_eq!(run_summary, expected)` (line 209). The 11-field derivation matches the production struct at `crates/vb_storage/src/recovery/types.rs:547-570`. |
| **C-2** Sentinel rejection of `RunId::new(0)` in non-empty branch | PASS | The pre-fix disjunctive acceptance `assert!(summary.run == run \|\| summary.run == RunId::new(0))` is gone. Verified by `rg 'assert!\([^)]+\|\|' fuzz/src/journal_target/readback.rs` → no matches (PO-003 forbidden-pattern gate). |
| **C-3** Empty-events path unchanged | PASS | `fuzz/src/journal_target/readback.rs:212` still calls `assert_typed_recovery_error(error)` on the `Err` arm. No `assert_eq!` introduced in the empty branch. `cargo test -p vb_storage --lib summarize_recovery_events` (12 passed) covers the empty-events `RecoveryError::NoRecoveryData` rail transitively. |
| **C-4** Frame-seed call site unchanged | PASS | `fuzz/src/journal_target/readback.rs:214-216` is byte-identical pre/post fix (verified by `jj show`: the diff at line 196 only). `cargo test -p vb_storage --lib recover_runtime_frame_seed_from_events` (6 passed) confirms the function's contract. |
| **C-5** No production-code change | PASS | `jj show` confirms the diff is restricted to `fuzz/src/journal_target/readback.rs`. `cargo test -p vb_storage --lib summarize_recovery_events` (12 passed) and `cargo test -p vb_storage --lib recover_runtime_frame_seed_from_events` (6 passed) are green, proving production is unchanged. |
| **C-6** No new error variant, no new type, no `unsafe`, no `unwrap`/`expect` outside `assert_eq!` | PASS | `fuzz/Cargo.toml:18-19` `lints.clippy.unwrap_used = "deny"`, `expect_used = "deny"`, plus `lints.rust.unsafe_code = "forbid"`. Build passes (`cargo build --bin recovery_decode --manifest-path fuzz/Cargo.toml` exits 0). `rg '.unwrap()'` and `rg '.expect('` over `readback.rs` return no matches. The only `unwrap_or(0)` at line 185 is on `Option<u8>` (not `RecoveryResult`), pre-existing, and out of the C-6 scope. |
| **C-7** Closure commands | PASS | `cargo test -p vb_storage --lib summarize_recovery_events` → 12 passed. `cargo test -p vb_storage --lib recover_runtime_frame_seed_from_events` → 6 passed. `cargo build --bin recovery_decode --manifest-path fuzz/Cargo.toml` → `Finished dev profile` (exit 0). All three closure commands green. |
| **C-8** Forbidden patterns | PASS | All six forbidden-pattern rg gates return no matches: `assert!(..||..)`, `matches!(run_summary, ..)`, `let _summary`, `dbg!(run_summary...)`, `.unwrap()`, `.expect(`. Captured in `.beads/vb-vzo9b/evidence/state12/PO-003b-forbidden-pattern-grep.txt`. |
| **Production-binding discipline** | N/A | No Verus/Kani/Flux obligation in scope (VLD-004, VLD-005, VLD-006 all `not_applicable surface_absent`). `bash scripts/check-verus-production-binding.sh` is not required for this bead (no Verus spec in `verification/verus/` for this test-only repair). The `forbidden-scan.sh` repo-wide scan returns `forbidden-scan: PASS — no forbidden patterns found` (captured in `.beads/vb-vzo9b/evidence/state13/forbidden-scan-state13.txt`). |
| **Proof/test/source parity** | PASS | PO-001, PO-002 are cargo-test gates on the production functions `summarize_recovery_events` and `recover_runtime_frame_seed_from_events`. PO-003 is a compile + source-lint gate on the rewritten fuzz body. No Kani `cover!`, no copied harness model, no design-model-only evidence. Every behavior-affecting claim hits production source plus executable tests (note: no behavior-affecting claims for this test-only repair; all PO rows are `behavior_affecting: false`). |

---

## PHASE 2: Farley Engineering Rigor

| Function | Lines | Limit | Status | Notes |
|----------|-------|-------|--------|-------|
| `fuzz_recovery_decode` (`fuzz/src/journal_target/readback.rs:183-217`) | 35 | 25 (production), 50 (test/fuzz) | MARGINAL | 35 lines total. 12 of those lines are the `RecoveryRuntimeSummary { ... }` struct literal (mandated by C-1 for exact-pin over all 11 fields). The remaining 23 lines are the harness body. **For a fuzz harness (test code), 35 lines is acceptable**: the struct literal is data, not logic; there is no way to compress 11 fields into fewer lines without violating C-1's exact-pin requirement. The pre-fix function was 21 lines. The function is single-purpose, single-cyclomatic, no parameters, no I/O side-effects beyond the production-function call. Recommend no rewrite (would require splitting the struct literal into a helper, which adds indirection without simplifying the assertion). |
| `assert_typed_recovery_error` (`fuzz/src/journal_target/errors.rs:57-72`) | 16 | 25 | OK | Pre-existing, unchanged. The `_ => {}` fallback is a pre-existing concern (see PHASE 3 finding LOW-001) but out of blast radius. |
| `assert_typed_journal_error` (`fuzz/src/journal_target/errors.rs:3-55`) | 53 | 25 | OK (pre-existing) | Pre-existing, unchanged. Same `_ => {}` concern. Out of blast radius. |

| Constraint | Status | Notes |
|---|---|---|
| Function over 25 lines | MARGINAL | See `fuzz_recovery_decode` above; acceptable for fuzz harness with mandatory 11-field struct literal. |
| Function with more than 5 parameters | PASS | `fuzz_recovery_decode` takes 1 parameter (`data: &[u8]`). |
| Pure logic vs I/O separation | PASS | The fuzz harness is purely a verification body; no I/O side-effects. |
| Test asserts behavior (WHAT) not implementation (HOW) | PASS | The new `assert_eq!` asserts the **output** shape (11 fields of `RecoveryRuntimeSummary`), not the **implementation** (e.g., does not assert accumulator state, derive set, or `summarize_recovery_events` internal call graph). The `if !events.is_empty()` guard and the `match` on `Ok`/`Err` correctly route the two behavior paths. |

---

## PHASE 3: Holzman Rust (The Big 6)

| Rule | Status | Evidence |
|------|--------|----------|
| Zero `unsafe` | PASS | `rg '\\bunsafe\\b' fuzz/src/journal_target/readback.rs` → no matches. `fuzz/Cargo.toml:18` `unsafe_code = "forbid"`. |
| Zero `.unwrap()`/`.expect()` on production result types | PASS | `rg '\.unwrap\(\)' fuzz/src/journal_target/readback.rs` → no matches. `rg '\.expect\(' fuzz/src/journal_target/readback.rs` → no matches. The only `unwrap_or(0)` at line 185 is on `Option<u8>` (a guard on fuzz-input length), not a production result. |
| Zero `panic!`/`todo!`/`dbg!` | PASS | `rg '\\bpanic!\|\\btodo!\|\\bdbg!' fuzz/src/journal_target/readback.rs` → no matches. The only "panic" surface is the desired `assert_eq!` macro expansion, which is the contract C-1 mandated behavior. |
| Checked arithmetic | N/A | The touched body does no arithmetic on counters/sequences; the `u64::from(data.first().copied().unwrap_or(0))` is a checked conversion of `u8 → u64` (widening, infallible). |
| Make illegal states unrepresentable | PASS | The `match` arm on `Ok`/`Err` makes the two behavior paths explicit; the `if !events.is_empty()` guard prevents constructing the `expected` struct for the empty-events branch (where no `Ok(hydration)` exists). |
| Parse, don't validate | N/A | The fuzz body is a verification target, not a parser. The `data: &[u8]` input is fuzz-shaped and the harness exhaustively accepts every shape. |

---

## PHASE 4: Ruthless Simplicity & DDD

| Check | Status | Notes |
|-------|--------|-------|
| No Option-based state machines | PASS | No `Option` is used to encode workflow state. The `Option<u8>` in `data.first().copied().unwrap_or(0)` is a default-when-empty guard, not a state machine. |
| CUPID compliant (Composable / Unix-philosophy / Predictable / Idiomatic / Domain-based) | PASS | **Composable**: the `fuzz_recovery_decode` is one of 4 fuzz entry points in `readback.rs`; the `assert_typed_recovery_error` helper is composed in for the `Err` arm. **Unix-philosophy**: does one thing — verify the fuzz payload shape. **Predictable**: deterministic — given `data` bytes, the events vector, the `Ok`/`Err` path, and the `assert_eq!` are all deterministic. **Idiomatic**: standard `match` arm, `if let` for the error sink, `assert_eq!` for exact-pin. **Domain-based**: uses `vb_storage::recovery::RecoveryRuntimeSummary` directly from the production domain. |
| No clever abstractions | PASS | The body is brutally simple: 11 fields, one `assert_eq!`, one error sink, one frame-seed call. No traits, no generics, no builders, no custom derives. |
| No boolean parameters | PASS | `fuzz_recovery_decode` takes 1 parameter (`data: &[u8]`), no booleans. |
| Newtypes respected | PASS | `RunId`, `EventSeq`, `WorkflowDigest`, `RecoveryRuntimeSummary` are all newtypes or newtype-encapsulated structs from the production domain. No raw `u64`/`u32` leaks into the domain logic. |

---

## PHASE 5: The Bitter Truth

The pre-fix body was a textbook example of "looks like a fuzz target but
silently accepts the wrong value." The post-fix body is the minimal,
structurally strongest possible replacement: a single `assert_eq!` over
the production `RecoveryRuntimeSummary` struct's full field set. The
11-field struct literal is data, not logic; you cannot shrink it without
losing the exact-pin guarantee that the contract C-1 demands. The harness
is 35 lines, 10 lines over the Farley 25-line limit, but the entire
delta from pre-fix is the 11-line `RecoveryRuntimeSummary { ... }` literal
+ the `assert_eq!` call. Splitting the literal into a helper would add
indirection without simplifying the assertion.

The body is **boring, predictable, and obvious** — exactly what a fuzz
harness should be. The contract C-1 `assert_eq!` shape is the strongest
possible test surface (full-struct equality via the existing
`PartialEq + Eq + Copy + Debug` derive set) and is more rigorous than
11 separate `assert!` calls because a future struct-field addition will
trip the type system (the new field is a compile error against the
explicit `expected` literal) rather than silently pass.

**YAGNI check**: the `expected` literal is fully expanded and uses
the existing `Some(digest)` constructor — no speculative future variants.

**Sniff test**: would a junior dev who is trying to be clever write this?
No. The cleanest possible implementation of C-1's exact-pin requirement
is what this body is. **Pass.**

---

## Findings (Ordered by Severity)

| Finding | Severity | File:Line | Status |
|---------|----------|-----------|--------|
| `fuzz_recovery_decode` is 35 lines, 10 over Farley 25-line limit | LOW | `fuzz/src/journal_target/readback.rs:183-217` | OPEN (deferred; structure is required by C-1; 12 lines are the 11-field struct literal) |
| `assert_typed_recovery_error` uses `_ => {}` catch-all fallback, silently accepting new error variants | LOW (pre-existing) | `fuzz/src/journal_target/errors.rs:70` | OPEN (pre-existing; out of blast radius; contract C-3 explicitly relies on this helper; consider `#![deny(unreachable_patterns)]` or `panic!` in the catch-all in a follow-on bead) |
| `assert_typed_journal_error` uses `_ => {}` catch-all fallback, silently accepting new error variants | LOW (pre-existing) | `fuzz/src/journal_target/errors.rs:53` | OPEN (pre-existing; out of blast radius; not used by the touched fuzz body) |
| `cargo clippy --bin recovery_decode --manifest-path fuzz/Cargo.toml` fails on 5 pre-existing clippy errors in non-touched files | DEFERRED_GLOBAL | `fuzz/src/expression_target.rs:257`, `fuzz/src/workflow_target/budget.rs:142`, `fuzz/src/workflow_target/collect.rs:87`, `fuzz/src/workflow_target/node_slots.rs:100`, `fuzz/src/ipc_target.rs:47` | DEFERRED_GLOBAL (pre-existing; not in blast radius; AGENTS.md "Tests must compile and run, but test clippy is not strict"; captured in `.beads/vb-vzo9b/evidence/02-postfix-clippy-recovery_decode.txt` and `.beads/vb-vzo9b/evidence/state12/PO-003-clippy-recovery_decode.txt`) |

No CRITICAL, HIGH, or MEDIUM findings. No bead-blocking defects. No
production-code defects. No parity drift. No vacuum proofs. No
behavior-affecting concerns.

---

## Quality Gates

| Gate | Result | Evidence |
|------|--------|----------|
| `cargo test -p vb_storage --lib summarize_recovery_events` | PASS | `test result: ok. 12 passed; 0 failed` (12 tests covering empty-events, multi-run rejection, multi-event counts, duplicate-action handling, etc.) |
| `cargo test -p vb_storage --lib recover_runtime_frame_seed_from_events` | PASS | `test result: ok. 6 passed; 0 failed` (6 tests covering empty-events, no-steps, asking/waiting step, pc reconstruction, dimensions+step states) |
| `cargo build --bin recovery_decode --manifest-path fuzz/Cargo.toml` | PASS | `Finished dev profile [unoptimized + debuginfo] target(s) in 0.07s, exit=0` |
| Forbidden-pattern rg gates (6 invocations over `fuzz/src/journal_target/readback.rs`) | PASS | All 6 return `rg exit=1` (no matches) — `assert!(..||..)`, `matches!(run_summary,..)`, `let _summary`, `dbg!(run_summary..)`, `.unwrap()`, `.expect(` |
| `cargo fmt --check -p vb_storage` | PASS | exit 0 |
| `cargo clippy -p vb_storage --lib --no-deps` | PASS | `Finished dev profile ... 3.90s`, no findings on `vb_storage` |
| `bash scripts/forbidden-scan.sh` | PASS | `forbidden-scan: PASS — no forbidden patterns found` (9 crates scanned) |
| `forbid(unsafe_code)` attribute present on touched fuzz crate | PASS | `fuzz/Cargo.toml:18` `lints.rust.unsafe_code = "forbid"` |
| Diff scope (single file) | PASS | `jj show` confirms only `fuzz/src/journal_target/readback.rs` is modified; production crates and other fuzz files are byte-identical pre/post fix |

---

## Attack Results

- **Reintroduce disjunctive acceptance**: rejected. `rg 'assert!(..||..)' fuzz/src/journal_target/readback.rs` → no matches. The post-fix body has a single `assert_eq!` covering all 11 fields.
- **Single-field assertion bypass**: rejected. `rg 'matches!(run_summary,..)' fuzz/src/journal_target/readback.rs` → no matches. The `assert_eq!` is over the full struct, not a single field.
- **Coverage-only `let _summary` regression**: rejected. `rg 'let _summary' fuzz/src/journal_target/readback.rs` → no matches.
- **`dbg!` silent failure**: rejected. `rg 'dbg!(run_summary..)' fuzz/src/journal_target/readback.rs` → no matches.
- **`unwrap`/`expect` on `RecoveryResult`**: rejected. `rg '\.unwrap\(\)'` and `rg '\.expect\('` over `readback.rs` → no matches. The only `unwrap_or(0)` is on `Option<u8>` (fuzz input length), not on a production result.
- **Production-code drift**: rejected. `jj show` confirms the diff is restricted to `fuzz/src/journal_target/readback.rs`. `cargo test` on the two production functions (12 + 6 passed) confirms production is unchanged.
- **Frame-seed call site drift**: rejected. The second `if let Err(error) = vb_storage::recovery::recover_runtime_frame_seed_from_events(&events)` call is byte-identical pre/post fix (`jj show` line range 214-216 unchanged).
- **Vacuum Verus proof**: rejected as N/A. No Verus obligation in scope (VLD-004 `not_applicable surface_absent`); no `verification/verus/` artifact for this bead. `bash scripts/check-verus-production-binding.sh` is not required because there is no Verus spec to bind.
- **Production-binding discipline**: PASS. No Verus spec, no `production_inner/` mirror, no shadow types. The fuzz body uses `vb_storage::recovery::RecoveryRuntimeSummary` directly via crate-root import — full production binding by definition.
- **Kani/Verus/Flux/loom/miri over-claim**: rejected as N/A. The six default-profile verifiers are all `not_applicable` with concrete SHA-256 evidence refs in `verifier-lane-decisions.jsonl` (VLD-004 through VLD-009). The defect is in test code; no Rust-local invariant to model; no concurrency; no `unsafe`.
- **Hardcoded single-shape Kani harness**: rejected as N/A. No Kani obligation in scope. The fuzz body is shape-deterministic by design (single `RunAccepted` event with `seq = EventSeq::new(1)`) — this is the contract C-1 fuzz payload, not a hardcoded Kani shape.
- **Test-clippy strictness** (`cargo clippy --bin recovery_decode --manifest-path fuzz/Cargo.toml`): classified as DEFERRED_GLOBAL. 5 pre-existing clippy errors in non-touched files (expression_target, workflow_target/budget, workflow_target/collect, workflow_target/node_slots, ipc_target). All pre-date this bead and are out of blast radius. AGENTS.md: "Tests must compile and run, but test clippy is not strict."

---

## Test-Design Audit

- **Test parity with martin-fowler-tests**: the cargo-test gate targets the production functions directly (`cargo test -p vb_storage --lib summarize_recovery_events`, `cargo test -p vb_storage --lib recover_runtime_frame_seed_from_events`). 12 + 6 unit tests are green, covering: empty-events, multi-event, multi-run rejection, overflow-seq, duplicate-action handling, completion/output mismatch, asking/waiting step, pc reconstruction, dimensions+step states. The fuzz body itself is a behavior-test target, not a "test of a test."
- **Asserts behavior (WHAT) not implementation (HOW)**: the `assert_eq!` asserts the production output shape (11 fields) without depending on `summarize_recovery_events`'s internal call graph. The `if !events.is_empty()` guard correctly routes the two behavior paths.
- **Determinism**: the fuzz body is deterministic given `data` bytes; the `digest` is `blake3::hash(data)`, `run` is `data[0]` (or 0), `seq` is `EventSeq::new(1)`. No random sources, no clocks, no I/O.
- **Mutation resistance**: a future struct-field addition to `RecoveryRuntimeSummary` would fail to compile (the new field is missing from the explicit `expected` literal) — this is the strongest possible mutation resistance and a strict improvement over the pre-fix disjunctive `assert!` which would silently drift.

---

## Verdict

**STATUS: APPROVED**

### Summary

The post-fix `fuzz_recovery_decode` body is the minimal, structurally
strongest replacement of the pre-fix disjunctive `assert!`. The single
`assert_eq!(run_summary, expected)` covers all 11 fields of
`RecoveryRuntimeSummary` simultaneously via the existing
`PartialEq + Eq + Copy + Debug` derive set, exactly as required by
contract C-1. The diff is restricted to `fuzz/src/journal_target/readback.rs`;
all three closure commands (`cargo test` x2, `cargo build` x1) are
green; all six forbidden-pattern rg gates return no matches; the
repo-wide `forbidden-scan.sh` returns PASS. The single LOW finding
(function length 35 lines, 10 over Farley limit) is structural and
defensible: the 12-line `RecoveryRuntimeSummary { ... }` literal is
mandated by C-1. The DEFERRED_GLOBAL clippy findings are pre-existing
in non-touched files and out of blast radius. The bead is
ready for state 14 (evidence packaging + truth-serum audit).

---

## Required Repair Actions

None. The bead is approved for state 14.

### Deferred observations (not blockers, addressed in follow-on beads)

1. **LOW**: `assert_typed_recovery_error` and `assert_typed_journal_error`
   use `_ => {}` catch-all fallbacks. If a new error variant is added to
   `RecoveryError` or `JournalError`, the fuzz harness will silently
   accept it. Consider adding `panic!("unmatched error variant: {:?}", _)` in
   the catch-all, or use `#[deny(unreachable_patterns)]` on the typed
   error enums so the fuzz build breaks when a new variant is added.
   Out of scope for vb-vzo9b; address in a follow-on fuzz-hardening bead.
2. **DEFERRED_GLOBAL**: 5 pre-existing clippy errors in non-touched fuzz
   files. Address in a follow-on fuzz-test-cleanup bead.
3. **LOW**: `fuzz_recovery_decode` is 35 lines. Cannot be shortened
   without violating C-1. Acceptable for a fuzz harness with mandatory
   11-field struct literal.
