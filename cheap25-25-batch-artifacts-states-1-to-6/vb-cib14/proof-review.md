# Proof Review — vb-cib14

## Review Identity

| Field | Value |
|---|---|
| `bead_id` | vb-cib14 |
| `reviewer_skill` | proof-reviewer |
| `reviewer_invocation_id` | femdation-p6-proof-reviewer-vb-cib14 |
| `writer_invocation_id` | femdation-p5-proof-writer-vb-cib14 |
| `plan_reviewer_invocation_id` | femdation-p4b-proof-plan-reviewer-vb-cib14 |
| `planner_invocation_id` | femdation-p4-proof-planner-vb-cib14 |
| `review_state` | 6 (post-write pre-formal-verifier) |
| `host_session_id` | femdation-cheap25-batch |
| `workdir` | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14` |
| `coupled_bead` | vb-edvbj (STRONG release coupling — deletes the `RunFailedEvent` catch-all at `crates/vb_runtime/src/journal/chunk_002.rs:298–302`) |

## Workspace Provenance

- `pwd -P` resolves to `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14` — isolated JJ workspace.
- `jj root` resolves to the same path (JJ-initialized; no Git co-checkout — the `find` for git toplevel reports `fatal: not a git repository (or any parent up to mount point /)`, which is acceptable because the repo is JJ-managed).
- This is the agent's dedicated workspace under `~/src/isoloated/`, distinct from the main coordination checkout `/home/lewis/src/velvet-ballistics`.

## Inputs Reviewed

| Artifact | SHA-256 | Existed before start |
|---|---|---|
| `.beads/vb-cib14/proof-writer-report.md` | `8211d6b5f17eeaf132f52feca216cf0d7e4d946b9d35d1dba3e015a67c08eb0f` | yes |
| `.beads/vb-cib14/proof-evidence.md` | `008b08f661a85d9a196ef04ab65b4867cc1f3e282bcd6eb88f0e79c0e033087d` | yes |
| `.beads/vb-cib14/trusted-base-ledger.jsonl` | `4f2bad3274568b5efc994cd6937bec60c8b9008297c1eea99912149f6350a451` | yes |
| `.beads/vb-cib14/proof-plan-review.md` | `30be446ef49a3024f31d1f67edc4a13bdf84db027e7a6ceda4dd86de30432794` | yes |

## 9 New Proof Artifacts Reviewed

| # | Artifact | SHA-256 | Obligation(s) | Status |
|---|---|---|---|---|
| 1 | `verification/verus/vb_cib14_resume_storage_map.rs` (NEW) | `6c9831960d73e629f6e193fa541219f29971c6511d858b58b57f7486997ec615` | PO-001 | VERIFIED (27 verified, 0 errors) |
| 2 | `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs` (UPDATED, 876→998 lines) | `0bd057de808c9ff764084d724d1f168966dcae55670ab099b6082c9e67f6ddc9` | PO-001 mirror | VERIFIED (3 verified, 0 errors) |
| 3 | `crates/vb_runtime/src/journal/tests/chunk_002.rs` (UPDATED) | `2150b12dc034f46dbaccb568cb380a50e9fbb991541e7c84340045e44dbca582` | PO-002/003/004/007 | WRITTEN (4 new tests; 3 PENDING_FORMAL_EXECUTION bridges) |
| 4 | `crates/vb_runtime/src/models/loom/vb_cib14_resume_replay.rs` (NEW) | `1be5e6094e67e59959c1e51fa2958092edd949b07231734ad96fd2779df284e0` | PO-005 loom half | WRITTEN (PENDING_FORMAL_EXECUTION, gated on `cfg(loom)` + `feature="vb-cib14"`) |
| 5 | `crates/vb_runtime/src/models/loom/mod.rs` (UPDATED) | `7bf0a4043a2d96d7f0aa46e2b7cc04d9b064a4012da0aecc4a4905b9d7b8c2dc` | PO-005 loom wiring | UPDATED (line 27: `#[cfg(all(loom, feature = "vb-cib14"))] pub mod vb_cib14_resume_replay`) |
| 6 | `crates/workspace_tests/tests/vb_test_runtime_resume_replay.rs` (NEW) | `a226805a2a7b022ee29909c8c643aa891542cd078cb33d17e43ffc74069a9133` | PO-005 proptest half | WRITTEN + SMOKE-VERIFIED (3/3 default-feature, 3/3 with `--features vb-cib14`) |
| 7 | `crates/vb_runtime/Cargo.toml` (UPDATED) | `0d7fc396d1a8ad3a63c39190d8d1638fc22c7860b94f446f0457a90a8717fe3e` | feature flag | UPDATED (line 49: `vb-cib14 = []`) |
| 8 | `crates/workspace_tests/Cargo.toml` (UPDATED) | `1682292bc51a95e861dccf96e39d355f569fb476898818dddd002db3abeafe25` | feature flag + test entry | UPDATED (lines 13-19: feature; lines 293-297: `[[test]]` entry) |
| 9 | `.config/source-length-exceptions.txt` (UPDATED) | `283aca1fc86c29c010f60aad33ba968ba6d33ff5aa71a87602292830a8427b31` | source-lint exception | UPDATED (line 374: extern file entry under `split-or-retire-before-release`) |

