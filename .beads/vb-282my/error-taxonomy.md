# Error Taxonomy — vb-282my

**Bead:** vb-282my (P1)
**Title:** Harness failure taxonomy for TLA bridge refinement
**Date:** 2026-05-29

## Error Classification

Errors are classified along two axes: **domain layer** (where the error originates) and **severity** (blocking vs. non-blocking).

| Severity | Description | Bridge Impact |
|----------|------------|---------------|
| **BLOCKING** | Prevents RRO closure. Must be fixed before bridge can pass. | Row remains PARTIAL/REJECTED. Bridge verdict: REJECTED. |
| **NON-BLOCKING** | Advisory. Does not prevent closure but should be addressed. | Row can close; finding is informational. |
| **FATAL** | Infrastructure error. Cannot proceed with verification at all. | All work stops. Requires tooling/environment fix. |

## Layer 1: RRO Structural Errors

Errors in the RRO JSONL row definition itself.

| Error Code | Severity | Description | Detection |
|-----------|----------|-------------|-----------|
| `RRO-INVALID-ID` | BLOCKING | RroId does not match `RRO-TLA-{MODEL}-{NNN}` pattern. | RRO JSONL validation. Regex: `^RRO-TLA-[A-Z]+-\d{3}$`. |
| `RRO-MISSING-SOURCE-SYMBOL` | BLOCKING | A `source_ref` points to a file without naming a symbol. | SourceRef parser. Must contain `::` with non-empty symbol. |
| `RRO-EMPTY-HARNESS-REFS` | BLOCKING | `refinement_harness_refs` is `[]` and no waiver exists. | Closure guard `validate_rro_closure_prerequisites()`. |
| `RRO-PLANNED-AT-CLOSURE` | BLOCKING | `mapping_status: planned` at State 12 when closure is required. | Lifecycle transition guard. |
| `RRO-UNBRIDGED-SOURCE-REF` | BLOCKING | A `source_ref` has no corresponding harness coverage. | Harness claim-coverage analysis. |
| `RRO-MISSING-BEHAVIOR-TEST` | BLOCKING | `behavior_test_refs` is empty. Every RRO needs at least one behavior test. | RRO validation. |
| `RRO-DUPLICATE-ID` | BLOCKING | Two RRO rows share the same ID. | JSONL dedup check. |
| `RRO-MISMATCHED-RISK-TAG` | NON-BLOCKING | Risk tags don't align with claim content (e.g., concurrency tag on a pure compile-time claim). | Manual review. |
| `RRO-STALE-EVIDENCE` | NON-BLOCKING | Evidence commands reference old paths or deleted artifacts. | Path existence check. |

## Layer 2: TLA+ Model Errors

Errors in the TLA+ specification or its TLC execution.

| Error Code | Severity | Description | Detection |
|-----------|----------|-------------|-----------|
| `TLA-MODEL-NOT-FOUND` | BLOCKING | `.tla` or `.cfg` file does not exist at the specified path. | File existence check. |
| `TLA-TLC-TIMEOUT` | BLOCKING | TLC exceeded time budget without reaching all states. | TLC exit status. |
| `TLA-TLC-COUNTEREXAMPLE` | BLOCKING | TLC found a counterexample to an invariant. Model does not hold. | TLC exit ≠ 0. |
| `TLA-UNBOUNDED-CONSTANT` | BLOCKING | A TLA+ constant is not bounded in the `.cfg`. | TLC warning/error. |
| `TLA-STALE-MODEL` | NON-BLOCKING | Model uses abstractions that no longer match production (e.g., removed error variant not in model). | Model-Rust diff analysis. |
| `TLA-MISSING-INVARIANT` | NON-BLOCKING | The model lacks an invariant covering a known production guard. | Claim-coverage gap analysis. |
| `TLA-WRONG-BOUNDS` | NON-BLOCKING | TLC bounds are too small to exercise interesting behavior (e.g., MaxSeq=1 cannot test monotonicity). | Bound-adequacy review. |

## Layer 3: Behavior Test Errors

Errors in the Rust behavior test evidence.

