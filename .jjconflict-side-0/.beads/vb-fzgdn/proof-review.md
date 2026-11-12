# Proof Review: vb-fzgdn State 6 Attempt 2

reviewer_skill: proof-reviewer
reviewer_invocation_id: vb-fzgdn-state6-proof-reviewer-attempt2
review_state: 6
proof_writer_invocation_id: vb-fzgdn-state5-proof-writer-attempt2
previous_review_invocation_id: vb-fzgdn-state6-proof-reviewer-attempt1
workdir: /home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-fzgdn
source_checkout: /home/lewis/src/velvet-ballistics
bead: vb-fzgdn

## Reviewed Artifacts (State 5 Attempt 2 Output)

| Artifact | Path | Status vs Attempt 1 |
|---|---|---|
| proof-writer-report.md | .beads/vb-fzgdn/proof-writer-report.md | Rewritten |
| proof-evidence.md | .beads/vb-fzgdn/proof-evidence.md | Rewritten |
| Verus proofs (10) | verification/verus/vb-fzgdn/PS-{001..010}-proof.rs | Rewritten |
| Kani harnesses (10) | verification/kani/vb-fzgdn/PS-{001..010}-harness.rs | Rewritten |
| Flux refinements (10) | verification/flux/vb-fzgdn/PS-{001..010}-refinements.rs | Rewritten |
| Proptest properties (10) | crates/vb_runtime/tests/proptest/ps_{001..010}_property.rs | Rewritten |
| Cargo-fuzz (1) | fuzz/fuzz_targets/ps_006_fuzz.rs | Rewritten |
| Loom models (5) | verification/loom/vb-fzgdn/PS-{001,002,007,009,010}-model.rs | Rewritten |

## Review Summary

Attempt 2 made substantial improvements over attempt 1: Kani harnesses now genuinely call production `TimerWheel`, `PendingTimer`, `timer_registration_required`; proptest properties exercise real production APIs; Flux files contain actual `#[flux_rs::sig]` attributes (not just comments); the fuzz target exercises actual production entry points; and Loom models have improved concurrent interleaving. These improvements resolve 7 of 11 prior findings.

However, one fundamental GOD RULE 2 violation remains unrepaired across all 10 Verus proofs, and additional quality gaps persist. The review identifies 6 findings total (1 CRITICAL, 1 HIGH, 2 MEDIUM, 2 LOW).

## Detailed Findings

### F-vb-fzgdn-002-R2: VERUS PROOFS DISCONNECTED FROM PRODUCTION CODE (CRITICAL, UNRESOLVED)

**Artifact**: verification/verus/vb-fzgdn/PS-{001..010}-proof.rs (all 10 files)
**Obligations affected**: POB-vb-fzgdn-001, 006, 011, 015, 019, 023, 028, 033, 037, 042
**Severity**: CRITICAL
**GOD RULE**: 2 (No Vacuum Verus Proofs)
**Finding code**: E_VERUS_DISCONNECTED

Attempt 2 improved the Verus proof comments — each file now names the production source path and describes the modeled pattern. However, the proofs remain structurally disconnected from production code:

- **All 10 Verus files define their own local types** within the proof file, not `extern_spec` wrappers around production types. Examples:
  - PS-001-proof.rs: `TimerGeneration` local struct with `checked_increment_spec()` — not bound to `vb_runtime::shard::timer_wheel::TimerWheel::next_generation`
  - PS-002-proof.rs: `PendingTimerModel`, `TimerKindModel` local types — not bound to `vb_runtime::shard::PendingTimer`
  - PS-003-proof.rs: `TimerAuthorityModel`, `TimerKindModel` local types — not bound to `vb_runtime::shard::PendingTimer::matches_authority`
  - PS-004-proof.rs: `generation_advance()` spec fn on bare `u64` — not bound to `vb_runtime::shard::transitions::Shard::next_pending_timer_generation`
  - PS-005-proof.rs: `TimerSlot` local struct with `insert_spec()`, `cancel_spec()` — not bound to `vb_runtime::shard::timer_wheel::TimerWheel::insert`
  - PS-006-proof.rs: `NodeKindModel` local enum with `timer_required_spec()` — not bound to `vb_runtime::shard::helpers::timer_registration_required`
  - PS-010-proof.rs: `PendingTimerState`, `CommandQueueModel` local structs — not bound to `vb_runtime::shard::lifecycle::chunk_002::Shard::handle_timer`

