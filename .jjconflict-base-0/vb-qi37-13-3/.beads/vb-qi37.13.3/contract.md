# Contract Specification: vb-qi37.13.3 — Emitter Fixes

## Context
- **Bead:** vb-qi37.13.3
- **Feature:** Fix emitter defects: dead code removal, u64 overflow fix, decode_yaml decision, postcard migration
- **Domain terms:**
  - `EmitterError::AnsiForbidden` - ANSI escape sequence detected
  - `EmitterError::MigrationRequired { from, to }` - schema version migration needed
  - `json_value_to_yaml` - converts JSON Values to Yaml nodes
  - `validate_no_ansi` - ANSI sequence validator (dead code candidate)
  - `decode_postcard` - postcard binary decoder with version checking
- **Assumptions:**
  - CLI output emitters are write-only (encode to YAML/text/postcard); no read-back required for CLI consumption
  - Postcard migration path is for forward-compatibility when schema version increments
  - u64→i64 overflow in YAML affects large numeric IDs (> i64::MAX)
- **Open questions:**
  - Is decode_yaml needed for any CLI input path (e.g., config files, stdin)?
  - What is the expected migration behavior for schema version downgrades?

## Preconditions
- PRE-FIX-001: `encode_yaml` input types must implement `serde::Serialize`
- PRE-FIX-002: `decode_postcard` requires at least CLI_HEADER_BYTES (52) input bytes
- PRE-FIX-003: `max_payload_len` parameter bounds allocation for postcard decode

## Postconditions
- POST-FIX-001: `validate_no_ansi` function is removed from `yaml.rs` (dead code elimination)
- POST-FIX-002: `json_value_to_yaml` returns `EmitterError::YamlEncodeFailed` for u64 values exceeding i64::MAX (no silent truncation)
- POST-FIX-003: Postcard migration path either implements actual version migration or returns a documented error with migration semantics
- POST-FIX-004: Any `decode_yaml` function (if added) returns `Result<T, EmitterError>` with exhaustive error variants

## Invariants
- INV-FIX-001: Large u64 values (> i64::MAX) produce an error in YAML encoding, not silent truncation
- INV-FIX-002: Postcard decode returns `MigrationRequired` only for downgrades (from < to), never for same-version or upgrades
- INV-FIX-003: Postcard version checking follows semver-compatible semantics (major version changes require migration)

## Error Taxonomy
- `EmitterError::YamlEncodeFailed` - YAML encoding failed (including overflow rejection)
- `EmitterError::PostcardDecodeFailed` - Postcard deserialization failed
- `EmitterError::MigrationRequired { from, to }` - schema version requires migration
- `EmitterError::UnsupportedSchemaVersion { version }` - version too new to migrate
- `EmitterError::AnsiForbidden` - ANSI escape sequences detected (used by removed validate_no_ansi)

## Contract Signatures
- `fn encode_yaml<T: Serialize>(payload: &T) -> Result<String, EmitterError>`
- `fn decode_yaml<T: Deserialize>(yaml: &str) -> Result<T, EmitterError>` (if needed)
- `fn decode_postcard<'a, T: Deserialize<'a> + core::fmt::Debug>(bytes: &'a [u8], expected_kind: EnvelopeKind, max_payload_len: u32) -> Result<T, EmitterError>`
- Removed: `fn validate_no_ansi(text: &str) -> Result<(), EmitterError>`

## Verus-Owned Clauses
- INV-FIX-001: u64→i64 overflow behavior in `json_value_to_yaml` is proven to error for values > i64::MAX

## TLA+-Owned Clauses
- None: These are pure Rust codec fixes with no temporal/state-over-time behavior

## Theorem-Owned Clauses
- None: Verus covers all Rust-local proof obligations

## Non-goals
- Adding new emit modes beyond text/yaml/postcard
- Modifying cli_postcard.rs (VCLA magic) - separate from this scope
- TLA+ modeling for deterministic codec behavior
