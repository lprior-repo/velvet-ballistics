# vb-qz2yn Code Review v20 — Independent Local 3090 Reviewer

## Verdict: APPROVE (narrow CODE only)

Tests and Moon CI gates remain pending; this review covers only the 6 changed production files against the contract and actual source.

## Reviewed Paths and Hash Applicability

All 12 files in the actual-source-packet-v20 manifest were read via :raw (14 parts):

- crates/vb_storage/src/admission.rs (sha256: 8addf153..., 481 lines)
- crates/vb_storage/src/constants.rs (sha256: e1407ede..., 121 lines)
- crates/vb_storage/src/journal/artifact_acceptance.rs (sha256: 204ec201..., 198 lines)
- crates/vb_storage/src/journal/core.rs (sha256: 0ccf8917..., 306 lines)
- crates/vb_storage/src/journal/mod.rs (sha256: e7d6fd0e..., 34 lines)
- crates/vb_storage/src/journal/readonly.rs (sha256: a135a716..., 101 lines)
- crates/vb_storage/src/journal/admission.rs (sha256: aadd843b..., 36 lines)
- crates/vb_storage/src/batch/commit.rs (sha256: 6c33deec..., 41 lines)
- crates/vb_storage/src/admission/verification.rs (sha256: cd1be552..., 272 lines)
- crates/vb_storage/src/error/mod.rs (sha256: cf5e9254..., 199 lines)
- /cache/cargo-shared/registry/src/index.crates.io-1949cf8c6b5b557f/fjall-3.1.4/src/batch/mod.rs (sha256: 3345faf8..., 183 lines)
- /cache/cargo-shared/registry/src/index.crates.io-1949cf8c6b5b557f/fjall-3.1.4/src/journal/writer.rs (sha256: 8a000fcf..., 380 lines)

## Contract Compliance (6 Production Delta Contracts)

### 1. Single private batch + lock
✅ PASS. `issue_artifact` (artifact_acceptance.rs:171-188) acquires `write_lock`, reads counter, increments, stamps, rebinds digest, encodes, stages both writes in one `OwnedWriteBatch`, commits, then drops lock. No public API extension.

### 2. StrictSyncAll policy parity
✅ PASS. Lines 151-157 apply `durability(Some(fjall::PersistMode::SyncAll))` only for `RuntimePolicy::Strict`. Verified against actual Fjall 3.1.4 batch commit code (fjall-3.1.4/src/batch/mod.rs:119-129) which calls `journal_writer.persist(mode)` when durability is set.

### 3. Error variants and envelope
✅ PASS. `CompiledIrReadback(#[source] Box<JournalError>)` variant exists at error/mod.rs:118-119 and is used correctly at admission.rs:387. `ArtifactMalformed` returned on readback None (admission.rs:389). `SequenceOverflow` on counter overflow (artifact_acceptance.rs:100). `MalformedKeyspaceRow` on counter decode length mismatch (artifact_acceptance.rs:73-77).

### 4. Digest binding
✅ PASS. `bind_artifact_digest` called after sequence stamp (artifact_acceptance.rs:104). Sets both `artifact.digest` and `artifact.verification.digest` (admission.rs:180-183).

### 5. Counter corruption / overflow protection
✅ PASS. Counter value must be exactly 8 bytes (artifact_acceptance.rs:72-86), uses safe `try_into()` for fixed array conversion. Increment uses `checked_add(1)` (artifact_acceptance.rs:100). No unchecked indexing or arithmetic.

### 6. 12 keyspaces / probe / readback
✅ PASS. `declared_keyspaces()` returns 12-element array (core.rs:167-182). `probe_health()` includes `artifact_acceptance` (core.rs:238). `open()` creates the keyspace (core.rs:136-138).

## Additional Engineering Checks

- No `unsafe`, `unwrap`, `expect`, `panic`, `todo`, `unimplemented`, `dbg` in changed files
- `IssuedArtifactSequence` wraps `NonZeroU64`, private, converts to `EventSeq` (artifact_acceptance.rs:32-50)
- First issued sequence is 1 (checked_add from 0)
- `prepare_compiled_artifact` stays seq=0 (verification.rs:45) — pure prepare unchanged
- Test-only fault hooks properly `#[cfg(test)]` gated (artifact_acceptance.rs:152-155, 158-164)
- Existing `persist_strict` fault hook remains active (core.rs:65-68, 256-264)
- Existing compiled IR readback hook remains active (core.rs:67-68, 266-280)
- Batch commit hook honors new batch path (batch/commit.rs:31-37)

## Findings

No blocking findings. One minor style observation:

- `artifact_acceptance.rs` does not have its own `#![forbid(unsafe_code)]` at module head; inherits from `journal/mod.rs`. Not a bug, but inconsistent with top-level files. Disposition: informational, no action required.

## Withdrawal of v17 Wrong Claims

This v20 review explicitly withdraws the following incorrect claims from the v17 review:

1. **Fjall version**: v17 claimed Fjall 3.1.10; actual source packet contains Fjall 3.1.4. The implementation correctly uses the Fjall 3.1.4 API.
2. **CompiledIrReadback variant**: v17 claimed this variant was invented; it exists at error/mod.rs:118-119 and is used correctly.
3. **Module attribute**: v17 claimed the new module has its own `#![forbid(unsafe_code)]`; it inherits from the parent module. Not a bug.
4. **Line count**: v17's claim of 198 lines for artifact_acceptance.rs is correct per manifest.

## Scope

This review is CODE only. No test-suite approval. No formal verification assessment. No performance claims. Public default API only (`submit_artifact`, `submit_artifact_with_contracts`). CLI and packed paths remain out of scope (parent bead vb-0qwfq).