| Error Code | Severity | Description | Detection |
|-----------|----------|-------------|-----------|
| `TEST-NOT-FOUND` | BLOCKING | Behavior test file or test function does not exist. | File/symbol existence check. |
| `TEST-FAILURE` | BLOCKING | Behavior test does not pass. | `cargo test` exit ≠ 0. |
| `TEST-NOT-INDEPENDENT` | BLOCKING | Test passes even when production behavior is stubbed out (test is vacuous). | Mutation analysis: stub the production function; test should fail. |
| `TEST-NOT-DETERMINISTIC` | NON-BLOCKING | Test has non-deterministic behavior (time, RNG, concurrency without loom). | `cargo test -- --nocapture` repeated runs. |
| `TEST-INSUFFICIENT-ASSERTIONS` | NON-BLOCKING | Test exercises the code path but has weak or no assertions. | Manual assertion-strength review. |
| `TEST-WRONG-RRO` | BLOCKING | Test exercises code not in the RRO's scope. | Source-ref alignment check. |

## Layer 4: Refinement Harness Errors

Errors specific to the refinement harness artifact and its verification.

### Kani Harness Errors

| Error Code | Severity | Description | Detection |
|-----------|----------|-------------|-----------|
| `KANI-HARNESS-NOT-FOUND` | BLOCKING | The Kani harness file does not exist. | File existence check. |
| `KANI-COMPILE-FAILURE` | BLOCKING | The Kani harness does not compile. | `cargo kani --only-codegen` exit ≠ 0. |
| `KANI-VERIFICATION-FAILURE` | BLOCKING | Kani found a counterexample (assertion failure, panic, overflow, UB). | `cargo kani` exit ≠ 0. |
| `KANI-HARDCODED-INPUT` | BLOCKING | Harness uses hardcoded structural inputs instead of `kani::any()` or `kani::Arbitrary`. **GOD RULE 1 violation.** | Manual review of harness code. |
| `KANI-INSUFFICIENT-UNWIND` | BLOCKING | `#[kani::unwind(N)]` is too small for the bounded state space. | Kani warning: "unwinding assertion". |
| `KANI-HARNESS-INCOMPLETE` | BLOCKING | Harness covers only a subset of the TLA+ claim. | Claim-coverage analysis against full claim text. |
| `KANI-NO-COVER-CHECKS` | NON-BLOCKING | Harness uses `kani::assert` but no `kani::cover!` for non-vacuity evidence. | Manual review. |
| `KANI-UNSOUND-ASSUME` | BLOCKING | `kani::assume` excludes reachable states, making proof vacuous. | Coverage analysis: do assume guards exclude production-reachable states? |

### Flux Refinement Errors

| Error Code | Severity | Description | Detection |
|-----------|----------|-------------|-----------|
| `FLUX-REFINEMENT-NOT-FOUND` | BLOCKING | The Flux annotation is not present on the production function. | Source grep for `#[sig]` or `#[refined_by]`. |
| `FLUX-VERIFICATION-FAILURE` | BLOCKING | `cargo flux` reports a refinement violation. | `cargo flux` exit ≠ 0. |
| `FLUX-TRUSTED-ABUSE` | BLOCKING | `#[flux_rs::trusted]` used on a behavior-affecting function without compensating evidence. **GOD RULE 3 extension.** | Manual review. |
| `FLUX-IGNORE-ABUSE` | BLOCKING | `#[flux_rs::ignore]` suppresses checks that would reveal a violation. | Manual review. |
| `FLUX-WEAK-CONTRACT` | NON-BLOCKING | Refinement annotation is weaker than the TLA+ claim (e.g., only checks types, not ordering). | Claim-coverage analysis. |
| `FLUX-OPAQUE-LEAK` | NON-BLOCKING | `#[flux_rs::opaque]` on a type prevents necessary cross-function reasoning. | Manual review. |

### Verus Spec Errors

| Error Code | Severity | Description | Detection |
|-----------|----------|-------------|-----------|
| `VERUS-SPEC-NOT-FOUND` | BLOCKING | Verus spec is not present on the production function. | Source grep for `proof fn` or `spec fn`. |
| `VERUS-VERIFICATION-FAILURE` | BLOCKING | Verus reports a proof failure. | `verus --crate-type=lib` exit ≠ 0. |
| `VERUS-VACUOUS-PROOF` | BLOCKING | Verus `proof fn` is not bound to an `exec fn` with `requires`/`ensures`. **GOD RULE 2 violation.** | Binding check: does `exec fn` have `requires`/`ensures`? |
| `VERUS-TRUSTED-BOUNDARY` | BLOCKING | `#[verus::trusted]` used on behavior-affecting code without review. | Manual review. |
| `VERUS-COMPUTE-ONLY` | BLOCKING | Proof relies on `by(compute)` for an unbounded domain. **GOD RULE 2 violation.** | Manual review. |
| `VERUS-GHOST-LIMITATION` | NON-BLOCKING | Ghost/tracked state is modeled but not all exec state is captured. | Coverage gap analysis. |

