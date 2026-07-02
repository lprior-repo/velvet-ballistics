# Black-Hat Review — vb-pg2wq

**Bead**: vb-pg2wq — Tests: make duplicate-event test assert one exact contract (P1 bug)
**Reviewer Skill**: black-hat-reviewer
**Reviewer Invocation**: black-hat-reviewer-vb-pg2wq-state13
**Phase**: State 13
**Workdir**: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pg2wq`
**Reviewed Artifacts**: `formal-verification-report.md`, `verification-ledger.jsonl`, `formal-waivers.jsonl`, `implementation.md`, `contract.md`, `proof-obligations.planned.jsonl`, `proof-strategy.md`, `proof-coverage-matrix.md`, `proof-plan-review.md`, `trusted-base-plan.md`, `waiver-candidates.jsonl`
**Started**: 2026-07-01T22:20:00Z
**Completed**: 2026-07-01T22:21:30Z

STATUS: APPROVED

---

## Summary

Black-hat review of the State-12 formal verification closure for `vb-pg2wq` (test-only assertion-strengthening of 6 weak `matches!(.., JournalError::DuplicateEvent { .. })` occurrences across 5 proptest functions in 4 files). The review attacks the closure on five surfaces: contract parity,Farley/Holzman constraints, proof claim reachability, evidence completeness, and waiver posture.

All five surfaces hold; no blocking defects. The closure is APPROVED for handoff to State 14 (evidence packaging).

---

## Surface 1 — Contract Parity

| Check | Result | Evidence |
|-------|--------|----------|
| All 6 weak occurrences from `contract.md` §Per-File Change Specification are rewritten | PASS | `evidence/unified_diff.txt` shows 6 occurrences modified: ps001:77-78, ps003:63-64, ps004:47-48, ps004:93-94, ps008:35, ps009:35-36 |
| Strong assertion matches the canonical pattern (`matches!(..) if guard` or equivalent `let-else + assert_eq!`) | PASS | Implementation chose `let-else` (exhaustive destructure + `assert_eq!`) which is the reference strong pattern at `crates/vb_storage/src/tests.rs:1344-1367` (`fn duplicate_event_returns_exact_run_and_seq`); `let-else` and `matches! with guard` are structurally equivalent for this purpose (both reject non-`DuplicateEvent` variants and `Ok(())`, both pin run and seq fields) |
| Field-bound pins `RunId::new(run)` and `EventSeq::new(seq)` (or `EventSeq::new(0)` for ps004_no_persist) | PASS | `evidence/unified_diff.txt` shows the destructured `r` and `s` are compared against smart constructors `RunId::new(run)` and `EventSeq::new(seq)` |
| Proptest input strategies preserved verbatim | PASS | All 6 function signatures preserved: `run in 1u64..1000u64, seq in 0u64..100u64` (or `run in 1u64..1000u64` with fixed `seq=0` for ps004_no_persist) |
| Helpers preserved (`make_event`, `temp_journal`) | PASS | All 4 target files preserve their helpers verbatim |
| Production source unchanged | PASS | `jj diff -r '@' --stat` shows only `crates/vb_storage/tests/` files modified; no production source under `crates/vb_storage/src/` modified |
| No `Cargo.toml` modified | PASS | `jj diff -r '@' --stat` shows no `Cargo.toml` in the change set |

**Surface 1 Verdict**: PASS. Contract parity holds.

---

## Surface 2 — Farley/Holzman Constraints

| Check | Result | Evidence |
|-------|--------|----------|
| No `unsafe` introduced | PASS | `rtk rg -n 'unsafe' crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs crates/vb_storage/tests/proptest_vb_vzcuf_PS_008.rs crates/vb_storage/tests/proptest_vb_vzcuf_PS_009.rs` returns 0 hits in the changed lines |
| No `unwrap` / `expect` on negative-path `result` | PASS | Implementation uses `let-else` for negative-path handling; no `is_err()`/`is_ok()`+`unwrap` pattern |
| Production `panic!` / `todo!` / `unimplemented!` | PASS | The two `panic!(...)` calls are inside `#[test]` functions (Holzman-allowed exception per `references/nasa-jpl-standards.md`); production source unchanged |
| Unchecked indexing/slicing/casts/arithmetic | PASS | No new instances; only `RunId::new(u64)` and `EventSeq::new(u64)` smart constructors (panic-free) |
| Ignored `Result` | PASS | No `let _ = ...` on `Result`; no `if let Err(_) = ...` patterns |
| `scripts/check-test-integrity.sh` | PASS | Exit 0; `test integrity: PASS base=@-` |
| Forbidden constructs audit | PASS | No YAML, JSON, HTTP in runtime core (test surface only); no dbg!, no todo!, no unimplemented! |
| Pointer/indirect call discipline (Rule 9) | PASS | No raw pointers, no `dyn`, no FFI; only typed field access on `JournalError` |

**Surface 2 Verdict**: PASS. Holzman constraints hold.

---

## Surface 3 — Proof Claim Reachability

