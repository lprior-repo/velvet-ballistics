# Assurance Bundle

bead_id: vb-t0iw9
source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-t0iw9
commit_or_change: rsvywymk 1d6c017f (AGENTS.md round10 forward-port) → qmpnxvym 20f731c6 (working copy, empty)
bead_type: BUG (P1)
chosen_repair: Option C — DocumentExpectedUserAction
bead_closure: DEFERRED_TO_USER_ACTION (not by this delivery)

## Requirement Coverage

| Requirement | Contract Clause | Proof/Test Evidence | Review Evidence | Status |
|---|---|---|---|---|
| REQ-T0IW9-001 — Dispatch-sandbox probe captures bd binary path, version, env | OB-001 | `evidence/bd-version.txt` (sha256 `be7341f3a07ecbf248de6e8d29753ef8140327af93b2016672211c4ff8781dae`); `evidence/workspace-gate.txt` (sha256 `30d20a472ad1d79001add43a438b46e9ca8f7f56f1c691538d7a4ec13104f4e9`) | proof-plan-review.md § Per-Seed Coverage Audit (PO-T0IW9-001 accepted); black-hat-review.md § Reviewed Artifacts | COVERED_BY_EVIDENCE |
| REQ-T0IW9-002 — Live schema introspection | OB-002 | `evidence/schema-before.txt` (sha256 `fc20435f1c64479990996c3759aed230970337a123e80a2aef45d3d59ab2dcf6`); `evidence/schema-migrations.txt` (sha256 `eb7f84f471c661bbb8c1bb30ba6aeb2780d0c5206a0818dd01e04dbe650bdb82`) | proof-plan-review.md § Per-Seed Coverage Audit (PS-T0IW9-002 transitively via PO-T0IW9-005); black-hat-review.md § Contract Parity Audit | COVERED_BY_EVIDENCE |
| REQ-T0IW9-003 — Reproduction re-invokes failing bd subcommand | OB-003 | `evidence/repro.txt` (sha256 `52651eefe5d270031c092ebf901ffc4965165b44551a8776e6c2e89238388a2a`); `evidence/supersede-flag.txt` (sha256 `067e5f0113d1bbb9dca32861618352823b4ab23e705b183ded05c33ec33e87bd`) | proof-plan-review.md § Per-Seed Coverage Audit (PO-T0IW9-002 accepted); black-hat-review.md § Bitter Truth Audit | COVERED_BY_EVIDENCE |
| REQ-T0IW9-004 — RepairDecision selected from closed table | OB-004 | `implementation.md` § Why Option C enumerates A/B/C + type-contracts.md § Repair decision table classification `DocumentExpectedUserAction { recipe: ... }` | proof-plan-review.md § Per-Seed Coverage Audit (PS-T0IW9-004 transitively via PO-T0IW9-005 anti-invariant `VerificationFailed`); black-hat-review.md § Strict DDD Audit | COVERED_BY_EVIDENCE |
| REQ-T0IW9-005 — Server-mode preservation | OB-005 | `evidence/check-beads-server-mode.txt` (sha256 `a62c2adbc160dfdc5d65ffa644357a69af2b06fd80d7259f9504be660003ab78`); verification-ledger.jsonl row 1 (OBL-T0IW9-S12-001, PASS, sha256 `d87ac6c7588030ce3319b9c9e66411a4bd19fe72e1748e55f37adaeb193a70db`) | proof-plan-review.md § Risk Class Coverage (covered transitively by PO-T0IW9-005); black-hat-review.md § Holzman Rust Audit (N/A for production Rust; explicit for Markdown/shell) | COVERED_BY_VERIFICATION_GATE |
| REQ-T0IW9-006 — STORED-column respect (depends_on_id) | OB-006 | `evidence/bd-version.txt` (migrations 0041-0042 STORED-generation); trusted-base-plan.md § TB-T0IW9-depends-on-id-stored-generation | proof-plan-review.md § Migrations 0041-0042 STORED-Generation Contract Preservation (4 layers); black-hat-review.md § Strict DDD Audit | COVERED_BY_TRUST_MARKER |
| REQ-T0IW9-007 — Config precedence honored | OB-007 | `evidence/port-drift.txt` (sha256 `b1dc3f266aa0c689e27892cf7d7fe1c8b56caa5e2b33bd2202e8dccc7888e2b0`); trusted-base-plan.md § TB-T0IW9-beads-config-precedence; `runbook.md` § Action A + Action B do not mix layers | proof-plan-review.md § Per-Seed Coverage Audit (PO-T0IW9-003 accepted); black-hat-review.md § Strict DDD Audit | COVERED_BY_EVIDENCE |
| REQ-T0IW9-008 — Git-cleanliness | OB-008 | `evidence/workspace-gate.txt` (jj status shows no .beads/dolt changes; no .beads/backup changes); runbook.md § MUST NOT do item 4 forbids git-adding runtime paths | proof-plan-review.md § Risk Class Coverage; black-hat-review.md § Strict DDD Audit | COVERED_BY_EVIDENCE |
| REQ-T0IW9-009 — Post-repair verification re-runs | OB-009 | verification-ledger.jsonl rows 1-3 (3 verification gates, all PASS); `evidence/state12-beads-server-mode.txt` (sha256 `16f48530d9fad86f4a934d02ccae646f9746be4a177138890fb8490d972828f3`); `evidence/state12-embeddeddolt-absent.txt` (sha256 `b186acaf36eb9979a6a89b6046cad55ea17458f2f3291d66f73a4e2a5131b86d`); `evidence/state12-bead-claim-state.json` (sha256 `6b60a75173596a0e969f9d671f170b7c02489e5cabb2eb45d4c9b2ff74cccf81`) | proof-plan-review.md § Per-Seed Coverage Audit (PO-T0IW9-005 accepted); black-hat-review.md § Contract Parity Audit | COVERED_BY_VERIFICATION_GATE |
| REQ-T0IW9-010 — Failure routing | OB-010 | `implementation.md` § Residual Risk enumerates 5 failure modes; `runbook.md` § MUST NOT do + § Residual Risk + verification re-run | proof-plan-review.md § Risk Class Coverage (PS-T0IW9-010 transitively via PO-T0IW9-005 anti-invariant `Escalate`); black-hat-review.md § Anti-Invariants | COVERED_BY_DOCUMENTATION |

