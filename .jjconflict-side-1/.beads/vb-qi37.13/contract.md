bead_id: vb-qi37.13
bead_title: cli: Reconcile structured output contract
phase: 3
updated_at: 2026-05-14T22:16:30Z
attempt: 1-of-7

# Contract Specification

## Context

The CLI structured-output parent bead reconciles closed child work into a stable operator contract. The active defect is that public exit-code and proof artifacts still admit code `9`, while the required public range is exactly `0..=8`. Postcard output/proof coverage must be integrated with executable repository routes, not only a standalone model.

## Preconditions

- PRE-001: CLI command parsing must select a supported command and output/emit mode or return a typed validation diagnostic.
- PRE-002: Postcard decode/encode callers must pass byte slices bounded by the CLI postcard protocol maximum before allocating payload buffers.
- PRE-003: Public CLI errors must be classified into the documented taxonomy before conversion to `std::process::ExitCode`.

## Postconditions

- POST-001: Every public CLI process exit status is in `0..=8`.
- POST-002: No public CLI path emits `DomainError = 9` or any other code outside `0..=8`.
- POST-003: Structured operator output includes stable `schema_version` and `kind` fields for supported structured envelopes.
- POST-004: Diagnostics are machine-readable and include stable code/message plus path/span/repair where the command has that context.
- POST-005: Postcard output/decode evidence is integrated into repository proof/test/fuzz or has an approved explicit waiver.
- POST-006: State 3 primary obligations and traceability matrix reconcile State 4/5/6 child evidence commands without placeholder commands or invented proof targets.

## Invariants

- INV-001: `CliExitCode` discriminants are bounded by `0..=8`.
- INV-002: Exit-code conversion is total for all public CLI error classes and never panics.
- INV-003: Postcard header validation checks magic, header length, and payload bound before exposing payload bytes.
- INV-004: Unsupported output or emit modes fail closed with validation diagnostics and exit code `1`.
- INV-005: Structured output schema version is monotonic and currently at least `1`.
- INV-006: Every required proof obligation command is either executable as written from `/home/lewis/src/vb-qi37-13-r2` or is an explicit waiver-review command with blocker details; contract-time rows never claim `PASS`.

## Error Taxonomy

- ERR-001 `ValidationFailed = 1`: parse errors, unsupported command/mode, invalid user input.
- ERR-002 `VerificationFailed = 2`: workflow/setup verification failure.
- ERR-003 `CompileFailed = 3`: compile/codegen/serialization failure.
- ERR-004 `RuntimeFailed = 4`: runtime execution/evaluation failure.
- ERR-005 `StorageError = 5`: storage/journal/persistence failure.
- ERR-006 `IpcError = 6`: IPC server/client failure.
- ERR-007 `ActionPolicyError = 7`: action policy or UI/action initialization failure.
- ERR-008 `ReplayDivergence = 8`: replay divergence.
- ERR-009: Any domain-specific business rule violation must map to one of `1..=8`; it must not expose public code `9`.
- ERR-010 `PostcardBadMagic`: postcard decode rejects an invalid magic value before payload exposure.
- ERR-011 `PostcardBadCrc`: postcard decode rejects a CRC mismatch before accepting payload content.
- ERR-012 `PostcardBadPayloadDigest`: postcard decode rejects payload digest mismatch before accepting payload content.
- ERR-013 `PostcardUnsupportedVersion`: postcard decode rejects unsupported future versions.
- ERR-014 `PostcardOldVersion`: postcard decode rejects obsolete versions.
- ERR-015 `PostcardPayloadTooLarge`: postcard decode rejects payloads beyond the protocol bound before allocating or exposing payload bytes.
- ERR-016 `PostcardWrongKind`: postcard decode rejects an envelope kind that does not match the expected structured-output kind.

## Contract Signatures

- `impl From<CliExitCode> for std::process::ExitCode` returns an exit code in `0..=8`.
- `impl From<vb_core::errors::CoreError> for CliExitCode` maps core errors to a public code in `1..=8`.
- `impl From<vb_storage::error::JournalError> for CliExitCode` maps storage errors to `StorageError = 5`.
- `cli_postcard::decode_postcard(data: &[u8]) -> Result<(&[u8], &[u8]), PostcardError>` validates header and bounds before payload exposure.

## Verus-Owned Clauses

- VERUS-EXIT-001 proves `spec_exit_code_discriminant(code) <= 8` for every public `SpecCliExitCode`.
- VERUS-SCHEMA-001 proves schema version validity for diagnostic envelopes.

## TLA+-Owned Clauses

- None. This bead is local CLI mapping/codec behavior, not a temporal lifecycle/protocol change. Runtime lifecycle state transitions are outside this parent reconciliation bead.

## Theorem-Owned Clauses

- None. Verus plus executable Rust tests/fuzz/Kani/proptest own this scoped Rust-local contract.

## Non-goals

- Adding new operator commands.
- Changing dependency versions or introducing new runtime JSON/HTTP dependencies.
- Full workspace release evidence refresh beyond scoped obligations.
