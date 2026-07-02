# Assurance Bundle — vb-pg2wq

## Scope

- **Bead**: vb-pg2wq — Tests: make duplicate-event test assert one exact contract (P1 bug)
- **Goal**: Strengthen 6 weak `matches!(.., JournalError::DuplicateEvent { .. })` occurrences in 5 proptest functions across 4 files under `crates/vb_storage/tests/` to a field-bound assertion that pins `run: RunId` and `seq: EventSeq` against the proptest inputs. Production contract at `crates/vb_storage/src/batch/append_event.rs:61-67` is preserved verbatim (test-only fix; no production source under `crates/vb_storage/src/` is modified).
- **Verifier scope**: proptest only; no Kani/Verus/Flux/Loom required.
- **Phase**: State 14 — Evidence Packaging (terminal state)
- **Workdir**: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pg2wq
- **jj workspace**: cheap25-vb-pg2wq
- **jj root**: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-pg2wq/wt
- **Parent change**: rsvywymk 1d6c017f (AGENTS.md round10 forward-port)
- **Working change**: plzptorw db94f1ea (vb-pg2wq: p11-holzman-rust — exact-tuple pin for duplicate-event tests)
- **Captured**: 2026-07-01T22:25:00Z

## Final Closure Status

| Surface | Status |
|---------|--------|
| State 12 — formal-verifier | PASS (3/3 obligations, 0 FAIL, 0 WAIVED, 0 BLOCKED) |
| State 13 — black-hat-reviewer | APPROVED (5/5 review surfaces PASS, 0 defects) |
| State 14 — evidence-packaging | APPROVED (this bundle) |
| **Final** | **APPROVED for landing** |

---

## Evidence Index

### State 1 — Baseline (`go-skill-vb-pg2wq-state1`)
- `STATE.md` (bead delivery state)
- `baseline-report.md` (workspace preflight)
- `global-readiness-report.md` (GREEN for state 2)
- `runtime-skill-provenance.json` (skill version + pointer)

### State 2 — Explore (`explore-vb-pg2wq-state2`)
- `codebase-map.md` (9f900a1816564661a06d03b749ba4dcd62846d66cbfb72f6b7b16503bc982008)
- `delivery-scope.jsonl` (5ef926eec75d0d231f8c0c6c66f5940670cfc1444ac1ef6518934f3cc8df82db)

### State 3 — Rust Contract (`rust-contract-vb-pg2wq-state3`)
- `domain-model.md` (4470627fbb3a327478d57c8053d3fe4033c21085635e62d9a877ecd8a458831c)
- `type-contracts.md` (fccf86cd422e1b6a84812cb7029c9a4dd7b92965107446c51a4d086bb2d29dbe)
- `workflow-model.md` (ccb5558b40bf10c70c58a3aa820c508a4bb402824fd956fefac515a36c451574)
- `error-taxonomy.md` (426b0ab72d75c84fc5d557b1e133654d418ea6e4990b8af4994e412924b88d56)
- `boundary-map.md` (d5d030a7c2890106740a74cbc977db3e2fa2d3f8ecaf34026e08869732bd10af)
- `hazard-analysis.md` (805ce34863e9080c8be932c1c58ef4ed48ded655d6086d6d3ccad3a765a40443)
- `contract.md` (dd4d338812807d0031826d05ce822d6fa342c0e7e89466de09ef08b5657daf05)
- `proof-seeds.jsonl` (780b0df9ba2bb1a3bf0ad4ea6c767e97de930d5c23fccbe6edbafe0ec941c118)
- `traceability-matrix.jsonl` (99df14c409b3b9f6e61014fa6cbd8f6cb363dfc19a689cd485dfe4ab1b3ec942)

### State 4 — Proof Plan (`proof-planner-vb-pg2wq-state4` + `proof-plan-reviewer-vb-pg2wq-state4`)
- `proof-strategy.md` (66c1035520c268cf99c523c2c373d968424570c79e3bcc6a1e8b4a2f946bd449)
- `proof-obligations.planned.jsonl` (4d1be51fcd6484be23d246aa62ba849a439886a461727f903807099229715f2c) — 3 rows
- `verifier-lane-decisions.jsonl` (b6a53461143d3dea7ce6f1adb4a420b14cf61acf1e8cdd53212feabd5c0c83c1) — 56 rows
- `verifier-lane-review.jsonl` (02267269023cd5c933280dfc1f43713c4e0c2fdef6110d2314e9ab3c2b4a0ba2) — 56 rows
- `verifier-lane-matrix.md`
- `proof-coverage-matrix.md` (156d49e0e869f376ca6601c971998a6ccb33b123b29259bdc953ea0a0d83bd19)
- `trusted-base-plan.md` (45c3b6cf6af60cef3d5d39a82e9ad73f5c99f0e3b5f2eb0dc043d81a25ac5981)
- `waiver-candidates.jsonl` (e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855) — empty by design
- `proof-plan-review.md` (29f59912898da31695858156e081ebcf86df3eac2ec933b2715149a54e8b013e) — STATUS: APPROVED