## Proof Evidence

This bead is **non-behavior-affecting** in the implementation sense: zero
production Rust is touched. The five `proof-obligations.planned.jsonl` rows
(PO-T0IW9-001..005) target hypothetical Rust parsers and harnesses that do
not exist (per `codebase-map.md §71`, `delivery-scope.jsonl:touched_crates`
is the empty list, `proof-strategy.md §1`).

| Obligation | Tool | Command | Artifact | Result | Waiver |
|---|---|---|---|---|---|
| PO-T0IW9-001 | proptest | `PROPTEST_CASES=64 cargo test --test bd_version_capture --release` | `tests/proptest/bd_version_capture.rs` (does not exist) | PENDING_NO_TARGET (non-behavior; no Rust surface) | n/a |
| PO-T0IW9-002 | cargo-fuzz | `cargo fuzz run SchemaErrorClass_parse -max_total_time=120` | `fuzz/SchemaErrorClass_parse_fuzz.rs` (does not exist) | PENDING_NO_TARGET (non-behavior; no Rust surface) | n/a |
| PO-T0IW9-003 | cargo-fuzz | `cargo fuzz run BeadsConfig_BeadsMetadata_parse -max_total_time=120` | `fuzz/BeadsConfig_BeadsMetadata_fuzz.rs` (does not exist) | PENDING_NO_TARGET (non-behavior; no Rust surface) | n/a |
| PO-T0IW9-004 | cargo-fuzz | `cargo fuzz run AddSchemaMigration_statement -max_total_time=120` | `fuzz/AddSchemaMigration_statement_fuzz.rs` (does not exist) | PENDING_NO_TARGET (non-behavior; no Rust surface) | n/a |
| PO-T0IW9-005 | proptest | `PROPTEST_CASES=16 cargo test --test bd_post_repair_verification --release` | `tests/proptest/bd_post_repair_verification.rs` (does not exist) | PENDING_NO_TARGET (non-behavior; no Rust surface) | n/a |
| OBL-T0IW9-S12-001 | bash (verification gate) | `bash scripts/check-beads-server-mode.sh` | `evidence/state12-beads-server-mode.txt` | PASS (exit 0; assertions 5/5 green) | n/a |
| OBL-T0IW9-S12-002 | bash (verification gate) | `test ! -e .beads/embeddeddolt` | `evidence/state12-embeddeddolt-absent.txt` | PASS (exit 0; .beads/embeddeddolt/ absent) | n/a |
| OBL-T0IW9-S12-003 | bash (verification gate) | `bd show vb-t0iw9 --json` | `evidence/state12-bead-claim-state.json` | PASS (exit 0; status=in_progress, priority=1, dependent_count=1) | n/a |