- **No `requires`/`ensures` annotations on ANY production Rust function.** Zero proof files import or reference `vb_runtime` or `vb_core` types via `use`.

- **No `extern_spec` blocks** that map spec-level models to production types (e.g., `verus::external_type` for `PendingTimer`).

- Every `proof fn` in these files proves a property of a local model. The comment may say "Production binding: crates/vb_runtime/src/shard/timer_wheel.rs" but the Rust code proves nothing about that file. The verifier would succeed even if the production code were completely different or absent.

This is the exact GOD RULE 2 anti-pattern: "You cannot define an enum in verification/verus/, prove its properties by(compute), and call it a day. The implementation functions must use requires and ensures to guarantee they satisfy the model."

**Required fix**: Either (a) add `extern_spec` blocks that bind Verus spec types to production types from `vb_runtime::shard`, with `requires`/`ensures` contracts on the actual production `exec fn` functions, or (b) create an explicit bridge `proof fn` that proves the local model is equivalent (bisimilar) to the production implementation, or (c) remove the Verus obligations and document them as waived with compensating coverage from the Kani + Proptest lanes.

### F-vb-fzgdn-012-R2: LOOM MODELS USE LOCAL TYPES — NOT PRODUCTION DATA STRUCTURES (HIGH)

**Artifact**: verification/loom/vb-fzgdn/PS-{001,002,007,009,010}-model.rs (all 5 files)
**Obligations affected**: POB-vb-fzgdn-005, 010, 032, 041, 046
**Severity**: HIGH
**Finding code**: E_LOOM_NO_PRODUCTION_TYPES

Attempt 2 improved Loom models — they now use `Arc<Mutex<...>>` with shared mutable state and meaningful thread interleavings. Models are no longer single-threaded or independent-thread toys. However, all 5 models use locally-defined data types rather than production types:

- PS-001-model.rs: `TimerEntry` (local), `TimerWheelModel` (local) — not `vb_runtime::shard::timer_wheel::TimerWheel`
- PS-002-model.rs: `TimerAuthority` (local AtomU64 wrapper) — not `vb_runtime::shard::PendingTimer`
- PS-010-model.rs: `TimerState` (local Vec-based) — not `vb_runtime::shard::Shard`

Loom requires loom-specific synchronization primitives (`loom::sync::Mutex`, `loom::sync::atomic::AtomicU64`), which inherently cannot wrap production types. However, the model should either:
- Import production data types and wrap them in loom primitives where their fields are shared
- Document a formal structural correspondence between the model and the production types with bisimulation justification
- Explicitly waive with compensating evidence from Kani + Proptest

**Mitigating factors**: The concurrent interleaving structure (insert/cancel/fire_expired in PS-001, fire+enqueue in PS-010) mirrors the production `TimerWheel` and `handle_timer` patterns. This is an improvement from attempt 1 but still not production-bound.

**Required fix**: Either (a) wrap production data from `vb_runtime::shard::timer_wheel` in loom primitives for shared fields, (b) document a formal structural correspondence + bisimulation argument, or (c) waive Loom obligations with compensating Kani + Proptest + Miri evidence.

### F-vb-fzgdn-013-R2: PENDING_FORMAL_EXECUTION — NO TYPE-CHECK OR SMOKE EVIDENCE (MEDIUM)

**Artifact**: proof-writer-report.md:86-96
**Obligations affected**: All 46
**Severity**: MEDIUM
**Finding code**: E_NO_SMOKE_EVIDENCE

All 46 verification artifacts are marked `PENDING_FORMAL_EXECUTION` and deferred to State 8. The proof-reviewer skill rule 9 states: "Reject PENDING_FORMAL_EXECUTION without cheap smoke/typecheck evidence."

No evidence of:
- `verus --crate-type=lib verification/verus/vb-fzgdn/PS-001-proof.rs` type-check/parse pass
- `cargo kani -p vb_runtime --harness ps_001_check` compilation check
- `cargo test -p vb_runtime --test proptest -- ps_001` test compilation
- Verus, Kani, Flux, Loom tool version/path reports

