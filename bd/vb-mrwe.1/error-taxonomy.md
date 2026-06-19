# Error Taxonomy — Storage Envelope & Digest Verification Family

**Beads**: `vb-mrwe.1`, `vb-mrwe.2`, `vb-mrwe.3`, `vb-mrwe.5`

This taxonomy enumerates the EXACT variant names currently in the codebase. The user prompt's brief suggested variant names (`JournalError::TrailingBytes { found, expected }`, `JournalError::DigestMismatch { kind: WorkflowSource | CompiledIr }`) do NOT match the code; the variants below are the source of truth.

## A. `JournalError` — codec / journal layer

Defined in `crates/vb_storage/src/error/mod.rs`. Selected variants relevant to this bead family:

| Variant | File | Wire-error-code | Used by |
|---|---|---|---|
| `BadMagic { found: u32 }` | error/mod.rs:95 | `JOURNAL_BAD_MAGIC_CODE` | `codec/header.rs::decode_record_header` |
| `UnsupportedSchemaVersion { found, max }` | error/mod.rs | `JOURNAL_UNSUPPORTED_SCHEMA_CODE` | `codec/validation.rs::validate_schema_version` |
| `UnknownRecordKind { found: u16 }` | error/mod.rs | `JOURNAL_UNKNOWN_RECORD_KIND_CODE` | `codec/validation.rs::validate_known_kind` |
| `KindFamilyMismatch { magic: u32, kind: u16 }` | error/mod.rs | `JOURNAL_KIND_FAMILY_MISMATCH_CODE` | `codec/validation.rs::validate_kind_family` |
| `HeaderLengthMismatch { found: u32 }` | error/mod.rs | `JOURNAL_HEADER_LENGTH_MISMATCH_CODE` | `codec/header.rs::decode_record_header` |
| `PayloadTooLarge { len: u32, max: u32 }` | error/mod.rs:120 | `JOURNAL_PAYLOAD_TOO_LARGE_CODE` | `codec/header.rs`, `codec/payload.rs` |
| `HeaderChecksumMismatch` | error/mod.rs:128 | `JOURNAL_HEADER_CHECKSUM_MISMATCH_CODE` | `codec/header.rs::decode_record_header` |
| `PayloadDigestMismatch` | error/mod.rs:131 | `PAYLOAD_DIGEST_MISMATCH_CODE` | `codec/payload.rs::verify_digest_match`, `journal/admission.rs::verify_content_digest` |
| `UnexpectedEof` | error/mod.rs:134 | `JOURNAL_UNEXPECTED_EOF_CODE` | `codec/header.rs`, `codec/payload.rs` |
| `UnexpectedTrailingBytes { declared_end: usize, actual_len: usize }` | error/mod.rs:137 | `JOURNAL_UNEXPECTED_TRAILING_BYTES_CODE` | `codec/payload.rs::reject_trailing_bytes`, `codec/envelope.rs::decode_envelope_only` |
| `PostcardDecodeFailed` | error/mod.rs:145 | `JOURNAL_POSTCARD_DECODE_FAILED_CODE` | `codec/record.rs::decode_record` |
| `InvalidEvent` | error/mod.rs:148 | `JOURNAL_INVALID_EVENT_CODE` | `codec/semantic.rs::validate_journal_event_semantics` |
| `ArtifactMalformed` | error/mod.rs:151 | `JOURNAL_ARTIFACT_MALFORMED_CODE` | `admission::validate_compiled_ir_record` |
| `ArtifactChecksumMismatch` | error/mod.rs:154 | `JOURNAL_ARTIFACT_CHECKSUM_MISMATCH_CODE` | admission layer |
| `MetadataMutation { digest: WorkflowDigest }` | error/mod.rs:159 | `JOURNAL_METADATA_MUTATION_CODE` | `journal/source.rs::validate_metadata_hash_is_consistent` |

### A.1 — Naming drift vs the user prompt