The five `proof-obligations.planned.jsonl` rows are classified
`PENDING_NO_TARGET` (a non-behavior classification reserved for
planned-but-unmaterialized obligations on config-only beads), NOT
`FAIL_REGRESSION` or `FAIL_LOCAL`. The three `verification-ledger.jsonl`
rows are operational AGENTS.md § Beads Dolt Remote gates that the
formal-verifier must execute regardless of bead characterization.

## Test Evidence

| Test/Gate | Command | Artifact | Result |
|---|---|---|---|
| scripts/check-beads-server-mode.sh | `bash scripts/check-beads-server-mode.sh` | `evidence/check-beads-server-mode.txt` + `evidence/state12-beads-server-mode.txt` | PASS (exit 0) |
| embeddeddolt absent | `test ! -e .beads/embeddeddolt` | `evidence/state12-embeddeddolt-absent.txt` | PASS (exit 0) |
| bead claim | `bd show vb-t0iw9 --json` (post-claim state inspection) | `evidence/state12-bead-claim-state.json` | PASS (exit 0; status=in_progress) |
| bead claim (State 11 transcript) | `bd update vb-t0iw9 --claim` | `evidence/claim-result.txt` | PASS (exit 0; claim succeeded) |
| workspace gate (pwd -P + jj root) | `pwd -P && jj root` | `evidence/workspace-gate.txt` | PASS (both equal `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-t0iw9`) |
| bd version capture | `bd version && bd info --whats-new` | `evidence/bd-version.txt` | PASS (bd v1.0.5 (dev); forward-skew guard in v1.0.5) |
| schema reproduction | `bd sql "SELECT replacement_seq FROM issues LIMIT 1"` | `evidence/repro.txt` | PASS (Dolt error 1105 reproduced) |
| schema DESCRIBE | `bd sql "DESCRIBE issues"` | `evidence/schema-before.txt` | PASS (no replacement_seq column; 54 columns) |
| schema_migrations | `bd sql "SELECT version FROM schema_migrations"` | `evidence/schema-migrations.txt` | PASS (v49 highest) |
| --ignore-schema-skew bypass | `bd --ignore-schema-skew supersede vb-qryp7 --with vb-t0iw9` | `evidence/supersede-flag.txt` | PASS (exit 0; guard bypassed; raw SQL still errors) |
| port-drift discovery | `.beads/config.yaml` + actual server port | `evidence/port-drift.txt` | DISCOVERED (43643 vs 45645 vs 43627; follow-up bead candidate) |

## Review Evidence

| Review | Artifact | Status | Findings |
|---|---|---|---|
| proof-plan-review (State 4b) | `.beads/vb-t0iw9/proof-plan-review.md` | `STATUS: APPROVED` | 4 minor findings (F-001..F-004); all `owner_approved_no_action` or `owner_approved_debt`; none blocker |
| formal-verification (State 12) | `.beads/vb-t0iw9/formal-verification-report.md` | `STATUS: PASS` (PASS — verification gates green; closure deferred to user) | 0 defects; 5 PENDING_NO_TARGET obligations classified correctly |
| black-hat-review (State 13) | `.beads/vb-t0iw9/black-hat-review.md` | `STATUS: APPROVED` | 0 defects; Contract Parity + Farley + Holzman + DDD + Bitter Truth all green |

