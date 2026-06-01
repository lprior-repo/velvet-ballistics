# Implementation Report - vb-e7tl follow-up repairs

Date: 2026-06-01
Beads: vb-6kgdo, vb-ebha5, vb-3xpfv, vb-9k771
Status: TARGETED EVIDENCE ONLY

## Code Changes

| File | Change |
|---|---|
| `crates/vb_storage/src/journal/mod.rs` | Added `parse` module and re-exported `parse_event` so the public parser is compiled and directly testable. |
| `crates/vb_storage/src/codec/tests.rs` | Added direct `decode_envelope_only` trailing-byte test with exact `UnexpectedTrailingBytes` fields. |
| `crates/vb_storage/src/journal/tests.rs` | Added direct public `parse_event` trailing-byte test with exact `UnexpectedTrailingBytes` fields. |
| `crates/vb_storage/src/tests.rs` | Added direct compiled-IR tests for outer `AcceptedArtifact` trailing bytes and inner `WorkflowParts` trailing bytes. |
| `.moon/tasks/all.yml` | Changed fuzz-smoke to execute `journal_event_fuzz` and copy the existing `journal_event` corpus into the target-specific smoke corpus when present. |

## Artifact Changes

| File | Change |
|---|---|
| `test-writer-report.md` | Preserved the existing report for its original scope. |
| `test-writer-report-vb-e7tl.md` | Added scoped vb-e7tl trailing-byte coverage report. |
| `evidence/test-writer/test-writer-report.md` | Preserved the existing evidence report for its original scope. |
| `evidence/test-writer/test-writer-report-vb-e7tl.md` | Added matching scoped vb-e7tl evidence report. |
| `proof-to-rust-map.md` | Preserved the active vb-fzgdn bridge. |
| `proof-to-rust-map-vb-e7tl.md` | Added scoped vb-e7tl final-diff bridge and source disclosure. |
| `rust-refinement-obligations.jsonl` | Preserved existing vb-fzgdn obligations and appended scoped vb-e7tl storage/fuzz obligations. |
| `regression-diff.md` | Added final-diff disclosure and scope classification. |
| `reports/fuzz-journal-event-fuzz.raw.txt` | Added raw fuzz completion evidence. |
| `verification-ledger.jsonl` | Added vb-e7tl/vb-9k771 test and fuzz ledger rows. |
| `reports/verification-ledger.jsonl` | Added matching scoped test and fuzz ledger rows. |

## Behavior-Affecting File Disclosure

The retained compiled-IR admission/readback behavior is explicitly in scope and covered by tests and bridge obligations. Behavior-affecting files are listed in `proof-to-rust-map-vb-e7tl.md` and `regression-diff.md`.

## Verification

| Command | Result |
|---|---|
| `cargo test -p vb_storage trailing_bytes -- --nocapture` | PASS; raw log: `reports/cargo-vb-storage-trailing-bytes.raw.txt` |
| `cargo test -p vb_storage --lib` | PASS: 1234 passed; raw log: `reports/cargo-vb-storage-lib.raw.txt` |
| `rtk cargo kani -p vb_storage --features kani-storage-trailing-bytes --harness kani_postcard_envelope_wire_trailing_bytes::vb_e7tl_trailing_bytes_required --harness kani_postcard_envelope_wire_trailing_bytes::vb_e7tl_trailing_bytes_rejected --exact -j 1 --output-format=regular` | PASS: 2 harnesses verified; raw log: `reports/kani-vb-storage-trailing-bytes.raw.txt` |
| `cargo fuzz run journal_event_fuzz --target x86_64-unknown-linux-gnu -- -runs=1024 -max_len=256` | PASS: direct default-corpus smoke completed `#1024 DONE`; raw log: `reports/fuzz-journal-event-fuzz.raw.txt` |
| `moon run :fuzz-smoke` | PASS: seeded smoke completed `#1165233 DONE`; raw log excerpt: `reports/fuzz-journal-event-fuzz.raw.txt` |

## Full Gate Limitations

This report does not claim canonical `moon ci` closure. `cargo fmt --check` reports pre-existing formatting drift outside this bead. This implementation intentionally did not rustfmt the workspace because doing so would rewrite unrelated concurrent work.