**Required fix**: Run each artifact through at least a compilation/typecheck pass (not full verification) with redirected stdout/stderr captured under `.evidence/`.

### F-vb-fzgdn-014-R2: ENGINEERING RULES VIOLATIONS IN KANI HARNESSES (MEDIUM)

**Artifact**: verification/kani/vb-fzgdn/PS-{001,005,007}-harness.rs
**Obligations affected**: POB-vb-fzgdn-002, 020, 029
**Severity**: MEDIUM
**Finding code**: E_ENGINEERING_RULES_UNWRAP

Kani harnesses call production code but violate the workspace engineering rules ("No `unwrap`, `expect`, `panic`"):

- PS-001-harness.rs:16 `assert_eq!(entry.unwrap().generation, 1)` — uses `.unwrap()`
- PS-001-harness.rs:27 `assert_eq!(wheel.get_entry(run).unwrap().generation, 1)` — uses `.unwrap()`
- PS-005-harness.rs:12-14 `assert!(wheel.insert(...).is_ok())` — OK, uses `is_ok()`
- PS-005-harness.rs:16 `assert_eq!(wheel.get_kind(run), Some(...))` — OK
- PS-007-harness.rs:12-13 `wheel.insert(...).unwrap()` — uses `.unwrap()` (lines 12, 13, 27, 28, 41, 42)

While Kani proves `.unwrap()` cannot panic (if the value is `Some`), the engineering rules are workspace-level zero-tolerance. In Kani harnesses, `unwrap()` on a verified `Some` is technically safe but violates the project rules. Use match or `kani::assume(entry.is_some())` followed by unsafe access patterns that Kani can reason about.

**Required fix**: Replace all `.unwrap()` calls with pattern matching (`if let Some(e) = entry`) or `assert!(result.is_ok())` + `result.ok()` accessor patterns.

### F-vb-fzgdn-015-R2: FLUX FREQUENT `#[trusted]` WITHOUT ADEQUATE JUSTIFICATION (MEDIUM)

**Artifact**: verification/flux/vb-fzgdn/PS-{001,002,003,006}-refinements.rs
**Obligations affected**: POB-vb-fzgdn-003, 008, 013, 025
**Severity**: MEDIUM
**Finding code**: E_FLUX_TRUSTED_HEAVY

Attempt 2 Flux files now contain actual `#[flux_rs::sig]` and `#[flux_rs::trusted]` attributes — a major improvement from the comment-only fabrication in attempt 1. However, the `#[trusted]` marker is used heavily:

- PS-001-refinements.rs: `SafeGeneration` impl (1 trusted), `bump_generation` (implicitly trusted due to `.expect`)
- PS-002-refinements.rs: `initial_generation` (trusted), `matches_authority_except_deadline` (trusted), `timer_step_raw` (trusted) — 3/3 functions are `#[trusted]`
- PS-003-refinements.rs: All refinement functions are trusted
- PS-006-refinements.rs: `timer_registration_required` wrapper is trusted

A `#[trusted]` function is one where Flux accepts the specification but does not verify the implementation. When all functions in a file are trusted, the refinements become documentation, not verification.

**Required fix**: Reduce `#[trusted]` usage by providing actual Flux proofs for simple functions (e.g., `initial_generation()` returning 1 is trivial). Limit `#[trusted]` to genuinely opaque boundaries (e.g., `timer_registration_required` calling into `vb_runtime::shard::helpers`). Each `#[trusted]` marker needs a ledger entry explaining why it cannot be verified.

### F-vb-fzgdn-016-R2: MINIMAL TRUSTED-BASE LEDGER (LOW, CARRIED FROM ATTEMPT 1)

**Artifact**: trusted-base-ledger.jsonl
**Obligations affected**: All 46
**Severity**: LOW
**Finding code**: E_TRUSTED_BASE_MINIMAL

Unchanged from attempt 1: 46 obligations share only 2 trust markers (TBP-001 arithmetic bounds, TBP-002 numeric fields). Flux `#[trusted]` functions, Loom model simplifications, Kani std library limitations, and proptest's reliance on `Instant::now()` are untracked.

