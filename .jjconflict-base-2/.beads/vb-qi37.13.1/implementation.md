# Implementation: vb-qi37.13.1 - vb_ui_model::envelope module

## Status

IMPLEMENTED.

## Files Changed

- `crates/vb_ui_model/src/envelope.rs` (new)
- `crates/vb_ui_model/src/lib.rs` (modified)
- `crates/vb_ui_model/Cargo.toml` (modified)

## Summary

Created `vb_ui_model::envelope` module with all required types:

- `CURRENT_SCHEMA_VERSION: SchemaVersion` — constant with validated u16 (1..=65535)
- `EnvelopeKind` enum — Success, Error, Diagnostic, Status, Event, Workflow
- `SchemaVersion::new(u16)` — validates range 1..=65535, returns `Result<Self, EnvelopeError>`
- `MetadataEnvelope::new(RunId, String, i64)` — run_id, command, timestamp
- `DiagnosticEnvelope::new(String, String, Option<String>)` — code, message, detail
- `PayloadEnvelope::from_json(Value)` / `as_json()` — JSON payload wrapper
- `OutputEnvelope::new(...)` — with invariants enforced:
  - Error must have diagnostic
  - Success cannot have diagnostic
  - Cannot have both diagnostic and payload

## Invariants Enforced

| Condition | Error |
|---|---|
| Schema version outside 1..=65535 | `EnvelopeError::InvalidSchemaVersion` |
| Success with diagnostic | `EnvelopeError::SuccessCannotHaveDiagnostic` |
| Error without diagnostic | `EnvelopeError::ErrorMustHaveDiagnostic` |
| Both diagnostic and payload | `EnvelopeError::DiagnosticAndPayloadMutuallyExclusive` |

## Constraints Satisfied

- Zero `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg`
- No unchecked indexing, slicing, casts, or arithmetic
- Fallible constructors return `Result<T, EnvelopeError>`
- `EnvelopeError` provides `Display` for error messages

## Test Coverage

15 unit tests covering:
- `SchemaVersion::new` validation (valid min, valid max, zero, above max)
- `EnvelopeKind::name()` for all 6 variants
- `MetadataEnvelope::new` construction
- `DiagnosticEnvelope::new` with and without detail
- `PayloadEnvelope::from_json` / `as_json` roundtrip
- `OutputEnvelope::new` invariants (success+diag, error-no-diag, both-diag-payload)
- `EnvelopeError` Display formatting

## Build Verification

To verify: `cargo build -p vb_ui_model`