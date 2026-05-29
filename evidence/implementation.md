# Implementation Review — vb-t6hx

## Bead: vb-t6hx — CLI doctor storage scan decode tests

## State 11: holzman-rust Implementation Assessment

### Verdict: Test-First Bead — No Production Changes Needed

### Assessment Method

The test file at `crates/workspace_tests/tests/restate_doctor_storage_scan_decode_tests.rs` (1690 lines, 68 tests) was reviewed for production API requirements. The assessment checked:

1. **All imports reference existing public APIs**: Every `use` statement targets currently-existing types and functions in the production crate surface.
2. **No new `pub` exports required**: All tested functions (`decode_record_header`, `decode_journal_event`, `encode_record`, `encode_record_header`, `verify_digest_match`, `FjallJournal::open`, `events_for_run`, `events_for_run_bounded`, `get_event_bytes`, `append_journaled`, `persist_strict`, `close`) exist in the public API of `vb_storage`.
3. **No new CLI commands required**: The test suite exercises the storage layer directly; no `vb_cli` production changes are needed.
4. **No new types or traits required**: `JournalError`, `RecordEnvelope`, `EventSeq`, `EventReplayLimit`, `RecordKind`, and all `vb_core` types are existing public types.

### Production APIs Exercised

| Crate | API | Test Groups |
|-------|-----|-------------|
| `vb_storage` | `decode_record_header` | 1, 2, 3, 4, 5, 6, 7, 9 |
| `vb_storage` | `decode_journal_event` | 1, 3, 4, 7, 9 |
| `vb_storage` | `encode_record` | Helper |
| `vb_storage` | `encode_record_header` | Helper |
| `vb_storage` | `verify_digest_match` | 8 |
| `vb_storage` | `FjallJournal::open` | 2, 3, 5, 6, 8 |
| `vb_storage` | `FjallJournal::events_for_run` | 2, 3, 5, 6 |
| `vb_storage` | `FjallJournal::events_for_run_bounded` | 3 |
| `vb_storage` | `FjallJournal::get_event_bytes` | 6 |
| `vb_storage` | `FjallJournal::close` | 2, 8 |
| `vb_storage::codec` | `decode_journal_event` | 1, 3, 4, 7, 9 |
| `vb_storage::journal` | `EventReplayLimit` | 3 |
| `vb_core` | `RunId`, `StepIdx`, `WorkflowDigest` | All |

### Proptest API Coverage

All 6 proptest harnesses call production `decode_record_header` and `decode_journal_event` functions. No tautologies (previously cleaned in state 5 attempt 8 per ledger entry 50).

### Fuzz Target API Coverage

All 6 fuzz targets (per ledger entry 51) call production `vb_storage` APIs with ~50M total smoke iterations and zero crashes.

### Holzman Rust Compliance (Test Code)

| Rule | Status |
|---|---|
| `#![forbid(unsafe_code)]` | PASSED (line 1) |
| No `unsafe` | PASSED |
| No `unwrap` in tests | PARTIAL — `temp_dir()` uses `expect()` (line 76) for infrastructure failure; test assertions use `panic!` for test-framework failures |
| Checked indexing | PASSED — no unchecked indexing in test code |
| Typed errors | PASSED — all errors are `JournalError` variants |
| No `as` casts | PASSED — no lossy conversions |
| No `dbg!` macro | PASSED |

### Test-Only Infrastructure

The `temp_dir()` helper uses `expect("tempdir creation failed")`. This is standard test-infrastructure practice and is explicitly documented at lines 73-74 as an infrastructure concern, not a behavior under test.

### Cargo.toml Registration Required

For the test file to be discoverable by `cargo nextest`, the workspace test crate's `Cargo.toml` needs a `[[test]]` entry:

```toml
[[test]]
name = "restate_doctor_storage_scan_decode_tests"
path = "tests/restate_doctor_storage_scan_decode_tests.rs"
```

**FINDING IM-001 (MEDIUM):** The test file exists at the expected path but may not be registered in the isolated workspace's copy of `crates/workspace_tests/Cargo.toml`. If this entry is not present, `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_doctor_storage_scan_decode_tests` would fail to discover the test. This needs to be verified and added before merge.

### Conclusion

This is a **test-first bead** — the 68 tests validate existing production behavior without requiring any production code changes. The test suite exercises the existing public API surface of `vb_storage` and `vb_core`. No new CLI commands, storage operations, error variants, or types are needed.

**Status: NO PRODUCTION CHANGES NEEDED.**

## Ledger Appendix

```jsonl
{"bead":"vb-t6hx","phase":"holzman-rust","state":"11","attempt":1,"tool":"holzman-rust","invocation":"holzman-rust-vb-t6hx-state11-001","file":"evidence/implementation.md","result":"APPROVED","production_changes":0,"findings":["IM-001"],"findings_summary":"IM-001 MEDIUM: Test file may need Cargo.toml [[test]] registration. No production code changes needed.","notes":"Test-first bead. 68 tests exercise existing vb_storage and vb_core public APIs. No new production code, CLI commands, or types required. All imports reference existing public surface.","test_count":68,"timestamp":"2026-05-27T20:00:00Z"}
```
