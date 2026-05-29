# Error Taxonomy - vb-dybj State 3

## Domain Error Families

| Error family | Variant / label | Cause | Expected surface | Retryable? |
|---|---|---|---|---|
| Scope error | `ContractAmbiguous` | Fixture does not state raw Postcard vs storage envelope ID. | Contract/test authoring | No |
| Compatibility error | `GoldenBytesChanged` | Current encoding differs from frozen fixture. | Compatibility test | No; requires migration |
| Migration error | `MigrationRequired` | Golden bytes changed intentionally but no named migration exists. | Compatibility test/release gate | No |
| Raw decode error | `RawPostcardDecodeFailed` | Direct `postcard::from_bytes` rejects malformed/short/trailing input. | Postcard dependency | No |
| Storage short input | `JournalError::UnexpectedEof` | Record bytes end before declared/required header or payload. | `vb_storage` decode | No |
| Storage payload decode | `JournalError::PostcardDecodeFailed` | Valid-enough envelope reaches Postcard typed payload decode and fails. | `vb_storage` decode | No |
| Fixture construction error | `InvalidFixtureShape` | Digest fixture is not 32 bytes or fixture does not match type width. | Test fixture authoring | No |
| Forbidden dependency error | `ForbiddenCodecIntroduced` | Bilrost/Protobuf/JSON wrapper introduced for this compatibility path. | Review/CI | No |

## Stable Existing VB Error Variants

- `JournalError::UnexpectedEof`: use for missing bytes on the storage envelope surface.
- `JournalError::PostcardDecodeFailed`: use for invalid Postcard payload after envelope validation.

## Non-VB Errors

- `postcard::Error`: acceptable only when the assertion explicitly targets raw Postcard behavior. Do not relabel it as a VB typed storage error.

## Error Invariants

- Error assertions must match typed variants or typed diagnostic codes, not string messages.
- A short storage record must not reach payload allocation or Postcard payload decode before returning `UnexpectedEof`.
- A trailing raw Postcard byte is an invalid exact-value decode.
- A trailing byte inside a storage envelope only becomes `PostcardDecodeFailed` if the envelope length and digest are constructed so that payload decode actually sees the trailing byte.
- Golden mismatch is not an implementation failure by itself; it is a release/migration decision point.

## Railway Result Shape for Later Tests

```text
External bytes
  -> select decode surface
  -> parse/validate envelope if storage surface
  -> decode typed Postcard payload
  -> compare typed value or typed error
  -> compare fixture bytes
```

No branch may fall back to string matching, silent default values, unchecked slicing, or JSON/text wrapper decoding.
