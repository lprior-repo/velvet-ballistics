# Proof-To-Rust Bridge Map - vb-e7tl trailing-byte storage repair

Date: 2026-06-01
Beads: vb-6kgdo, vb-ebha5, vb-3xpfv, vb-9k771
Status: UPDATED FOR TARGETED EVIDENCE

## Scope Decision

The compiled-IR admission/readback strictness introduced by commit `98cce991d` is retained, not reverted. It is no longer hidden inside the codec-only change: this bridge explicitly maps the admission/readback behavior, direct tests, fuzz evidence, and Moon fuzz-smoke repair.

## Behavior-Affecting Source Disclosure

| File | Behavior |
|---|---|
| `crates/vb_storage/src/codec/payload.rs` | Shared record-payload trailing-byte rejection through `reject_trailing_bytes` |
| `crates/vb_storage/src/codec/envelope.rs` | Envelope-only parser rejects bytes after declared payload |
| `crates/vb_storage/src/codec/mod.rs` | Public codec exports and `decode_record`/`decode_journal_event` strict decode path |
| `crates/vb_storage/src/journal/parse.rs` | Public journal-event parser propagates strict record decode errors |
| `crates/vb_storage/src/journal/mod.rs` | Exposes `parse_event` so public parser tests execute |
| `crates/vb_storage/src/admission.rs` | Compiled-IR `AcceptedArtifact` and inner `WorkflowParts` postcard envelopes reject trailing bytes |
| `crates/vb_storage/src/journal/source.rs` | `put_compiled_ir` and `compiled_ir` validate compiled-IR records at write and read boundaries |
| `crates/vb_storage/src/error/mod.rs` | `UnexpectedTrailingBytes` carries exact `declared_end` and `actual_len` offsets |
| `crates/vb_storage/src/error/codes.rs` | `UnexpectedTrailingBytes` maps to `JOURNAL_UNEXPECTED_TRAILING_BYTES` / `0x4030` |
| `crates/vb_core/src/diagnostic.rs` | Registers `JOURNAL_UNEXPECTED_TRAILING_BYTES` in the diagnostic registry |
| `crates/vb_storage/src/lib.rs` | Re-exports `decode_envelope_only`; adds accepted compiled-IR test fixture only under `cfg(test)` |
| `.moon/tasks/all.yml` | Runs the real libFuzzer target `journal_event_fuzz` in fuzz-smoke and copies `fuzz/corpus/journal_event` into the target-specific smoke corpus when present |

Non-behavior record formatting files from `98cce991d` are disclosed as formatting-only: `crates/vb_storage/src/records.rs`, `crates/vb_storage/src/records/entities.rs`, `crates/vb_storage/src/records/kinds.rs`, and `crates/vb_storage/src/records/status.rs`.

## Bridge Matrix

| Obligation | Claim | Source Refs | Behavior Tests | Evidence |
|---|---|---|---|---|
| VB-E7TL-001 | Storage record envelopes reject trailing bytes with exact offsets | `codec/payload.rs:82-108`, `codec/envelope.rs:27-52` | `decode_rejects_trailing_bytes_beyond_payload`, `decode_envelope_only_rejects_trailing_bytes_with_exact_offsets` | `reports/cargo-vb-storage-trailing-bytes.raw.txt`; `reports/kani-vb-storage-trailing-bytes.raw.txt` |
| VB-E7TL-002 | Public journal event parsing rejects trailing bytes | `journal/parse.rs:28-31`, `journal/mod.rs:12,21` | `parse_event_rejects_trailing_bytes_with_exact_offsets` | `reports/cargo-vb-storage-trailing-bytes.raw.txt` |
| VB-E7TL-003 | Compiled-IR `AcceptedArtifact` envelopes reject trailing bytes at the direct journal write/read admission boundaries | `admission.rs:361-375`, `journal/source.rs:48-79` | `put_compiled_ir_rejects_accepted_artifact_envelope_trailing_bytes` | `reports/cargo-vb-storage-lib.raw.txt` |
| VB-E7TL-004 | Compiled-IR inner `WorkflowParts` envelopes reject trailing bytes on read revalidation | `admission.rs:417-438` | `compiled_ir_rejects_workflow_parts_inner_trailing_bytes` | `reports/cargo-vb-storage-lib.raw.txt` |
| VB-E7TL-005 | Journal-event fuzz evidence is libFuzzer smoke-completion evidence, not stdin smoke output or deep fuzz coverage | `.moon/tasks/all.yml:462-474`, `fuzz/fuzz_targets/journal_event.rs:14-38` | N/A | `reports/fuzz-journal-event-fuzz.raw.txt` |

## Reclassification

The binary named `journal_event` in `fuzz/src/bin/journal_event.rs` is a stdin smoke runner and does not produce libFuzzer completion stats. It is not counted as fuzz proof. The scoped smoke obligation is satisfied by the libFuzzer target `journal_event_fuzz`, with direct empty-corpus completion and Moon seeded smoke completion recorded in `reports/fuzz-journal-event-fuzz.raw.txt`.

No behavior-affecting obligation is waived.
