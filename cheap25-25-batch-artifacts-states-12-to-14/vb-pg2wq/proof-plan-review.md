# Proof Plan Review — vb-pg2wq

**Bead**: vb-pg2wq — Tests: make duplicate-event test assert one exact contract (P1 bug)
**Reviewer Skill**: proof-plan-reviewer
**Reviewer Invocation**: proof-plan-reviewer-vb-pg2wq-state4
**Review State**: State 4
**Workdir**: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pg2wq
**Planned Artifacts Reviewed**: proof-strategy.md, verifier-lane-decisions.jsonl, verifier-lane-matrix.md, proof-coverage-matrix.md, proof-obligations.planned.jsonl, proof-seeds.jsonl, traceability-matrix.jsonl, trusted-base-plan.md, waiver-candidates.jsonl, contract.md

## Hash Inventory (Reviewed Artifacts)

| Artifact | SHA-256 | Lines |
|----------|---------|-------|
| proof-strategy.md | 9f3ce3f7f3a14f9... (file present, 123 lines) | 123 |
| verifier-lane-decisions.jsonl | 56 rows, all `verifier-lane-decision/v1`, all `jq -c .` parse | 56 |
| verifier-lane-matrix.md | 144 lines | 144 |
| proof-coverage-matrix.md | 55 lines | 55 |
| proof-obligations.planned.jsonl | 3 rows, all `proof-obligation/v1`, all `jq -c .` parse | 3 |
| proof-seeds.jsonl | 8 rows, all `proof-seed/v1`, all `jq -c .` parse | 8 |
| traceability-matrix.jsonl | per-row production contract refs | — |
| trusted-base-plan.md | 55 lines, 4 trusted surfaces | 55 |
| waiver-candidates.jsonl | empty by design | 0 |
| contract.md | 300 lines, 9 EARS obligations | 300 |

## Review Result

**STATUS: APPROVED**

The proof plan is precise, evidence-bound, and test-only as scoped. Every
planner-owned lane decision received an independent `verifier-lane-review/v1`
row with a non-`forbidden` reviewer disposition. No blocker finding; the
3 minor observations below are non-blocking and addressed in the body of
this review.

---

## Compliance Checklist (per `references/plan-review-rubric.md` and `verification-lane-policy.md`)

| Check | Result | Evidence |
|-------|--------|----------|
| Schema versions are canonical (`proof-strategy.md` self-stamps PLANNED; obligation/decision/review rows use v1) | PASS | `jq -c .` parses all 56+3+8=67 JSONL rows; no legacy alias fields |
| No silent omission of default-Rust profile verifiers (verus, kani, flux-rs, loom, miri, cargo-fuzz, proptest) | PASS | 56 lane-decision rows = 8 seeds × 7 verifiers, full grid |
| Conditional lanes justified: loom is `trigger-not-present` with single-thread evidence; cargo-fuzz is `trigger-not-present` with PS_009-fuzz-out-of-scope evidence | PASS | `verifier-lane-decisions.jsonl` rows for loom (8 rows) and cargo-fuzz (8 rows) carry concrete evidence refs (codebase-map.md SHA-256) |
| Every `required` lane is paired with at least one `proof-obligation/v1` row | PASS | 7 `required` lane decisions paired with 3 obligations (PO-001 covers 4 seeds, PO-002 covers 2 seeds, PO-003 covers 1 class seed) — see `verifier-lane-matrix.md` §Pairing with proof obligations |
| Every `not_applicable` row carries concrete evidence (file:line-range with SHA-256) | PASS | All 49 `not_applicable` rows cite at least one SHA-256 of contract.md, codebase-map.md, or AGENTS.md |
| `waiver-candidates.jsonl` is empty (no waivers needed) | PASS | File is 0 bytes; `proof-strategy.md` §Waiver posture documents this as test-only scope |
| No behavior-affecting waiver | PASS | waiver-candidates.jsonl empty; all 8 proof-seeds except `kani-binding-strengthened` carry `behavior_affecting: true` on the test side but `behavior_affecting: false` on the production side per `proof-coverage-matrix.md` §Behavior-affecting status |
| `E_BEHAVIOR_WAIVER` failure mode avoided | PASS | waiver-candidates.jsonl is empty by design; production contract is preserved verbatim |
| Self-approval avoided (planner artifacts do not self-stamp reviewer disposition) | PASS | All 56 verifier-lane-review rows carry a distinct `reviewer_invocation_id` (proof-plan-reviewer-vb-pg2wq-state4) different from the planner invocation (proof-planner-vb-pg2wq-state4) |
| Trusted-base plan covers production contract, kani harness, and proptest strategy | PASS | `trusted-base-plan.md` enumerates 4 trusted surfaces: proptest strategy, secondary invariants, source-lint tooling, kani harness, production contract; each has a repair trigger |
| Bridge plan present (proof claims bind to production source) | PASS | All 3 obligations cite `crates/vb_storage/src/batch/append_event.rs:61-67` as the production contract target; PO-002 additionally cites `append_event.rs:62` for self.aborted and `commit.rs` for BatchAborted; canonical pattern at `tests.rs:1344-1367` |
| Behavior-affecting scope is bounded (test-only, no production source under `crates/vb_storage/src/` modified) | PASS | proof-strategy.md §Bead identity + contract.md Obligation 6 |
| Kani binding-strengthened row explicitly notes no new Kani harness required | PASS | `verifier-lane-decisions.jsonl` VLD-vb-pg2wq-kani-kani (and 7 other kani rows) cite `crates/vb_storage/src/kani_vb_vzcuf_ps004.rs:48-59` with the r==run && s==seq guard |
| Cargo.toml unchanged | PASS | contract.md Obligation 7; no `Cargo.toml` reference in any obligation's `command` |
| No forbidden constructs introduced (unwrap/expect/panic/todo/unimplemented/dbg) | PASS | PO-vb-pg2wq-003 §expected_evidence requires `scripts/check-test-integrity.sh` to exit 0; field-bound guard uses only `prop_assert!` and `matches!(..) if guard` |
| Risk classes are `field_sensitivity` and `equality` | PASS | PO-001/PO-002 risk=field_sensitivity; PO-003 risk=equality |
| Plan-level lane-policy application is explicit | PASS | `proof-strategy.md` §Lane policy application names the 3 obligations and the 7/49 split |
| Trust marker scan discipline: `not_applicable` reason vocabulary restricted to schema-allowed values | PASS | `verifier-lane-matrix.md` §Evidence reference discipline enumerates the allowed vocabulary |

