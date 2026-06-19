# Workflow Model — Storage Envelope & Digest Verification Family

**Beads**: `vb-mrwe.1`, `vb-mrwe.2`, `vb-mrwe.3`, `vb-mrwe.5`
**Scope**: The decode/put/recover workflows that touch envelope strictness, digest verification, and digest-check level selection.

## Workflow 1 — `decode_record` (envelope strictness)

**Trigger**: Any external byte slice enters the storage layer (recovery hydration, doctor, IPC ingress).

```
                  ┌────────────────────────────────────────────────┐
                  │   External bytes                               │
                  └───────────────────┬────────────────────────────┘
                                      │
                                      ▼
                  ┌────────────────────────────────────────────────┐
                  │ decode_record_header(bytes)                    │
                  │   - magic == expected_magic?                   │
                  │   - schema_version == CURRENT_SCHEMA_VERSION?  │
                  │   - record_kind is known?                      │
                  │   - (magic, kind) family consistent?            │
                  │   - header_len == RECORD_HEADER_LEN?           │
                  │   - payload_len <= max_payload_len?            │
                  │   - CRC32C of header prefix matches?           │
                  └───────────────────┬────────────────────────────┘
                                      │ Ok(RecordHeader)
                                      ▼
                  ┌────────────────────────────────────────────────┐
                  │ payload = bytes[RECORD_HEADER_BYTES..end]      │
                  │   - try_into(payload_len_usize)?                │
                  │   - payload_start + payload_len overflows?      │
                  │   - bytes.get(payload_start..payload_end)?     │
                  └───────────────────┬────────────────────────────┘
                                      │ Ok(&[u8] payload)
                                      ▼
                  ┌────────────────────────────────────────────────┐
                  │ verify_digest_match(payload, header.digest)    │  (TC-2)
                  └───────────────────┬────────────────────────────┘
                                      │ Ok
                                      ▼
                  ┌────────────────────────────────────────────────┐
                  │ reject_trailing_bytes(payload_end, bytes.len)? │  (TC-1)
                  └───────────────────┬────────────────────────────┘
                                      │ Ok
                                      ▼
                  ┌────────────────────────────────────────────────┐
                  │ postcard::from_bytes(payload)                  │
                  └───────────────────┬────────────────────────────┘
                                      │ Ok(T)
                                      ▼
                  ┌────────────────────────────────────────────────┐
                  │ Ok((RecordEnvelope, T))                        │
                  └────────────────────────────────────────────────┘
```

**Failure outcomes** (all are typed `JournalError`, no panic):
- `BadMagic { found }` — magic mismatch.
- `UnsupportedSchemaVersion { found, max }` — schema version outside accepted range.
- `UnknownRecordKind { found }` — kind ID not in `RecordKind`.
- `KindFamilyMismatch { magic, kind }` — magic/kind pair not allowed together.
- `HeaderLengthMismatch { found }` — header_len ≠ RECORD_HEADER_LEN.
- `PayloadTooLarge { len, max }` — declared payload too big.
- `HeaderChecksumMismatch` — CRC32C of header prefix disagrees.
- `UnexpectedEof` — slice shorter than declared payload.
- `PayloadDigestMismatch` — `blake3(payload) ≠ header.digest`.
- `UnexpectedTrailingBytes { declared_end, actual_len }` — slice longer than declared payload.
- `PostcardDecodeFailed` — payload bytes are not valid postcard for `T`.
- `InvalidEvent` — semantic invariants of the decoded `JournalEvent` failed.

**Terminal**: `Ok((RecordEnvelope, T))` or one of the above typed errors.

## Workflow 2 — `put_workflow_source` / `put_compiled_ir` (admission)

```
                  ┌────────────────────────────────────────────────┐
                  │ Caller: WorkflowSourceRecord                   │
                  │   { source: Vec<u8>, digest: WorkflowDigest } │
                  └───────────────────┬────────────────────────────┘
                                      │
                                      ▼
                  ┌────────────────────────────────────────────────┐
                  │ verify_content_digest(source, digest)         │  (TC-3)
                  │   blake3(source) == digest?                    │
                  └───────────────────┬────────────────────────────┘
                                      │ Err(PayloadDigestMismatch) on mismatch
                                      │ Ok
                                      ▼
                  ┌────────────────────────────────────────────────┐
                  │ key = workflow_source_key(digest)              │
                  │ value = encode_record(MAGIC_WF_SOURCE, kind=1, │
                  │                          seq=0, payload=record)│
                  └───────────────────┬────────────────────────────┘
                                      │
                                      ▼
                  ┌────────────────────────────────────────────────┐
                  │ fjall_partition.insert(key, value)              │
                  └────────────────────────────────────────────────┘
```

For compiled IR, an extra stage is inserted between digest check and insert:

```
                  ┌────────────────────────────────────────────────┐
                  │ validate_compiled_ir_record(record)?            │
                  │   - envelope structure? warnings? seq bounds?  │
                  └───────────────────┬────────────────────────────┘
                                      │ Ok
                                      ▼
                  ┌────────────────────────────────────────────────┐
                  │ h_pending = compute_pending_metadata_hash(rec) │
                  │   = blake3 over canonical envelope fields:     │
                  │     source_digest, policy_digest, ir,          │
                  │     verification, accepted_at_seq,             │
                  │     required_capabilities                       │
                  └───────────────────┬────────────────────────────┘
                                      │
                                      ▼
                  ┌────────────────────────────────────────────────┐
                  │ existing = load_existing_compiled_ir(key)?     │
                  └───────────────────┬────────────────────────────┘
                                      │
                          ┌───────────┴────────────┐
                          │                        │
                       None                   Some(existing)
                          │                        │
                          ▼                        ▼
                  ┌───────────────┐    ┌────────────────────────────────┐
                  │ Ok — first    │    │ validate_metadata_hash_consistent│
                  │ write         │    │   h_pending == existing_hash?  │
                  └───────┬───────┘    │   (or == computed if hash was   │
                          │            │    absent in older records)    │
                          │            └────────────┬───────────────────┘
                          │             Ok           │ Err(MetadataMutation)
                          ▼                         ▼
                  ┌────────────────────────────────────────────────┐
                  │ insert(key, value with metadata_hash=h_pending)│
                  └────────────────────────────────────────────────┘
```

**Failure outcomes** (all typed):
- `PayloadDigestMismatch` — forged digest at admission.
- `ArtifactMalformed` — envelope structure invalid.
- `MetadataMutation { digest }` — same-digest re-write with divergent metadata.
- `PayloadTooLarge { len, max }` — envelope encoding rejects oversized payloads.
- Any codec failure surfaced by `encode_record`.

**Idempotence requirement**: re-writing the same `(digest, metadata_hash)` MUST be allowed. Re-writing the same `digest` with a different `metadata_hash` MUST be rejected.

## Workflow 3 — `verify_digests` (recovery boundary, vb-mrwe.3)

```
                  ┌────────────────────────────────────────────────┐
                  │ Caller: digest, level, optional config         │
                  └───────────────────┬────────────────────────────┘
                                      │
                                      ▼
                  ┌────────────────────────────────────────────────┐
                  │ check_workflow_and_ir                          │
                  │   if level.checks_workflow_source():           │
                  │     journal.events_for_run(run)                │
                  │       iterate to RunAccepted                   │
                  │       workflow_digest_bytes_equal(expected)?   │
                  │   if level.checks_compiled_ir():               │
                  │     check_compiled_ir_digest(expected, found)? │
                  └───────────────────┬────────────────────────────┘
                                      │ Ok
                                      ▼
                  ┌────────────────────────────────────────────────┐
                  │ check_full_level(config, level)                │
                  │   if level != Full: return Ok                  │
                  │   if config is None:                           │
                  │     return Err(FullDigestCheckConfigMissing)   │
                  │   if cfg.action_abi_entries is None:          │
                  │     return Err(FullDigestCheckConfigMissing)   │
                  │   if cfg.policy_entries is None:               │
                  │     return Err(FullDigestCheckConfigMissing)   │
                  │   check_action_abi_digests(entries)?           │
                  │   check_policy_digests(entries)?               │
                  └───────────────────┬────────────────────────────┘
                                      │ Ok
                                      ▼
                  ┌────────────────────────────────────────────────┐
                  │ Ok(())                                         │
                  └────────────────────────────────────────────────┘
```

**Strict-monotonicity requirement**: a strictly stronger level must verify a strict superset of digest classes. The current ordering `WorkflowSourceOnly ⊊ WorkflowAndIr ⊊ Full` reflects this.

**Failure outcomes**:
- `Journal(_)` — underlying journal error.
- `WorkflowSourceDigestMismatch { expected, found }` — workflow source disagrees.
- `CompiledIrDigestMismatch { expected, found }` — compiled IR disagrees.
- `NoRecoveryData { run }` — no `RunAccepted` event for the run.
- `ActionAbiMismatch { action_id, expected, found }` — first action ABI divergence (Full only).
- `PolicyDigestMismatch { step, expected, found }` — first policy divergence (Full only).
- `FullDigestCheckConfigMissing` — Full requested without config, or with one of the two slices omitted.

## Workflow 4 — `decode_journal_event` with typestate

The typestate `ValidatedJournalRecord` is the bridge between structural decode and semantic validity:

```
                  decode_record::<JournalEvent>(bytes)
                          │ Ok((envelope, event))
                          ▼
                  ValidatedJournalRecord::try_new(envelope, event)
                          │ checks: run_id != 0, seq != 0, attempt != 0, etc.
                          │
                          ├── Err(JournalError::InvalidEvent)
                          └── Ok(ValidatedJournalRecord)
                                      │
                                      ▼
                                  .into_parts() -> (RecordEnvelope, JournalEvent)
```

The typestate makes "structurally valid but semantically invalid event" unrepresentable at the API boundary — callers cannot obtain a `JournalEvent` from this path without the semantic checks passing.

## Cancellation / retry / idempotence

- All four workflows are **non-cancellable** at the function level. They are synchronous, total functions over their input slices; there is no I/O cancellation surface inside them.
- All four workflows are **idempotent** at the boundary they protect: re-decoding the same bytes yields the same `Ok`/`Err`; re-putting the same `(digest, metadata_hash)` for compiled IR is the only idempotent write path.
- Retries belong at the Fjall partition layer (`crates/vb_storage/src/journal`), not in these functions.

## Storage hazards specific to these workflows

- **Truncated writes** between header and payload insert are mitigated by the Fjall atomic per-key insert. The envelope itself never spans multiple keys.
- **Concurrent same-digest re-writes** are linearized by Fjall's per-key serialization; the second writer observes the first writer's `metadata_hash` and either matches (Ok) or rejects with `MetadataMutation`.
- **Header-only records** (e.g. recovery stamps) use a separate magic family (`MAGIC_RECOVERY_STAMP`) so they cannot collide with journal events even if their kind IDs ever drift.