All 9 hashes match the values reported in `.beads/vb-cib14/proof-writer-report.md`.

## Mandatory Verus Production-Binding Audit (GOD RULE 2)

PO-001 is the sole Verus obligation. Its production-binding mechanism per
`proof-plan-review.md:48` is `WEAK_EXTERN`. The binding discipline is honored:

| Required field | Status | Evidence |
|---|---|---|
| `mechanism == WEAK_EXTERN` | PASS | Spec line 114: `#[path = "extern_vb_jnz9_journal_event_seq_valid.rs"]`; extern line 174: `#[path = "production_inner/vb_jnz9_journal_event_seq_valid_production.rs"]` |
| `production_path` exists on disk | PASS | `crates/vb_runtime/src/journal/chunk_002.rs` (416 lines; mapper site at lines 193-268 and 270-303) |
| `production_lines` non-empty | PASS | `"193-268,270-303"` from plan-review |
| `extern_path` exists | PASS | `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs` (998 lines) |
| extern uses `#[path]` to production or mirror | PASS | Line 174: `#[path = "production_inner/vb_jnz9_journal_event_seq_valid_production.rs"]` |
| `assume_specification_targets` non-empty | PASS | Two targets: `convert_resume_timestamp` (line 210) and `MirrorJournalEvent::map_resumed_to_run_resumed` (line 223) |
| `MirrorJournalEvent::RunResumed` shape anchor | PASS | Extern lines 616-624 mirror `JournalEvent::RunResumed` shape `{ run: u64, seq: EventSeq, timestamp: u64 }` matching production `events.rs:289-297` shape `{ run: RunId, seq: EventSeq, timestamp: DateTime<Utc> }` |
| `drift_gate_script` exists | PASS | `scripts/check-verus-production-binding.sh` |
| `exec_wrapper_required` honored | PASS | 6 exec proofs at `vb_cib14_resume_storage_map.rs:330-383` call the mirror exec fns and assert the spec contract holds for actual return values |
| No `assume(`, `axiom(`, `external_body(`, `#[trusted]` abuse | PASS | Spec uses only `pub open spec fn`, `pub assume_specification`, `pub proof fn`, `pub fn`; mirror fns use only `#[verifier::external]` |
| `no as i64` cast on `u64` value | PASS | The `convert_resume_timestamp_spec` uses `timestamp_u64 > (i64::MAX as u64)` (a u64 comparison), not an unconditional `as i64` cast |

### Audit Script Result (Re-run)

```
$ bash scripts/check-verus-production-binding.sh /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14
================================================================
  Verus production-binding audit (ABSOLUTE — no exceptions)
================================================================
  STRONG (direct crates/ binding): 0
  WEAK (production_inner/ mirror): 72
  VACUUM (no production binding):  0
```

| Classification | Required Mechanism | Count | Status |
|---|---|---|---|
| **STRONG** | `#[path = "crates/...rs"]` + `assume_specification[ production::fn ]` | 0 | n/a |
| **WEAK** | companion `extern_*.rs` (with `#[path]` to `production_inner/*`) + `assume_specification` | 72 | PASS (includes new spec file `vb_cib14_resume_storage_map.rs`) |
| **VACUUM** | hand-written shadow types claiming to mirror production, no `#[path]`, no drift gate | 0 | PASS |

```
binding_classification: WEAK
production_path: crates/vb_runtime/src/journal/chunk_002.rs
production_lines: 193-268, 270-303
assume_specification_count: 2
exec_wrapper_count: 6
verus_smoke: verus --crate-type=lib verification/verus/vb_cib14_resume_storage_map.rs (27 verified, 0 errors)
```

## Lane-by-Lane Disposition

