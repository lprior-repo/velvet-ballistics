# Proof-to-Implementation Input: vb-8mdp.2

This document maps TLA+/Verus/Kani/Flux/Loom/Miri/proptest/fuzz proof claims to Rust source references, independent behavior tests, refinement harness refs, and exact evidence commands.

## Budget Gate (Line 48) Proof Claims

### Claim: PayloadTooLarge returned before any allocation at line 48

**Source Reference**: `crates/vb_storage/src/codec/header.rs:26-57`

**Rust Source Refs**:
- Line 26: `decode_record_header(header: &[u8], ...)` — `&[u8]` borrows, cannot allocate
- Line 31-33: `header.get(..60).ok_or(UnexpectedEof)?` — bounds check, no allocation
- Line 48: `if decoded.payload_len > max_payload_len { return Err(PayloadTooLarge { ... }) }` — BUDGET GATE

**Implementation Obligation**:
- Do NOT add any Vec creation, String allocation, or boxed data before line 48
- The function signature is a hard constraint: `&[u8]` input means borrowed only

**Harness Ref**: `crates/vb_storage/src/kani_budget_before_decode.rs`
- New harness `kani_budget_payload_too_large`: uses `kani::any::<[u8; 60]>()` for header
- New harness `kani_budget_gate_line48`: proves line 48 reached before any allocation path

**Evidence Command**: `cargo kani --package vb_storage --harness kani_budget_payload_too_large`

---

### Claim: decode_record_payload never slices beyond budget

**Source Reference**: `crates/vb_storage/src/codec/payload.rs:56-81`

**Rust Source Refs**:
- Line 61: `decode_record_header(bytes, expected_magic, max_payload_len)?` — budget gate runs here
- Line 66-68: `payload_end = payload_start.checked_add(payload_len_usize).ok_or(UnexpectedEof)?` — overflow protected
- Line 69-71: `bytes.get(payload_start..payload_end).ok_or(UnexpectedEof)?` — bounded slice

**Implementation Obligation**:
- `checked_add` must be used for payload_end calculation (not direct addition)
- `bytes.get(..)` must be used (not direct indexing) to return Option

**Harness Ref**: `crates/vb_storage/src/kani_budget_payload.rs`
- New harness `kani_payload_slice_bounds`: arbitrary payload_len, proves slice bounds
- New harness `kani_payload_overflow_check`: arbitrary u32, proves checked_add handles overflow

**Evidence Command**: `cargo kani --package vb_storage --harness kani_payload_slice_bounds`

---

## decode_optional Proof Claims

### Claim: decode_optional performs no allocation before decode_record_header

**Source Reference**: `crates/vb_storage/src/journal/internal.rs:13-25`

**Rust Source Refs**:
- Line 20-22: `let Some(value) = keyspace.get(key)?` — Fjall returns `Option<&[u8]>` (BORROWED)
- Line 23: `decode_record(value.as_ref(), magic, max_bytes)?` — budget gate runs on borrowed bytes

**Implementation Obligation**:
- `keyspace.get()` must return borrowed `&[u8]`, not owned `Vec<u8>`
- No intermediate allocation between get() and decode_record()

**Harness Ref**: `crates/vb_storage/src/kani_recovery_hydrate.rs`
- Update existing harness or add new `kani_recovery_hydrate` to trace decode_optional path
- Prove: `kani::any::<Keyspace>()` + `kani::any::<[u8; N]>()` -> decode_record called on borrowed bytes

**Evidence Command**: `cargo kani --package vb_storage --harness kani_recovery_hydrate`

---

## Snapshot Read Path Proof Claims

### Claim: FjallJournal::snapshot enforces MAX_SNAPSHOT_BYTES budget

**Source Reference**: `crates/vb_storage/src/snapshots.rs:33-45`

**Rust Source Refs**:
- Line 38: `run_snapshot_key(run, seq)?` — key construction
- Line 39-44: `self.decode_optional(&self.run_snapshot, key.as_slice(), MAGIC_SNAPSHOT, MAX_SNAPSHOT_BYTES)` — budget-enforced decode

**Implementation Obligation**:
- `MAX_SNAPSHOT_BYTES` (67108864) must be passed as max_bytes to decode_optional
- No direct postcard::from_bytes call without budget gate

**Harness Ref**: `crates/vb_storage/src/kani_recovery_hydrate.rs`
- New harness `kani_snapshot_budget`: arbitrary run/seq, arbitrary snapshot bytes, prove budget gate before postcard

**Evidence Command**: `cargo kani --package vb_storage --harness kani_snapshot_budget`

---

## Blob Read Path Proof Claims