## Findings Disposition

Every reviewer finding at every severity uses a canonical
`finding/v1.disposition`: `fixed_with_evidence`, `owner_approved_debt`,
`owner_approved_no_action`, or `blocker`.

| Finding | Severity | Source Review | Disposition | Evidence Or Owner Approval |
|---|---|---|---|---|
| F-001: missing state 3 / state 4 rows in `agent-invocation-ledger.jsonl` | minor (informational) | proof-plan-review.md | `owner_approved_no_action` | "artifacts exist, the review can proceed with the inferred planner invocation ID, and the controller owns the ledger backfill as a follow-up. Not blocking." (proof-plan-review.md:144) |
| F-002: `proof-coverage-matrix.md` over-counts verifier mix on PO-T0IW9-003 | minor (documentation inconsistency) | proof-plan-review.md | `owner_approved_debt` | "accepted as documentation debt; the structural coverage is sound (cargo-fuzz PO-003 fully covers OB-007's cross-mixing + precedence-inversion risk). Owner: proof-planner to fix the matrix text at next planner rerun; debt_ref: this review." (proof-plan-review.md:153) |
| F-003: cargo-fuzz seed corpus path under `.beads/vb-t0iw9/seed_corpus/` | minor (informational) | proof-plan-review.md | `owner_approved_no_action` | "non-blocking. The deviation is intentional and bead-justified." (proof-plan-review.md:162) |
| F-004: 12 lane decisions accepted; no blockers | positive | proof-plan-review.md | `fixed_with_evidence` | `verifier-lane-review.jsonl` (this review's own output); all 12 rows `reviewer_disposition: accepted`. (proof-plan-review.md:170) |

No low / minor / observation / informational finding is omitted or lacks
disposition. No blocker is packaged as approval.

## Waivers And Deferred Work

Waivers and deferred work are not finding dispositions. Findings must use
only canonical `finding/v1.disposition` values.

| Item | Reason | Owner | Expiry/Follow-up | Compensating Evidence |
|---|---|---|---|---|
| Action A (bd dolt commit) execution | user action required; not in femdation scope | Lewis (user) | follow-up after bead delivery; user runs Action A in `/home/lewis/src/velvet-ballistics` | `runbook.md` Action A (lines 33-86); verification commands at lines 162-168 |
| Action B (bd upgrade to migration 50+) execution | user action required; not in femdation scope | Lewis (user) | follow-up if Action A is unavailable on the user's network or if Action A's ALTER TABLE is rejected by a future `bd migrate` | `runbook.md` Action B (lines 88-132) |
| Port-drift (43643 vs 45645 vs 43627) repair | out of bead scope (cosmetic config drift, not the root cause) | Lewis (user) / follow-up bead | new bead candidate; evidence at `evidence/port-drift.txt` | `runbook.md` § Related Drift Discovered (lines 144-158) |
| Five `proof-obligations.planned.jsonl` rows PENDING_NO_TARGET | no Rust surface exists in this bead | proof-planner at next planner rerun | next planner rerun or next bead that introduces Rust | `proof-plan-review.md` § Per-Seed Coverage Audit; `formal-verification-report.md` §4 |
| `verification-ledger/v1` rows for state 3 / state 4 | ledger backfill is a follow-up | femdation controller | ledger backfill at next state | `proof-plan-review.md` Finding F-001 |

## Truth Serum Audit

- report: `.beads/vb-t0iw9/truth-serum-report.md`
- status: APPROVED (see truth-serum-report.md for full audit)
- final decision: `.beads/vb-t0iw9/final-evidence-decision.md` (STATUS: APPROVED with bead closure DEFERRED_TO_USER_ACTION)