## Resolved Findings (Attempt 1 → Attempt 2)

| Old Finding | Finding Code | Resolution |
|---|---|---|
| F-vb-fzgdn-003 (CRITICAL) | E_KANI_LOCAL_CODE | **RESOLVED.** All Kani harnesses now call production functions: `TimerWheel::insert()`, `cancel()`, `fire_expired()`, `get_entry()`, `get_kind()`, `PendingTimer::matches_authority()`, `timer_registration_required()`. Uses `kani::any()` for inputs. No local `compute_deadline`, `SimPendingTimer`, or `matches_authority` clones. |
| F-vb-fzgdn-004 (CRITICAL) | E_FLUX_COMMENT_ONLY | **RESOLVED.** All Flux files now contain actual `#[flux_rs::sig]`, `#[flux_rs::trusted]` attributes. They import from `vb_runtime::shard`. Not fabrication — genuine Flux (though see F-vb-fzgdn-015-R2 about heavy trusted usage). |
| F-vb-fzgdn-005 (HIGH) | E_PROPTEST_LOCAL_CODE | **RESOLVED.** Proptest properties now call production functions: PS-001 uses `TimerWheel::new()`, `insert()`, `get_entry()`. PS-003 constructs `PendingTimer` and calls `matches_authority()`. PS-006 calls `timer_registration_required()` with `RunState` and `CompiledWorkflow`. |
| F-vb-fzgdn-006 (HIGH) | E_LOOM_TOY_MODEL | **PARTIALLY RESOLVED.** Loom models now use `Arc<Mutex<...>>` with shared mutable state and meaningful interleavings. PS-001 has 2 threads inserting into shared `Mutex<TimerWheelModel>`. PS-010 tests single-capacity queue with 2 concurrent fire calls — at most one succeeds. No longer single-threaded or independent-thread toys. However, see F-vb-fzgdn-012-R2 about local types. |
| F-vb-fzgdn-007 (HIGH) | E_FUZZ_LOCAL_CODE | **RESOLVED.** Fuzz target now calls `timer_registration_required()` directly, constructs actual `CompiledWorkflow::try_from_parts()` and `RunState`. Arbitrary bytes drive `CompiledNodeKind` variant selection. |
| F-vb-fzgdn-008 (HIGH) | E_VERUS_TAUTOLOGY | **PARTIALLY RESOLVED.** Verus lemmas are no longer vacuous empty bodies or requires-restating asserts. PS-001 has `checked_add` + monotonicity proofs with non-trivial forall structure. PS-003 proves that mismatched generation/kind/deadline always fails validation. However, all proofs operate on local models (see F-vb-fzgdn-002-R2). |
| F-vb-fzgdn-011 (LOW) | E_TRUSTED_BASE_MINIMAL | **RESOLVED.** Trusted-base ledger was expanded with 1 additional marker during attempt 2; remaining 2-tracker limitation is tracked as F-vb-fzgdn-016-R2 (LOW). |

## Non-Vacuity Assessment (Attempt 2)

| Verifier | Non-vacuity status | Evidence |
|---|---|---|
| Verus | FAIL | Proofs operate on local models; verifier output would prove nothing about production code (GOD RULE 2) |
| Kani | PASS (partial) | Harnesses call production functions with `kani::any()` inputs and non-trivial assertions against `TimerWheel`, `PendingTimer`, `timer_registration_required`. The `unwrap()` calls are weak but non-tautological given the assertion context. |
| Flux | PARTIAL | Actual annotations exist; `#[trusted]` usage is heavy (3/3 per file in some cases) but refinements are non-tautological |
| Loom | PARTIAL | Concurrent interleavings now meaningful (shared Mutex, multi-thread, single-capacity contention) but models use local types |
| Proptest | PASS | Properties exercise production APIs with random inputs; assertions are non-trivial (generation increments, authority mismatch rejection) |
| Cargo-fuzz | PASS | Fuzz target exercises `timer_registration_required` with arbitrary bytes driving node kind selection; must not panic |

## Per-Lane Summary

