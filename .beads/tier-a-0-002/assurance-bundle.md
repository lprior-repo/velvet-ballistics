STATUS: APPROVED
reviewer_skill: evidence-packaging
reviewer_invocation_id: tier-a-0-002-s14-evidence-packaging-gpt55
writer_invocation_id: tier-a-0-002-s14-truth-serum-gpt55
parent_invocation_id: tier-a-0-002-s14-truth-serum-gpt55
parent_entry_hash: 25221c19c83e31358e575602876a648574fb916c1ca9d7a8e838059e6cdf8d6a
bead_id: tier-a-0-002
bead_title: cli: install residue quarantine CI gate via moon ci
state: 14 evidence-packaging
workspace: /home/lewis/src/femdation-tier-a-0-002
source_checkout: /home/lewis/src/velvet-ballistics
artifact_root: .beads/tier-a-0-002
generated_at_utc: 2026-06-18T08:37:04Z
model: openai/gpt-5.5

# Assurance Bundle — tier-a-0-002

## Disposition

Approved for the local residue-quarantine CI-gate scope. The evidence proves the targeted `forbid-runtime-fmt` gate, its self-tests, its Moon task wiring, its source binding, and its review chain. The broader `moon run :check` remains red because of an unrelated `check-removed-crate-residue` `vb_codegen` hit; that is carried as a residual global blocker and is not used as local closure evidence.

## Required Artifact Inventory

| Artifact | Status | Evidence |
|---|---:|---|
| `delivery-scope.jsonl` | present / JSONL | 13 scoped artifacts, lines 1-13 |
| `contract.md` | present | Contract clauses §3.2, §3.3, §3.4, §3.5, §6, §8 |
| `traceability-matrix.jsonl` | present / JSONL | 20 rows, `TM-001`..`TM-020` |
| `proof-review.md` | approved | first nonblank `STATUS: APPROVED`; State 6 accepts State 5 proof outputs |
| `test-plan-review.md` | approved | first nonblank `STATUS: APPROVED`; repaired test plan closure matrix lines 20-28 |
| `test-suite-review.md` | approved | first nonblank `STATUS: APPROVED`; red tests accepted for State 11 repair |
| `formal-verification-report.md` | pass | first nonblank `STATUS: PASS`; required PO/RRO closure lines 9-17 |
| `verification-ledger.jsonl` | present / JSONL | 18 rows: 15 PASS, 1 FAIL_GLOBAL, 2 FAIL_LOCAL audit rows |
| `machine-gate-report.md` | pass with stale State 12 note | first nonblank `STATUS: PASS`; line 24 is superseded by later State 13 re-review |
| `regression-diff.md` | pass with global note | first nonblank `STATUS: PASS`; lines 7-9 classify scoped regression PASS and global Moon blocker |
| `black-hat-review.md` | approved | first nonblank `STATUS: APPROVED`; BH-001..BH-005 closed lines 36-42 |
| `truth-serum-report.md` | approved | first nonblank `STATUS: APPROVED`; raw rerun evidence lines 33-180 |

## Proof / Refinement / Source / Command Map

