# Final Evidence Decision — vb-pg2wq

STATUS: APPROVED

## Decision

State 12 (formal-verifier), State 13 (black-hat-reviewer), and State 14 (evidence-packaging) are all approved for bookmark-ready handoff.

The bead `vb-pg2wq` (Tests: make duplicate-event test assert one exact contract (P1 bug)) is APPROVED for landing by the femdation controller.

**Landing action** (per `velvet-ballistics-MASTER.md` lifecycle):
1. Close the bead: `bd close vb-pg2wq`.
2. Push bead state: `bd dolt push`.
3. Land the jj change `plzptorw db94f1ea` (vb-pg2wq: p11-holzman-rust — exact-tuple pin for duplicate-event tests) onto the parent commit `rsvywymk 1d6c017f` (AGENTS.md round10 forward-port) per the master document's beading policy.
4. Push to remote: `git push` from `/home/lewis/src/velvet-ballistics` (coord checkout) after the jj→git sync.

Do not land main directly; landing remains serialized by the femdation master controller.

---

## Required Raw Evidence (all PASS)

| Gate | Command | Result | Evidence |
|------|---------|--------|----------|
| ps001 | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_001 ps001_duplicate_rejected --no-fail-fast` | PASS (1 passed, 6 filtered out, 1.44s) | `evidence/state12_test_ps001_duplicate_rejected.log` |
| ps003 | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_003 ps003_dup_fields --no-fail-fast` | PASS (1 passed, 5 filtered out, 1.57s) | `evidence/state12_test_ps003_dup_fields.log` |
| ps004_no_persist | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_004 ps004_no_persist --no-fail-fast` | PASS (1 passed, 4 filtered out, 1.57s) | `evidence/state12_test_ps004_no_persist.log` |
| ps004_empty_commit_after_rej | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_004 ps004_empty_commit_after_rej --no-fail-fast` | PASS (1 passed, 4 filtered out, 1.56s) | `evidence/state12_test_ps004_empty_commit_after_rej.log` |
| ps008 | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_008 ps008_dup_before_queue --no-fail-fast` | PASS (1 passed, 4 filtered out, 1.55s) | `evidence/state12_test_ps008_dup_before_queue.log` |
| ps009 | `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_009 ps009_dup_rejected --no-fail-fast` | PASS (1 passed, 5 filtered out, 1.51s) | `evidence/state12_test_ps009_dup_rejected.log` |
| source-lint (weak pattern) | `rtk rg -n -- 'JournalError::DuplicateEvent \{ \.\. \}' crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs ...` | PASS (0 hits) | `evidence/state12_weak_pattern_scan.txt` |
| source-lint (test integrity) | `bash scripts/check-test-integrity.sh` | PASS (`test integrity: PASS base=@-`) | `evidence/state12_check_test_integrity.log` |
| source-lint (fmt) | `cargo fmt --all --check` | DEFERRED_GLOBAL (drift in 3 unrelated files: vb_core/src/lib.rs:26, vb_core/src/time.rs:71, vb_runtime/src/frame_pool/tests.rs:85/114/139; 5 changed test files formatting-clean) | `evidence/state12_cargo_fmt.log` |
| regression sweep | `cargo test -p vb_storage --tests --no-fail-fast` | PASS (1669 passed, 0 failed, 16 suites, 9.79s) | `evidence/state12_vb_storage_all_tests_full.log` (1766 lines), `evidence/state12_vb_storage_test_results.txt` |

---

## Obligation Closure Roll-up

| Obligation | Verifier | Classification | Behavior Affecting | Waiver |
|------------|----------|----------------|--------------------|--------|
| PO-vb-pg2wq-001 (field-bound guard, 4 functions) | proptest | PASS | false | none |
| PO-vb-pg2wq-002 (field-bound guard + secondary invariants, 2 functions) | proptest | PASS | false | none |
| PO-vb-pg2wq-003 (cross-cutting pattern-discipline + source-lint) | proptest | PASS | false | none |

**3 PASS / 0 FAIL / 0 WAIVED / 0 BLOCKED.**

---

## Residual Risks (advisory; not blocking)

1. **RR-1**: Pre-existing `cargo fmt --all --check` drift in 3 unrelated files. Out of scope; 5 changed test files formatting-clean. Documented in `formal-verification-report.md`, `black-hat-review.md`, `assurance-bundle.md`, `truth-serum-report.md`. Owner-approved pre-existing drift.
2. **RR-2**: Pre-existing BLOCK_GLOBAL compile errors in `crates/vb_compile/tests/common/mod.rs` (out of scope for `-p vb_storage`). `cargo test -p vb_storage --tests` returns 1669 passed.
3. **RR-3**: Kani binding-strengthened (not re-discharged). Existing Kani harness at `kani_vb_vzcuf_ps004.rs:48-59` already models the field-bound contract; runtime↔Kani alignment strengthened by this bead; Kani re-execution is sibling-bead responsibility.

None are bead defects. None are blocking.

---

## Required Pre-Handoff State

All required pre-handoff conditions are satisfied:

- [x] Implementation report exists (`implementation.md`, 399 lines)
- [x] Evidence captured (17 state-11 files + 16 state-12 files + 5 state-14 files)
- [x] Formal verification report exists (`formal-verification-report.md`)
- [x] Verification ledger exists (`verification-ledger.jsonl`, 3 rows, all PASS)
- [x] Formal waivers exist (`formal-waivers.jsonl`, empty as required)
- [x] Black-hat review exists (`black-hat-review.md`, STATUS: APPROVED)
- [x] Defects file exists (`defects.md`, empty)
- [x] Assurance bundle exists (`assurance-bundle.md`)
- [x] Truth-serum report exists (`truth-serum-report.md`, STATUS: APPROVED)
- [x] Final evidence decision exists (this file, STATUS: APPROVED)
- [x] Production contract preserved verbatim (verified by `jj diff -r '@' --stat`)
- [x] No Cargo.toml modified (verified by `jj diff -r '@' --stat`)
- [x] No forbidden constructs introduced (verified by `scripts/check-test-integrity.sh`)
- [x] All SHA-256 hashes captured (`evidence/state14_*_hashes.txt`)

---

## Decision Provenance

- Final State: 14 (evidence-packaging)
- Invocation ID: `final-evidence-decision-vb-pg2wq-state14`
- Parent Invocation: `truth-serum-vb-pg2wq-state14`
- Started: 2026-07-01T22:28:00Z
- Completed: 2026-07-01T22:29:00Z
- Host Session: `femdation-cheap25-batch`

---

## Verdict

**STATUS: APPROVED.** The bead is ready for landing. The femdation controller has full discretion over the landing order per the master document's lifecycle.