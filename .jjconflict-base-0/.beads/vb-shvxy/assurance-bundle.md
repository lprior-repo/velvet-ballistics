# Assurance Bundle

bead_id: vb-shvxy
source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/isolated/velvet-ballistics-fresh-replacements/vb-shvxy
commit: 46cf615913a97de8dc036113ff593a2c166ba978
title: "Global blocker: restore formal verifier tooling lanes"
state: 14 (evidence-packaging)
date: 2026-05-30

---

## Requirement Coverage

| Requirement | Contract Clause | Proof/Test Evidence | Review Evidence | Status |
|---|---|---|---|---|
| REQ-SHVXY-001: Closed lane identity + typed evidence classification | C-001, C-004 | PO-001..PO-012L (16 obligations over Kani/Flux/Proptest/Fuzz/Loom) | proof-review.md STATUS: APPROVED | **COVERED** |
| REQ-SHVXY-002: Missing tools/scripts/jars fail closed | C-002 | PO-003 (feature gate exit 1), PO-005a/b (selector rejection exit 2), PO-006 (zero-test exit 1) | proof-review.md C-002 covered | **COVERED** |
| REQ-SHVXY-003: Zero applicable tests/harnesses/models rejected | C-003, C-008 | PO-006 (zero-test fail-closed exit 1), PO-007 (5 tests > 0) | proof-review.md C-003/C-008 covered | **COVERED** |
| REQ-SHVXY-004: Kani feature and inventory contracts | C-005 | PO-001 (198 harnesses vb_core), PO-002 (17 harnesses vb_runtime), PO-003 (fail-closed feature gate) | proof-review.md C-005 covered | **COVERED** |
| REQ-SHVXY-005: Flux package wrapper rejects unsupported selectors | C-006 | PO-004 (package smoke PASS), PO-005a/b (selector rejection) | proof-review.md C-006 covered | **COVERED** |
| REQ-SHVXY-006: TLC runner portability and raw evidence | C-007 | Waived via proof-review.md WC-001 (TLA+ globally removed) | proof-review.md C-007 covered by waiver | **WAIVED (accepted)** |
| REQ-SHVXY-007: Cargo-fuzz target preflight and sanitizer triple | C-009 | PO-008 (58 targets registered), PO-009 (all compiled GNU target) | proof-review.md C-009 covered | **COVERED** |
| REQ-SHVXY-008: Loom cfg/dependency parity | C-010 | PO-010 (13 tests passed under cfg(loom)), PO-011 (5 models enumerated) | proof-review.md C-010 covered | **COVERED** |
| REQ-SHVXY-009: Prior capped evidence not reused as fresh pass | C-011, C-012 | All 16 obligations produce fresh evidence from source checkout; no prior vb-ttyc evidence repackaged | proof-review.md C-011/C-012 covered | **COVERED** |

---

## Proof Evidence

| Obligation | Lane | Command | Exit | Applicable Count | Result | Raw Evidence |
|---|---|---|---|---|---|---|
| PO-001 | Kani | `bash scripts/kani-list.sh vb_core` | 0 | 198 harnesses, 29 files | **PASS** | `.evidence/vb-shvxy/po-001-kani-list-vb-core.raw.log`, `.evidence/kani-list/vb_core.json` |
| PO-002 | Kani | `bash scripts/kani-list.sh vb_runtime` | 0 | 17 harnesses, 6 files | **PASS** | `.evidence/vb-shvxy/po-002-kani-list-vb-runtime.raw.log`, `.evidence/kani-list/vb_runtime.json` |
| PO-003 | Kani | `KANI_FEATURES=vb_runtime/kani-diagnostic-codes bash scripts/kani-list.sh vb_runtime` | 1 | 0 (fail-closed) | **PASS** | `.evidence/vb-shvxy/po-003-kani-feature-gate.raw.log` |
| PO-004 | Flux-rs | `bash scripts/flux-check-package.sh vb_core` | 0 | 1 package | **PASS** | `.evidence/vb-shvxy/po-004-flux-check-vb-core.raw.log` |
| PO-005a | Flux-rs | `bash scripts/flux-check-package.sh vb_core --lib` | 2 | 0 (rejected) | **PASS** | `.evidence/vb-shvxy/po-005a-flux-lib-rejection.raw.log` |
| PO-005b | Flux-rs | `bash scripts/flux-check-package.sh vb_core --test` | 2 | 0 (rejected) | **PASS** | `.evidence/vb-shvxy/po-005b-flux-test-rejection.raw.log` |
| PO-006 | Proptest | `bash scripts/guard-zero-tests.sh -- cargo test -p vb_core --test aggregate_resource_budget_properties_red -- nonexistent_filter_xyz` | 1 | 0 (fail-closed) | **PASS** | `.evidence/vb-shvxy/po-006-zero-test-failclosed.raw.log` |
| PO-007 | Proptest | `bash scripts/guard-zero-tests.sh -- cargo test -p vb_core --test aggregate_resource_budget_properties_red` | 0 | 5 tests | **PASS** | `.evidence/vb-shvxy/po-007-proptest-nonvacuous.raw.log` |
| PO-008 | Cargo-fuzz | `cargo fuzz list` | 0 | 58 targets | **PASS** | `.evidence/vb-shvxy/po-008-fuzz-list.raw.log` |
| PO-009 | Cargo-fuzz | `cargo fuzz build --target x86_64-unknown-linux-gnu` | 0 | 58 compiled | **PASS** | `.evidence/vb-shvxy/po-009-fuzz-build-gnu.raw.log` |
| PO-010 | Loom | `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime --lib -- models::loom` | 0 | 13 tests | **PASS** | `.evidence/vb-shvxy/po-010-loom-execution.raw.log` |
| PO-011 | Loom | `bash scripts/loom-list.sh` | 0 | 5 models | **PASS** | `.evidence/vb-shvxy/po-011-loom-list.raw.log` |
| PO-012K | Kani (closure) | PO-001 + PO-002 + PO-003 | N/A | 215 harnesses + 1 fail-closed gate | **PASS** | formal-verification-report.md §Cross-Cutting Closure |
| PO-012F | Flux-rs (closure) | PO-004 + PO-005a/b | N/A | 1 package smoke, 2 selectors rejected | **PASS** | formal-verification-report.md §Cross-Cutting Closure |
| PO-012P | Proptest (closure) | PO-006 + PO-007 | N/A | 5 tests, zero-test guard operational | **PASS** | formal-verification-report.md §Cross-Cutting Closure |
| PO-012C | Cargo-fuzz (closure) | PO-008 + PO-009 | N/A | 58 targets registered + compiled | **PASS** | formal-verification-report.md §Cross-Cutting Closure |
| PO-012L | Loom (closure) | PO-010 + PO-011 | N/A | 13 tests + 5 models | **PASS** | formal-verification-report.md §Cross-Cutting Closure |