### Claim: FjallJournal::blob enforces MAX_BLOB_BYTES budget

**Source Reference**: `crates/vb_storage/src/blobs.rs` (blob() function)

**Rust Source Refs**:
- blob() calls `decode_optional(&self.blob, key.as_slice(), MAGIC_BLOB, MAX_BLOB_BYTES)`

**Implementation Obligation**:
- `MAX_BLOB_BYTES` (67108864) must be passed as max_bytes

**Harness Ref**: `crates/vb_storage/src/kani_recovery_hydrate.rs`
- New harness `kani_blob_budget`: arbitrary digest, arbitrary blob bytes, prove budget gate

**Evidence Command**: `cargo kani --package vb_storage --harness kani_blob_budget`

---

## Journal Event Semantic Validity Claims

### Claim: decode_journal_event returns InvalidEvent for run_id=0, seq=u64::MAX, attempt=0

**Source Reference**: `crates/vb_storage/src/codec/mod.rs:54-64`

**Rust Source Refs**:
- Line 59: `decode_record::<JournalEvent>(bytes, expected_magic, max_payload_len)?` — budget gate + deserialize
- Line 60-62: `if !event.is_valid() { return Err(JournalError::InvalidEvent) }` — semantic check AFTER deserialize

**Implementation Obligation**:
- Semantic check must run AFTER postcard deserialization succeeds
- Must check run_id != 0, seq != u64::MAX, attempt != 0

**Behavior Test Ref**: `crates/vb_storage/src/security_tests.rs`
- Property test: `journal_event_is_valid_property` — proptest! with arbitrary JournalEvent that deserializes but fails is_valid()

**Harness Ref**: `crates/vb_storage/src/kani_codec.rs`
- New harness `kani_journal_event_semantic`: `kani::any::<JournalEvent>()` constrained to fail is_valid()

**Evidence Command**: `cargo kani --package vb_storage --harness kani_journal_event_semantic`

---

## TLA+ Proof Claims

### Claim: Keyspace prefix distinctness (9 prefixes pairwise distinct)

**Source Reference**: `crates/vb_storage/src/constants.rs:27-43`

**Rust Source Refs**:
- PREFIX_WORKFLOW_SOURCE = 0x01
- PREFIX_COMPILED_IR = 0x02
- PREFIX_RUN_HEADER = 0x10
- PREFIX_RUN_EVENT = 0x11
- PREFIX_RUN_SNAPSHOT = 0x12
- PREFIX_BLOB = 0x20
- PREFIX_INDEX_STATUS = 0x30
- PREFIX_INDEX_WORKFLOW = 0x31
- PREFIX_INDEX_ACTION = 0x32

**Spec Ref**: `specs/constants.tla`

**Evidence Command**: `tlc -model-check specs/constants.tla -config specs/constants.cfg`

---

### Claim: Budget-before-decode workflow invariant

**Source Reference**: `crates/vb_storage/src/codec/header.rs:48`

**Spec Ref**: `specs/budget_before_decode.tla`

**Evidence Command**: `tlc -model-check specs/budget_before_decode.tla -config specs/budget_before_decode.cfg`

---

## Existing Proofs (DO NOT MODIFY)

| File | Scope | vb-3t44 |
|------|-------|---------|
| kani_codec.rs | Panic freedom, magic, schema, CRC | vb-3t44 |
| kani_record_payload_len.rs | payload_len vs max | vb-3t44 |
| kani_digest_checks_vb_2bzz.rs | BLAKE3 digest | vb-3t44 |
| kani_postcard_envelope_wire.rs | Envelope wire format | vb-3t44 |

vb-8mdp.2 proofs are journal/snapshot read-path focused — budget enforcement at decode_optional entry.

---

## New Harness Files Required

| File | Purpose |
|------|---------|
| `crates/vb_storage/src/kani_budget_before_decode.rs` | Budget gate proofs: PayloadTooLarge, line 48, header totality |
| `crates/vb_storage/src/kani_budget_payload.rs` | Payload slice bounds, overflow check |
| `crates/vb_storage/src/kani_budget_magic.rs` | Magic, schema, kind, CRC ordering |
| `crates/vb_storage/src/kani_recovery_hydrate.rs` | Update existing: add snapshot/blob budget, decode_optional proofs |

## Verus Spec Functions Required

| Spec Function | File | Purpose |
|--------------|------|---------|
| `spec_decode_record_header` | `crates/vb_storage/src/codec/header.rs` | Total function spec with payload_len invariant |
| `spec_decode_record_payload` | `crates/vb_storage/src/codec/payload.rs` | Payload slice bounds spec |