| Req | Contract | Proof / RRO | Source refs | Test / command evidence | Result |
|---|---|---|---|---|---:|
| `RQ-001` | `contract.md` §3.2 pass iff no active residue | `PO-RQ-001`, `RRO-RQ-001`; `verification-ledger.jsonl` rows `VL-PO-RQ-001`, `VL-RRO-RQ-001` | `scripts/forbid-runtime-fmt.rs` lines 692-699, 809-815; `scripts/forbid-runtime-fmt.sh` lines 31-76 | `bash scripts/test-forbid-runtime-fmt.sh test_quarantine_gate_blocks_json_import`; raw log `evidence/state12-repair-po-rq-001.log`; full suite log lines 4-7 | PASS |
| `RQ-002` | `contract.md` §3.4 closed set / master parity | `PO-RQ-002`, `RRO-RQ-002`; `VL-PO-RQ-002`, `VL-RRO-RQ-002` | `ForbiddenImportName` lines 20-67; `ForbiddenImport::from_name` lines 90-108; `master_line_matches` / `expected_master_trigger` lines 551-573 | `bash scripts/test-forbid-runtime-fmt.sh test_static_evidence_binds_master_rejection_triggers`; raw log `evidence/state12-repair-rro-rq-002.log`; full suite lines 21-22 | PASS |
| `RQ-003` | `contract.md` §3.2 exit-code correctness | `PO-RQ-003`, `RRO-RQ-003`; `VL-PO-RQ-003`, `VL-RRO-RQ-003` | `GateDecision::exit_code` lines 654-665; `ResidueQuarantine::decide` lines 809-815; unbounded matcher lines 979-990 | `bash scripts/test-forbid-runtime-fmt.sh test_quarantine_gate_blocks_unbounded_channel`; raw log `evidence/state12-repair-po-rq-003.log`; full suite lines 8-13 | PASS |
| `RQ-004` | `contract.md` §3.4 allowlist precedence and §3.5 Moon wiring | `PO-RQ-004`, `RRO-RQ-004`; `VL-PO-RQ-004`, `VL-RRO-RQ-004` | `diff_against_allowlist` lines 788-806; `.moon/tasks/all.yml` lines 105-120 and 122-139; wrapper timeout lines 34 and 62 | `bash scripts/test-forbid-runtime-fmt.sh test_moon_ci_quarantine_dependency_correctly_ordered`; raw log `evidence/state12-repair-po-rq-004.log`; full suite lines 14-20 | PASS |
| `RQ-005` | `contract.md` §3.3 deterministic stderr/stdout format | `PO-RQ-005`, `RRO-RQ-005`; `VL-PO-RQ-005`, `VL-RRO-RQ-005` | `active_line` lines 271-279; `allowlisted_line` lines 281-289; `summary_line` lines 634-643; `emit_pass`/`emit_fail` lines 675-690; wrapper `sort -u` line 70 | `bash scripts/test-forbid-runtime-fmt.sh test_static_evidence_binds_real_formatter_symbols`; raw log `evidence/state12-repair-rro-rq-005.log`; truth-serum deterministic replay lines 135-164 | PASS |

No separate Verus/Kani/Flux/Loom/Miri/fuzz harness is claimed. The approved plan and ledger classify this build-time scanner as execution-bound, with executable bash tests and source-bound static checks as the closing evidence.

## Traceability Matrix Coverage (`TM-001`..`TM-020`)

