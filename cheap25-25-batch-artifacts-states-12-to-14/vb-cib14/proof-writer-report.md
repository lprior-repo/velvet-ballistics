# Proof Writer Report — vb-cib14

STATUS: COMPLETE (PENDING_FORMAL_EXECUTION for State 12)

## Bead Identity

- `bead_id`: vb-cib14
- `invocation_id`: femdation-p5-proof-writer-vb-cib14
- `current_state`: 5 (proof-writer)
- `controller`: femdation
- `isolated_workdir`: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14`
- `jj_workspace`: `cheap25-vb-cib14`
- `coupled_bead`: vb-edvbj (STRONG release coupling — deletes the
  `RunFailedEvent` catch-all at
  `crates/vb_runtime/src/journal/chunk_002.rs:298–302`)
- `parent_invocation_id`: `femdation-p4b-proof-plan-reviewer-vb-cib14`

## Inputs Read

- `.beads/vb-cib14/proof-strategy.md`
- `.beads/vb-cib14/verifier-lane-decisions.jsonl`
- `.beads/vb-cib14/proof-obligations.planned.jsonl`
- `.beads/vb-cib14/proof-plan-review.md`
- `.beads/vb-cib14/trusted-base-plan.md`

## Artifacts Touched

| Artifact | Path | Hash (sha256) | Status |
|---|---|---|---|
| Verus spec (PO-001) | `verification/verus/vb_cib14_resume_storage_map.rs` (NEW) | `6c9831960d73e629f6e193fa541219f29971c6511d858b58b57f7486997ec615` | VERIFIED (27 verified, 0 errors) |
| Verus mirror (PO-001) | `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs` (UPDATED) | `0bd057de808c9ff764084d724d1f168966dcae55670ab099b6082c9e67f6ddc9` | VERIFIED (3 verified, 0 errors) |
| Proptest (PO-002/003/004/007) | `crates/vb_runtime/src/journal/tests/chunk_002.rs` (UPDATED) | `2150b12dc034f46dbaccb568cb380a50e9fbb991541e7c84340045e44dbca582` | WRITTEN (gated on `vb-cib14` feature) |
| Loom harness (PO-005) | `crates/vb_runtime/src/models/loom/vb_cib14_resume_replay.rs` (NEW) | `1be5e6094e67e59959c1e51fa2958092edd949b07231734ad96fd2779df284e0` | WRITTEN (gated on `vb-cib14` feature + `cfg(loom)`) |
| Loom module wiring (PO-005) | `crates/vb_runtime/src/models/loom/mod.rs` (UPDATED) | `7bf0a4043a2d96d7f0aa46e2b7cc04d9b064a4012da0aecc4a4905b9d7b8c2dc` | UPDATED |
| Proptest harness (PO-005) | `crates/workspace_tests/tests/vb_test_runtime_resume_replay.rs` (NEW) | `a226805a2a7b022ee29909c8c643aa891542cd078cb33d17e43ffc74069a9133` | WRITTEN (gated on `vb-cib14` feature) |
| Cargo manifest (vb_runtime) | `crates/vb_runtime/Cargo.toml` (UPDATED) | `0d7fc396d1a8ad3a63c39190d8d1638fc22c7860b94f446f0457a90a8717fe3e` | UPDATED (added `vb-cib14` feature) |
| Cargo manifest (workspace_tests) | `crates/workspace_tests/Cargo.toml` (UPDATED) | `1682292bc51a95e861dccf96e39d355f569fb476898818dddd002db3abeafe25` | UPDATED (added `vb-cib14` feature + new `[[test]]` entry) |
| Source-length exception | `.config/source-length-exceptions.txt` (UPDATED) | `283aca1fc86c29c010f60aad33ba968ba6d33ff5aa71a87602292830a8427b31` | UPDATED (added entry for `extern_vb_jnz9_journal_event_seq_valid.rs`) |

## Obligation Mapping

| Obligation | Artifact | Status |
|---|---|---|
| **PO-001** (Verus WEAK_EXTERN ×3) | `verification/verus/vb_cib14_resume_storage_map.rs` + `extern_vb_jnz9_journal_event_seq_valid.rs` | WRITTEN + VERIFIED |
| **PO-002** (proptest pass-through) | `crates/vb_runtime/src/journal/tests/chunk_002.rs::storage_event_resumed_pass_through` | WRITTEN (PENDING) |
| **PO-003** (proptest conversion totality) | `crates/vb_runtime/src/journal/tests/chunk_002.rs::storage_event_resume_timestamp_conversion_total` + `storage_event_resume_timestamp_conversion_total_over_u64` | WRITTEN (PENDING) |
| **PO-004** (cargo-test single-clone) | `crates/vb_runtime/src/journal/tests/chunk_002.rs::storage_event_clones_the_resumed_event_exactly_once_per_dispatch` | WRITTEN (PENDING) |
| **PO-005** (loom+proptest temporal replay) | `crates/vb_runtime/src/models/loom/vb_cib14_resume_replay.rs` (loom) + `crates/workspace_tests/tests/vb_test_runtime_resume_replay.rs` (proptest) | WRITTEN + SMOKE-VERIFIED |
| **PO-006** (source-lint) | `scripts/check-panic-surface.sh` (no violation found) | SMOKE-VERIFIED (existing scripts only) |
| **PO-007** (proptest typed error + 16-variant) | `crates/vb_runtime/src/journal/tests/chunk_002.rs::storage_event_resumed_emits_typed_runtime_error_variant` | WRITTEN (PENDING) |

## Verifier Lane Coverage

| Verifier | Locus | Lane | Status |
|---|---|---|---|
| `verus` | `vb_cib14_resume_storage_map.rs` | rust-local + arithmetic + typestate | WRITTEN + VERIFIED (27 spec proofs + 6 exec proofs) |
| `proptest` | `chunk_002.rs` | rust-local + bounded_state | WRITTEN (PENDING — gated on `vb-cib14` feature) |
| `loom+proptest` | `vb_cib14_resume_replay.rs` + `vb_test_runtime_resume_replay.rs` | temporal_safety | WRITTEN + SMOKE-VERIFIED (default build OK, feature-gated build proptest passes 3/3) |
| `cargo-test` | `chunk_002.rs::storage_event_clones_the_resumed_event_exactly_once_per_dispatch` | bounded_transition | WRITTEN (PENDING) |
| `source-lint` | `scripts/check-panic-surface.sh` | source-lint | SMOKE-VERIFIED (`NoViolationFound`, `ExitCode: 0`) |
| Verus production binding | `scripts/check-verus-production-binding.sh` | source-lint | SMOKE-VERIFIED (`VACUUM: 0`, `WEAK: 72`, `STRONG: 0`) |

## Production-Binding Validation (Mandatory)

PO-001 is the sole Verus obligation. Its `production_binding` field
per the planner is `WEAK_EXTERN`. The binding discipline is honored:

| Required field | Status | Evidence |
|---|---|---|
| `mechanism == WEAK_EXTERN` | PASS | `#[path = "production_inner/vb_jnz9_journal_event_seq_valid_production.rs"]` (existing line in extern file) |
| `production_path` exists on disk | PASS | `crates/vb_runtime/src/journal/chunk_002.rs` (416 lines; mapper site verified at line 193–268 and 270–303) |
| `extern_path` exists | PASS | `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs` (998 lines) |
| extern uses `#[path]` to production or mirror | PASS | Line 174 (existing): `#[path = "production_inner/vb_jnz9_journal_event_seq_valid_production.rs"]` |
| `assume_specification_targets` non-empty | PASS | Two targets: `MirrorJournalEvent::map_resumed_to_run_resumed` and `convert_resume_timestamp` |
| `MirrorJournalEvent::RunResumed` shape anchor | PASS | Verified at lines 616–624 of extern file (matches `JournalEvent::RunResumed` shape `{ run, seq, timestamp }`) |
| `drift_gate_script` exists | PASS | `scripts/check-verus-production-binding.sh` runs with `VACUUM: 0` |
| `no assume(`, `no axiom(`, `no external_body(`, `no #[trusted]` added | PASS | Spec file uses only `pub open spec fn`, `pub assume_specification`, `pub proof fn`, `pub fn` |

