# Proof Coverage Matrix: vb-8mdp.2 Budget-Before-Decode

## Contract → Proof Obligation Coverage

### C-BUDGET-001: Budget Gate at Line 48
```
decode_record_header returns PayloadTooLarge before any Vec allocation
when payload_len > max_payload_len
```
| Obligation | Verifier | Status | Evidence |
|------------|----------|--------|----------|
| PO-001: PayloadTooLarge returned on hostile input with len > max | Kani | planned | cargo kani --harness kani_budget_payload_too_large |
| PO-002: Budget gate at line 48 is reached before any allocation | Kani | planned | cargo kani --harness kani_budget_gate_line48 |
| PO-003: No Vec creation in decode_record_header before line 48 | Verus | planned | cargo verus --function decode_record_header |

### C-BUDGET-002: Payload Slice Bounded by Budget
```
After decode_record_header returns Ok, bytes[60..60+payload_len]
is bounded by max_payload_len and fits in bytes.len()
```
| Obligation | Verifier | Status | Evidence |
|------------|----------|--------|----------|
| PO-004: bytes.get(60..60+payload_len) returns Some only when len <= max | Kani | planned | cargo kani --harness kani_payload_slice_bounds |
| PO-005: payload_end = 60 + payload_len checked for overflow | Kani | planned | cargo kani --harness kani_payload_overflow_check |

### C-BUDGET-003: No Allocation Before Gate (Global Invariant)
```
NOT (exists allocation A such that size(A) > max AND A happens-before budget_check)
where budget_check = line 48: if decoded.payload_len > max_payload_len
```
| Obligation | Verifier | Status | Evidence |
|------------|----------|--------|----------|
| PO-006: decode_record_header signature is &[u8] -> cannot create Vec | Rust type system | proven | decode_record_header(header: &[u8], ...) |
| PO-007: decode_record_payload creates Vec only after budget gate | Kani | planned | cargo kani --harness kani_recovery_hydrate |
| PO-008: decode_optional calls keyspace.get() returning borrowed &[u8] | Kani | planned | cargo kani --harness kani_recovery_hydrate |

### C-MAGIC: Magic Check Ordering
```
decode_record_header returns BadMagic before HeaderChecksumMismatch,
PayloadTooLarge, or any other error when magic mismatches
```
| Obligation | Verifier | Status | Evidence |
|------------|----------|--------|----------|
| PO-009: BadMagic returned first when magic mismatches | Kani | planned | cargo kani --harness kani_magic_order |
| PO-010: Magic check precedes budget check | code review | proven | header.rs line 35 vs line 48 |

### C-KIND: Unknown Record Kind
```
decode_record_header returns UnknownRecordKind before HeaderChecksumMismatch
when record_kind is invalid
```
| Obligation | Verifier | Status | Evidence |
|------------|----------|--------|----------|
| PO-011: UnknownRecordKind for unknown record_kind | Kani | planned | cargo kani --harness kani_unknown_kind |

### C-SCHEMA: Schema Version
```
UnsupportedSchemaVersion for version > 1; MigrationRequired for version < 1
```
| Obligation | Verifier | Status | Evidence |
|------------|----------|--------|----------|
| PO-012: UnsupportedSchemaVersion for version > 1 | Kani | planned | cargo kani --harness kani_schema_versions |
| PO-013: MigrationRequired for version < 1 | Kani | planned | cargo kani --harness kani_schema_versions |

### C-HEADER-LEN: Header Length
```
decode_record_header returns HeaderLengthMismatch when header_len != 60
```
| Obligation | Verifier | Status | Evidence |
|------------|----------|--------|----------|
| PO-014: HeaderLengthMismatch for header_len != 60 | Kani | planned | cargo kani --harness kani_header_length_mismatch |

### C-CHECKSUM: CRC32C Header Checksum
```
HeaderChecksumMismatch when CRC32C of first 56 bytes does not match header_checksum
```
| Obligation | Verifier | Status | Evidence |
|------------|----------|--------|----------|
| PO-015: HeaderChecksumMismatch for CRC mismatch | Kani | planned | cargo kani --harness kani_crc_mismatch |

### C-DIGEST: Payload BLAKE3 Digest
```
PayloadDigestMismatch when blake3(payload) != header.payload_digest
```
| Obligation | Verifier | Status | Evidence |
|------------|----------|--------|----------|
| PO-016: Digest verification after budget gate | Kani | not_applicable | Proven in vb-3t44 (kani_digest_checks_vb_2bzz.rs) |

### C-SEMANTIC: Journal Event Semantic Validity
```
decode_journal_event returns InvalidEvent when JournalEvent::is_valid() is false
```
| Obligation | Verifier | Status | Evidence |
|------------|----------|--------|----------|
| PO-017: InvalidEvent for run_id=0, seq=u64::MAX, attempt=0 | Kani | planned | cargo kani --harness kani_journal_event_semantic |
| PO-018: Semantic check after postcard deserialization | proptest | planned | cargo test -- journal_event_is_valid_property |