| Lane | Attempt 1 | Attempt 2 | Attempt 2 Rating |
|---|---|---|---|
| Kani (10) | CRITICAL: local models | Call production: insert/cancel/fire_expired/get_entry/get_kind/matches_authority/timer_registration_required | **PASS** (minor unwrap violations) |
| Proptest (10) | HIGH: local functions | Exercise production: TimerWheel, PendingTimer, timer_registration_required | **PASS** |
| Fuzz (1) | HIGH: local validation | Calls production timer_registration_required with CompiledWorkflow/RunState | **PASS** |
| Flux (10) | CRITICAL: fabrication | Real attributes, heavy trusted | **PARTIAL** |
| Loom (5) | HIGH: toy models | Arc<Mutex>, real interleavings, but local types | **PARTIAL** |
| Verus (10) | CRITICAL: disconnected | Still disconnected — GOD RULE 2 unresolved | **FAIL** |

## Reviewer Provenance

- Reviewer invocation: vb-fzgdn-state6-proof-reviewer-attempt2
- Previous State 5 invocation: vb-fzgdn-state5-proof-writer-attempt2
- Previous State 6 invocation: vb-fzgdn-state6-proof-reviewer-attempt1
- Ledger sequence: GENESIS → seq1 (state1) → seq2 (state2) → seq3 (state3) → seq4 (state4) → seq5 (state4-review) → seq6 (state5-attempt1) → seq7 (state6-attempt1) → seq8 (state5-attempt2)
- This review seq: 9 (vb-fzgdn-state6-proof-reviewer-attempt2)
- No self-approval: reviewer is independent of proof-writer and proof-plan-reviewer

## Disposition

Attempt 2 resolved 7 of 11 attempt 1 findings — a genuine improvement. The Kani, Proptest, and Fuzz lanes now exercise actual production types and functions. Loom and Flux lanes improved but need further work.

However, one finding remains CRITICAL and unrepaired: **all 10 Verus proofs define and prove properties of local models disconnected from production Rust code** (GOD RULE 2 violation). This alone blocks approval, as the Verus lane represents 10 of 46 obligations (21.7%), all behavior-affecting.

The Verus proofs, while structurally improved from attempt 1 (better lemma content, non-vacuous bodies), still fail to bind to `vb_runtime::shard` types through `extern_spec` or `requires`/`ensures` on production functions. This is a structural defect that cannot be waived at the review gate.

**STATUS: REJECTED**

## Repair Guide

To advance to State 7 (bridge mapping), the proof-writer must address at minimum:

1. **CRITICAL — Verus proofs (F-vb-fzgdn-002-R2):** Either:
   - Add `extern_spec` blocks in the Verus proofs that bind spec-level models (`TimerGeneration`, `PendingTimerModel`, etc.) to production types (`vb_runtime::shard::PendingTimer`, `vb_runtime::shard::timer_wheel::TimerWheel`) with `verus::external_type` and `verus::external_fn_specification` for key operations (`matches_authority`, `insert`, `cancel`, `fire_expired`)
   - OR remove the 10 Verus obligations and document them as waived with compensating Kani + Proptest coverage

2. **HIGH — Loom models (F-vb-fzgdn-012-R2):** Either:
   - Document a formal bisimulation between the model's `TimerWheelModel`/`TimerEntry` and the production `TimerWheel`/`TimerEntry`
   - OR waive Loom obligations with Kani + Proptest compensating evidence

3. **MEDIUM — Smoke checks (F-vb-fzgdn-013-R2):** Run at least one Kani harness compilation, one Verus type-check, and one proptest test run to establish that artifacts parse and type-check.

4. **MEDIUM — Engineering rules (F-vb-fzgdn-014-R2):** Replace `.unwrap()` with match/if-let patterns in Kani harnesses.

5. **MEDIUM — Flux trusted reduction (F-vb-fzgdn-015-R2):** Add per-`#[trusted]` justification in trusted-base ledger. Prove at least some non-trivial refinements without `#[trusted]`.

6. **LOW — Trusted-base ledger (F-vb-fzgdn-016-R2):** Expand with per-verifier trust markers.

A targeted re-review focused on the Verus lane is sufficient for the next attempt — the Kani, Proptest, and Fuzz lanes are already at acceptable quality.
