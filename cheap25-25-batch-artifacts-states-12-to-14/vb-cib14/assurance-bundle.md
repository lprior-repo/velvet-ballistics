# Assurance Bundle

bead_id: vb-cib14
source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14
commit_or_change: jj working-copy `zpmskmnz 96dfa778` (parent `b2a2ee46`); book mark `cheap25-vb-cib14`
coupled_bead: vb-edvbj (STRONG release coupling — deletes the `RunFailedEvent` catch-all at `crates/vb_runtime/src/journal/chunk_002.rs:298–302`)

## Requirement Coverage

| Requirement | Contract Clause | Proof/Test Evidence | Review Evidence | Status |
|---|---|---|---|---|
| C1: Resumed maps to RunResumed in `boundary_storage_event` | contract.md#C1 | PO-001 (Verus 27/27 PASS) + PO-002 (proptest 65536 cases PASS) + PO-007 (proptest 4096 cases PASS) + PO-004 (cargo-test PASS) + 16-variant enumeration in `chunk_004.rs:1077-1090` | proof-review.md (APPROVED) + proof-to-rust-review.md (APPROVED) + black-hat-review.md (APPROVED) | ✅ |
| C2: Timestamp conversion is total, explicit, no `as i64` | contract.md#C2 | PO-001 (Verus `convert_resume_timestamp_spec` total over `u64`) + PO-003 (proptest 65536 cases + boundary sentinels `[0, 1, 1700000000, i64::MAX, i64::MAX+1, u64::MAX-1, u64::MAX]` PASS) | proof-review.md (APPROVED) + black-hat-review.md (APPROVED) | ✅ |
| C3: Storage dispatch totality (paired with vb-edvbj) | contract.md#C3 | PO-004 (cargo-test PASS) + PO-007 (16-variant enumeration at chunk_004.rs:1077-1090 PASS) | proof-review.md (APPROVED) + proof-to-rust-review.md (APPROVED) + black-hat-review.md (APPROVED with STRONG-coupling reference to vb-edvbj) | ✅ |
| C4: Single-clone invariant (STORAGE_EVENT_CLONE_COUNT == 1) | contract.md#C4 | PO-004 (cargo-test PASS; thread-local migration tested at 1812/1812 full feature run) | proof-review.md (APPROVED) + black-hat-review.md (APPROVED) | ✅ |
| C5: Recovery classifies RunResumed as Active | contract.md#C5 + REFINEMENT-RRO-RESUME | PO-005 (loom 2/2 PASS + proptest 3/3 PASS) | proof-review.md (APPROVED) + proof-to-rust-review.md (APPROVED) + black-hat-review.md (APPROVED) | ✅ |
| C6: Seq + RunId pass-through | contract.md#C6 | PO-001 (Verus `proof_run_resumed_passes_through_spec` PASS) + PO-002 (proptest 65536 cases asserts `mapped_event.seq() == seq` and `mapped_event.run_id() == run`) | proof-review.md (APPROVED) + black-hat-review.md (APPROVED) | ✅ |
| C7: Public error surface adds ResumeTimestampOverflow struct variant | contract.md#C7 | PO-007 (cargo-test PASS; field-shape match) + PO-006 source-lint (RuntimeError remains `#[non_exhaustive]`; new variant is struct variant) | proof-review.md (APPROVED) + black-hat-review.md (APPROVED) | ✅ |
| VERUS-MIRROR: Mirror stays in sync with production `JournalEvent::RunResumed` shape | contract.md#verus-mirror-binding | PO-006 source-lint (`check-verus-production-binding.sh` 0 VACUUM / 72 WEAK; `check-production-inner-drift.sh` 0 drift) | proof-review.md (APPROVED) + black-hat-review.md (APPROVED) | ✅ |

## Proof Evidence