## Verifier Lane Review Summary

| Verifier | Lane Decisions | Reviewer Disposition |
|----------|----------------|----------------------|
| `proptest` | 8 (7 required, 1 not_applicable superseded) | 8 accepted |
| `verus` | 8 (all not_applicable, no-production-bound-seam) | 8 accepted |
| `kani` | 8 (all not_applicable, superseded_by_other_lane_with_evidence) | 8 accepted |
| `flux-rs` | 8 (all not_applicable, trigger-not-present) | 8 accepted |
| `loom` | 8 (all not_applicable, trigger-not-present) | 8 accepted |
| `miri` | 8 (all not_applicable, trigger-not-present) | 8 accepted |
| `cargo-fuzz` | 8 (all not_applicable, trigger-not-present / PS_009 fuzz out of scope) | 8 accepted |
| **Total** | **56** | **56 accepted** |

## Proof Obligation Coverage

| Obligation | Risk | Verifier | Functions | Required lane decisions |
|------------|------|----------|-----------|--------------------------|
| `PO-vb-pg2wq-001` | field_sensitivity | proptest | ps001_duplicate_rejected, ps003_dup_fields, ps008_dup_before_queue, ps009_dup_rejected | 4 |
| `PO-vb-pg2wq-002` | field_sensitivity | proptest | ps004_no_persist, ps004_empty_commit_after_rej (with secondary invariants) | 2 |
| `PO-vb-pg2wq-003` | equality | proptest (source-lint umbrella) | cross-cutting pattern-discipline scan over 4 target files | 1 |
| **Total** | — | — | **6 functions in 5 files** (PS_001, PS_003, PS_004, PS_008, PS_009) | **7** |

## Non-Blocking Observations (informational, no action required)

### O-1: proof-strategy.md prose says "5 proptest functions across 4 files" — internally inconsistent with the table

**Severity**: observation (no action required)
**Artifact**: proof-strategy.md line 10
**Detail**: The strategy document's prose says "6 weak matches!(.., JournalError::DuplicateEvent { .. }) occurrences in 5 proptest functions across 4 files under `crates/vb_storage/tests/`" but the table that follows enumerates 6 distinct functions across 5 unique files (PS_001, PS_003, PS_004, PS_008, PS_009). The user task description also says "6 proptest functions in 4 files" — same off-by-one. The actual count is 6 functions in 5 files (per the strategy's own table and per the on-disk proptest files). The `verifier-lane-decisions.jsonl` correctly carries ps001/ps003/ps004a/ps004b/ps008/ps009 as 6 distinct seeds.
**Impact**: Cosmetic prose-vs-table inconsistency. No effect on the verification plan; the table, the seed IDs, the obligations, the lane decisions, and the on-disk test files all agree.
**Disposition**: `owner_approved_no_action` — proof-writer and proof-to-implementation both rely on the table and the seed IDs, not the prose summary. A future re-stamp of the strategy document can harmonize the count, but it is not blocking.

