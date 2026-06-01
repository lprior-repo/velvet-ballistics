# Test Writer Report - vb-e7tl trailing-byte storage repair

Date: 2026-06-01
Beads: vb-ebha5, vb-6kgdo, vb-3xpfv, vb-9k771
Status: TARGETED EVIDENCE ONLY

## Scope

This scoped report covers vb-e7tl storage trailing-byte rejection and the follow-up review gaps for envelope-only parsing, public journal parsing, and compiled-IR inner envelopes. It does not replace unrelated bead test-writer artifacts.

## New Direct Tests

| Behavior | Test | File |
|---|---|---|
| `decode_envelope_only` rejects bytes after the declared payload with exact offsets | `decode_envelope_only_rejects_trailing_bytes_with_exact_offsets` | `crates/vb_storage/src/codec/tests.rs:1273` |
| Public `journal::parse_event` rejects bytes after the declared payload with exact offsets | `parse_event_rejects_trailing_bytes_with_exact_offsets` | `crates/vb_storage/src/journal/tests.rs:41` |
| `put_compiled_ir` rejects an otherwise valid `AcceptedArtifact` postcard envelope with trailing bytes | `put_compiled_ir_rejects_accepted_artifact_envelope_trailing_bytes` | `crates/vb_storage/src/tests.rs:1887` |
| `compiled_ir` read revalidation rejects an otherwise valid inner `WorkflowParts` postcard payload with trailing bytes | `compiled_ir_rejects_workflow_parts_inner_trailing_bytes` | `crates/vb_storage/src/tests.rs:1946` |

## Existing Supporting Tests

| Behavior | Existing Coverage |
|---|---|
| Generic `decode_record` trailing bytes | `decode_rejects_trailing_bytes_beyond_payload`, `decode_rejects_one_trailing_byte`, `decode_rejects_hundred_trailing_bytes`, `decode_rejects_large_trailing_boundary` |
| Typed family decode trailing bytes | `decode_rejects_trailing_bytes_across_record_magic_families` |
| Compiled IR digest and read revalidation | `put_compiled_ir_rejects_forged_digest`, `compiled_ir_read_revalidates_persisted_record` |

## Mutation Sensitivity Expectations

These mutants were not executed with `cargo-mutants`; this table records the intended tests that should fail if each behavior is removed.

| Mutant | Expected Detecting Test |
|---|---|
| Remove `payload::reject_trailing_bytes` from `decode_record_payload` | Existing `decode_rejects_*trailing*` tests |
| Remove `decode_envelope_only` trailing length check | `decode_envelope_only_rejects_trailing_bytes_with_exact_offsets` |
| Remove public parser trailing-byte propagation | `parse_event_rejects_trailing_bytes_with_exact_offsets` |
| Remove `decode_accepted_artifact_envelope` trailing-byte rejection | `put_compiled_ir_rejects_accepted_artifact_envelope_trailing_bytes` |
| Remove inner `WorkflowParts` trailing-byte rejection | `compiled_ir_rejects_workflow_parts_inner_trailing_bytes` |

## Executed Evidence

| Command | Result |
|---|---|
| `cargo test -p vb_storage trailing_bytes -- --nocapture` | PASS; raw log: `reports/cargo-vb-storage-trailing-bytes.raw.txt` |
| `cargo test -p vb_storage --lib` | PASS: 1234 passed; raw log: `reports/cargo-vb-storage-lib.raw.txt` |
| `rtk cargo kani -p vb_storage --features kani-storage-trailing-bytes --harness kani_postcard_envelope_wire_trailing_bytes::vb_e7tl_trailing_bytes_required --harness kani_postcard_envelope_wire_trailing_bytes::vb_e7tl_trailing_bytes_rejected --exact -j 1 --output-format=regular` | PASS: 2 harnesses verified; raw log: `reports/kani-vb-storage-trailing-bytes.raw.txt`; inventory: `reports/kani-vb-storage-trailing-bytes-list.json` |
| `cargo fuzz run journal_event_fuzz --target x86_64-unknown-linux-gnu -- -runs=1024 -max_len=256` | PASS: direct default-corpus smoke completed `#1024 DONE`; raw log: `reports/fuzz-journal-event-fuzz.raw.txt` |
| `moon run :fuzz-smoke` | PASS: seeded smoke completed `#1165233 DONE`; raw log excerpt: `reports/fuzz-journal-event-fuzz.raw.txt` |

## Full Gate Note

This report does not claim canonical `moon ci` closure. `cargo fmt --check` still reports pre-existing formatting drift in unrelated files and broad historical test modules. This bead did not run whole-workspace rustfmt because that would rewrite unrelated concurrent work. The evidence above is scoped to the targeted storage/fuzz/Kani obligations.