| Obligation | Tool | Command | Artifact | Result | Waiver |
|---|---|---|---|---|---|
| PO-001 | verus | `verus --crate-type=lib --edition=2021 verification/verus/vb_cib14_resume_storage_map.rs` | `.beads/vb-cib14/evidence/state12-verus-vb-cib14-po-001.log` | PASS (27 verified, 0 errors) | none |
| PO-002 | proptest | `PROPTEST_CASES=65536 cargo +nightly test -p vb_runtime --features vb-cib14 --lib -- storage_event_resumed_pass_through storage_event_resume_timestamp_conversion_total` | `.beads/vb-cib14/evidence/state12-proptest-po-002-003.log` | PASS (3/3) | none |
| PO-003 | proptest | (same command as PO-002, plus boundary sentinels) | `.beads/vb-cib14/evidence/state12-proptest-po-002-003.log` | PASS (3/3) | none |
| PO-004 | cargo-test | `cargo +nightly test -p vb_runtime --features vb-cib14 --lib -- storage_event_clones_the_event_exactly_once_per_dispatch storage_event_clones_the_resumed_event_exactly_once_per_dispatch` | `.beads/vb-cib14/evidence/state12-cargo-test-po-004.log` | PASS (2/2) | none |
| PO-005 | loom+proptest | `RUSTFLAGS="--cfg loom" cargo +nightly test -p vb_runtime --features vb-cib14 --lib models::loom::vb_cib14_resume_replay` + proptest half | `.beads/vb-cib14/evidence/state12-loom-vb-cib14-po-005.log` + `.beads/vb-cib14/evidence/state12-cargo-workspace-tests-vb_test_runtime_resume_replay.log` | PASS (2/2 loom) + PASS (3/3 proptest) | none |
| PO-006 | source-lint | `bash scripts/check-panic-surface.sh && bash scripts/check-hot-cold-forbidden-apis.sh && bash scripts/check-source-length.sh && bash scripts/check-verus-production-binding.sh && bash scripts/check-error-exhaustiveness.sh` | `.beads/vb-cib14/evidence/state12-lint-po-006-{panic,hot-cold,length}.log` + `.beads/vb-cib14/evidence/check-verus-production-binding-state12.log` | PASS (NoViolationFound; 0 VACUUM; chunk_002.rs + extern file ledgered) | none |
| PO-007 | proptest | `PROPTEST_CASES=4096 cargo +nightly test -p vb_runtime --features vb-cib14 --lib -- storage_event_resumed_emits_typed_runtime_error_variant` | `.beads/vb-cib14/evidence/state12-proptest-po-007.log` | PASS (1/1) | none |

## Test Evidence

| Test/Gate | Command | Artifact | Result |
|---|---|---|---|
| `cargo test -p vb_runtime --lib --features vb-cib14 storage_event` (6 tests: 4 single-clone + 1 typed-error + 1 conversion) | `cargo +nightly test -p vb_runtime --lib --features vb-cib14 storage_event` | `.beads/vb-cib14/evidence/state12-cargo-vb-runtime-storage_event.log` | PASS (6/6) |
| `cargo test -p vb_runtime --lib --features vb-cib14 runtime_journal_event_resumed_has_correct_timestamp` (chunk_004 test) | `cargo +nightly test -p vb_runtime --lib --features vb-cib14 runtime_journal_event_resumed_has_correct_timestamp` | `.beads/vb-cib14/evidence/state12-cargo-vb-runtime-chunk004-runtime_journal_event_resumed.log` | PASS (1/1) |
| `cargo test -p velvet-ballistics-workspace-tests --test vb_test_runtime_resume_replay --features vb-cib14` (PO-005 proptest half) | `cargo +nightly test -p velvet-ballistics-workspace-tests --test vb_test_runtime_resume_replay --features vb-cib14` | `.beads/vb-cib14/evidence/state12-cargo-workspace-tests-vb_test_runtime_resume_replay.log` | PASS (3/3) |
| `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime --features vb-cib14 --lib models::loom::vb_cib14_resume_replay` (PO-005 loom half) | `RUSTFLAGS="--cfg loom" cargo +nightly test -p vb_runtime --features vb-cib14 --lib models::loom::vb_cib14_resume_replay` | `.beads/vb-cib14/evidence/state12-loom-vb-cib14-po-005.log` | PASS (2/2) |
| `cargo test -p vb_runtime --lib --features vb-cib14` (full feature run) | `cargo +nightly test -p vb_runtime --lib --features vb-cib14` | (live in machine-gate-state14.log) | PASS (1812/1812) |
| `cargo test -p vb_runtime --lib` (default build) | `cargo +nightly test -p vb_runtime --lib` | (live in machine-gate-state14.log) | PASS (1807/1807) |
| `cargo build -p vb_runtime --all-targets --all-features` | `cargo +nightly build -p vb_runtime --all-targets --all-features` | (live in machine-gate-state14.log) | PASS (warning-free) |
| `scripts/check-panic-surface.sh` (PO-006) | `bash scripts/check-panic-surface.sh` | `.beads/vb-cib14/evidence/state12-lint-po-006-panic.log` | PASS (NoViolationFound, ExitCode 0) |
| `scripts/check-verus-production-binding.sh` (PO-006 / GOD RULE 2) | `bash scripts/check-verus-production-binding.sh /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14` | `.beads/vb-cib14/evidence/check-verus-production-binding-state12.log` | PASS (0 VACUUM, 72 WEAK, 0 STRONG) |
| `scripts/check-test-integrity.sh` (production test integrity) | `bash scripts/check-test-integrity.sh` | (live in machine-gate-state14.log) | PASS (base=@-) |
| `scripts/forbidden-scan.sh` (forbidden pattern scan) | `bash scripts/forbidden-scan.sh` | (live in machine-gate-state14.log) | PASS (no forbidden patterns) |
| `scripts/check-nightly-features.sh` (Rust nightly feature scope) | `bash scripts/check-nightly-features.sh` | (live in machine-gate-state14.log) | PASS (exit 0) |
| `scripts/check-workspace-assertions.sh` (workspace-level assertions) | `bash scripts/check-workspace-assertions.sh` | (live in machine-gate-state14.log) | PASS (exit 0) |

