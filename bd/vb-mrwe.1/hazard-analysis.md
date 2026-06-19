# Hazard Analysis — Storage Envelope & Digest Verification Family

**Beads**: `vb-mrwe.1`, `vb-mrwe.2`, `vb-mrwe.3`, `vb-mrwe.5`

## H-1 — Truncated records (parser hazard, vb-mrwe.1)

**Risk**: A byte slice shorter than `RECORD_HEADER_BYTES + header.payload_len` could be silently decoded with garbage if the parser doesn't check.
**Current defense**: `decode_record_header` rejects slices shorter than `RECORD_HEADER_BYTES` with `UnexpectedEof`. `decode_record_payload` re-checks via `bytes.get(payload_start..payload_end).ok_or(UnexpectedEof)`. Offsets are computed with `usize::try_from` and `checked_add` to prevent arithmetic overflow on adversarial 32-bit lengths.
**Proof seed**: PS-SEED-001 (trailing-byte bounds proof already written at `crates/vb_storage/src/kani_postcard_envelope_wire_trailing_bytes.rs`).

## H-2 — Trailing bytes after declared payload (parser hazard, vb-mrwe.1)

**Risk**: An attacker (or a buggy writer) appends bytes after the declared payload. A naive parser would ignore them and only verify the declared prefix — masking tampering.
**Current defense**: `decode_record_payload` calls `reject_trailing_bytes(payload_end, bytes.len())` after digest verification. `decode_envelope_only` has its own check.
**Invariant**: `payload_end == bytes.len()` is required for `Ok`.
**Proof seed**: PS-SEED-001 (already wired into Kani via `kani_postcard_envelope_wire_trailing_bytes.rs::vb_e7tl_trailing_bytes_required` and `vb_e7tl_trailing_bytes_rejected`).

## H-3 — Forged envelope digest (parser hazard, vb-mrwe.2 envelope level)

**Risk**: An attacker rewrites a record's payload while leaving the header digest unchanged. A parser that only checks the header magic and CRC32C would accept the forgery.
**Current defense**: `verify_digest_match` recomputes `blake3(payload)` and compares to the header's stored digest. Mismatch → `JournalError::PayloadDigestMismatch`. `decode_record_header` reads the digest from the header via `digest_from_header` (range-checked slice into `[u8; DIGEST_BYTES]`).
**Invariant**: `blake3(payload) == header.payload_digest` is required for `Ok`.

## H-4 — Forged workflow source / compiled IR digest (put-path hazard, vb-mrwe.2 admission)

**Risk**: A caller hands the journal a `WorkflowSourceRecord` whose `digest` does not match `blake3(source)`. The journal would store under the wrong key.
**Current defense**: `put_workflow_source` calls `verify_content_digest(&record.source, &record.digest.as_bytes())` BEFORE any key derivation or insert. Same-digest on `put_compiled_ir` is enforced via `validate_compiled_ir_record` and the metadata-hash invariant.

## H-5 — Metadata mutation attack (concurrency / multi-writer hazard)

**Risk**: A second writer with the same compiled-IR digest but different accepted-envelope metadata (different `accepted_at_seq`, different `required_capabilities`, different `verification` flags, etc.) corrupts the accepted contract.
**Current defense**: `validate_metadata_hash_is_consistent` computes `h_pending = blake3(canonical_envelope_fields)` and compares to the existing record's metadata hash (or recomputes from the stored envelope if the older record lacks a stored hash). Mismatch → `JournalError::MetadataMutation { digest }`.

## H-6 — DigestCheck level silently disabled (recovery-boundary hazard, vb-mrwe.3)

**Risk**: A `DigestCheck::Full` request is downgraded to a weaker check because config was omitted.
**Current defense**: `check_full_level` returns `Err(FullDigestCheckConfigMissing)` for `level == Full && config.is_none()` and for either of `cfg.action_abi_entries`/`cfg.policy_entries` being `None`. Empty slices are valid (caller has no entries to verify).