| TM row | Requirement | Contract source | Proof/test/source/command mapping | Disposition |
|---|---|---|---|---:|
| `TM-001` | `R-S2-JSON` / `serde_json` | master §2 line 99; contract §3.2 | `RQ-001`; `test_quarantine_gate_blocks_json_import`; `ForbiddenImportName::SerdeJson` lines 20-52; raw `state12-repair-po-rq-001.log` | covered |
| `TM-002` | `R-S2-HTTP` / `hyper` | master §2 line 100; contract §3.4 | `RQ-002`; `ForbiddenImportName::Hyper` lines 23, 47 and `from_name` line 98; CrateName classifier line 122; static test `state12-repair-rro-rq-002.log`; direct active-free scan `state12-repair-forbid-runtime-fmt-direct.log` | covered |
| `TM-003` | `R-S2-HTTP` / `reqwest` | master §2 line 100; contract §3.4 | `RQ-002`; `ForbiddenImportName::Reqwest` lines 24, 48 and `from_name` lines 99-101; CrateName classifier line 122; static test `state12-repair-rro-rq-002.log`; direct active-free scan | covered |
| `TM-004` | `R-S2-HTTP` / `axum` | master §2 line 100; contract §3.4 | `RQ-002`; `ForbiddenImportName::Axum` lines 25, 49 and `from_name` line 102; CrateName classifier line 122; static test `state12-repair-rro-rq-002.log`; direct active-free scan | covered |
| `TM-005` | `R-S2-MAP` / `HashMap<String,_>` | master §2 line 102; contract §3.4 | `RQ-002` + `RQ-005`; enum line 26 / string line 50 / source match line 124; static test `state12-repair-rro-rq-002.log`; formatter binding `state12-repair-rro-rq-005.log`; direct active-free scan | covered |
| `TM-006` | `R-S2-UNB` / unbounded mpsc | master §2 line 97; contract §3.2 | `RQ-003`; direct, grouped, and spaced unbounded forms in `test_quarantine_gate_blocks_unbounded_channel`; matcher lines 979-990; raw `state12-repair-po-rq-003.log` | covered |
| `TM-007` | `R-S12-YAML` / `serde_yaml` | master §12 line 421; contract §3.4 | `RQ-002`; `ForbiddenImportName::SerdeYaml` lines 22, 46 and `from_name` lines 95-97; CrateName classifier line 122; static test `state12-repair-rro-rq-002.log`; direct active-free scan | covered |
| `TM-008` | §43 trigger 7 allocation behavior | master §43 line 2038 | `RQ-003` unbounded gate plus `RQ-004` allowlist/Moon ordering; source lines 979-990 and `.moon/tasks/all.yml` lines 122-139; raw logs `state12-repair-po-rq-003.log`, `state12-repair-po-rq-004.log` | covered |
| `TM-009` | §43 trigger 8 hot-path behavior | master §43 line 2039 | All five RQ rows; four hot crates are closed by `HotCrateName::all` lines 133-153 and Moon inputs lines 114-117; full self-test `state12-repair-test-forbid-runtime-fmt-all.log` | covered |
| `TM-010` | §43 trigger 9 Fjall persistence if touched (`vb_storage`) | master §43 line 2040 | `RQ-002` source/master binding for JSON/YAML plus four-hot-crate scope (`HotCrateName::VbStorage` lines 137, 150; Moon input line 116); direct active-free scan | covered |
| `TM-011` | §43 trigger 10 IPC if touched (`vb_ipc`) | master §43 line 2041 | `RQ-002` source/master binding for HTTP variants plus four-hot-crate scope (`HotCrateName::VbIpC` lines 138, 151; Moon input line 117); direct active-free scan | covered |
| `TM-012` | §44.6 JSON/HTTP absent from hot crates | master §44 line 2078 | Direct command `bash scripts/forbid-runtime-fmt.sh` raw `state12-repair-forbid-runtime-fmt-direct.log` lines 1-6 and Moon command `state12-repair-moon-forbid-runtime-fmt.log` lines 1-12 | covered |
| `TM-013` | §78 Tier A gate `scripts/forbid-runtime-fmt.sh exit 0` | master §78 line 6147 | Direct gate raw log lines 1-6; Moon task raw log lines 1-12; verification ledger rows `VL-AUX-DIRECT-FORBID-RUNTIME-FMT` and `VL-AUX-MOON-FORBID-RUNTIME-FMT` | covered |
| `TM-014` | Moon dependency ordering | contract §3.5 | `RQ-004`; `.moon/tasks/all.yml` lines 105-120 and 122-139; `test_moon_ci_quarantine_dependency_correctly_ordered`; raw `state12-repair-po-rq-004.log` | covered |
| `TM-015` | sibling removed-crate gate reference | contract §3.5 reference only | Out of local scope by `traceability-matrix.jsonl` status `out_of_scope_but_referenced`; non-closing reference only. The global removed-crate gate is red on a separate `vb_codegen` residue in `state12-repair-moon-check.log` lines 874-901. | explicit nonlocal blocker |
| `TM-016` | sibling removed-feature gate reference | contract §3.5 reference only | Out of local scope by `traceability-matrix.jsonl` status `out_of_scope_but_referenced`; the sibling gate itself reports active=0 in global Moon log lines 902-908. | referenced |
| `TM-017` | sibling hot/cold forbidden APIs reference | contract §3.5 reference only | Out of local scope by `traceability-matrix.jsonl` status `out_of_scope_but_referenced`; sibling gate reports `violations=0` in global Moon log line 872. | referenced |
| `TM-018` | bead test: JSON import blocks | contract §8 test 1 | `RQ-001`; `scripts/test-forbid-runtime-fmt.sh` lines 664-681; raw `state12-repair-po-rq-001.log` and full suite lines 4-7 | covered |
| `TM-019` | bead test: unbounded channel blocks | contract §8 test 2 | `RQ-003`; `scripts/test-forbid-runtime-fmt.sh` lines 683-723; raw `state12-repair-po-rq-003.log` and full suite lines 8-13 | covered |
| `TM-020` | bead test: Moon dependency ordered | contract §8 test 3 | `RQ-004`; `scripts/test-forbid-runtime-fmt.sh` lines 725-769; raw `state12-repair-po-rq-004.log` and full suite lines 14-20 | covered |

## Raw Command Evidence Summary