### Proptest Property Errors

| Error Code | Severity | Description | Detection |
|-----------|----------|-------------|-----------|
| `PROPTEST-SHRINKING-FAILURE` | BLOCKING | Proptest found a counterexample and minimal shrinking reveals a bug. | `cargo test` output shows minimal failing case. |
| `PROPTEST-INSUFFICIENT-CASES` | NON-BLOCKING | Too few test cases to exercise edge conditions. | Review of `proptest!(|(x in ...)| ...)` case count. |
| `PROPTEST-WEAK-PROPERTY` | BLOCKING | Property is true but doesn't match the TLA+ claim (e.g., checks "no panic" when claim is "preserves ordering"). | Claim-coverage analysis. |
| `PROPTEST-DETERMINISTIC-SEED` | NON-BLOCKING | Hardcoded seed prevents exploring the full input space across CI runs. | Manual review of `ProptestConfig`. |

## Layer 5: Waiver Errors

Errors in proportional waiver creation and approval.

| Error Code | Severity | Description | Detection |
|-----------|----------|-------------|-----------|
| `WAIVER-BEHAVIOR-AFFECTING` | **BLOCKING (IMMEDIATE REJECT)** | Waiver covers a `behavior_affecting: true` claim. This is categorically forbidden (INV-3). | `validate_waiver_behavior_scope()` → Err. |
| `WAIVER-NO-COMPENSATING-EVIDENCE` | BLOCKING | Waiver does not cite TLC + behavior test evidence explaining why it is sufficient. | Waiver document review. |
| `WAIVER-WEAK-BOUNDARY` | BLOCKING | Waiver's boundary argument is insufficient (e.g., "it works" without boundary description). | Manual review. |
| `WAIVER-EXPIRED` | BLOCKING | Waiver has passed its expiry date. | ISO-8601 comparison. |
| `WAIVER-NOT-APPROVED` | BLOCKING | Waiver was drafted but never reviewer-approved. | `review_status` ≠ `approved`. |
| `WAIVER-STALE` | NON-BLOCKING | Waiver was approved before source code changed; compensating evidence may be invalid. | Source diff since waiver approval date. |
| `WAIVER-SELF-APPROVED` | BLOCKING | Waiver was approved by the same agent that drafted it. **INV-5 violation.** | Invocation provenance check. |
| `WAIVER-DUPLICATE` | BLOCKING | Two active waivers cover the same RRO. | RRO-waiver dedup check. |
| `WAIVER-MISSING-EXPIRY` | BLOCKING | Waiver has no expiry or expiry is in the past. | `expiry` field validation. |
| `WAIVER-INCORRECT-HASH` | BLOCKING | Waiver references a stale RRO row (hash mismatch). | `waiver_candidate_hash` ≠ current RRO hash. |

## Layer 6: Bridge Review Errors

Errors in the overall bridge verdict and review process.

| Error Code | Severity | Description | Detection |
|-----------|----------|-------------|-----------|
| `BRIDGE-SELF-REVIEWED` | BLOCKING | Bridge reviewer is the same invocation that produced the harness. **INV-5 violation.** | Invocation provenance check. |
| `BRIDGE-PARTIAL-CLOSURE` | BLOCKING | Bridge passes but some RRO rows are still PARTIAL. | Row status enumeration: all must be VERIFIED/CLOSED. |
| `BRIDGE-MISSING-FINDING-RESOLUTION` | BLOCKING | `TLA-BRIDGE-REFINEMENT-HARNESS-GAP` finding persists but bridge is claimed as PASS. | Finding inventory check. |
| `BRIDGE-EVIDENCE-STALE` | NON-BLOCKING | Evidence commands were run against a different code commit. | Commit hash comparison. |
| `BRIDGE-INCOMPLETE-MATRIX` | BLOCKING | `traceability-matrix.jsonl` does not cover all 7 RRO rows. | Matrix-RRO coverage check. |
| `BRIDGE-MISSING-INVOCATION` | BLOCKING | Reviewer disposition is claimed without an independent `agent-invocation/v1` row. | Invocation ledger check. |