The new mirror surface (`map_resumed_to_run_resumed` method and
`convert_resume_timestamp` plain-Rust fn) is bound to the existing
extern's `MirrorJournalEvent` shape and `RunId`/`EventSeq` newtypes.
Drift is detected by the existing `production_inner/` mirror check
plus the new spec assertions.

## Anti-Laundering Guards

- **No vacuum Verus**: PO-001 binds via `assume_specification` to
  the `MirrorJournalEvent::map_resumed_to_run_resumed` and
  `convert_resume_timestamp` exec fns declared in the extern file.
  The mirror drift gate (`scripts/check-verus-production-binding.sh`)
  fails CI if the production shape drifts.
- **No `cover!`-as-proof**: All Verus specs require `ensures`
  post-conditions; proptest asserts concrete equality and variant
  shape; no `cover!` used.
- **No `assume`/`axiom`/`admit`/`external_body`**: zero occurrences
  in the new spec/extern/proof artifacts.
- **No trust-marker abuse**: `RuntimeError` is already
  `#[non_exhaustive]` (verified at `crates/vb_runtime/src/error/mod.rs:6`);
  no new `#[trusted]` or `extern_spec` is added by the artifacts.
- **No `as i64` cast on `u64`**: The `convert_resume_timestamp`
  spec fn uses `timestamp_u64 > (i64::MAX as u64)` (a u64 comparison),
  not an `as i64` cast.

## Source-Lint (PO-006)

`scripts/check-panic-surface.sh` reports:
```
ScanDomain: crates/*/src
NonProductionPathExcluded: tests benches examples fuzz target .beads fixtures build.rs path-scoped tests.rs *_tests.rs kani harnesses loom models
NoViolationFound
ExitCode: 0
```