## H-7 — DigestCheck rank drift (invariant hazard, vb-mrwe.3)

**Risk**: Adding a new `DigestCheck` variant with the wrong rank could make `checks_*` predicates silently agree or disagree incorrectly.
**Current defense**: All `checks_*` predicates are RANK-DERIVED (`hierarchy_rank() >= SomeOtherRank`). The rank function is a single `match`; any new variant forces a rank decision at the type level.

## H-8 — StepSucceeded ↔ SlotWrittenEvent kind collision (wire-format hazard, vb-mrwe.5)

**Risk**: If two journal events accidentally map to the same wire `RecordKind` ID, replay cannot distinguish them.
**Current defense**: `RecordKind::StepSucceeded = 29` and `RecordKind::SlotWritten = 12`. The IDs are documented in `kinds.rs::id()` and verified by `kani_vb_mrwe5_record_kind.rs::vb_mrwe5_record_kind_injectivity`.
**Proof seed**: PS-SEED-005 (already wired).

## H-9 — Recovery stamp cross-family contamination (wire-format hazard)

**Risk**: Recovery stamps (magic `MAGIC_RECOVERY_STAMP`, kind 7) might be mis-decoded by the journal-event path.
**Current defense**: `validate_kind_family` rejects `(MAGIC_JOURNAL_EVENT, 7)` and `(MAGIC_RECOVERY_STAMP, 1/2/3/...)`. Each magic owns its own kind IDs exclusively. This is not a vb-mrwe.X hazard but is mentioned here for cross-family coverage.

## H-10 — Fjall crash mid-put (storage hazard)

**Risk**: The metadata-hash check passes, but a crash between the read of `existing` and the write of `record_to_store` could leave the partition in an inconsistent state.
**Current defense**: Fjall's per-key insert is atomic. The second writer either sees the first writer's record and validates against it, or sees an empty slot and accepts the first write. There is no window for a partial cross-record state at the same key.

## H-11 — Hostile input at IPC ingress (parser hazard)

**Risk**: Adversarial bytes from an IPC ingress reach the codec with arbitrary `magic`, `kind`, `payload_len`, and digest.
**Current defense**: Every check in the codec is bounds-checked with `bytes.get(..n).ok_or(...)` and arithmetic with `checked_add`/`try_from`. The trailing-bytes proof (`PS-SEED-001`) explicitly covers the `actual_len = declared_end + usize::MAX` corner case.

## H-12 — Performance / latency hazard

**Risk**: Each put/decode path calls `blake3::hash` at least once. For high-throughput paths (recovery hydration of large journals), this could dominate.
**Current defense**: BLAKE3 is the agreed content hash for the entire system; there is no faster path that preserves the security property. The performance contract (`docs/performance-contract.md`) governs accepted throughput; changes to the hashing path require a separate performance bead.

## H-13 — API/release hazard

**Risk**: Renaming a `JournalError` or `RecoveryError` variant is a breaking change for any consumer matching on the variant. Renaming a `RecordKind::id()` is a wire-format break.
**Current defense**: `#[non_exhaustive]` on the public error enums prevents downstream exhaustiveness; the kind ID table is the integration contract. Both invariants are guarded by tests.

## H-14 — Concurrency / Loom-relevant hazard

**Risk**: Concurrent writers to the same compiled-IR digest key race on the metadata-hash comparison.
**Current defense**: Fjall linearizes per-key writes. The second writer's `load_existing_compiled_ir` is sequenced with its own `insert`. The current code does NOT use internal mutexes beyond Fjall's per-key ordering; a loom model would be applicable if multi-process concurrency is introduced. For single-process Fjall, no loom model is required.

## H-15 — Unsafe / provenance hazard

**Risk**: None. `#![forbid(unsafe_code)]` at the crate root, and every offset/length computation is bounds-checked.