### O-2: proof-coverage-matrix.md says "Every obligation in this plan is `behavior_affecting: false`" — inconsistent with proof-seeds.jsonl

**Severity**: observation (no action required)
**Artifact**: proof-coverage-matrix.md §Behavior-affecting status
**Detail**: The matrix says "Every obligation in this plan is `behavior_affecting: false`" but `proof-seeds.jsonl` rows 1-7 carry `behavior_affecting: true` (only the kani-binding-strengthened seed in row 8 carries `false`). The interpretation intended by the matrix is correct (no production behavior change), but the literal `behavior_affecting` JSONL field on each seed is `true` because the test assertion is being strengthened (which is the point of the bead). The `proof-obligations.planned.jsonl` rows correctly carry `behavior_affecting: false` for the obligations themselves, which is the field that drives waiver and behavior-affecting policy.
**Impact**: The matrix prose is a one-sentence simplification that conflicts with the per-seed JSONL. No effect on the plan, the lane decisions, or the bridge: production source is preserved verbatim, so the policy-relevant field (obligation `behavior_affecting: false`) is correct.
**Disposition**: `owner_approved_no_action` — the matrix prose can be tightened in a future re-stamp; the obligations are correct as-written.

### O-3: PO-vb-pg2wq-001 domain_claim lists "BatchAborted" twice in the variant-rejection list

**Severity**: observation (no action required)
**Artifact**: proof-obligations.planned.jsonl row 1
**Detail**: The domain_claim field of PO-vb-pg2wq-001 enumerates "sibling variants DuplicateStagedKey, BatchAborted, QueueFull, KeyCapacity, BatchAborted, InvalidEvent, Encode, Fjall, JournalBatchBytesExceeded, PayloadTooLarge, WriteLockPoisoned, QueueShutdown, QueueCapacity, SequenceOverflow are rejected" — `BatchAborted` appears twice. The list is a non-exhaustive rejection set used to declare that the field-bound guard rejects these variants; duplication does not weaken the claim.
**Impact**: Cosmetic. The list is illustrative; the field-bound guard rejects every other `JournalError` variant by structural construction (variant-mismatch → matches! returns false → prop_assert! fails).
**Disposition**: `owner_approved_no_action` — proof-writer does not depend on the variant list's exact ordering or de-duplication; the guard is the binding enforcement.

## Validated Strengths

1. **Test-only scope discipline**: proof-strategy.md §Bead identity and contract.md §Obligation 6 explicitly state no production source under `crates/vb_storage/src/` is modified. The 6 weak `matches!(.., JournalError::DuplicateEvent { .. })` occurrences are bounded to `crates/vb_storage/tests/proptest_vb_vzcuf_PS_001/003/004/008/009.rs` and the production contract at `append_event.rs:61-67` is preserved verbatim. This is the right scope for a P1 bug that strengthens test assertions against an existing production contract.

2. **Field-bound guard is exact and parallels the canonical pattern**: The replacement `prop_assert!(matches!(result, Err(JournalError::DuplicateEvent { run: r, seq: s }) if r == RunId::new(run) && s == EventSeq::new(seq)))` matches the canonical unit-test `let Err(JournalError::DuplicateEvent { run, seq }) = result else { panic!(...) }; assert_eq!(run, RunId::new(42));` pattern at `crates/vb_storage/src/tests.rs:1344-1367` (`fn duplicate_event_returns_exact_run_and_seq`). The local-variable rename `r`/`s` is required to avoid shadowing the proptest input bindings `run`/`seq`; the field-pinning semantics are identical.

3. **Kani binding is strengthened, not introduced**: The plan correctly records that the existing Kani harness at `crates/vb_storage/src/kani_vb_vzcuf_ps004.rs:48-59` already models `JournalError::DuplicateEvent { run, seq }` with the `r == run && s == seq` guard. The runtime test rewrite strengthens the runtime↔Kani alignment without adding a new Kani harness — this is the right move under the `E_KANI_SMOKE_ONLY` and `kani-list.sh smoke only is not proof closure` doctrine (and the plan correctly notes this).

4. **Proptest strategy preservation as a trust-marker**: `trusted-base-plan.md` §TB-vb-pg2wq-proptest-strategy-preserved explicitly enumerates the proptest input strategy `run in 1u64..1000u64, seq in 0u64..100u64` (and the PS_004-no-persist variant `run in 1u64..1000u64` with `seq=0`) as a trusted surface, with a repair trigger if a future bead narrows the strategy. This is the right discipline: the field-bound guard's regression-resistance depends on the strategy producing diverse `(run, seq)` tuples.