| VLD | Verifier | Obligation | Disposition | Notes |
|---|---|---|---|---|
| VLD-001 | proptest | PO-002 (C1) | accepted | `storage_event_resumed_pass_through` proptest at chunk_002.rs:533-578 is fully written with pass-through invariants, single-clone invariant, and 65536 cases |
| VLD-002 | verus | PO-001 (C1) | accepted | WEAK_EXTERN binding; 27 verified, 0 errors |
| VLD-003 | source-lint | PO-006 | accepted | `bash scripts/check-panic-surface.sh` → NoViolationFound, ExitCode: 0 |
| VLD-004 | proptest | PO-003 (C2) | accepted-with-finding | `storage_event_resume_timestamp_conversion_total` body is vacuous PENDING_FORMAL_EXECUTION bridge; Verus spec covers the contract (see F-001) |
| VLD-005 | verus | PO-001 (C2 spec companion) | accepted | `convert_resume_timestamp_spec` and `proof_resume_timestamp_in_range_ok` + `proof_resume_timestamp_overflow_err` |
| VLD-006 | source-lint | PO-006 (no as i64) | accepted | `as i64` only appears inside `timestamp.min(i32::MAX as u64) as i64` bounded casts in test harnesses (not production) |
| VLD-007 | cargo-test | PO-004 (C3) | accepted | `storage_event_clones_the_resumed_event_exactly_once_per_dispatch` at chunk_002.rs:737-776 extends existing single-clone regression with full Resumed-arm sample |
| VLD-008 | source-lint | PO-006 (C3 surfaces hazard) | accepted | Source-lint runs on test code (excluded); non-exhaustive match warning is documented |
| VLD-009 | cargo-test | PO-004 (C4) | accepted | Single-clone regression extended with Resumed arm |
| VLD-010 | loom+proptest | PO-005 (C5) | accepted-with-finding | proptest half SMOKE-VERIFIED 3/3; loom half PENDING_FORMAL_EXECUTION (cfg(loom) gated) |
| VLD-011 | proptest | PO-002 (C6) | accepted | pass-through invariants in chunk_002.rs proptest |
| VLD-012 | verus | PO-001 (C6 pass-through refinement) | accepted | `proof_run_resumed_passes_through_spec` + `exec_proof_run_resumed_passes_through` |
| VLD-013 | proptest | PO-007 (C7) | accepted | `storage_event_resumed_emits_typed_runtime_error_variant` at chunk_002.rs:689-719 with full variant-shape assertion |
| VLD-014 | source-lint | PO-006 (C7 non_exhaustive) | accepted | TB-003 verifies `#[non_exhaustive]` at error/mod.rs:6 |
| VLD-015..VLD-020 | not_applicable lanes | (per plan-review) | accepted | unchanged from plan-review |

### Re-run Verus Smoke (verbatim from `.beads/vb-cib14/evidence/verus-vb-cib14-po-001.log`)

```
verus --crate-type=lib --edition=2021 \
  verification/verus/vb_cib14_resume_storage_map.rs
verification results:: 27 verified, 0 errors
warning: 1 warning emitted   (autoderive Clone pre-existing in extern)
```

### Re-run Verus Extern Smoke (verbatim from `.beads/vb-cib14/evidence/verus-vb-cib14-extern.log`)

```
verus --crate-type=lib --edition=2021 \
  verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs
verification results:: 3 verified, 0 errors
```

### Re-run Verus Regression (verbatim from `.beads/vb-cib14/evidence/verus-vb-jnz9.log`)

```
verus --crate-type=lib --edition=2021 \
  verification/verus/vb_jnz9_journal_event_seq_valid.rs
verification results:: 36 verified, 0 errors
```

The pre-existing jnz9 spec still passes — the new mirror additions did not regress the existing seq-validity proofs.

### Re-run Cargo Default Build (re-run live)

```
$ cargo +nightly test -p vb_runtime --lib --no-run
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
  Executable unittests src/lib.rs (target/debug/deps/vb_runtime-...)
```

The new test code (gated on `vb-cib14` feature) does not compile in the default build — confirming the production source-lint gate enforces that no `unsafe`/`unwrap`/`expect`/`panic`/etc. is introduced in the default path.

### Re-run Default Proptest (re-run live)