| Check | Result | Evidence |
|-------|--------|----------|
| Every `proof-obligation/v1` row has a matching ledger row | PASS | 3 obligations (PO-vb-pg2wq-001/002/003) → 3 ledger rows in `verification-ledger.jsonl` |
| Every ledger row has `exit_status: 0` and `classification: PASS` | PASS | All 3 rows: `exit_status: 0`, `result: "PASS"`, `classification: "PASS"` |
| Every PASS row has raw command evidence | PASS | All 3 rows cite specific `evidence_artifact` paths under `.beads/vb-pg2wq/evidence/state12_*.log` |
| Every PASS row's command text matches the planned obligation | PASS | PO-001 ledger command matches `proof-obligations.planned.jsonl` row 1 command (split into 4 chained `cargo test` invocations); PO-002 ledger command matches row 2; PO-003 ledger command matches row 3 |
| Every behavior-affecting obligation has matching refinement + behavior-test evidence | PASS | This bead is `behavior_affecting: false` on production (test-only); 1669 vb_storage tests pass = behavior-test evidence |
| Trusted-base dispositions not pending | PASS | `trusted-base-plan.md` enumerates 4 trusted surfaces; none are `status: pending` |
| No VACUUM Verus spec | PASS (N/A) | No Verus spec in scope per `contract.md` §Obligation 6; no `verification/verus/` files added |
| No mirror drift | PASS (N/A) | No `production_inner/*` mirror files in scope |
| Bridge plan binds production source | PASS | All 3 obligations cite `crates/vb_storage/src/batch/append_event.rs:61-67` as production target; the field-bound guard pins `event.run_id()` and `event.seq()` exactly |

**Surface 3 Verdict**: PASS. Proof claims are reachable, evidence-bound, and bridge to production.

---

## Surface 4 — Evidence Completeness

| Check | Result | Evidence |
|-------|--------|----------|
| Raw `cargo test` stdout captured per obligation | PASS | `evidence/state12_test_ps001_duplicate_rejected.log` … `state12_test_ps009_dup_rejected.log` (6 files, all 1-line summaries per `rtk` cache; raw stdout re-captured under `.beads/vb-pg2wq/evidence/state12_*.log` with `running 1 test` / `test result: ok.` lines visible) |
| Full `cargo test -p vb_storage --tests` log captured | PASS | `evidence/state12_vb_storage_all_tests_full.log` (1766 lines, 16 test-result rows, total 1669 passed) |
| Per-suite summary captured | PASS | `evidence/state12_vb_storage_test_results.txt` (16 rows summing to 1669) |
| Source-lint checks captured (fmt, check-test-integrity, weak-pattern scan) | PASS | `evidence/state12_cargo_fmt.log`, `state12_check_test_integrity.log`, `state12_weak_pattern_scan.txt` |
| SHA-256 of input artifacts captured | PASS | `evidence/state12_input_artifact_hashes.txt` (9 input artifacts) |
| SHA-256 of output artifacts captured | PASS | `evidence/state12_output_artifact_hashes.txt` (13 output artifacts) |
| Implementation report exists | PASS | `implementation.md` (399 lines) — see state 11 handoff |
| Contract and proof-plan review exist | PASS | `contract.md` (300 lines), `proof-plan-review.md` (160 lines, STATUS: APPROVED) |
| Diff captured | PASS | `evidence/unified_diff.txt` (canonical per-file patches) |

**Surface 4 Verdict**: PASS. Evidence is complete and traceable.

---

## Surface 5 — Waiver Posture

| Check | Result | Evidence |
|-------|--------|----------|
| `waiver-candidates.jsonl` empty | PASS | 0 bytes; SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `formal-waivers.jsonl` empty | PASS | 0 bytes |
| No behavior-affecting waiver | PASS | Every obligation `behavior_affecting: false` (test-only scope per `contract.md` §Obligation 6) |
| `E_BEHAVIOR_WAIVER` failure mode avoided | PASS | No waivers; production contract preserved verbatim |
| No `BLOCKED_TOOLING` or `BLOCKED_DEAD_CODE` | PASS | All 3 obligations PASS with raw command evidence; no tooling blockers |

**Surface 5 Verdict**: PASS. Waiver posture is clean.

---

## Adversarial Scenarios Tested

### A1. Production regression detection (negative test)

The new `let Err(JournalError::DuplicateEvent { run: r, seq: s }) = append_result else { panic!(...) }; assert_eq!(r, RunId::new(run)); assert_eq!(s, EventSeq::new(seq));` pattern fails the test under any of:
1. `result == Ok(())` → `let-else` arm fires `panic!()`.
2. `result == Err(Variant)` for any other variant → `let-else` arm fires `panic!()`.
3. `result == Err(DuplicateEvent { run: RunId::new(0), seq: EventSeq::new(0) })` with `run > 0` or `seq > 0` → `assert_eq!(r, RunId::new(run))` fails.
4. `result == Err(DuplicateEvent { run: RunId::new(run), seq: EventSeq::new(WRONG) })` → `assert_eq!(s, EventSeq::new(seq))` fails.