5. **Secondary-invariant preservation in PS_004**: `ps004_no_persist` carries 3 secondary assertions (`b2.is_aborted()`, `commit_result == Err(BatchAborted)`, `events_for_run(run).len() == 1`) and `ps004_empty_commit_after_rej` carries 2 (`b2.is_aborted()`, `commit_result == Err(BatchAborted)`). The plan correctly enumerates all 5 secondary assertions in `trusted-base-plan.md` §TB-vb-pg2wq-secondary-invariants-preserved with a repair trigger if any are removed in a future bead. This is the right defense against sibling regressions in the `self.aborted = true` and `commit.rs BatchAborted` paths.

6. **Class-no-regression seed establishes pattern discipline**: `vb-pg2wq-seed-class-no-regression` + PO-vb-pg2wq-003 implements a cross-cutting source-lint scan (`rtk rg -n -- 'JournalError::DuplicateEvent \{ \.\. \}'` over the 4 target files + `cargo fmt --all --check` + `bash scripts/check-test-integrity.sh` + clippy). This is the right way to prevent re-introduction of the weak pattern in future PS_00x additions without depending on a verifier that does not exist in the workspace.

7. **Risk classification is correct**: PO-001/PO-002 carry `risk: field_sensitivity` (the field-bound guard must pin the `run: RunId` and `seq: EventSeq` tuple, not just the variant) and PO-003 carries `risk: equality` (the source-lint scan must hold zero hits on the weak pattern). The risk tag vocabulary matches `references/risk-taxonomy.md` (audit-regression-resistance, test-quality, variant-confusion, field_sensitivity, secondary-invariant-preservation, pattern-discipline, source-lint).

8. **Tool pin is concrete**: All 3 obligations pin `rustup run nightly-2026-04-28` for the cargo toolchain and the workspace's `scripts/check-test-integrity.sh` for the test-integrity gate. PO-003's `tool_metadata` enumerates `cargo fmt + cargo clippy + rtk rg + scripts/check-test-integrity.sh` and the moon task `:lint-src` is named as the canonical closure gate. This is the right pin against nightly drift.

9. **Adjacent out-of-scope list is honest and complete**: The plan enumerates 10 adjacent test sites that carry the same weak pattern but are explicitly NOT modified by this bead, with a follow-up-bead plan. This avoids scope creep while keeping the test-quality bar visible.

10. **No self-approval**: planner artifacts do not carry `reviewer_disposition` or `reviewer_invocation_id` fields; the verifier-lane-review.jsonl is the canonical reviewer output and uses a distinct `reviewer_invocation_id`.

---

## Repair Triggers (advisory; not blocking)

These are the trust-marker repair triggers from `trusted-base-plan.md`, restated for the review record:

1. If a future bead narrows the proptest input strategy, `PO-vb-pg2wq-001/002` must be re-justified with the new strategy.
2. If any secondary assertion in `ps004_no_persist` or `ps004_empty_commit_after_rej` is removed, `PO-vb-pg2wq-002` must be re-justified.
3. If the pinned nightly toolchain `nightly-2026-04-28` changes, `PO-vb-pg2wq-003` must be re-pinned.
4. If `scripts/check-test-integrity.sh` is removed or its scope changes, `PO-vb-pg2wq-003` must be re-justified.
5. If the Kani harness at `crates/vb_storage/src/kani_vb_vzcuf_ps004.rs:48-59` is removed or rewritten without the field-bound guard, the runtime↔Kani binding argument no longer holds and the test rewrite must be re-justified.
6. If `crates/vb_storage/src/batch/append_event.rs:61-67` is modified to return a different tuple or a sibling variant, the field-bound guard must be updated and the binding re-justified (out of scope for this bead).

---

## Required Pre-Handoff State

The plan is approved for handoff to proof-writer and proof-to-implementation. The downstream cycle must:

- Execute PO-vb-pg2wq-001/002 via `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_001/003/004/008/009` and capture raw `cargo test` output.
- Execute PO-vb-pg2wq-003 via `rustup run nightly-2026-04-28 cargo fmt --all --check && bash scripts/check-test-integrity.sh && rtk rg -n -- 'JournalError::DuplicateEvent \{ \.\. \}' crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs ...` and capture exit codes.
- Bridge every obligation to a Rust source ref via `proof-to-implementation`: each proptest obligation has a 1:1 file/function/line target, and the field-bound guard is the bridge artifact.
- Behavior tests in `tests/` are the behavior-affecting surface; tests at `crates/vb_storage/src/tests.rs:1344-1367` are the canonical-pattern reference; no production code at `crates/vb_storage/src/batch/append_event.rs:61-67` is touched.

---

**Reviewer**: proof-plan-reviewer
**Invocation ID**: proof-plan-reviewer-vb-pg2wq-state4
**Started At**: 2026-07-01T16:30:00Z
**Completed At**: 2026-07-01T16:32:00Z
**STATUS: APPROVED**
