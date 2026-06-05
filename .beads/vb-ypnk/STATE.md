# STATE.md — vb-ypnk: quality: Add evidence bundle format and writers

## Bead
- **ID**: vb-ypnk
- **Title**: quality: Add evidence bundle format and writers
- **Status**: in_progress
- **Source checkout**: /home/lewis/src/velvet-ballistics
- **Isolated workspace**: /home/lewis/src/velvet-work/go-skill-vb-ypnk
- **Isolation verified**: YES (workspace path not inside source checkout)
- **Claimed**: 2026-05-18

## Current State: State 12 COMPLETE — Black-hat review REJECTED
- Findings: 1 CRITICAL (lossy PathBuf→String conversion in GateEvidencePostcard::from_gate), 2 HIGH (4 public API functions >25 lines, bundle.rs 505 lines >300 limit), 1 MEDIUM (OBL-008 Miri harness missing), 2 LOW (unwrap_or masking)
- CRITICAL: bundle.rs:442 — GateEvidencePostcard::from_gate uses log.to_string_lossy() which silently corrupts non-UTF-8 paths on deserialization (violates INV-006 and R-012)
- Required fix: Use OsString wire type or add explicit UTF-8 validation
- Workspace created: go-skill-vb-ypnk
- Baseline captured: moon ci --force shows 7 completed, 5 failed, 11 skipped
- Pre-existing compile errors in: xtask (4), vb_cli test (2), vb_storage test (21)
- Formal verification report: .beads/vb-ypnk/formal-verification-report.md

## State Progression
| State | Status | Notes |
|-------|--------|-------|
| 1 | COMPLETE | Claim + isolate + baseline — workspace at /home/lewis/src/velvet-work/go-skill-vb-ypnk |
| 2 | COMPLETE | Explore — codebase-map.md + delivery-scope.jsonl written |
| 3 | COMPLETE | Contract — contract.md written (12 requirements, 7 invariants, 5 types) |
| 4 | COMPLETE | Proof plan — proof-strategy.md + proof-plan-review-input.jsonl + proof-obligations.planned.jsonl (8 obligations, all required) |
| 5 | COMPLETE | Proof write — Kani harnesses, proptest properties, Miri tests, reports written. All compile. |
| 6 | COMPLETE | Proof review — Kani codegen pass OBL-001–004; proptest OBL-005–007 (10 tests); Miri OBL-008 pending |
| 7 | COMPLETE | Test plan — 18 behaviors, 18 BDD scenarios, 9 proptest invariants, 6 mutation checkpoints, coverage matrix |
| 8 | COMPLETE | Test write — 20 gap tests written, 29 total pass |
| 9 | COMPLETE | Test review — REJECTED: F-001 (B-015 error path not exercised), F-002 (B-001 not explicit), F-003 (B-010 not explicit) |
| 10 | PENDING | Implementation |
| 11 | COMPLETE | Execute gates — OBL-001-004: Kani codegen_pass (waived); OBL-005-007: proptest PASS; OBL-008: FAIL_LOCAL (missing test harness) |
| 12 | COMPLETE (REJECTED) | Black-hat review — 1 CRITICAL (lossy PathBuf conversion), 2 HIGH (function line limits), 1 MEDIUM (OBL-008 missing), 2 LOW findings |
| 13 | PENDING | Evidence + truth-serum |
| 14 | PENDING | Landing |
| 15 | PENDING | Cleanup |

## Retry Budget
- Total attempts remaining: 1
- Last failure: State 12 — CRITICAL: lossy PathBuf conversion (bundle.rs:442), HIGH: 4 functions >25 lines, bundle.rs >300 lines
- Next repair target: Fix lossy PathBuf conversion in GateEvidencePostcard::from_gate; refactor 4 public API functions to ≤25 lines; split bundle.rs ≤300 lines; write Miri harness for OBL-008

## Risks
- Pre-existing compile errors in xtask, vb_cli, vb_storage may affect evidence capture scope
- Bead depends on vb-6f02 (check if resolved)
- CRITICAL FINDING-1: GateEvidencePostcard::from_gate is lossy for non-UTF-8 PathBuf — postcard round-trip violates INV-006
- BLOCKER: 4 public API functions exceed 25-line Farley limit (parse_bundle_schema_version:58, validate_bundle:43, write_bundle:48, read_bundle:31)
- BLOCKER: bundle.rs at 505 lines exceeds 300-line architectural limit