## Review Evidence

| Review | Artifact | Status | Findings |
|---|---|---|---|
| Proof plan review | `.beads/vb-cib14/proof-plan-review.md` | STATUS: APPROVED | 0 blockers |
| Proof review | `.beads/vb-cib14/proof-review.md` | STATUS: APPROVED | 5 observations (F-001..F-005) all `owner_approved_*` |
| Proof-to-rust bridge review | `.beads/vb-cib14/proof-to-rust-review.md` | STATUS: APPROVED | 0 findings |
| Formal-verification report | `.beads/vb-cib14/formal-verification-report.md` | STATUS: APPROVED — all 7 obligations PASS | 0 blockers; pre-flight Verus production-binding 0 VACUUM |
| Black-hat review | `.beads/vb-cib14/black-hat-review.md` | STATUS: APPROVED — with STRONG-coupling reference to vb-edvbj | 6 LOW (F-001..F-006): pre-existing structural hazards or documented design choices |
| Verification ledger | `.beads/vb-cib14/verification-ledger.jsonl` | 7 rows, all PASS, hash chain validated | n/a |
| Truth-serum audit | `.beads/vb-cib14/truth-serum-report.md` | STATUS: PASS | 0 critical, 0 high, 0 medium |
| Final evidence decision | `.beads/vb-cib14/final-evidence-decision.md` | STATUS: APPROVED | n/a |

## Findings Disposition

All findings use canonical `finding/v1.disposition` values.

| Finding | Severity | Source Review | Disposition | Evidence Or Owner Approval |
|---|---|---|---|---|
| F-001 (proof-review): PO-003 proptest/cargo-test bodies are PENDING_FORMAL_EXECUTION bridges | observation | proof-review.md | `owner_approved_debt` | resolved at State 12: proptest + cargo-test bodies filled in and execute cleanly; `state12-proptest-po-002-003.log` shows 3/3 PASS |
| F-002 (proof-review): PO-005 loom half is PENDING_FORMAL_EXECUTION | observation | proof-review.md | `owner_approved_debt` | resolved at State 12: loom half executes 2/2 PASS at `state12-loom-vb-cib14-po-005.log` |
| F-003 (proof-review): TB-014 `reviewer_disposition` field is absent | observation | proof-review.md | `owner_approved_no_action` | n/a — blocked entries don't yet have a disposition; production fix has now landed and the blocker is unblocked |
| F-004 (proof-review): TB-014 scope text says "All 7" but lists 6 obligation_ids | observation | proof-review.md | `owner_approved_no_action` | narrative inconsistency only; structural `obligation_id` is authoritative |
| F-005 (proof-review): Verus spec fn `convert_resume_timestamp_spec` uses `Result<bool, bool>` stand-in for opaque types | observation | proof-review.md | `owner_approved_no_action` | documented design choice; Verus spec fns cannot carry chrono / RuntimeError types; exec proofs at lines 330-359 exercise boundary cases |
| F-001 (black-hat): `storage_event` is 29 logical lines (over 25 cap) | LOW | black-hat-review.md | `owner_approved_no_action` | pre-existing structural hazard; will shrink once vb-edvbj removes the `_ =>` catch-all (STRONG coupling) |
| F-002 (black-hat): `boundary_storage_event` is 65 logical lines (one declarative match) | LOW | black-hat-review.md | `owner_approved_no_action` | pre-existing baseline 317 lines; vb-cib14 added 30; ledgered at `.config/source-length-exceptions.txt:111` under `split-or-retire-before-release` |
| F-003 (black-hat): extern file is 998 lines (over 800 verus cap) | LOW | black-hat-review.md | `owner_approved_no_action` | pre-existing 876-line baseline + 122-line vb-cib14 addition; ledgered at `.config/source-length-exceptions.txt:374` |
| F-004 (black-hat): `Result<bool, bool>` stand-in for opaque types in Verus mirror | LOW | black-hat-review.md | `owner_approved_no_action` | documented at file level; spec fn `convert_resume_timestamp_spec` is the algebraic model; exec proofs exercise actual mirror return values |
| F-005 (black-hat): `boundary_storage_event` is one large exhaustive match without per-arm helper extraction | LOW | black-hat-review.md | `owner_approved_no_action` | declarative exhaustiveness is the contract enforcement surface; per-arm extraction would lose the compile-time total-match check |
| F-006 (black-hat): `RuntimeError::ResumeTimestampOverflow` is the only struct variant in error/mod.rs without a high-level `runtime_code()` | LOW | black-hat-review.md | `owner_approved_no_action` | `None` arm is intentional per the diagnostic-code-only model |

