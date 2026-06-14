P2-17r2 submit-artifact-runtime-wrapper: Add Runtime::submit_artifact thin wrapper per master §66 spec, calling existing vb_storage::admission::submit_artifact internally

# Verification excerpts (read-before-write)

## Master doc §66 (line 3421) — VERBATIM
```rust
pub fn submit_artifact(&self, run: RunId, artifact_digest: WorkflowDigest, input: &[u8], capabilities: &[Capability]) -> RuntimeResult<()>
```

## crates/vb_storage/src/admission.rs (706 lines)
- Line 230-236: `pub fn submit_artifact(journal: &FjallJournal, workflow: &vb_core::CompiledWorkflow, policy: vb_core::RuntimePolicy) -> Result<AcceptedArtifact, JournalError>` — REAL storage-level function.
- This is the internal storage-layer function, NOT the Runtime-level wrapper that master §66 requires.

## crates/vb_runtime/src/runtime/mod.rs (564 lines)
- Line 48-65: `pub fn new_with_journal(shard_count: NonZeroUsize, config: ShardConfig, journal: SharedRuntimeJournal) -> Self` — 3-arg constructor.
- Line 198-200: `pub fn snapshot_run(&self, run: RunId, correlation: u64) -> RuntimeResult<InspectResponse>` — returns InspectResponse.
- Line 343-362: `#[cfg(feature = "test-util")] pub fn recover(&mut self, journal: &crate::journal::SharedRuntimeJournal) -> RuntimeResult<Vec<RunId>>` — gated.
- NO `pub fn submit_artifact` on Runtime exists currently.

## crates/vb_cli/src/run_compiled_runtime.rs (286 lines)
- Line 234-261: `pub(crate) fn store_compiled_artifact(...)` — calls `vb_storage::admission::submit_artifact` at line 256.
- The current CLI bypasses the Runtime facade and calls the storage-level function directly.

# Scope (verified, no fabrication)

Add a thin Runtime-level wrapper method per master §66 spec:
```rust
impl Runtime {
    pub fn submit_artifact(
        &self,
        run: RunId,
        artifact_digest: WorkflowDigest,
        input: &[u8],
        capabilities: &[Capability],
    ) -> RuntimeResult<()> {
        // Validate artifact_digest matches a stored compiled_ir entry.
        // Wire capability check (per master §66 admission flow).
        // Call vb_storage::admission::submit_artifact internally.
        // Record RunAccepted journal event.
        // Return ().
    }
}
```

The wrapper:
1. Validates `artifact_digest` matches a stored `compiled_ir` entry (per master §66 admission flow line 3374).
2. Validates `input` against the workflow's declared input schema.
3. Checks every capability in `capabilities` is granted (per master §66 lines 3426-3443).
4. Calls the existing `vb_storage::admission::submit_artifact` internally.
5. Returns `()` after recording the `RunAccepted` journal event (per master §66 line 3405).

# Acceptance test

```rust
#[test]
fn runtime_submit_artifact_records_run_accepted_event() {
    // Open a test FjallJournal.
    // Build a CompiledWorkflow with artifact_digest=0xABCD and input=valid bytes.
    // Call Runtime::submit_artifact(run, digest, input, &caps).
    // Assert: RunAccepted journal event recorded.
    // Assert: Return is Ok(()).
}

#[test]
fn runtime_submit_artifact_rejects_ungranted_capability() {
    // Build a workflow that requires capability X.
    // Call submit_artifact with capabilities not including X.
    // Assert: Return is Err(CapabilityDenied).
}
```

# Anti-hallucination guards

- DO NOT fabricate `IpcCommand::SubmitArtifact` — verified `crates/vb_ipc/src/commands.rs:12` shows only `SubmitRun=1`, `SubmitRunInline=2`, etc.
- DO NOT use the storage-level signature `(journal, workflow, policy) -> Result<AcceptedArtifact>` as the Runtime method — master §66 requires a different signature.
- DO NOT add a `SubmissionReceipt` return type — master §66 returns `()`.

# Kani harness

`#[cfg(kani)]` at `crates/vb_runtime/src/kani/submit_artifact.rs`:
```rust
#[cfg(kani)]
mod proof {
    use kani::Arbitrary;
    impl Arbitrary for RunId { fn any() -> Self { RunId::new(kani::any()) } }
    impl Arbitrary for WorkflowDigest {
        fn any() -> Self {
            let bytes: [u8; 32] = kani::any();
            WorkflowDigest::from_bytes(bytes)
        }
    }
    impl Arbitrary for Capability {
        fn any() -> Self {
            let name_len: usize = kani::any();
            kani::assume(name_len < 64);
            // Generate a bounded name
            ...
        }
    }

    #[kani::proof]
    fn submit_artifact_never_panics_on_invalid_digest() {
        let runtime = build_test_runtime();
        let run: RunId = kani::any();
        let digest: WorkflowDigest = kani::any();
        let input: Vec<u8> = vec![0u8; 16];
        let caps: Vec<Capability> = vec![];
        // Expect Err(...), no panic.
        let _ = runtime.submit_artifact(run, digest, &input, &caps);
    }
}
```

# Dependency

Depends on P0-4r2 (the §19 mock match arms use the same `ActionOutcome::Suspended(ticket)` error type path).
