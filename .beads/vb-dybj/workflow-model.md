# Workflow Model - vb-dybj State 3

## Workflow: Establish Postcard Newtype Compatibility Contract

This is a test/design workflow, not a runtime workflow.

### States

| State | Meaning | Legal next states |
|---|---|---|
| `ScopeRead` | Bead, master contract, and explored code map have been read. | `SurfaceSelected` |
| `SurfaceSelected` | The test chooses raw Postcard, storage envelope, or both for each assertion. | `FixtureFrozen`, `RejectedAmbiguousSurface` |
| `FixtureFrozen` | Expected bytes are explicit constants tied to type/value/surface. | `EncodedCompared` |
| `EncodedCompared` | Current encoding is compared to frozen bytes. | `DecodeChecked`, `MigrationRequired` |
| `DecodeChecked` | Frozen bytes decode to the original typed value. | `MalformedChecked` |
| `MalformedChecked` | Trailing and missing input paths return the intended typed error for the selected surface. | `Accepted` |
| `MigrationRequired` | Bytes changed from fixture; change cannot be accepted without named migration. | terminal |
| `RejectedAmbiguousSurface` | Test cannot tell raw Postcard representation from envelope persisted ID. | terminal |
| `Accepted` | Compatibility behavior is fully specified by fixtures and typed error checks. | terminal |

### Commands

- `SelectRawPostcardFixture { type_name, value_name }`
- `SelectStorageEnvelopeFixture { record_kind, payload_type }`
- `FreezeGoldenBytes { fixture_name, bytes }`
- `CompareCurrentEncoding { fixture_name }`
- `DecodeGoldenBytes { fixture_name }`
- `DecodeMalformedInput { malformed_case, expected_error_surface }`
- `DeclareMigrationRequired { old_fixture, new_fixture, migration_name }`

### Guards

- `fixture_name` must include the type and selected value.
- `expected_error_surface` must be `postcard` or `JournalError`, never a stringly text message.
- `RecordKind` fixture must name whether it asserts `postcard(enum)` or `envelope_id(u16_le)`.
- `RunId` zero and max cases must be considered legal values.
- No command may introduce JSON wrappers, Protobuf/Bilrost, HTTP, YAML runtime parsing, or external Restate code.

### Outcomes

- `CompatibilityHeld`: current Postcard bytes match the fixture and decode to the value.
- `TypedDecodeRejected`: malformed bytes were rejected by the expected typed surface.
- `MigrationNeeded`: current bytes differ from the fixture.
- `ContractAmbiguous`: surface selection is unclear; downstream work must clarify before writing assertions.

## Workflow: Decode Malformed Bytes

### Raw Postcard branch

1. Input bytes enter `postcard::from_bytes::<T>`.
2. If bytes encode one complete value and no extra bytes remain, decode succeeds.
3. If trailing bytes remain, raw Postcard returns `postcard::Error`.
4. If required bytes are missing, raw Postcard returns `postcard::Error`.

### Storage envelope branch

1. Input bytes enter `vb_storage::decode_record::<T>` with expected family/kind/limits.
2. Header length, magic, schema, kind, payload length, CRC, and digest are validated before payload decode.
3. If header or declared payload bytes are missing, return `JournalError::UnexpectedEof`.
4. If envelope is valid but typed payload bytes cannot decode as `T`, return `JournalError::PostcardDecodeFailed`.

## Terminal Error States

- `UnexpectedEof`: storage record too short for required header/payload bytes.
- `PostcardDecodeFailed`: storage envelope valid enough to reach payload Postcard decode but typed payload decode fails.
- `RawPostcardDecodeFailed`: direct Postcard decode failure; not a VB typed storage error.
- `MigrationRequired`: golden bytes changed without accepted migration.
