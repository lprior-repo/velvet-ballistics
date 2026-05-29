# Domain Model - vb-dybj State 3

## Bead

- ID: `vb-dybj`
- Title: `core: Add Postcard newtype compatibility tests`
- State: 3, domain-contract
- Scope: domain/type contract for fixed-wire Postcard compatibility tests only.

## Ubiquitous Language

| Term | Meaning | Owner |
|---|---|---|
| Postcard payload | Compact binary bytes produced by `postcard` for a typed Rust value, excluding the storage envelope header. | `postcard`, serde derives on VB types |
| Golden fixture | A frozen byte sequence representing an intentional wire contract. Any change requires a named migration. | Workspace compatibility tests |
| Numeric newtype | `#[repr(transparent)]` Rust wrapper over an integer that derives `Serialize` and `Deserialize`, e.g. `RunId`. | `vb_core::ids` |
| Digest newtype | `WorkflowDigest([u8; 32])` wrapper used for workflow/source/compiled artifact identity. | `vb_core::ids` |
| Persisted record identifier | Stable storage record kind ID exposed by `RecordKind::id()` and represented in the envelope as `u16`. | `vb_storage::records` |
| Raw Postcard decode | Decoding with `postcard::from_bytes` directly, returning `postcard::Error`. | Test boundary |
| Storage decode | Decoding via `vb_storage::decode_record`, which validates envelope/header/digest before Postcard payload decode and maps errors to `JournalError`. | Storage boundary |
| Typed decode error | Stable error variant such as `JournalError::UnexpectedEof` or `JournalError::PostcardDecodeFailed`, not a string match. | `vb_storage::error` |

## Domain Actors

- Compatibility test author: freezes selected byte fixtures and documents migration requirement.
- Storage decoder: validates bounded envelope bytes before Postcard payload decode.
- Future migration author: must intentionally rename/update golden fixtures when wire bytes change.
- Runtime core: consumes stable numeric/digest/storage identifiers without JSON/YAML/HTTP interpretation.

## Entities and Value Objects

### `RunId`

- Newtype over `u64`.
- Constructor `RunId::new(value)` accepts every `u64`, including `0` and `u64::MAX`.
- `RunId::ZERO` is a valid explicit constant, not an error state.
- Postcard bytes for representative values are compatibility facts once frozen by tests.

### `WorkflowDigest`

- Newtype over exactly `[u8; 32]`.
- `WorkflowDigest::from_bytes([u8; 32])` is the only shape required for compatibility fixture construction.
- Serialized shape must remain exactly the digest bytes as encoded by Postcard for a fixed 32-byte array.
- Any shorter or longer external byte input is invalid at the parser/decode boundary, never a core digest value.

### `RecordKind`

- Non-exhaustive enum with stable explicit `u16` discriminants.
- `RecordKind::id()` is the authoritative persisted envelope ID.
- Postcard enum bytes and envelope `u16` IDs are distinct compatibility surfaces and must not be conflated.
- For this bead, tests must clearly name which surface each fixture covers.

### `RunHeaderStatus`

- Lossless `u8` persisted status newtype.
- Known status classification is separate from byte preservation.
- Unknown status bytes remain representable as persisted data, not as known runtime states.

## Aggregate Boundary

Compatibility fixtures form a `PostcardWireCompatibility` aggregate with these members:

1. Selected VB value object (`RunId`, `WorkflowDigest`, `RecordKind`, optionally `RunHeaderStatus`).
2. Frozen byte fixture.
3. Decode surface (`raw-postcard` or `storage-envelope`).
4. Migration rule: byte fixture change requires named migration evidence.

The aggregate invariant is: a fixture may be accepted only if the encoded bytes equal the frozen bytes and decoding yields the original typed value or a typed error for malformed input.

## Invariants

- Postcard compatibility tests must not introduce Bilrost, Protobuf, JSON wrappers, HTTP routing, or runtime YAML/JSON interpretation.
- Newtype compatibility is byte-level, not debug-string-level.
- `RunId::new(0)` and `RunId::ZERO` are equivalent valid values.
- `RunId::new(u64::MAX)` is valid and must not overflow during fixture construction or decode.
- `WorkflowDigest` fixtures must contain exactly 32 digest bytes before construction.
- `RecordKind::id()` values must match the master storage table for all selected variants.
- Direct raw Postcard trailing-byte failures are `postcard::Error`; storage-level typed mapping requires `decode_record` or a dedicated typed helper.
- Storage short-record failures must occur before payload allocation and before Postcard decode.

## Out of Scope

- Production code changes.
- Writing tests or proof artifacts in State 3.
- Selecting final verifier lanes or commands.
- Copying Restate code/API/layout/wire formats.
