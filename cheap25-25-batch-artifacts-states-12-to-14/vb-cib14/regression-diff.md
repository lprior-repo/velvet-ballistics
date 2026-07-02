# Regression Diff — vb-cib14 (State 14)

Workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14
Base: jj working-copy change `zpmskmnz 96dfa778` (parent `b2a2ee46`)

## Files Modified by vb-cib14

### Production source

| File | Change | Lines (before → after) | Risk |
|---|---|---|---|
| `crates/vb_runtime/src/journal/chunk_002.rs` | Added explicit `Resumed` arm in `boundary_storage_event` (lines 252-256); added `convert_resume_timestamp` helper (lines 360-364); promoted `STORAGE_EVENT_CLONE_COUNT` from `static AtomicUsize` to `thread_local! RefCell<AtomicUsize>` (lines 321-325); updated test call sites to use thread-local access pattern. | 317 → 447 (+130) | Test infrastructure change for thread-local counter; no production behavior change. |
| `crates/vb_runtime/src/error/mod.rs` | Added `RuntimeError::ResumeTimestampOverflow { run: RunId, timestamp: u64 }` struct variant at lines 210-215. | 217 → 237 (+20) | Adding to `#[non_exhaustive]` enum is non-breaking. |
| `crates/vb_runtime/src/error/display.rs` | Added static Display message at lines 64-66. | 145 → 147 (+2) | Adds non-empty Display for new variant. |
| `crates/vb_runtime/src/error/diagnostics.rs` | Added `RESUME_TIMESTAMP_OVERFLOW_CODE = DiagnosticCode::new(0x2020)` and wired into `diagnostic_code()` (line 100); added `None` arm in `runtime_code()` (line 165). | 211 → 213 (+2) | Diagnostic code is non-breaking. |
| `crates/vb_runtime/src/error/equality.rs` | Added `runtime_error_resume_field_eq` helper (lines 219-227) and wired into `runtime_error_field_eq`. | 213 → 227 (+14) | Field equality for new struct variant. |

### Tests

| File | Change | Lines |
|---|---|---|
| `crates/vb_runtime/src/journal/tests/chunk_002.rs` | Updated proptest timestamp range cap to chrono's representable upper bound; made `storage_event_resume_timestamp_conversion_total_over_u64` actually exercise the production helper with `Ok`-path and overflow-path boundary sentinels; updated all `STORAGE_EVENT_CLONE_COUNT` call sites to use thread-local access pattern; gated the `CHRONO_MAX_SECS` constant on the `vb-cib14` feature. | 786 → 806 (+20) |

### Configuration

| File | Change |
|---|---|
| `.config/source-length-exceptions.txt` | Updated row 111 (chunk_002.rs exception) for new line count; row 374 added for extern_vb_jnz9_journal_event_seq_valid.rs under `split-or-retire-before-release` for vb-cib14. |

## Verification Artifacts Added

### Proof artifacts (NEW in State 5, validated in State 12)

| File | Lines |
|---|---|
| `verification/verus/vb_cib14_resume_storage_map.rs` (NEW) | 385 |
| `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs` (UPDATED) | 998 (was 876) |
| `crates/vb_runtime/src/models/loom/vb_cib14_resume_replay.rs` (NEW) | (loom-gated) |
| `crates/vb_runtime/src/models/loom/mod.rs` (UPDATED) | +1 line (module wiring) |
| `crates/workspace_tests/tests/vb_test_runtime_resume_replay.rs` (NEW) | (workspace_tests) |
| `crates/vb_runtime/Cargo.toml` (UPDATED) | +1 line (feature flag `vb-cib14`) |
| `crates/workspace_tests/Cargo.toml` (UPDATED) | feature + `[[test]]` entry |

## Diff Summary

```
.config/source-length-exceptions.txt             |  2 +-
crates/vb_runtime/src/journal/chunk_002.rs       | 25 ++++++++++++++++++-------
crates/vb_runtime/src/journal/tests/chunk_002.rs | 21 +++++++++++----------
```

(`jj diff --stat` confirms this is the working-copy delta.)

## Regression Risk Assessment

### Production Code Path

The production fix:
1. Adds a single explicit arm in `boundary_storage_event` for `Resumed` (5 lines).
2. Adds a 5-line helper `convert_resume_timestamp` that uses `i64::try_from(u64)` + `DateTime::<Utc>::from_timestamp(secs, 0)`.
3. Adds a struct variant `RuntimeError::ResumeTimestampOverflow { run, timestamp }` to the `#[non_exhaustive]` `RuntimeError` enum.
4. Wires the new variant through Display, Diagnostics, Equality.
5. Migrates the test-only `STORAGE_EVENT_CLONE_COUNT` from `static AtomicUsize` to `thread_local! RefCell<AtomicUsize>` to eliminate a cross-thread race that proptest 1.11 introduced when running the gated proptest alongside the pre-existing single-clone regression test.

**Zero new production-side lint violations.** Zero new `unsafe`, `unwrap`,
`expect`, `panic`, `todo`, `unimplemented`, `dbg`, or `as i64` cast on `u64`.

### Behavior Path

The production behavior change:
- Before: `storage_event(Resumed, _)` falls through the `_ => Self::boundary_storage_event(clone_for_dispatch(&event), seq)` arm at `chunk_002.rs:297`. The post-fix `boundary_storage_event::Resumed` arm was a no-op (`Ok(None)`). The catch-all `Ok(JournalEvent::RunFailedEvent { .. })` at lines 302-306 then mapped Resumed to a synthetic failure event.
- After: `storage_event(Resumed, _)` falls through the same `_ =>` arm. The post-fix `boundary_storage_event::Resumed` arm now returns `Ok(Some(JournalEvent::RunResumed { run, seq, timestamp: convert_resume_timestamp(run, timestamp)? }))`. The catch-all in `storage_event` is no longer reached for `Resumed` because `boundary_storage_event` returns `Some(_)`.

The user-visible behavior change: a `Resumed` runtime event now produces
`JournalEvent::RunResumed { run, seq, timestamp }` instead of the synthetic
`JournalEvent::RunFailedEvent { run, seq, attempt: 1 }`. The recovery-side
classifier `incident.rs::event_to_lifecycle` classifies `RunResumed` as
`LifecycleState::Active` (verified at `incident.rs:203`). The user-visible
symptom — a resumed run reported as `Failed` — is removed.

### Test Coverage

- 4 new proptest/cargo-test bodies in `tests/chunk_002.rs` (PO-002, PO-003, PO-007 + extended PO-004).
- 1 extended regression test in `tests/chunk_002.rs` (PO-004 single-clone Resumed arm).
- 1 new proptest module in `workspace_tests/tests/vb_test_runtime_resume_replay.rs` (PO-005 proptest half).
- 1 new loom module in `models/loom/vb_cib14_resume_replay.rs` (PO-005 loom half).

Total: 7 new tests + 1 extended test. All pass with raw command evidence in
`.beads/vb-cib14/evidence/state12-*.log`.

### Coupling to vb-edvbj

STRONG release coupling: vb-cib14 must land before (or simultaneously with)
vb-edvbj, which deletes the `Ok(JournalEvent::RunFailedEvent { .. })`
catch-all at `chunk_002.rs:298-302`. After vb-edvbj removes the catch-all,
the dispatch remains total (verified by PO-004 single-clone regression +
PO-007 16-variant enumeration).

## STATUS: APPROVED — for landing with vb-edvbj coupling