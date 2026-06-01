# Regression Diff - vb-e7tl final-diff disclosure

Date: 2026-06-01
Status: UPDATED FOR TARGETED EVIDENCE

## Scope Classification

The vb-e7tl landing is not only a generic storage codec change. It also changes compiled-IR admission/readback strictness by validating `AcceptedArtifact` and inner `WorkflowParts` postcard envelopes at write/read boundaries. That behavior is retained and explicitly covered by vb-6kgdo, vb-ebha5, vb-3xpfv, and vb-9k771.

## Behavior-Affecting Files

| File | Classification |
|---|---|
| `crates/vb_storage/src/codec/payload.rs` | Strict storage record payload trailing-byte rejection |
| `crates/vb_storage/src/codec/envelope.rs` | Strict envelope-only trailing-byte rejection |
| `crates/vb_storage/src/codec/mod.rs` | Public codec strict decode path/export |
| `crates/vb_storage/src/journal/parse.rs` | Public journal parser strict decode path |
| `crates/vb_storage/src/journal/mod.rs` | Parser module/export wiring |
| `crates/vb_storage/src/admission.rs` | Compiled-IR outer and inner postcard strictness, fallible policy digest |
| `crates/vb_storage/src/journal/source.rs` | Compiled-IR write/read validation |
| `crates/vb_storage/src/error/mod.rs` | Exact trailing-byte error fields |
| `crates/vb_storage/src/error/codes.rs` | Trailing-byte diagnostic code mapping |
| `crates/vb_core/src/diagnostic.rs` | Trailing-byte diagnostic registry entry |
| `crates/vb_storage/src/lib.rs` | Public `decode_envelope_only` export and test-only accepted artifact fixture |
| `.moon/tasks/all.yml` | Fuzz-smoke executes the libFuzzer journal-event target |

## Test and Evidence Files

| File | Classification |
|---|---|
| `crates/vb_storage/src/codec/tests.rs` | Direct and existing trailing-byte behavior tests |
| `crates/vb_storage/src/journal/tests.rs` | Public parser trailing-byte behavior test |
| `crates/vb_storage/src/tests.rs` | Compiled-IR admission/readback trailing-byte behavior tests |
| `crates/vb_storage/src/codec/trailing_bytes_proptests.rs` | Property coverage for record trailing-byte rejection |
| `crates/vb_storage/src/kani_postcard_envelope_wire_trailing_bytes.rs` | Kani harness for trailing-byte bounds |
| `reports/cargo-vb-storage-trailing-bytes.raw.txt` | Raw targeted trailing-byte test evidence |
| `reports/cargo-vb-storage-lib.raw.txt` | Raw `vb_storage` lib test evidence |
| `reports/kani-vb-storage-trailing-bytes.raw.txt` | Raw Kani trailing-byte bounds evidence |
| `reports/kani-vb-storage-trailing-bytes-list.json` | Kani harness inventory for the scoped feature run |
| `reports/fuzz-journal-event-fuzz.raw.txt` | Raw libFuzzer smoke-completion evidence |
| `proof-to-rust-map-vb-e7tl.md` | Scoped vb-e7tl proof-to-Rust bridge |
| `test-writer-report-vb-e7tl.md` | Scoped vb-e7tl test-writer report |
| `evidence/test-writer/test-writer-report-vb-e7tl.md` | Scoped vb-e7tl evidence report |
| `verification-ledger.jsonl` | Updated scoped test/fuzz rows |
| `reports/verification-ledger.jsonl` | Updated scoped test/fuzz rows |

## Reclassified Evidence

The `journal_event` binary is a stdin smoke runner and is not counted as fuzz proof. The scoped libFuzzer smoke obligation is mapped to `journal_event_fuzz`, which has direct default-corpus completion output and Moon seeded smoke completion output. No behavior-affecting obligation is waived.