### State 11 — Holzman-Rust Implementation (`holzman-rust-vb-pg2wq-state11`)
- `implementation.md` (bb415f3949fa9e6d07ff2fcf40606af24fe4552887c8237af72233fd86e627fe) — 399 lines
- `evidence/unified_diff.txt` (e0ac09062014b02296ba8a25d44eb20619f3cb3464f56804ee55bdbcffe7a0fa) — canonical per-file patches
- `evidence/full_diff.txt` (3bc748137afdfa8d711c1f9a24ee45bc16f1cbcb0b3aab3f6448cfcb76120428) — `jj diff -r '@'` raw output
- `evidence/diff_summary.txt` (60398c6d3b9f1d5754d38ab87971e9f2df7ec1ef8a375f07f7bea7afa1d14591)
- `evidence/cargo_check_vb_storage_lib.log` (00e25b1fb73b0adb1c1a88c04d25de2f619dce9d628818fc9cf0f016d3581ef3)
- `evidence/clippy_vb_storage.log` (caa636ec9c7cba2c4f265005f356629e3a1e8fe35395de581375a782de9931bc)
- `evidence/vb_storage_all_tests.log` (04002fab5ee189554fd0d2a8ea93e71709c1daea4d6af3d430e1be0f38087f27)
- `evidence/test_ps001_duplicate_rejected.log` (cf1cd0fc0081c42aa53503451e0010230a9ebdc03341f450b09affe065774ac7)
- `evidence/test_ps001_full_suite.log` (5cad5e1a46460c920558f603fc27bff7460abeb294c68f92298a3438a2ddc1c5)
- `evidence/test_ps003_dup_fields.log` (4da875621ea76aba5d026ba89b8727a5594261026fe4a1034234a295fb371724)
- `evidence/test_ps003_full_suite.log` (5243290af262f975e0e9d89c1e9924034b3e3b031318cfab77d37279e5ce4381)
- `evidence/test_ps004_no_persist.log` (0525fdcc8d0a728f0cb7caaab6b8d6d3b46633bc889dec666cbd2b5c6c2f8173)
- `evidence/test_ps004_empty_commit_after_rej.log` (2d9e6e1c44af7d61db76e095ac5ab2d1be26851449e43659e0ce03679f196c03)
- `evidence/test_ps004_full_suite.log` (ac1de7f335a5a675a5e3c83ec431a343c95d811d17840961390f0b3f53c6f74a)
- `evidence/test_ps008_dup_before_queue.log` (654d02c4044a12c2eab8d62263dfef0cd4679b4ea9b33e1fdf11048c18616d7f)
- `evidence/test_ps008_full_suite.log` (b548f5639c0bc01d79f4fcff4ad541ddc0b6052be3c4b04461d3ad5362eb6cd0)
- `evidence/test_ps009_dup_rejected.log` (fe1e2597f4454b44ff2d8a27fc4a7252caf97de6e863386a65cacabf892d1a8a)
- `evidence/test_ps009_full_suite.log` (60069406ba998b48da2ba6db8a93aaf9c4ce7b0f022757bedc9cf4e49b377b2e)

