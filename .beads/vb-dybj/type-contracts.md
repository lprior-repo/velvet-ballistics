# Type Contracts - vb-dybj State 3

## Public Types Under Contract

### `vb_core::RunId`

Contract shape:

- Representation: transparent wrapper over `u64`.
- Construction: `RunId::new(u64) -> RunId` is total.
- Access: `RunId::get() -> u64` returns the exact constructor input.
- Constants: `RunId::ZERO == RunId::new(0)`.
- Serialization: serde-derived Postcard serialization must be stable for fixed input values used in golden fixtures.

Illegal states made unrepresentable by the existing type:

- A run ID with non-`u64` numeric shape.
- A stringly run ID inside the core compatibility surface.

Representable but legal:

- `0`.
- `u64::MAX`.

Representable hazards to test/document:

- Future serde implementation changes can alter bytes while preserving semantic equality.
- Postcard varint bytes are not little-endian fixed-width bytes.

### `vb_core::WorkflowDigest`

Contract shape:

- Representation: transparent wrapper over `[u8; 32]`.
- Construction: `WorkflowDigest::from_bytes([u8; 32]) -> WorkflowDigest` is total for already-computed digest bytes.
- Access: `WorkflowDigest::as_bytes() -> [u8; 32]` returns the exact bytes.
- Serialization: Postcard encoding of selected fixed byte arrays is frozen by golden fixtures.

Illegal states made unrepresentable by the existing type:

- Digest with fewer than 32 bytes.
- Digest with more than 32 bytes.
- Heap/string digest in the core type.

Representable but legal:

- All-zero digest bytes.
- All-`0xff` digest bytes.
- Arbitrary 32-byte sequence; cryptographic validity is not checked by this newtype.

### `vb_storage::RecordKind`

Contract shape:

- Representation: `#[repr(u16)]` enum with explicit discriminants.
- Authoritative persisted ID: `RecordKind::id() -> u16`.
- Serialization: serde-derived Postcard enum representation is a separate compatibility surface from `RecordKind::id()`.
- Non-exhaustive: downstream code cannot assume no future variants.

Required distinction:

- Envelope record-kind field is little-endian `u16` from `RecordKind::id()`.
- Postcard bytes for `RecordKind` are produced by serde/Postcard enum serialization.
- Tests must not assert one while naming the other.

Selected variants for later tests should include:

- `RecordKind::RunAccepted`, because bead acceptance explicitly names it.
- One persisted record family variant such as `RecordKind::RunHeader` or `RecordKind::CompiledIr`, because storage envelope compatibility depends on family IDs.

### `vb_storage::RunHeaderStatus`

Contract shape:

- Representation: transparent wrapper over `u8`.
- Construction: `RunHeaderStatus::from_byte(u8)` is lossless and total.
- Access: `RunHeaderStatus::as_byte() -> u8` returns the exact byte.
- Known interpretation: `known()` returns typed known/unknown result without altering the persisted byte.

## Decode Error Contracts

### Raw Postcard surface

- Input: arbitrary byte slice intended to represent exactly one typed value.
- Success: decoded typed value and no trailing bytes.
- Failure: `postcard::Error` for malformed, too-short, or trailing data.
- Constraint: raw Postcard errors are external dependency errors, not VB `JournalError` variants.

### Storage envelope surface

- Input: storage record bytes containing the fixed 60-byte envelope and declared payload length.
- Success path: header validation, payload length validation, payload digest validation, then typed Postcard decode.
- Short input failure: `JournalError::UnexpectedEof`.
- Payload Postcard failure after valid envelope/digest: `JournalError::PostcardDecodeFailed`.
- Constraint: tests that require typed VB errors must use this surface or a typed VB adapter.

## Smart Constructor and Parser Boundaries

- No new smart constructor is required for this bead.
- Existing constructors are sufficient if tests only observe compatibility.
- Any future helper that converts bytes into selected values must return a typed `Result` and may not use string matching, unchecked slicing, `unwrap`, `expect`, or panics.

## Behavior-Affecting Type Invariants

- Golden fixture equality is byte-for-byte equality.
- Decode roundtrip equality is typed equality, not debug/display equality.
- Fixture names must encode the type, selected value, and wire surface.
- Migration changes require explicit fixture renaming or a named migration comment/assertion.