## Layer 7: Infrastructure Errors (FATAL)

Errors that prevent any verification work from proceeding.

| Error Code | Severity | Description | Detection |
|-----------|----------|-------------|-----------|
| `INFRA-KANI-NOT-INSTALLED` | FATAL | `cargo kani` is not available in the environment. | Tool version check. |
| `INFRA-FLUX-NOT-INSTALLED` | FATAL | `cargo flux` is not available. | Tool version check. |
| `INFRA-VERUS-NOT-INSTALLED` | FATAL | `verus` binary is not available. | Tool version check. |
| `INFRA-TLC-NOT-INSTALLED` | FATAL | `tlc` (TLA+ tools) are not available. | Tool version check. |
| `INFRA-NIGHTLY-MISSING` | FATAL | Rust nightly toolchain is not installed. | `rustup show` check. |
| `INFRA-WORKSPACE-CORRUPTED` | FATAL | Cargo workspace cannot resolve dependencies. | `cargo check` exit ≠ 0. |
| `INFRA-OUT-OF-DISK` | FATAL | No disk space for verification artifacts. | `df -h` check. |
| `INFRA-OUT-OF-MEMORY` | FATAL | Verification process killed by OOM. | Exit code 137 (SIGKILL). |

## Error Mapping: Per-RRO Row

Each RRO row has specific expected error paths:

| RRO Row | Expected Failure Modes |
|---------|----------------------|
| CHOOSE-LOWERING-001 | `KANI-COMPILE-FAILURE` (cross-crate harness), `KANI-INSUFFICIENT-UNWIND` (fanout ≤ 64 requires unwind ≥ 65), `KANI-HARDCODED-INPUT` if harness uses fixed branch tables |
| CHOOSE-REPLAY-001 | `KANI-VERIFICATION-FAILURE` (overflow in branch index increment), `KANI-HARNESS-INCOMPLETE` if only true-branch path is tested |
| ASK-ANSWER-001 | `FLUX-WEAK-CONTRACT` if refinement only checks types not ordering, `KANI-VERIFICATION-FAILURE` if journal monotonicity violated |
| RETRY-FSM-001 | `KANI-HARNESS-INCOMPLETE` (existing harness covers monotonicity only, missing exhaustion and terminal typing), `KANI-UNSOUND-ASSUME` if assume excludes max_attempts edge cases |
| RETRY-JOURNAL-001 | `KANI-VERIFICATION-FAILURE` (key collision found), `KANI-HARDCODED-INPUT` if key encoding is hardcoded |
| RESUME-001 | `KANI-VERIFICATION-FAILURE` (state-machine invariant broken), `FLUX-WEAK-CONTRACT` if RuntimeState refinement is too coarse |
| ADMISSION-001 | `KANI-VERIFICATION-FAILURE` (live state created before journaling), `KANI-UNSOUND-ASSUME` if assume skips append failure path |

## Cross-Cutting Error Rules

1. **Behavior-affecting waivers are categorically rejected** (INV-3). No exception. No appeal.
2. **Self-approval is categorically rejected** (INV-5). Reviewer must be an independent agent invocation.
3. **Hardcoded inputs invalidate Kani proofs** (GOD RULE 1). All harness inputs must use `kani::any()` or `kani::Arbitrary`.
4. **TLA-only closure is rejected** (INV-6). TLA+ is temporal-design evidence, not implementation proof.
5. **Harness-test collisions are rejected** (INV-2). A behavior test file cannot double as a refinement harness.
6. **Claim-coverage gaps are blocking** (INV-8). Harness must cover the full TLA+ claim, not a subset.

## Error Recovery Procedures

| Error | Recovery |
|-------|----------|
| KANI-VERIFICATION-FAILURE | Fix production code (GOD RULE 4: never weaken the contract). Re-run harness. |
| HARSH-HARDCODED-INPUT | Rewrite harness to use `kani::any()` / `kani::Arbitrary`. |
| WAIVER-BEHAVIOR-AFFECTING | Abandon waiver path. Write a refinement harness instead. |
| BRIDGE-SELF-REVIEWED | Request review from independent proof-reviewer agent with different invocation_id. |
| INFRA-* | Fix environment. Re-run all evidence commands. Do not close any row until tooling is restored. |
| KANI-HARNESS-INCOMPLETE | Extend harness to cover all claim sub-clauses. Document which claim components are covered by which assertion. |
