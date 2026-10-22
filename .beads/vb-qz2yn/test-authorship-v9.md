# Artifact Counter V8 Test — Actual API & Behavior

This documents the actual 7-case test suite against the real vb_storage API.
All API signatures and error structures are derived from the compiled source,
not from design documents.

## Actual AcceptedArtifact Fields

```rust
pub struct AcceptedArtifact {
    pub digest: WorkflowDigest,
    pub source_digest: WorkflowDigest,
    pub policy_digest: WorkflowDigest,
    pub ir: Vec<u8>,
    pub verification: VerificationProof,
    pub accepted_at_seq: EventSeq,
    pub required_capabilities: Box<[Capability]>,
    pub action_contracts: Box<[ActionContract]>,
    pub executor_profile: ExecutorProfile,
}
```

No `artifact_digest` field. No `sequence` field. Use `digest` and
`accepted_at_seq.get()`.

## Actual API Signatures

```rust
pub fn prepare_compiled_artifact(
    workflow: &CompiledWorkflow,
    policy: RuntimePolicy,
    contracts: &[ActionContract],
) -> Result<AcceptedArtifact, JournalError>

pub fn verify_accepted_artifact_for_policy(
    artifact: &AcceptedArtifact,
    policy: RuntimePolicy,
) -> Result<CompiledWorkflow, JournalError>

pub fn submit_artifact(
    journal: &FjallJournal,
    workflow: &CompiledWorkflow,
    policy: RuntimePolicy,
) -> Result<AcceptedArtifact, JournalError>
```

## Actual MalformedKeyspaceRow Error

```rust
MalformedKeyspaceRow {
    prefix: u8,
    expected_len: usize,
    actual_len: usize,
}
```

No `keyspace` or `key` fields. Only prefix/lengths.

## Actual Test Cases

1. first_submission_is_seq_1_for_all_policies — seq 1 for Relaxed/Journaled/Strict
2. repeated_submissions_get_distinct_digests_and_both_readback — seq 1,2 with distinct digests
3. pure_prepare_stays_seq_0_submit_after_gets_seq_1 — prepare=0, submit=1, counter absent
4. counter_persists_across_reopen — seq 1 close/reopen seq 2
5. corrupted_counter_length_7_causes_malformed_row — prefix=0 expected=8 actual=7
6. corrupted_counter_length_9_causes_malformed_row — prefix=0 expected=8 actual=9
7. counter_at_u64_max_causes_sequence_overflow — unchanged counter, no compiled_ir

## Readback Verification Pattern

The actual readback verification must:
- Read by `artifact.digest` (not artifact_digest)
- Assert `digest_artifact(&artifact) == artifact.digest`
- Assert `artifact.verification.digest == artifact.digest`
- Call `verify_accepted_artifact_for_policy(artifact, policy)` — passes artifact directly, not the record
- Decode record.ir via postcard and compare full `AcceptedArtifact` struct with PartialEq/Eq
- Convert JournalError to String via map_err, not `?` in a String-returning function