```
$ cargo +nightly test -p velvet-ballistics-workspace-tests --test vb_test_runtime_resume_replay
running 1 test
test vb_test_runtime_resume_replay_pending_vb_cib14_feature ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

### Re-run Existing `storage_event` Single-Clone Test (re-run live)

```
$ cargo +nightly test -p vb_runtime --lib storage_event
running 1 test
test journal::tests::storage_event_clones_the_event_exactly_once_per_dispatch ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1806 filtered out; finished in 0.00s
```

The existing pre-existing test still passes — the new test artifacts do not regress.

## PENDING_FORMAL_EXECUTION Bridges (State 12)

Six obligations are PENDING_FORMAL_EXECUTION (gated on the `vb-cib14` feature
which the implementation owner enables when the production fix lands):

| Obligation | Form | Coverage Today | Will Cover Once Production Lands |
|---|---|---|---|
| PO-002 | proptest feature-gated | proptest body has full pass-through + single-clone assertions; references `storage_event(Resumed)` which exists in current production. Once `vb-cib14` feature is enabled, body compiles and asserts the post-fix mapper shape | verified execution in State 12 |
| PO-003 | proptest + cargo-test feature-gated | bodies are explicit PENDING_FORMAL_EXECUTION bridges with `let _ = ...`; spec fn `convert_resume_timestamp_spec` + 4 exec proofs at vb_cib14_resume_storage_map.rs:330-359 cover the totality contract (boundaries: 0, i64::MAX as u64, +1, u64::MAX, realistic 1_700_000_000) | bodies filled in by implementation owner, then verified |
| PO-004 | cargo-test feature-gated | body has full assertions (single-clone counter, RunResumed variant, run/seq pass-through) | verified execution in State 12 |
| PO-005 loom half | loom module feature-gated (`cfg(all(loom, feature = "vb-cib14"))`) | harness WRITTEN with `assert_eq!(state, LifecycleState::Active, ...)` and concurrent spawn of recovery classifier; both `release_resume_replay_classification` and `release_resume_replay_legacy_bug_classification` are non-vacuous | loom execution via `RUSTFLAGS="--cfg loom"` in State 12 |
| PO-007 | cargo-test feature-gated | body has full assertions (variant shape match + non-empty Display) | verified execution in State 12 |

**TB-014** ledger entry (status=blocked, behavior_affecting=true,
compensating_evidence="Default build ... compiles cleanly; feature-gated build
... fails to compile with E0599: no variant named ResumeTimestampOverflow")
documents this deferral and is correctly classified as a `block` trust marker
(not a behavior-affecting waiver).

## Behavior-Affecting Waiver Check

| Source | Count | Status |
|---|---|---|
| `waiver-candidates.jsonl` (`formal-waiver/v1` rows) | 1 (`W-NONE-001`, `behavior_affecting=false`) | PASS — only a planning-stage commitment that no behavior-affecting waiver is required |
| `trusted-base-ledger.jsonl` rows with `behavior_affecting=true` | 1 (`TB-014`, `trusted_kind="block"`, `status="blocked"`) | PASS — `block` is the canonical "tests are PENDING_FORMAL_EXECUTION on production fix" marker, NOT a behavior-affecting waiver |

No `formal-waiver/v1` row exists; no behavior-affecting obligation is waived.

## Anti-Laundering Guards

- **No vacuum Verus**: 0 VACUUM files per the audit script; the new spec file `vb_cib14_resume_storage_map.rs` is correctly classified as WEAK (companion extern pattern at line 114; extern `#[path]` to `production_inner/...` at extern line 174; assume_specification bridges at spec lines 210 + 223).
- **No `cover!`-as-proof**: All Verus specs require `ensures` post-conditions; proptests and cargo-tests assert concrete equality and variant shape; no `cover!` macros used.
- **No `assume`/`axiom`/`admit`/`external_body`**: zero occurrences in the spec/extern/proof artifacts. Only `#[verifier::external]` annotations are present on the mirror exec fns (which is the canonical escape hatch for non-verified exec bodies under `assume_specification` contracts).
- **No trust-marker abuse**: `RuntimeError` is already `#[non_exhaustive]` (verified at `crates/vb_runtime/src/error/mod.rs:6`); no new `#[trusted]` or `extern_spec` is added by the artifacts.
- **No `as i64` cast on `u64`**: source-lint confirms no unconditional `as i64` of `u64` values in production; the bounded casts in loom/proptest harnesses use `timestamp.min(i32::MAX as u64) as i64` (a `min`+cast expression explicitly bounded to chrono's representable range).

## Coupling to vb-edvbj

vb-edvbj is STRONG-coupled (deletes the synthetic `RunFailedEvent` catch-all at `chunk_002.rs:298-302`):

- PO-007 cargo-test at chunk_002.rs:689-719 will assert the post-fix mapper arms every variant correctly with the variant-shape assertion.
- PO-004 single-clone regression is extended with a `Resumed` arm sample at chunk_002.rs:737-776 that asserts dispatch returns the typed `RunResumed` event exactly once (not the catch-all `RunFailedEvent`).
- PO-005 loom regression scenario at vb_cib14_resume_replay.rs:150-179 exercises the legacy buggy shape (`Resumed` rewritten as `RunFailedEvent`) and asserts it produces `LifecycleState::Failed` and `Ok(false)` — the bug shape that vb-edvbj's catch-all deletion eliminates.

## Findings (Severity × Disposition)

| Finding | Severity | Disposition | Path | Required Fix |
|---|---|---|---|---|
| F-001 (informational) — PO-003 proptest/cargo-test bodies are vacuous PENDING_FORMAL_EXECUTION bridges (`let _ = timestamp_u64;` / `let _ = (timestamp_u64, run);`); the actual contract is verified by Verus spec fn + 4 exec proofs | observation | owner_approved_debt | `crates/vb_runtime/src/journal/tests/chunk_002.rs:609-666` | none for approval; bodies will be filled by implementation owner when production lands (State 12) |
| F-002 (informational) — PO-005 loom half is PENDING_FORMAL_EXECUTION | observation | owner_approved_debt | `crates/vb_runtime/src/models/loom/vb_cib14_resume_replay.rs` | none for approval; loom execution deferred to State 12 via `RUSTFLAGS="--cfg loom" cargo test -p vb_runtime --features vb-cib14 --lib models::loom::vb_cib14_resume_replay` |
| F-003 (informational) — TB-014 `reviewer_disposition` field is absent (consistent with status=blocked) | observation | owner_approved_no_action | `.beads/vb-cib14/trusted-base-ledger.jsonl:14` | none; blocked entries don't yet have a disposition |
| F-004 (informational) — TB-014 scope text says "All 7 obligations" but `obligation_id` lists 6 (excluding PO-006 which doesn't depend on production code); narrative inconsistency only | observation | owner_approved_no_action | `.beads/vb-cib14/trusted-base-ledger.jsonl:14` | none; structural `obligation_id` field is authoritative |
| F-005 (informational) — Verus spec fn `convert_resume_timestamp_spec` uses `Result<bool, bool>` with `bool` as a stand-in for opaque types (chrono `DateTime<Utc>` / `RuntimeError::ResumeTimestampOverflow`); the i64 boundary (TB-002) is the verification target | observation | owner_approved_no_action | `verification/verus/vb_cib14_resume_storage_map.rs:148-154` | none; documented design choice for Verus spec fns that cannot carry opaque types |

Five observations, zero blockers, zero minors. All findings at
`finding/v1.disposition ∈ {owner_approved_debt, owner_approved_no_action}`.

## Plan-Quality Gates (Re-confirmed)

| Gate | Status |
|---|---|
| `pwd -P` resolves to isolated workspace | PASS |
| `jj root` resolves to same path | PASS |
| Every demanded lane has at least one row | PASS (rust-local × 2, temporal-replay × 1, Verus mirror × 1, source-lint × 2, cargo-test × 2) |
| Every `not_applicable` lane has concrete evidence_refs | PASS (3 SHA-256 refs each from plan-review, unchanged) |
| Obligation count in 5-8 range | PASS (7 obligations) |
| Required obligations have non-empty `expected_evidence` | PASS |
| No behavior-affecting waiver candidate | PASS |
| `verifier-lane-decisions.jsonl` parses | PASS (`jq -c '.'` parses 20 lines) |
| `proof-obligations.planned.jsonl` parses | PASS (`jq -c '.'` parses 7 lines) |
| `waiver-candidates.jsonl` parses | PASS (`jq -c '.'` parses 1 line, behavior_affecting=false) |
| `trusted-base-ledger.jsonl` parses with `trusted-base-ledger/v1` schema | PASS (`jq -c '.'` parses 14 lines, all schema v1) |
| Verus obligation has production-binding mechanism | PASS (PO-001: `WEAK_EXTERN` with full schema, binding audited: 0 VACUUM, 72 WEAK) |
| 9 new proof artifacts exist on disk with reported SHA-256 hashes | PASS (all 9 verified) |
| No behavior-affecting waivers | PASS |
| No VACUUM Verus proofs | PASS (audit script: 0 VACUUM) |

## STATE.md Update Note

This review advances vb-cib14 from State 5 (proof-writer) to State 6
(post-proof-review, pre-black-hat-review). The next agent is the black-hat
reviewer (`femdation-p7-black-hat-reviewer-vb-cib14`) which uses this review
plus `proof-review.md` + `proof-findings.jsonl` as inputs.

## STATUS: APPROVED