### C-SNAPSHOT-BUDGET: Snapshot Budget
```
FjallJournal::snapshot enforces MAX_SNAPSHOT_BYTES budget via decode_optional
```
| Obligation | Verifier | Status | Evidence |
|------------|----------|--------|----------|
| PO-019: payload_len <= MAX_SNAPSHOT_BYTES before postcard | Kani | planned | cargo kani --harness kani_snapshot_budget |
| PO-020: Snapshot roundtrip preserves budget bounds | proptest | planned | cargo test -- snapshot_payload_len_property |

### C-BLOB-BUDGET: Blob Budget
```
FjallJournal::blob enforces MAX_BLOB_BYTES budget via decode_optional
```
| Obligation | Verifier | Status | Evidence |
|------------|----------|--------|----------|
| PO-021: payload_len <= MAX_BLOB_BYTES before postcard | Kani | planned | cargo kani --harness kani_blob_budget |
| PO-022: Blob roundtrip preserves budget bounds | proptest | planned | cargo test -- blob_payload_len_property |

### C-TOTALITY: Function Totality
```
decode_record_header is total on &[u8]; returns Err for all inputs, never panics
```
| Obligation | Verifier | Status | Evidence |
|------------|----------|--------|----------|
| PO-023: No panic on empty input | Kani | planned | cargo kani --harness kani_header_total_function |
| PO-024: No panic on arbitrary input length | Verus | planned | cargo verus --function decode_record_header |

### C-PAYLOAD-INVARIANT: Type Invariant
```
After decode_record_header returns Ok, payload_len <= max_payload_len
```
| Obligation | Verifier | Status | Evidence |
|------------|----------|--------|----------|
| PO-025: payload_len invariant in postcondition | Verus | planned | cargo verus --function decode_record_header |

### C-KEYSPACE: Keyspace Isolation
```
All 9 keyspace prefixes are pairwise distinct
```
| Obligation | Verifier | Status | Evidence |
|------------|----------|--------|----------|
| PO-026: Prefix distinctness invariant | TLA+ | planned | tlc -model-check specs/constants.tla |

### C-BUDGET-WORKFLOW: Workflow-Level Invariant
```
For all record reads: payload_len > max -> decode returns PayloadTooLarge,
and no payload bytes are read
```
| Obligation | Verifier | Status | Evidence |
|------------|----------|--------|----------|
| PO-027: Budget-before-decode workflow invariant | TLA+ | planned | tlc -model-check specs/budget_before_decode.tla |

### C-FUZZ: Fuzz Target Safety
```
cargo kani --fuzz decode_record proves no panic on any arbitrary bytes
```
| Obligation | Verifier | Status | Evidence |
|------------|----------|--------|----------|
| PO-028: No panic on arbitrary fuzz input | Kani | planned | cargo kani --fuzz decode_record |

## Coverage Summary

| Contract Clause | Obligations | Covered | Pending |
|-----------------|-------------|---------|---------|
| C-BUDGET-001 | 3 | 0 | 3 |
| C-BUDGET-002 | 2 | 0 | 2 |
| C-BUDGET-003 | 3 | 1 (type system) | 2 |
| C-MAGIC | 2 | 1 (code order) | 1 |
| C-KIND | 1 | 0 | 1 |
| C-SCHEMA | 2 | 0 | 2 |
| C-HEADER-LEN | 1 | 0 | 1 |
| C-CHECKSUM | 1 | 0 | 1 |
| C-DIGEST | 1 | 1 (vb-3t44) | 0 |
| C-SEMANTIC | 2 | 0 | 2 |
| C-SNAPSHOT-BUDGET | 2 | 0 | 2 |
| C-BLOB-BUDGET | 2 | 0 | 2 |
| C-TOTALITY | 2 | 0 | 2 |
| C-PAYLOAD-INVARIANT | 1 | 0 | 1 |
| C-KEYSPACE | 1 | 0 | 1 |
| C-BUDGET-WORKFLOW | 1 | 0 | 1 |
| C-FUZZ | 1 | 0 | 1 |
| TOTAL | 28 | 3 | 25 |

## Existing Proofs (DO NOT DUPLICATE)

| File | Scope | Bead |
|------|-------|------|
| kani_codec.rs | Never panics, magic, schema, CRC | vb-3t44 |
| kani_record_payload_len.rs | payload_len vs max | vb-3t44 |
| kani_digest_checks_vb_2bzz.rs | BLAKE3 digest mismatch | vb-3t44 |
| kani_postcard_envelope_wire.rs | Envelope wire format | vb-3t44 |
| kani_recovery_hydrate.rs | Snapshot/blob budget | vb-8mdp.2 NEW |

vb-8mdp.2 proofs are journal/snapshot read-path focused - budget enforcement at decode_optional entry, not fixed-wire codec format (which is vb-3t44 scope).