### State 12 — Formal Verification (`formal-verifier-vb-pg2wq-state12`)
- `formal-verification-report.md` (0123dd737ddcefbec221db94c13977f6874aae2e71289a390e353a1218875bec)
- `verification-ledger.jsonl` (9d0efc72df7dbacb625d870ed6d384abdd67cb5ed74378dd4e930f3a63304c95) — 3 rows, all PASS
- `formal-waivers.jsonl` (e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855) — empty (0 bytes)
- `evidence/state12_test_ps001_duplicate_rejected.log` (b2626937d56fd7faee6bd92e79c2c5a1abd020b34be05a111f367324d35b423b)
- `evidence/state12_test_ps003_dup_fields.log` (95430c7bab33fdb755bfeed34e49e1206407176bbf639d46bbd3a10a5266f027)
- `evidence/state12_test_ps004_no_persist.log` (f09f25f05edbcd9181f64d680e704e3c217e0c675919722a09e87d0d18495531)
- `evidence/state12_test_ps004_empty_commit_after_rej.log` (ff3587abf839a7f11f79956f0a0395fcf420db2469fb7483501925ff25b65ece)
- `evidence/state12_test_ps008_dup_before_queue.log` (9065284ed6e10abc98783c6ba2ffa2be63f7ef0393d386fa28e03cc126f8983e)
- `evidence/state12_test_ps009_dup_rejected.log` (9f814312aa98be06683cb83922c84ece0e865eef85984f352d36aa85befa5ddd)
- `evidence/state12_vb_storage_all_tests.log` (e06c149c5c74051d107238f51138d2130370441a176cf02c8374bd832b35ea45)
- `evidence/state12_vb_storage_all_tests_full.log` (bb6ce71a4d44ed2cc6ce2704f76b20a8e6247775567d59aab95fb7ca467e124c) — 1766 lines
- `evidence/state12_vb_storage_test_results.txt` (cfa05ff86c22f983b390604b6ae4ed46943b8eca4f8cd145d7b8100c9de39e5f) — 16 rows summing to 1669
- `evidence/state12_weak_pattern_scan.txt` (e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855) — 0 hits
- `evidence/state12_check_test_integrity.log` (8b1b8d5259f1dcb2e7529f7b9e1cf8e6d4677a148523314783398d04b68452f6) — PASS
- `evidence/state12_cargo_fmt.log` (1cd279c1a51dc797d3229469cddfe63cbb7ac0d3a822c421769b497bfd9553b8) — drift in 3 unrelated files (RR-1)
- `evidence/state12_input_artifact_hashes.txt` (53500a79d713bb401867465c1c29b47cf88a86684113afa1bf781883f93f9738)
- `evidence/state12_output_artifact_hashes.txt` (9cc66827e62d935914a3ec291480cd913f5b88c0a6e3abdb8f9c42ccf7327f44)

### State 13 — Black-Hat Review (`black-hat-reviewer-vb-pg2wq-state13`)
- `black-hat-review.md` (3748c2de604dfe67089a2a24785f8cedc1f0a7af877e9770d1ca32da046c8bee) — STATUS: APPROVED
- `defects.md` (8bd77a70bea89b5c4c11d4e0a97029747c6b7ba31d557cca8b42d5e12031855a) — empty

### State 14 — Evidence Packaging (`evidence-packaging-vb-pg2wq-state14`)
- `assurance-bundle.md` (this file)
- `truth-serum-report.md`
- `final-evidence-decision.md` — STATUS: APPROVED

---

## Raw Gate Evidence

### PO-vb-pg2wq-001 (proptest — 4 functions)

```
$ cargo test -p vb_storage --test proptest_vb_vzcuf_PS_001 ps001_duplicate_rejected --no-fail-fast
running 1 test
test ps001_duplicate_rejected ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 6 filtered out; finished in 1.44s

$ cargo test -p vb_storage --test proptest_vb_vzcuf_PS_003 ps003_dup_fields --no-fail-fast
running 1 test
test ps003_dup_fields ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 1.57s

$ cargo test -p vb_storage --test proptest_vb_vzcuf_PS_008 ps008_dup_before_queue --no-fail-fast
running 1 test
test ps008_dup_before_queue ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 1.55s

$ cargo test -p vb_storage --test proptest_vb_vzcuf_PS_009 ps009_dup_rejected --no-fail-fast
running 1 test
test ps009_dup_rejected ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 1.51s
```

### PO-vb-pg2wq-002 (proptest — 2 functions with secondary invariants)

```
$ cargo test -p vb_storage --test proptest_vb_vzcuf_PS_004 ps004_no_persist --no-fail-fast
running 1 test
test ps004_no_persist ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 1.57s

$ cargo test -p vb_storage --test proptest_vb_vzcuf_PS_004 ps004_empty_commit_after_rej --no-fail-fast
running 1 test
test ps004_empty_commit_after_rej ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 1.56s
```

### PO-vb-pg2wq-003 (source-lint umbrella)