**Total: 16/16 PASS (11 direct + 5 closure). 0 FAIL_LOCAL. 0 FAIL_GLOBAL. 0 BLOCKED.**

---

## Test Evidence

| Test/Gate | Command/Artifact | Result |
|---|---|---|
| Proof review gate | proof-review.md (State 6) | **STATUS: APPROVED** — all 11 direct obligations PASS, 5 closure deferred to State 10 |
| Test plan review gate | test-plan-review.md (State 8/10) | **STATUS: APPROVED** — 37 behaviors, 51 bash tests, 4 fuzz targets, 20 mutation checkpoints |
| Test suite review gate | test-review.md (State 10, attempt 2) | **STATUS: APPROVED** — all 8 prior findings resolved, zero structural greps, 20/20 mutation kill rate |
| Behavior tests (State 9, attempt 2) | 9 bash test files (917 lines), 4 fuzz targets | All 9 behavioral tests rewritten from structural greps, B020 failure propagation proven, 100% mutation kill |
| 51 integration tests PASS | Evidence from test-writer-report-attempt2.md | 51 bash integration tests pass across kani-list, flux-check, guard-zero-tests, proptest, fuzz, loom scripts |

---

## Review Evidence

| Review | Artifact | Status | Findings |
|---|---|---|---|
| Proof review (State 6) | `.beads/vb-shvxy/proof-review.md` | **APPROVED** | 0 BLOCKER, 3 WARN (pipefail fragility, untracked files, trust dispositions), 3 INFO |
| Test plan review (State 10) | `.beads/vb-shvxy/test-plan-review.md` | **APPROVED** | Accepted plan structure; 1 INFO (THIN-STATIC-STRUCTURAL flagged for State 10 suite reviewer) |
| Test suite review (State 10, attempt 2) | `.beads/vb-shvxy/test-review.md` | **APPROVED** | 0 CRITICAL, 0 WARN, 2 INFO (hardcoded /tmp path, C-007 plan-level gap) — neither blocks approval |
| Black-hat review | N/A | **SKIPPED** | Tooling bead — per instructions, State 13 black-hat not required; tooling infrastructure has no production Rust behavior changes |

---

## Waivers And Deferred Work

| Item | Reason | Owner | Compensating Evidence |
|---|---|---|---|
| C-007 TLC portability (WC-001) | TLA+ globally removed from verifier fleet; TLC runner portability out of scope | proof-reviewer | Accepted in proof-review.md §Contract Clause Coverage |
| State 13 black-hat review | Tooling bead — no production Rust behavior, no unsafe, no performance claims | femdation | Evidence packaging at State 14 per femdation directive |
| machine-gate-report.md | Not produced for this bead — tooling bead does not require machine-gate review | femdation | 16/16 formal verification PASS covers all gate checks |
| regression-diff.md | Not produced — bead vb-shvxy is pure tooling infrastructure; no regression surface | femdation | All verifier lane commands produce deterministic exit codes and counts |

---

## Evidence Inventory

See `evidence-inventory.jsonl` for machine-readable enumeration of all 12 raw evidence files, 3 JSONL ledgers, and 4 review artifacts.

---

## Truth Serum Audit

- report: `.beads/vb-shvxy/truth-serum-report.md`
- status: **APPROVED**

12/12 raw evidence files verified on disk. 9/9 requirements trace to evidence. 3 waived gate artifacts with compensating evidence. 0 hallucinated claims.

---

## Final Evidence Decision

- report: `.beads/vb-shvxy/final-evidence-decision.md`
- status: **STATUS: APPROVED**

All 16 tooling obligations (PO-001 through PO-012L) pass with non-vacuous, fail-closed, fresh evidence. Every contract clause (C-001 through C-012) maps to verified proof/test/review evidence or an accepted waiver. No blockers, no unresolved findings, no hallucinated evidence. Global verifier tooling blocker RESOLVED.

---

*Assurance bundle generated by evidence-packaging agent (deepseek-v4-pro) on 2026-05-30. All evidence sourced from existing artifacts in the isolated workspace. No new correctness claims created during packaging.*
