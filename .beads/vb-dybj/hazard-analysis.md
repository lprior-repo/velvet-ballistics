# Hazard Analysis - vb-dybj State 3

## H1: Raw Postcard vs Envelope ID Confusion

- Hazard: A test named `RecordKind Postcard bytes` may actually assert `RecordKind::id()` little-endian envelope bytes, or vice versa.
- Impact: False compatibility confidence; migration break could pass unnoticed.
- Contract control: Every fixture name must include `postcard_enum` or `envelope_id_u16_le`.
- Proof seed: distinguish serialization surface equivalence/non-equivalence.

## H2: Typed Error Surface Mismatch

- Hazard: Direct `postcard::from_bytes` errors are asserted as if they were `JournalError` variants.
- Impact: Acceptance criteria for typed errors appears satisfied while storage API is untested.
- Contract control: Trailing/missing byte tests must declare raw or storage surface. VB typed errors require `decode_record` or explicit typed adapter.

## H3: Varint Misunderstanding for `RunId`

- Hazard: Fixture author assumes `u64` encodes as fixed 8-byte little-endian.
- Impact: Incorrect fixtures lock in wrong contract or produce brittle tests.
- Contract control: Golden bytes must be generated/confirmed from the pinned `postcard` dependency and then frozen.

## H4: Zero Run ID Rejected by Invented Invariant

- Hazard: Tests assume `RunId(0)` is invalid.
- Impact: Tests contradict current constructor contract and `RunId::ZERO`.
- Contract control: Zero fixture must assert legal behavior.

## H5: Maximum Value Overflow During Fixture Construction

- Hazard: `u64::MAX` fixture construction or decode uses unchecked arithmetic/casts.
- Impact: Panic/overflow or silently wrong fixture.
- Contract control: Use direct constructor and typed equality; no arithmetic required.

## H6: Digest Length Drift

- Hazard: Digest fixture uses variable-length vector/string or hex text and tests a wrapper rather than `[u8; 32]` binary bytes.
- Impact: JSON/text compatibility sneaks into runtime/core contract.
- Contract control: Digest fixtures must be typed `[u8; 32]` values and compare raw Postcard bytes.

## H7: Accidental Dependency Introduction

- Hazard: Bilrost, Protobuf, or JSON tooling is introduced while copying inspiration from Restate.
- Impact: Violates master contract and no-copy fence.
- Contract control: This bead requires only existing `postcard` and `serde`; dependency changes are suspect.

## H8: Golden Byte Change Without Migration

- Hazard: Future serde/Postcard/type-layout change alters bytes and tests are updated silently.
- Impact: Existing persisted data becomes unreadable without migration evidence.
- Contract control: Test names/comments must state named migration requirement; golden changes trigger release decision.

## H9: Non-Exhaustive `RecordKind` Assumption

- Hazard: Tests or helper tables assume exhaustive variant list and fail future compatibility additions improperly.
- Impact: Future extension becomes unsafe or misleading.
- Contract control: Select explicit variants; do not require exhaustive matching in compatibility tests unless a separate contract demands it.

## H10: Short Decode Allocation/Ordering Regression

- Hazard: Storage decode allocates payload or invokes Postcard before checking length.
- Impact: Hostile/truncated input could waste resources or change typed error order.
- Contract control: Missing bytes contract is `UnexpectedEof` before payload decode.

## Residual Illegal-State Risks

- `RecordKind` serde representation and `RecordKind::id()` remain two legal representations of related concepts; naming discipline is needed to prevent ambiguity.
- Raw `postcard::Error` is external and less semantically stable than `JournalError`; tests using raw errors should assert failure category carefully.
- `WorkflowDigest::from_bytes` cannot prove cryptographic origin; it only enforces 32-byte shape.
