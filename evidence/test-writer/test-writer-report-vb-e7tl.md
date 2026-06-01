# Test Writer Report - vb-e7tl trailing-byte storage repair

Date: 2026-06-01
Beads: vb-ebha5, vb-6kgdo, vb-3xpfv, vb-9k771
Status: TARGETED EVIDENCE ONLY

## Scope

This scoped report covers vb-e7tl storage trailing-byte rejection and the follow-up review gaps for envelope-only parsing, public journal parsing, and compiled-IR inner envelopes. It does not replace unrelated Section 16 diagnostic artifacts.

## New Direct Tests

| Behavior | Test | File |
|---|---|---|
| `decode_envelope_only` rejects bytes after the declared payload with exact offsets | `decode_envelope_only_rejects_trailing_bytes_with_exact_offsets` | `crates/vb_storage/src/codec/tests.rs:1273` |
| Public `journal::parse_event` rejects bytes after the declared payload with exact offsets | `parse_event_rejects_trailing_bytes_with_exact_offsets` | `crates/vb_storage/src/journal/tests.rs:41` |
| `put_compiled_ir` rejects an otherwise valid `AcceptedArtifact` postcard envelope with trailing bytes | `put_compiled_ir_rejects_accepted_artifact_envelope_trailing_bytes` | `crates/vb_storage/src/tests.rs:1887` |
| `compiled_ir` read revalidation rejects an otherwise valid inner `WorkflowParts` postcard payload with trailing bytes | `compiled_ir_rejects_workflow_parts_inner_trailing_bytes` | `crates/vb_storage/src/tests.rs:1946` |

## Evidence

| Command | Result |
|---|---|
| `cargo test -p vb_storage trailing_bytes -- --nocapture` | PASS; raw log: `reports/cargo-vb-storage-trailing-bytes.raw.txt` |
| `cargo test -p vb_storage --lib` | PASS: 1234 passed; raw log: `reports/cargo-vb-storage-lib.raw.txt` |
| `rtk cargo kani -p vb_storage --features kani-storage-trailing-bytes --harness kani_postcard_envelope_wire_trailing_bytes::vb_e7tl_trailing_bytes_required --harness kani_postcard_envelope_wire_trailing_bytes::vb_e7tl_trailing_bytes_rejected --exact -j 1 --output-format=regular` | PASS: 2 harnesses verified; raw log: `reports/kani-vb-storage-trailing-bytes.raw.txt`; inventory: `reports/kani-vb-storage-trailing-bytes-list.json` |
| `cargo fuzz run journal_event_fuzz --target x86_64-unknown-linux-gnu -- -runs=1024 -max_len=256` | PASS: direct default-corpus smoke completed `#1024 DONE`; raw log: `reports/fuzz-journal-event-fuzz.raw.txt` |
| `moon run :fuzz-smoke` | PASS: seeded smoke completed `#1165233 DONE`; raw log excerpt: `reports/fuzz-journal-event-fuzz.raw.txt` |