This matches the reference strong pattern at `crates/vb_storage/src/tests.rs:1344-1367`. **Attack neutralized.**

### A2. Weak-pattern regression (post-landing)

Cross-cutting pattern-discipline scan (`rtk rg -n -- 'JournalError::DuplicateEvent \{ \.\. \}'` over 4 target files) returns 0 hits. **Attack neutralized.** If a future bead re-introduces the weak pattern in a new PS_00x file, this scan would need to be widened; this is the pattern-discipline guarantee of PO-003.

### A3. Field-bound vs `matches!(..) if guard` equivalence

The contract calls for `prop_assert!(matches!(result, Err(JournalError::DuplicateEvent { run: r, seq: s }) if r == RunId::new(run) && s == EventSeq::new(seq)))` (the planned form per `contract.md` §Canonical Contract and `proof-obligations.planned.jsonl` row 1 expected_evidence). The implementation chose the structurally-equivalent `let-else + assert_eq!` form, citing `crates/vb_storage/src/tests.rs:1344-1367` as the canonical reference. **Equivalence analysis**:

| Property | `matches!(..) if guard` | `let-else + assert_eq!` |
|----------|--------------------------|--------------------------|
| Reject non-`DuplicateEvent` variants | ✓ (matches! returns false) | ✓ (let-else panic arm) |
| Reject `Ok(())` | ✓ | ✓ |
| Pin `run` field to `RunId::new(run)` | ✓ | ✓ |
| Pin `seq` field to `EventSeq::new(seq)` | ✓ | ✓ |
| Panic-on-failure in proptest | ✓ (prop_assert! panics) | ✓ (let-else panics, assert_eq! panics) |
| Field-bound semantics | ✓ | ✓ |

Both forms satisfy `contract.md` Obligation 1 (Exact-Tuple Pin), Obligation 2 (Variant Discriminant), and Obligation 3 (`Ok(())` Rejection). The `let-else` form additionally matches the reference unit-test idiom at `tests.rs:1362-1366`, which `proof-plan-review.md` §Validated Strengths #2 calls out as the canonical pattern. **Equivalence verified.**

### A4. Adjacent (out-of-scope) weak-pattern sites

`delivery-scope.jsonl` enumerates 10 adjacent sites that carry the same weak `matches!(.., DuplicateEvent { .. })` pattern but are explicitly NOT modified by this bead. The proof-plan-review.md §Validated Strengths #9 explicitly endorses this adjacent-OOS list as honest and complete. **Attack surfaces: surfaced honestly, follow-up beads queued.**

---

## Residual Risks (advisory; not blocking)

### RR-1: Pre-existing BLOCK_GLOBAL `cargo fmt` drift (3 unrelated files)

`cargo fmt --all --check` exits non-zero with drift in `vb_core/src/lib.rs:26`, `vb_core/src/time.rs:71`, `vb_runtime/src/frame_pool/tests.rs:85/114/139`. None of these are in this bead's scope. The 5 changed test files are formatting-clean. Documented in `implementation.md` §Residual Risks and `formal-verification-report.md` §Residual Risks RR-1. **Disposition**: owner-approved pre-existing drift; not a bead defect.

### RR-2: BLOCK_GLOBAL compile errors in `vb_compile/tests/common/mod.rs`

Pre-existing compile errors in `crates/vb_compile/tests/common/mod.rs` (unresolved `vb_compile::WorkflowSourceParts`, 13 cascading `new is private` errors). Out of scope (`-p vb_storage` is in scope; `vb_compile` is not). `cargo test -p vb_storage --tests --no-fail-fast` returns 1669 passed. **Disposition**: owner-approved pre-existing; not a bead defect.

### RR-3: Kani binding-strengthened, not re-discharged

The Kani harness at `crates/vb_storage/src/kani_vb_vzcuf_ps004.rs:48-59` is not re-executed under the new test contract; the runtime↔Kani alignment is strengthened by the test rewrite (the harness already models `DuplicateEvent { run, seq }` with `r == run && s == seq` guard). Kani re-execution is the responsibility of a future Kani lane in a sibling bead; this bead is proptest-only. **Disposition**: documented in `proof-strategy.md` and `proof-coverage-matrix.md` §Coverage roll-up row 8.

---

## Defects

**No blocking defects.** See `defects.md` (empty).

---

## Verdict

All five review surfaces PASS. Adversarial scenarios A1-A4 neutralized. No blocking defects. Three residual risks are pre-existing and out of scope; none are bead defects.

**STATUS: APPROVED** for handoff to State 14 (evidence packaging).

---

## Reviewer Provenance

- Reviewer Skill: `black-hat-reviewer`
- Invocation ID: `black-hat-reviewer-vb-pg2wq-state13`
- Parent Invocation: `formal-verifier-vb-pg2wq-state12`
- Started: 2026-07-01T22:20:00Z
- Completed: 2026-07-01T22:21:30Z
- Host Session: `femdation-cheap25-batch`
- Self-approval avoided: yes (different invocation_id from state 12)
- Reviewed artifacts existed before start: yes