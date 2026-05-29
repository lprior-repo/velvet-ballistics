# Contract - vb-dybj State 3

## Purpose

Define the domain and type contract for adding fixed-wire Postcard compatibility tests for selected VB newtypes and persisted record identifiers.

## Scope

In scope for later implementation/test states:

- `RunId` golden Postcard fixtures for zero, representative, and maximum values.
- `WorkflowDigest` golden Postcard fixture for a selected fixed 32-byte digest.
- `RecordKind` compatibility fixture with explicit surface naming:
  - `postcard_enum`, and/or
  - `envelope_id_u16_le` from `RecordKind::id()`.
- Typed malformed decode assertions for trailing bytes and missing bytes on the explicitly selected decode surface.
- Workspace test target registration for the bead-specified test file.

Out of scope:

- Production behavior changes unless downstream failing tests expose a missing typed public API.
- Verifier artifacts, proof plans, reviews, or implementation in State 3.
- Runtime JSON/YAML/HTTP paths.
- Restate code/API/wire-format copying.

## Functional Contract

1. `RunId::new(v).get() == v` for all selected `v`, including `0` and `u64::MAX`.
2. `RunId::ZERO == RunId::new(0)`.
3. Current Postcard bytes for selected `RunId` values must equal frozen fixtures.
4. Decoding frozen `RunId` fixtures through the selected surface must yield the original `RunId`.
5. `WorkflowDigest::from_bytes(bytes).as_bytes() == bytes` for exactly `[u8; 32]`.
6. Current Postcard bytes for selected `WorkflowDigest` bytes must equal frozen fixtures.
7. Selected `RecordKind::id()` values must match the master storage IDs when asserting envelope IDs.
8. Current Postcard enum bytes for selected `RecordKind` values must equal frozen fixtures when asserting Postcard enum compatibility.
9. Trailing data must be rejected by exact-value decode on the selected surface.
10. Missing bytes must be rejected by the selected surface; storage envelope short input must return `JournalError::UnexpectedEof`.
11. Storage payload decode failure must return `JournalError::PostcardDecodeFailed` only after envelope validation reaches Postcard payload decode.
12. Golden byte changes require a named migration, not silent fixture edits.

## Non-Functional Contract

- No new runtime dependency is required.
- No Bilrost or Protobuf is introduced.
- No JSON wrapper is used for core/storage compatibility.
- Tests belong under `crates/workspace_tests`, not repository root.
- Tests may use test-only helpers but must not weaken first-party production no-unsafe/no-panic rules.
- Any speed/performance claims are out of scope.

## Acceptance Mapping

| Bead acceptance | Contract response |
|---|---|
| RunId Postcard bytes match golden fixture | Freeze selected `RunId` fixture bytes and assert exact equality. |
| RecordKind Postcard bytes match golden fixture | Assert `postcard_enum` bytes, or explicitly rename if asserting `envelope_id_u16_le`. |
| Invalid input trailing bytes return typed decode error | Use selected raw or storage surface; VB typed error requires storage/adapter surface. |
| Missing bytes return typed short decode error | Storage surface must return `JournalError::UnexpectedEof`. |
| Zero value newtype behavior | Zero is legal and equals `RunId::ZERO`. |
| Maximum value newtype behavior | `u64::MAX` is legal for `RunId`; no arithmetic needed. |
| Changing golden bytes requires named migration | Fixture tests must document/name migration requirement. |
| Postcard path contains no JSON wrapper | Compatibility path uses `postcard` directly or storage codec, not JSON. |

## Open Domain Questions

1. Should later tests assert both `RecordKind` Postcard enum bytes and envelope `RecordKind::id()` little-endian bytes, or only the bead-literal Postcard enum bytes?
2. Should typed trailing-byte acceptance be satisfied at raw Postcard level with dependency error type, or at storage level with `JournalError::PostcardDecodeFailed` using a deliberately valid envelope containing trailing payload bytes?

These questions do not block State 3, but downstream test writing must answer them explicitly in test names.