```
$ rtk rg -n -- 'JournalError::DuplicateEvent \{ \.\. \}' \
    crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs \
    crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs \
    crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs \
    crates/vb_storage/tests/proptest_vb_vzcuf_PS_008.rs \
    crates/vb_storage/tests/proptest_vb_vzcuf_PS_009.rs
(no output — 0 hits)

$ bash scripts/check-test-integrity.sh
test integrity: PASS base=@-

$ cargo fmt --all --check
[non-zero exit; drift in 3 unrelated files:
  crates/vb_core/src/lib.rs:26
  crates/vb_core/src/time.rs:71
  crates/vb_runtime/src/frame_pool/tests.rs:85, 114, 139
The 5 changed test files are formatting-clean.]
```

### Regression sweep (cargo test -p vb_storage)

```
$ cargo test -p vb_storage --tests --no-fail-fast
test result: ok. 1530 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 29 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 42 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
total: 1669 passed, 0 failed, 16 suites
```

---

## Obligation Closure Roll-up

| Obligation | Risk | Verifier | Classification | Behavior Affecting | Waiver |
|------------|------|----------|----------------|--------------------|--------|
| PO-vb-pg2wq-001 | field_sensitivity | proptest | PASS | false | none |
| PO-vb-pg2wq-002 | field_sensitivity | proptest | PASS | false | none |
| PO-vb-pg2wq-003 | equality | proptest (source-lint) | PASS | false | none |

**All 3 obligations PASS. 0 FAIL. 0 WAIVED. 0 BLOCKED.**

---

## Contract → Proof → Implementation → Verification Bridge

```
contract.md (O1-exact-tuple-pin-and-variant-discriminant, O8-no-forbidden-constructs)
  ↓
proof-obligations.planned.jsonl (3 rows: PO-001, PO-002, PO-003)
  ↓
implementation.md (let-else + assert_eq! pattern, 5 files, 6 occurrences)
  ↓
verification-ledger.jsonl (3 rows, all PASS)
  ↓
formal-verification-report.md (closure roll-up)
  ↓
black-hat-review.md (5 review surfaces PASS, 0 defects)
  ↓
assurance-bundle.md (this file)
```

Every step is anchored to:
- Production contract: `crates/vb_storage/src/batch/append_event.rs:61-67`
- Reference strong pattern: `crates/vb_storage/src/tests.rs:1344-1367` (`fn duplicate_event_returns_exact_run_and_seq`)
- Kani binding-strengthened: `crates/vb_storage/src/kani_vb_vzcuf_ps004.rs:48-59`

---

## Residual Risks

| ID | Description | Disposition |
|----|-------------|-------------|
| RR-1 | Pre-existing `cargo fmt --all --check` drift in 3 unrelated files (vb_core/lib.rs:26, vb_core/time.rs:71, vb_runtime/frame_pool/tests.rs:85/114/139) | Out of scope; 5 changed test files formatting-clean; documented in formal-verification-report.md and black-hat-review.md |
| RR-2 | Pre-existing BLOCK_GLOBAL compile errors in `crates/vb_compile/tests/common/mod.rs` (out of scope; `-p vb_storage` is in scope) | Out of scope; `cargo test -p vb_storage --tests --no-fail-fast` returns 1669 passed |
| RR-3 | Kani binding-strengthened (not re-discharged) | Existing Kani harness at `kani_vb_vzcuf_ps004.rs:48-59` already models `DuplicateEvent { run, seq }` with field-bound guard; runtime↔Kani alignment strengthened by this bead; Kani re-execution is a sibling-bead responsibility |

None are bead defects. None are blocking.

---

## Provenance Chain

```
go-skill-vb-pg2wq-state1                (baseline)
  ↓
explore-vb-pg2wq-state2                 (codebase scout)
  ↓
rust-contract-vb-pg2wq-state3           (9 contract artifacts)
  ↓
proof-planner-vb-pg2wq-state4           (proof-strategy, obligations, lane decisions)
  ↓
proof-plan-reviewer-vb-pg2wq-state4     (STATUS: APPROVED)
  ↓
holzman-rust-vb-pg2wq-state11           (implementation.md + 17 evidence files)
  ↓
formal-verifier-vb-pg2wq-state12        (formal-verification-report.md, verification-ledger.jsonl, formal-waivers.jsonl)
  ↓
black-hat-reviewer-vb-pg2wq-state13     (black-hat-review.md STATUS: APPROVED, defects.md empty)
  ↓
evidence-packaging-vb-pg2wq-state14     (this bundle + truth-serum-report.md + final-evidence-decision.md STATUS: APPROVED)
```

---

## Closure

All planned obligations closed. All review surfaces PASS. All evidence captured. All artifacts SHA-256 hashed.

**Final Status: APPROVED for landing** by femdation controller.