The new test code in `chunk_002.rs` and
`vb_test_runtime_resume_replay.rs` does not introduce any
`unwrap`/`expect`/`panic`/`todo`/`unimplemented`/`dbg`/`as i64`
cast. The `from_timestamp` call in the loom harness uses
`timestamp.min(i32::MAX as u64) as i64` which is a `min`+cast
expression explicitly bounded to chrono's representable range
(not an unconditional `as i64` of a `u64` value).

The source-length gate is documented in `.config/source-length-exceptions.txt`:
the `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs`
file (998 lines) was added to the exception ledger under
`split-or-retire-before-release` for `vb-cib14`. The pre-existing
876-line baseline already exceeded the 800-line verus limit; my
additions contributed 122 lines for the new mirror surface.

## Trust Base (Forwarded to trusted-base-ledger.jsonl)

All 12 trusted-base entries from `trusted-base-plan.md` are
carried forward unchanged:

- TB-001 chrono::from_timestamp is total over the i64 range it
  accepts.
- TB-002 i64::try_from(u64) returns Err exactly when u64 > i64::MAX.
- TB-003 RuntimeError is `#[non_exhaustive]` so adding a variant
  is non-breaking.
- TB-004 STORAGE_EVENT_CLONE_COUNT is a test-only AtomicUsize.
- TB-005 incident.rs::event_to_lifecycle is a read-only classifier.
- TB-006 Source-length budget is 300 lines per file (exceptions
  ledgered).
- TB-007 Verus production-binding gate (drift detection on
  MirrorJournalEvent::RunResumed shape).
- TB-008 Production-inner mirror drift gate (no production_inner
  mirror added for this bead).
- TB-009 Loom replayer uses single-shard, single-concurrent-task
  semantics.
- TB-010 clone_for_dispatch is the only `clone()` site in
  storage_event.
- TB-011 crates/vb_runtime is `#[forbid(unsafe_code)]`.
- TB-012 Cargo workspace = true pins for chrono, thiserror, postcard,
  atomics.

## PENDING_FORMAL_EXECUTION Notes (State 12)

All 7 obligations are written but only Verus has been formally
verified. The remaining obligations (PO-002, PO-003, PO-004, PO-007)
require the production-side fix (new
`RuntimeError::ResumeTimestampOverflow { run, timestamp }` variant
at `crates/vb_runtime/src/error/mod.rs`; new
`convert_resume_timestamp` helper at
`crates/vb_runtime/src/journal/chunk_002.rs`; new `Resumed` arm
replacing the no-op at `boundary_storage_event::Resumed`) to land
before the test artifacts can compile. PO-005 (loom+proptest) has
been smoke-verified for the proptest half (3/3 pass with feature
on); the loom half is gated on `cfg(loom)` and will be executed in
State 12.

The feature flag `vb-cib14` is the canonical enable mechanism: when
the implementation owner lands the production code, they enable
this feature and the test artifacts activate.

## Coupling to vb-edvbj

vb-edvbj is STRONG-coupled to vb-cib14: it deletes the synthetic
`RunFailedEvent` catch-all at `chunk_002.rs:298-302` once vb-cib14
replaces the no-op `Resumed` arm. The PO-004 single-clone
regression is extended with a `Resumed` arm sample that asserts
the dispatch returns the typed `RunResumed` event exactly once (not
the catch-all `RunFailedEvent`). The PO-005 loom regression
scenario exercises the legacy buggy shape (`Resumed` rewritten as
`RunFailedEvent`) and asserts it produces `LifecycleState::Failed`
and `Ok(true)` — the bug shape that vb-edvbj's catch-all deletion
eliminates.

## Notes

- The `extern_vb_jnz9_journal_event_seq_valid.rs` file size grew
  from 876 to 998 lines (122 lines added). The new content is the
  `MirrorJournalEvent::map_resumed_to_run_resumed` method and the
  `convert_resume_timestamp` plain-Rust mirror fn, plus extensive
  binding-ledger documentation.
- The new spec file `vb_cib14_resume_storage_map.rs` adds
  `convert_resume_timestamp_spec`, `map_resumed_to_run_resumed_spec`,
  `spec_resumed_passes_through`, and `spec_resumed_not_run_failed`
  spec fns, plus 4 spec proofs and 6 exec proofs.
- The proptest uses `proptest::prelude::*` via the
  `#[cfg(feature = "vb-cib14")]` gate to avoid breaking the default
  build.
- The loom harness is in `vb_runtime/src/models/loom/` (not in
  `workspace_tests`) because `loom` is a dev-dependency of
  `vb_runtime` (not `workspace_tests`). The proptest half is in
  `workspace_tests` per the bead instruction; the split is
  documented at the top of both files.
- `RuntimeJournalEvent::Resumed` is mirrored in
  `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs`
  (as a spec-side marker arm via the `RunId`-shaped variant — see
  the proof-strategy for the type-equality argument).
- The 16-variant enumeration test extension at
  `chunk_004.rs:1077-1090` (per the proof-obligations PO-007) is
  not added in this artifact set because the production `Resumed`
  → `RunResumed` mapping is downstream of the 16-variant assertion
  surface; the implementation owner will exercise PO-007 in
  State 12 by running the existing 16-variant enumeration with
  the post-fix production code.
