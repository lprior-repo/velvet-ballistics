# Boundary Map: VB Storage Budget-Before-Decode

## Boundary Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        OUTER BOUNDARY: Process                          │
└─────────────────────────────────────────────────────────────────────────┘
    │
    │  Fjall Keyspace.get() returns Option<&[u8]>
    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  BOUNDARY A: Fjall Storage (vb_storage → fjall)                        │
│                                                                         │
│  FjallJournal {                                                        │
│    workflow_source: Keyspace  ──►  PREFIX_WORKFLOW_SOURCE (0x01)      │
│    compiled_ir:    Keyspace  ──►  PREFIX_COMPILED_IR    (0x02)        │
│    run_header:    Keyspace  ──►  PREFIX_RUN_HEADER     (0x10)        │
│    events:        Keyspace  ──►  PREFIX_RUN_EVENT      (0x11)        │
│    run_snapshot:  Keyspace  ──►  PREFIX_RUN_SNAPSHOT   (0x12)        │
│    blob:          Keyspace  ──►  PREFIX_BLOB           (0x20)        │
│    index_status:  Keyspace  ──►  PREFIX_INDEX_STATUS   (0x30)        │
│    index_workflow:Keyspace  ──►  PREFIX_INDEX_WORKFLOW (0x31)        │
│    index_action:  Keyspace  ──►  PREFIX_INDEX_ACTION   (0x32)        │
│  }                                                                     │
│                                                                         │
│  Fjall is the ONLY I/O boundary. All reads return borrowed &[u8].      │
└─────────────────────────────────────────────────────────────────────────┘
    │
    │  Option<&[u8]> — borrowed bytes, zero-copy from Fjall
    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  BOUNDARY B: Codec Boundary (immutable core / imperative shell)        │
│                                                                         │
│  decode_record_header(bytes, magic, max_payload_len)                   │
│      │                                                                 │
│      ├── Line 48: payload_len > max_payload_len → PayloadTooLarge      │
│      │    ★ BUDGET GATE — no allocation before this point ★            │
│      │                                                                 │
│      └── Returns RecordHeader { payload_len } with budget validated    │
│                                                                         │
│  decode_record_payload(bytes, magic, max_payload_len)                  │
│      │                                                                 │
│      ├── decode_record_header (budget gate)                            │
│      ├── bytes.get(60..60+payload_len) → bounded slice                │
│      └── verify_digest_match(payload, header.payload_digest)           │
│                                                                         │
│  decode_record(bytes, magic, max_payload_len) → (RecordEnvelope, T)    │
│      │                                                                 │
│      ├── decode_record_payload (budget gate + bounded slice)           │
│      └── postcard::from_bytes(payload) → typed T                      │
│                                                                         │
│  Pure codec functions: NO mutation, NO I/O, NO time, NO allocation     │
│  before budget gate.                                                    │
└─────────────────────────────────────────────────────────────────────────┘
    │
    │  (RecordEnvelope, T) — typed record, bounded payload
    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  BOUNDARY C: Application Layer (journal module methods)                │
│                                                                         │
│  decode_optional(keyspace, key, magic, max_bytes)                       │
│      │                                                                 │
│      ├── keyspace.get(key) → Option<&[u8]> (borrowed from Fjall)       │
│      └── decode_record(bytes, magic, max_bytes)                        │
│                                                                         │
│  FjallJournal methods: snapshot(), blob(), workflow_source(), etc.      │
│  These are the PUBLIC API of the storage module.                        │
└─────────────────────────────────────────────────────────────────────────┘
    │
    │  Result<Option<T>, JournalError>
    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│  BOUNDARY D: Recovery Layer (replay/recover/hydrate)                   │
│                                                                         │
│  recover_snapshot_plus_tail() → uses snapshot() + events_for_run()     │
│  recover_full_journal() → uses events_for_run()                        │
│  hydrate_run_frame() → uses snapshot() + events_for_run()              │
│                                                                         │
│  Recovery NEVER parses YAML, NEVER calls arbitrary code, NEVER         │
│  allocates beyond budget.                                                │
└─────────────────────────────────────────────────────────────────────────┘
```

## Boundary Responsibilities

### Boundary A: Fjall ↔ vb_storage
| Responsibility | Side |
|---------------|------|
| Keyspace creation and lifecycle | Fjall |
| Key prefix enforcement | vb_storage (key construction via `keys` module) |
| Data persistence and durability | Fjall |
| Returns borrowed `&[u8]` for reads | Fjall |
| `Option<&[u8]>` — None if key absent | Fjall |

### Boundary B: Codec (Pure Core)
| Responsibility | Constraint |
|---------------|-----------|
| No allocation before budget gate (line 48) | Hard rule |
| No I/O | Hard rule |
| No mutation | Hard rule |
| No time/randomness | Hard rule |
| Returns `Result<RecordHeader, JournalError>` | Total function |
| Bounded slice: `bytes[60..60+payload_len]` | After budget gate only |
| BLAKE3 digest verification | After budget gate |
| Postcard deserialize | After budget gate + digest |

### Boundary C: Journal Methods (Imperative Shell)
| Responsibility | Constraint |
|---------------|-----------|
| Key construction via `keys` module | Must use correct prefix |
| Calls decode_optional with correct magic/max_bytes | Type-safe |
| Maps Fjall errors to JournalError | journal-specific |
| Returns `Result<Option<T>, JournalError>` | Public API |

## Codec/Envelope vs. Storage Boundary

```
Fjall (bytes) ──► decode_record_header (line 48 budget check) ──► RecordHeader
      │                                                                  │
      │  & [u8] (borrowed)                                              │ payload_len
      │  raw wire bytes                                                 │ validated ≤ max
      ▼                                                                  ▼
decode_record_payload ─────────────────────────────────────────────► RecordEnvelope
      │                                                                       │
      │  & [u8] (borrowed)                                                 │ bounded slice
      │  [60..60+payload_len]                                              │ of exactly
      ▼                                                                       │ payload_len bytes
postcard::from_bytes ─────────────────────────────────────────────► T (typed)
```

**Key invariant**: The jump from raw `&[u8]` to typed `T` is gated by TWO independent checks:
1. `PayloadTooLarge` at line 48 (budget)
2. `PayloadDigestMismatch` at line 72 (integrity)

Neither check requires allocation.

## vb_storage vs. Fjall Boundary

| Concern | vb_storage responsibility | Fjall responsibility |
|---------|---------------------------|---------------------|
| Key format | `keys` module builds fixed-size keys with correct prefixes | Validates key bytes |
| Value format | 60-byte envelope + payload; codec handles encode/decode | Stores/retrieves raw bytes |
| Magic validation | `decode_record_header` checks magic matches expected | None |
| Budget enforcement | `decode_record_header` line 48 checks payload_len ≤ max | None |
| Serialization | Postcard encode/decode | None |
| Durability | Write path calls `keyspace.insert()` | Fjall handles fsync/persistence |

## Forbidden Cross-Boundary Effects

1. **No pre-budget allocation in codec**: All `Vec` creation in `decode_record_payload` and `decode_record` must occur AFTER line 48 budget check
2. **No Fjall mutation in read path**: Reads use `.get()` which is immutable lookup
3. **No YAML/JSON in codec**: Postcard only; YAML authoring is cold path only
4. **No HTTP/network in storage**: Storage is purely local filesystem via Fjall
5. **No time/randomness in codec**: Codec functions are pure; no `SystemTime` or `rand`