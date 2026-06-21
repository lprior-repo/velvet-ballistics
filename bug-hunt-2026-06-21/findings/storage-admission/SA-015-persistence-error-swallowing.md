# SA-015: `verify_persisted_artifact_present` and `serialize_accepted_artifact` swallow source errors into `ArtifactMalformed`

- **Severity**: Low
- **Category**: correctness
- **Location**: `crates/vb_storage/src/admission/persistence.rs:27-46`
- **Confidence**: confirmed

## Description

Both functions map any error from the underlying Fjall / postcard call to `JournalError::ArtifactMalformed`, discarding the actual cause. The caller cannot distinguish "the artifact bytes are corrupted" from "the journal's LSM tree threw an IO error" or "postcard serialization failed for an internal reason".

## Evidence

```rust
// crates/vb_storage/src/admission/persistence.rs:27-46
pub(crate) fn serialize_accepted_artifact(
    artifact: &AcceptedArtifact,
) -> Result<Vec<u8>, JournalError> {
    postcard::to_allocvec(artifact).map_err(|_| JournalError::ArtifactMalformed)  // <-- discard
}

pub(crate) fn verify_persisted_artifact_present(
    journal: &FjallJournal,
    digest: vb_core::WorkflowDigest,
) -> Result<(), JournalError> {
    let stored = journal
        .compiled_ir(digest)
        .map_err(|_| JournalError::ArtifactMalformed)?;                            // <-- discard
    if stored.is_some() { Ok(()) } else { Err(JournalError::ArtifactMalformed) }
}
```

`JournalError` has rich variants (`Fjall`, `PostcardDecodeFailed`, `ArtifactNotFound`, etc.) that would carry the real cause. The current mapping loses them.

## Adversarial Check

For `serialize_accepted_artifact`, `postcard::to_allocvec` returns `postcard::Error`, which is a single enum that today has limited variants but could in principle carry meaningful reasons (size limit, allocation failure, etc.). For `verify_persisted_artifact_present`, `journal.compiled_ir(digest)` returns `Result<Option<CompiledIrRecord>, JournalError>` — the `JournalError` is already typed (could be `Fjall(_)`, `HeaderChecksumMismatch`, `PayloadDigestMismatch`, etc.) and is already a `JournalError`, so the `map_err(|_| JournalError::ArtifactMalformed)` actually *downgrades* from a more specific variant to a less specific one. A reader of the resulting `ArtifactMalformed` cannot tell whether the journal was unreadable or the artifact was simply absent — both produce the same error.

## Suggested Fix

For `verify_persisted_artifact_present`, propagate the underlying error and use `ArtifactNotFound` for the absent case:

```rust
pub(crate) fn verify_persisted_artifact_present(
    journal: &FjallJournal,
    digest: vb_core::WorkflowDigest,
) -> Result<(), JournalError> {
    match journal.compiled_ir(digest)? {
        Some(_) => Ok(()),
        None => Err(JournalError::ArtifactNotFound { digest }),
    }
}
```

For `serialize_accepted_artifact`, either preserve the postcard error via `#[from]` if `JournalError` supports it, or introduce `JournalError::PostcardEncodeFailed`.