| User prompt suggests | Codebase has | Decision |
|---|---|---|
| `JournalError::TrailingBytes { found, expected }` | `JournalError::UnexpectedTrailingBytes { declared_end, actual_len }` | Use existing. The codebase name preserves the precise semantics (exclusive declared end vs total actual length) and is already wired into wire-error codes. |
| `JournalError::DigestMismatch { kind: WorkflowSource \| CompiledIr }` | `JournalError::PayloadDigestMismatch` (envelope + admission) AND `RecoveryError::WorkflowSourceDigestMismatch` / `CompiledIrDigestMismatch` (recovery boundary) | Use existing. The two layers (envelope/put vs recovery-boundary) intentionally use distinct error enums because they sit at different crate boundaries. Adding a single combined `DigestMismatch { kind }` would either collapse the two error envelopes or duplicate information. |

If a future bead requires a single combined variant, that is its own contract work and must rebind both layers.

## B. `RecoveryError` — recovery boundary

Defined in `crates/vb_storage/src/recovery/types/error.rs`. Selected variants:

| Variant | File | Used by |
|---|---|---|
| `Journal(#[from] JournalError)` | recovery/types/error.rs:13 | recovery orchestrator (wraps `JournalError`) |
| `WorkflowSourceDigestMismatch { expected: WorkflowDigest, found: WorkflowDigest }` | recovery/types/error.rs:16 | `recovery/recover.rs::check_workflow_source_digest` |
| `CompiledIrDigestMismatch { expected, found }` | recovery/types/error.rs:24 | `recovery/recover.rs::check_compiled_ir_digest` |
| `ActionAbiMismatch { action_id: ActionId, expected, found }` | recovery/types/error.rs:34 | `recovery/recover.rs::check_action_abi_digests` (Full only) |
| `PolicyDigestMismatch { step: StepIdx, expected, found }` | recovery/types/error.rs:43 | `recovery/recover.rs::check_policy_digests` (Full only) |
| `PolicyDigestUnavailable { run, step, expected }` | recovery/types/error.rs:53 | replay/admission.rs |
| `PolicyDigestExpectationMissing { run }` | recovery/types/error.rs:63 | replay/admission.rs |
| `FullDigestCheckConfigMissing` | recovery/types/error.rs:70 | `recovery/recover.rs::check_full_level` |
| `RunAdmissionArtifactDigestMismatch { run, expected, found }` | recovery/types/error.rs:75 | replay/admission.rs |
| `NonIdempotentActionBlocked { action, step }` | recovery/types/error.rs:87 | replay path |

### B.1 — `FullDigestCheckConfigMissing` semantics

This is the **fail-closed** sentinel for the `DigestCheck::Full` workflow. It is returned when:

1. `level == DigestCheck::Full` AND `config` is `None`.
2. `level == Digest::Full` AND `config` is `Some(_)` but `config.action_abi_entries` is `None`.
3. Same for `config.policy_entries`.

It is **not** returned for empty slices (no entries to verify) — empty slice is a valid "nothing to check" input. It is **not** returned for `level < Full` — those levels do not require config at all.

## C. Cross-enum invariants

1. **No silent fallthrough**: every decode/put/recover path returns a typed `Result`. No panic paths.
2. **No stringly-typed errors**: all variants carry typed fields (digest bytes, action IDs, step indices, declared/actual offsets). `thiserror` derives `Display` and `Error` for human-readable output but the typed fields are the contract.
3. **No `unwrap()`/`expect()` on `Result`** in any of the file paths covered by this contract.
4. **Wire-error codes are stable**: each `JournalError` variant maps to a code via `error/codes.rs::wire_error_code` (the codes module is the single point of truth for externally-visible error codes).
5. **Variant field types are byte-exact**: digests are `[u8; 32]` (or `WorkflowDigest` which is a newtype around it); offsets are `usize`; record-kind IDs are `u16`; magic words are `u32`. No narrowing casts without an explicit `try_from`.
