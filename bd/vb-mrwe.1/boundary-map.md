# Boundary Map — Storage Envelope & Digest Verification Family

**Beads**: `vb-mrwe.1`, `vb-mrwe.2`, `vb-mrwe.3`, `vb-mrwe.5`

## Layering

```
                ┌──────────────────────────────────────────────────────────┐
                │                   IMPERATIVE SHELL                       │
                │  (CLI, IPC ingress, runtime hydration, doctor, replay)    │
                └──────────────────────────┬───────────────────────────────┘
                                           │  bytes
                                           ▼
   ┌─────────────────────────────────────────────────────────────────────┐
   │                          PARSER BOUNDARY                             │
   │   codec/payload.rs::decode_record_payload                            │
   │   codec/record.rs::decode_record / decode_journal_event             │
   │   codec/envelope.rs::decode_envelope_only                           │
   │                                                                      │
   │   Validates: magic, schema, kind, family, header_len, payload_len,  │
   │              CRC32C header checksum, BLAKE3 payload digest,         │
   │              trailing-bytes strict length, postcard shape,          │
   │              journal-event semantic invariants.                     │
   │   Returns: typed JournalError on every malformed input.              │
   └──────────────────────────────────┬──────────────────────────────────┘
                                      │  RecordEnvelope + T
                                      ▼
   ┌─────────────────────────────────────────────────────────────────────┐
   │                            PURE CORE                                 │
   │   recovery/types/digest.rs   (DigestCheck enum + predicates)         │
   │   recovery/digest.rs         (workflow_digest_bytes_equal,           │
   │                               first_action_abi_mismatch,             │
   │                               first_policy_mismatch)                │
   │   records/kinds.rs           (RecordKind enum + wire ID map)         │
   │   constants.rs               (MAGIC_*, MAX_*_BYTES, RECORD_HEADER_*) │
   │   records.rs                 (RecordEnvelope, RecordHeader)          │
   │                                                                      │
   │   No I/O. No time. No randomness. Total functions over inputs.       │
   └──────────────────────────────────┬──────────────────────────────────┘
                                      │
                                      ▼
   ┌─────────────────────────────────────────────────────────────────────┐
   │                       ADMISSION + PUT BOUNDARY                       │
   │   journal/source.rs::put_workflow_source                            │
   │     - verify_content_digest before insert                           │
   │   journal/source.rs::put_compiled_ir                                │
   │     - validate_compiled_ir_record                                   │
   │     - compute_pending_metadata_hash                                 │
   │     - validate_metadata_hash_is_consistent (rejects mutation)       │
   │     - insert_compiled_ir_record                                     │
   │   journal/admission.rs::verify_content_digest                       │
   │                                                                      │
   │   Returns: typed JournalError on every admission failure.            │
   └──────────────────────────────────┬──────────────────────────────────┘
                                      │  Fjall partition insert
                                      ▼
   ┌─────────────────────────────────────────────────────────────────────┐
   │                       STORAGE / ASYNC SHELL                         │
   │   crates/vb_storage/src/journal (FjallJournal, partition readers)    │
   │   crates/vb_storage/src/recovery (recovery orchestrator)             │
   │   crates/vb_storage/src/blobs, src/snapshots (separate envelopes)   │
   └─────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
   ┌─────────────────────────────────────────────────────────────────────┐
   │                       RECOVERY BOUNDARY                             │
   │   recovery/recover.rs::verify_digests                               │
   │     - check_workflow_and_ir                                         │
   │     - check_full_level (with fail-closed config check)              │
   │   recovery/recover.rs::check_workflow_source_digest                 │
   │   recovery/recover.rs::check_compiled_ir_digest                     │
   │   recovery/recover.rs::check_action_abi_digests                     │
   │   recovery/recover.rs::check_policy_digests                         │
   │                                                                      │
   │   Returns: typed RecoveryError on every digest mismatch.             │
   └─────────────────────────────────────────────────────────────────────┘
```

## Boundary contracts

### BC-1 — Parser → Core
- **Input**: `&[u8]` from any source (network, disk, IPC, Fjall).
- **Output**: `Result<(RecordEnvelope, T), JournalError>` or `Result<ValidatedJournalRecord, JournalError>`.
- **Guarantee**: any malformed input that survives the parser boundary has been decoded into a typed value with all envelope invariants satisfied (magic, schema, kind, family, length, checksum, digest, no trailing bytes, valid postcard).

### BC-2 — Core → Admission
- **Input**: typed record (e.g. `WorkflowSourceRecord`, `CompiledIrRecord`).
- **Output**: `Result<(), JournalError>` from the put path.
- **Guarantee**: every put either succeeds with the value durably stored under the digest-keyed partition, or returns a typed error without partial state. The metadata-mutation invariant for compiled IR is enforced before any second insert.

### BC-3 — Admission → Recovery
- **Input**: a `FjallJournal` reference, a `RunId`, expected digests, a `DigestCheck` level, optional `DigestCheckConfig`.
- **Output**: `RecoveryResult<()>`.
- **Guarantee**: recovery-boundary digest checks fail closed at Full strictness (missing or partial config → `FullDigestCheckConfigMissing`). Mismatches return typed errors with `(expected, found)` digests. No silent acceptance.

### BC-4 — Storage → Fjall
- **Input**: `Vec<u8>` value, key derived from digest.
- **Output**: Fjall's per-key atomic insert result.
- **Guarantee**: Fjall linearizes same-key writes; the second writer's metadata-hash comparison observes the first writer's stored hash.

## Forbidden layers

- **No `unsafe`** in any layer (`#![forbid(unsafe_code)]` at crate root).
- **No `unwrap`/`expect`/`panic`/`todo`/`unimplemented`/`dbg`** in any layer.
- **No `std::time`, `Instant::now`, `SystemTime`** in the pure core. Time is an async-shell concern.
- **No `std::net`, async runtime, channel primitives** in the pure core. The parser boundary is `&[u8]`-in, typed-value-out.
- **No YAML, JSON, HTTP** in the storage layer (matches the master engineering rule "no YAML, JSON, or HTTP in the runtime core").
- **No direct Fjall calls** from the pure core or parser layers. The admission boundary is the only place the storage layer touches Fjall.

## External dependency surface

- `vb_core::WorkflowDigest`, `vb_core::ActionId`, `vb_core::StepIdx`, `vb_core::RunId` — typed IDs from the domain crate.
- `vb_core::action::action_ticket_has_valid_key` — pure helper for ticket validity.
- `postcard` — serialize/deserialize the payload.
- `blake3` — content hashing.
- `crc32c` — header checksum (modeled in `#[cfg(kani)]`).
- `serde` — derive `Serialize`/`Deserialize` for typed record shapes.
- `thiserror` — derive `Error` for both error enums.

No time, no network, no filesystem in the codec / digest pure-core paths.