All 11 findings have a canonical disposition. Zero CRITICAL, zero HIGH, zero MEDIUM. All observations are `owner_approved_*` (none blockers).

## Waivers And Deferred Work

| Item | Reason | Owner | Expiry/Follow-up | Compensating Evidence |
|---|---|---|---|---|
| `W-NONE-001` (`waiver-candidates.jsonl`) | No behavior-affecting waiver is required. The 5 demanded lanes (rust-local, temporal-replay, Verus mirror, source-lint, cargo test) cover every proof seed. The 4 not-applicable lanes (flux-rs, miri, cargo-fuzz, kani, tla-plus) carry concrete `non_applicability_evidence_refs` with typed `limitation_kind`. | proof-planner | 2026-12-31T00:00:00Z | `proof-obligations.planned.jsonl` (all 7 required, none waived); `verifier-lane-decisions.jsonl` (14 required + 6 not_applicable + 0 blocked_tooling); `trusted-base-plan.md` (12 trusted-base entries). |
| `TB-014` (block) | `blocked` until production fix lands (now landed at State 11). Was a `block` trust marker, NOT a behavior-affecting waiver. | implementation-owner | when production fix lands (DONE) | production symbols `convert_resume_timestamp`, `RuntimeError::ResumeTimestampOverflow`, and the explicit `Resumed` arm are now defined; `vb-cib14` feature builds; all 7 obligations PASS at State 12. |
| `chunk_002.rs` source-length exception (447 lines vs 300 limit) | Pre-existing structural hazard (baseline 317 lines); vb-cib14 added 30. | lewis | when the file is split by domain responsibility | row 111 of `.config/source-length-exceptions.txt` (`split-or-retire-before-release`). The post-fix mapper is structurally simple; once vb-edvbj removes the `_ =>` catch-all, the top-level `storage_event` will shrink. |
| `extern_vb_jnz9_journal_event_seq_valid.rs` source-length exception (998 lines vs 800 limit) | Pre-existing 876-line baseline + 122-line vb-cib14 addition for the new `MirrorJournalEvent::map_resumed_to_run_resumed` and `convert_resume_timestamp` mirror surface. | lewis | when the file is split (a future split would separate the `MirrorJournalEvent` mirror from the new vb-cib14 mirror surface into `extern_vb_jnz9_journal_event_seq_valid_vb_cib14.rs`). | row 374 of `.config/source-length-exceptions.txt` (`split-or-retire-before-release`). The mirror types are required to be line-by-line with the production `JournalEvent` and `RuntimeError` shapes. |

No behavior-affecting waivers exist. `waiver-candidates.jsonl` has 1 row with
`behavior_affecting=false` (planning-stage commitment that no waivers are
required). `trusted-base-ledger.jsonl` has 14 rows; 13 are `accepted`, 1
(`TB-014`) was `blocked` (now unblocked because production fix has landed).

## Truth Serum Audit

- report: `.beads/vb-cib14/truth-serum-report.md`
- status: APPROVED (PASS — 0 critical, 0 high, 0 medium, 1 informational)

## STATUS: APPROVED — for landing with vb-edvbj coupling