| Command | Exit | Raw evidence | Classification |
|---|---:|---|---|
| `bash scripts/test-forbid-runtime-fmt.sh` | 0 | `evidence/state12-repair-test-forbid-runtime-fmt-all.log` lines 1-27 | local gate self-test PASS |
| `bash scripts/forbid-runtime-fmt.sh` | 0 | `evidence/state12-repair-forbid-runtime-fmt-direct.log` lines 1-6 | targeted residue gate PASS (`active=0`) |
| `moon run :forbid-runtime-fmt` | 0 | `evidence/state12-repair-moon-forbid-runtime-fmt.log` lines 1-12 | targeted Moon task PASS (`active=0`) |
| `rustup run nightly-2026-04-28 rustfmt --edition 2024 --check scripts/forbid-runtime-fmt.rs` | 0 | `evidence/state12-repair-rustfmt-nightly-edition2024-forbid-runtime-fmt-rs.log` | scanner formatting PASS |
| `rustup run nightly-2026-04-28 rustc --edition=2024 -D warnings scripts/forbid-runtime-fmt.rs -o target/gate-tools/forbid-runtime-fmt-rustc-check` | 0 | `evidence/state12-repair-rustc-forbid-runtime-fmt-rs.log` | standalone compile PASS |
| `timeout 120s moon run :check` | 1 | `evidence/state12-repair-moon-check.log` lines 1-3, 41-42, 874-901, 914-920 | FAIL_GLOBAL: unrelated removed-crate residue |

## Review Evidence And Findings Disposition

| Review / finding | Severity | Source artifact | Canonical disposition | Evidence |
|---|---:|---|---|---|
| Proof plan review | n/a | `proof-plan-review.md` | owner_approved_no_action for non-blocking JSONL documentation gap | Review approved; later `verifier-lane-decisions.jsonl` has 33 rows including not-applicable rows |
| `PF-RQ-001` | observation | `proof-findings.jsonl` | owner_approved_no_action | `proof-review.md` lines 191-223 |
| Test-plan prior blockers | blocker | `test-plan-review.md` lines 20-28 | fixed_with_evidence | Repaired test plan covers hot roots, GateError scenarios, allowlist precedence, exact diagnostics, and hard timeout |
| `BH-001` grouped/spaced unbounded bypass | CRITICAL | `black-hat-review.md` lines 30, 38, 50-52 | fixed_with_evidence | `test_quarantine_gate_blocks_unbounded_channel` PASS; source matcher lines 979-990 |
| `BH-002` wrong master binding | CRITICAL | `black-hat-review.md` lines 31, 39, 51 | fixed_with_evidence | `test_static_evidence_binds_master_rejection_triggers` PASS; source lines 90-108 and 551-573 |
| `BH-003` nonexistent formatter symbol | HIGH | `black-hat-review.md` lines 32, 40, 54 | fixed_with_evidence | `test_static_evidence_binds_real_formatter_symbols` PASS; source lines 271-289, 634-690 |
| `BH-004` unbounded compile | HIGH | `black-hat-review.md` lines 33, 41, 53 | fixed_with_evidence | wrapper `timeout 30s` lines 34 and 62; Moon order test PASS |
| `BH-005` function size | MEDIUM | `black-hat-review.md` lines 34, 42, 73-80 | fixed_with_evidence | function-size table in re-review; rustfmt/rustc logs PASS |
| Truth-serum residuals | n/a | `truth-serum-report.md` lines 226-237 | owner_approved_debt for residual risks outside local gate | local evidence approved; global Moon blocker and line-scanner limitation disclosed |

## Waivers, Residual Risks, And Nonlocal Blockers

| Item | Status | Owner / follow-up | Compensating evidence |
|---|---:|---|---|
| Formal waivers | none | n/a | `verification-ledger.jsonl` rows have `formal_waiver_id=null`; `formal-verification-report.md` line 48 says waivers none |
| Project-wide `moon run :check` | residual global blocker | outside tier-a-0-002 local residue scope; route separately if not already tracked | Local `forbid-runtime-fmt` passes before the global failure (`state12-repair-moon-check.log` lines 41-42); failure is `check-removed-crate-residue` active `vb_codegen` at lines 874-901 |
| Conservative line scanner, not full Rust parser | residual risk | future bead if new syntax form bypasses closed patterns | grouped and spaced unbounded forms are now tested; source matcher lines 979-990 |
| Master-line drift fail-closed | residual maintenance risk | update scanner refs and evidence together on master edits | `expected_master_trigger` and `master_line_matches` lines 551-573; RQ-002 static test PASS |

## Truth-Serum Audit

- Report: `.beads/tier-a-0-002/truth-serum-report.md`
- Status: APPROVED
- Invocation: `tier-a-0-002-s14-truth-serum-gpt55`
- Raw reruns: artifact/ledger audit, syntax/static scan, full self-test, direct gate, Moon targeted gate, rustfmt, rustc, deterministic replay, and bounded global Moon check are recorded in lines 33-180.

## Final Evidence Decision Input

This bundle is the evidence map consumed by `.beads/tier-a-0-002/final-evidence-decision.md`. It does not claim the global `moon run :check` passed; it approves only the local residue-quarantine gate scope with the residual global blocker disclosed.
