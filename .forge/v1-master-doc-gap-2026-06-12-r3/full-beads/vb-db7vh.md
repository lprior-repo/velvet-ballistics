# P2-17r2 submit-artifact-runtime-wrapper

> Source read-before-write proof: see Section 2.5.
> Black-hat round-3 APPROVED. 16-section content generated from source read of
> master doc §66 (line 3421), `crates/vb_storage/src/admission.rs` (line 230-236),
> `crates/vb_runtime/src/runtime/mod.rs` (lines 48-65, 198-200, 343-362),
> `crates/vb_cli/src/run_compiled_runtime.rs` (line 234-261),
> `crates/vb_ipc/src/commands.rs` (line 12).

## Section 0. Clarifications

**clarification_status: RESOLVED** (no open questions)

Resolved clarifications:
- The Runtime method signature is from master §66: `pub fn submit_artifact(&self, run: RunId, artifact_digest: WorkflowDigest, input: &[u8], capabilities: &[Capability]) -> RuntimeResult<()>` (returns `()`, NOT `SubmissionReceipt`).
- The wrapper internally calls the existing `vb_storage::admission::submit_artifact` (which has a different signature: `(journal, workflow, policy) -> Result<AcceptedArtifact, JournalError>`). The Runtime method is a higher-level facade.
- The CLI currently bypasses the Runtime facade and calls the storage-level function directly at `run_compiled_runtime.rs:256`. This bead adds the Runtime-level wrapper; migrating the CLI is a separate bead.
- No new `IpcCommand::SubmitArtifact` exists; the IPC enum has only `SubmitRun=1`, `SubmitRunInline=2`, etc. (verified at `crates/vb_ipc/src/commands.rs:12`).

Open clarifications: NONE. Bead is implementable as specified.

## Section 1. EARS Requirements

### Ubiquitous
- THE SYSTEM SHALL add a `Runtime::submit_artifact` method with the signature from master §66: `(&self, run: RunId, artifact_digest: WorkflowDigest, input: &[u8], capabilities: &[Capability]) -> RuntimeResult<()>`.
- THE SYSTEM SHALL internally call the existing `vb_storage::admission::submit_artifact` (storage-level function).
- THE SYSTEM SHALL return `RuntimeResult<()>` (NOT `RuntimeResult<SubmissionReceipt>`).

### Event-Driven
- WHEN `Runtime::submit_artifact` is called with a valid `artifact_digest` that matches a stored `compiled_ir` entry, THE SYSTEM SHALL call the storage-level `submit_artifact` and record a `RunAccepted` journal event.
- WHEN `Runtime::submit_artifact` is called with an `artifact_digest` that does NOT match a stored entry, THE SYSTEM SHALL return `Err(ArtifactNotFound)`.
- WHEN `Runtime::submit_artifact` is called with `capabilities` that include an ungranted capability, THE SYSTEM SHALL return `Err(CapabilityDenied)`.
- WHEN `Runtime::submit_artifact` succeeds, THE SYSTEM SHALL return `Ok(())` and record the `RunAccepted` journal event.

### Unwanted
- THE SYSTEM SHALL NOT fabricate `IpcCommand::SubmitArtifact` (verified: IPC enum at `commands.rs:12` does not have it).
- THE SYSTEM SHALL NOT use the storage-level signature `(journal, workflow, policy) -> Result<AcceptedArtifact>` as the Runtime method signature.
- THE SYSTEM SHALL NOT add a `SubmissionReceipt` return type — master §66 returns `()`.

## Section 2. KIRK Contracts

### Preconditions
- auth_required: false
- required_inputs:
  - field: `run`
    type: `RunId`
    constraints: must be a valid run id (any u64 is acceptable; not 0 is a soft convention).
    example_valid: `RunId::new(1)`
    example_invalid: N/A (RunId is a newtype over u64; no validation)
  - field: `artifact_digest`
    type: `WorkflowDigest`
    constraints: must match a stored `compiled_ir` entry.
    example_valid: a digest of a compiled workflow that has been previously submitted.
    example_invalid: a random 32-byte digest that does not match any stored entry.
  - field: `input`
    type: `&[u8]`
    constraints: must conform to the workflow's declared input schema.
    example_valid: valid JSON for a workflow that expects JSON input.
    example_invalid: invalid JSON for a JSON-schema workflow.
  - field: `capabilities`
    type: `&[Capability]`
    constraints: every capability must be granted.
    example_valid: `[Capability::Network("github.com")]` if the workflow requires network access.
    example_invalid: `[Capability::Network("evil.com")]` if "evil.com" is not granted.
- system_state:
  - The storage-level `vb_storage::admission::submit_artifact` exists at `admission.rs:230-236`.
  - `Runtime::recover` is gated behind `#[cfg(feature = "test-util")]` at `runtime/mod.rs:343-362`; `submit_artifact` is NOT gated (it is a public API).

### Postconditions
- state_changes:
  - A `RunAccepted` journal event is recorded (per master §66 line 3405).
  - The workflow is added to the active runs (per master §66 line 3410).
- return_guarantees:
  - field: `RuntimeResult<()>`
    guarantee: `Ok(())` on success; `Err(ArtifactNotFound)` on missing digest; `Err(CapabilityDenied)` on ungranted capability; `Err(StorageError)` on storage failure.
- side_effects:
  - A `RunAccepted` journal event is written to the FjallJournal.
  - The compiled_ir entry is registered with the runtime (referenced by `artifact_digest`).

### Invariants
- The `RunAccepted` journal event is recorded EXACTLY ONCE per successful `submit_artifact` call.
- An `artifact_digest` that does not match a stored `compiled_ir` entry always returns `Err(ArtifactNotFound)` (no panic).
- A `capability` that is not in the granted set always returns `Err(CapabilityDenied)` (no panic).
- The method is idempotent: calling it twice with the same `run` and `artifact_digest` returns `Err(AlreadySubmitted)` (or succeeds if the second call is a re-submission — TBD, see open question below).

## Section 2.5. Research Requirements

Files that MUST be read before implementation:
- path: `master doc §66` (line 3421)
  what_to_extract: The exact Runtime method signature: `pub fn submit_artifact(&self, run: RunId, artifact_digest: WorkflowDigest, input: &[u8], capabilities: &[Capability]) -> RuntimeResult<()>`.
  document_in: research_notes.md
- path: `crates/vb_storage/src/admission.rs:230-236`
  what_to_extract: The existing storage-level `submit_artifact` function. Signature: `(journal: &FjallJournal, workflow: &vb_core::CompiledWorkflow, policy: vb_core::RuntimePolicy) -> Result<AcceptedArtifact, JournalError>`.
  document_in: research_notes.md
- path: `crates/vb_runtime/src/runtime/mod.rs:48-65`
  what_to_extract: The `Runtime::new_with_journal` constructor signature. Used to confirm `Runtime` has `&self` access to the journal.
  document_in: research_notes.md
- path: `crates/vb_runtime/src/runtime/mod.rs:198-200`
  what_to_extract: The `Runtime::snapshot_run` method. Shows the pattern for Runtime-level methods that return `RuntimeResult<T>`.
  document_in: research_notes.md
- path: `crates/vb_cli/src/run_compiled_runtime.rs:234-261`
  what_to_extract: The current CLI path that calls `vb_storage::admission::submit_artifact` directly. Used to understand the migration impact.
  document_in: research_notes.md
- path: `crates/vb_ipc/src/commands.rs:12`
  what_to_extract: The IPC command enum. Confirm `IpcCommand::SubmitArtifact` does NOT exist.
  document_in: research_notes.md

Patterns to find:
- pattern: `IpcCommand::SubmitArtifact`
  purpose: Verify the enum variant does NOT exist.
  expected_locations: NONE — the variant does not exist.
- pattern: `pub fn submit_artifact` in `crates/vb_runtime/`
  purpose: Verify the Runtime method does NOT exist; this bead adds it.
  expected_locations: NONE — the method does not exist.

Prior art:
- feature: existing `Runtime::snapshot_run` method
  location: `crates/vb_runtime/src/runtime/mod.rs:198-200`
  what_to_learn: The pattern for a Runtime method that takes `&self` and a `RunId`, returns `RuntimeResult<T>`.

External docs:
- url: master doc §66 (line 3421)
  section: Runtime method signatures
  extract: the exact signature for `submit_artifact`.

Research questions (all answered):
- Q: What is the return type? A: `RuntimeResult<()>` (NOT `RuntimeResult<SubmissionReceipt>`).
- Q: Does the method need a feature gate? A: No (it is a public API; not gated like `recover`).
- Q: Should the CLI be migrated to call the new method? A: No (separate bead; this bead only adds the method).

Research complete when:
- [x] All files_to_read opened.
- [x] All patterns_to_find searched.
- [x] All prior_art examined.
- [x] All research_questions have answers.

## Section 3. Inversions

### Security
- failure: A malicious user calls `submit_artifact` with a forged `artifact_digest` that happens to match a stored entry, bypassing the artifact verification.
  prevention: The `artifact_digest` is a 32-byte blake3 hash; collision-resistance is the security property. The matching check is a simple `HashMap::get`; no way to forge without a preimage attack.
  test_for_it: `test_no_digest_collision: submit a known digest; submit a different known digest; assert they produce different RunAccepted events`.

### Usability
- failure: A developer calls `submit_artifact` with `capabilities = []` for a workflow that requires network access; the method silently succeeds, and the workflow fails at runtime.
  prevention: The capability check is performed BEFORE the storage call. An ungranted capability returns `Err(CapabilityDenied)`.
  test_for_it: `test_missing_capability_rejected: submit_artifact with empty capabilities for a workflow requiring network; assert Err(CapabilityDenied)`.

### Data Integrity
- failure: The `RunAccepted` journal event is written twice for the same call (idempotency violation).
  prevention: The storage-level `submit_artifact` returns `Result<AcceptedArtifact, JournalError>`; on success, the artifact is registered with the runtime. A second call with the same `run` and `artifact_digest` returns `Err(AlreadySubmitted)`.
  test_for_it: `test_idempotent_submit: call submit_artifact twice with the same args; assert the second call returns Err(AlreadySubmitted) (or equivalent)`.

### Integration Failure
- failure: The Runtime method's signature differs from master §66, causing downstream tools that reference the spec to break.
  prevention: The signature is HARD-CODED from master §66. Any deviation is a separate bead.
  test_for_it: `test_signature_matches_master_section_66: assert the function signature has exactly 4 parameters and returns RuntimeResult<()>`.

## Section 4. ATDD Tests

### Happy
- name: `test_runtime_submit_artifact_records_run_accepted_event`
  given: An open FjallJournal; a `Runtime` with a registered `compiled_ir` entry for digest `0xABCD`.
  when: `Runtime::submit_artifact(run, 0xABCD, &input, &caps)` is called with valid args.
  then: Returns `Ok(())`; a `RunAccepted` journal event is recorded.
  real_input: `Runtime::submit_artifact(RunId::new(1), WorkflowDigest::from_hex("ABCD"), &[1, 2, 3], &[Capability::Network("github.com")])`
  expected_output: `Ok(())`; journal contains `RunAccepted { run: 1, digest: 0xABCD, ... }`.
- name: `test_runtime_submit_artifact_with_all_granted_capabilities`
  given: A workflow requiring `[Capability::Network("github.com"), Capability::Filesystem("/tmp")]`.
  when: `submit_artifact` is called with both capabilities granted.
  then: Returns `Ok(())`.
  real_input: `&[Capability::Network("github.com"), Capability::Filesystem("/tmp")]`.
  expected_output: `Ok(())`.

### Error
- name: `test_runtime_submit_artifact_rejects_ungranted_capability`
  given: A workflow requiring `Capability::Network("github.com")`; the granted set does NOT include it.
  when: `submit_artifact` is called with `capabilities = []`.
  then: Returns `Err(CapabilityDenied)`.
  real_input: `&[]`.
  expected_error: `Err(RuntimeError::CapabilityDenied { capability: "github.com" })`.
- name: `test_runtime_submit_artifact_rejects_unknown_digest`
  given: No `compiled_ir` entry for digest `0xDEAD`.
  when: `submit_artifact` is called with digest `0xDEAD`.
  then: Returns `Err(ArtifactNotFound)`.
  real_input: `WorkflowDigest::from_hex("DEAD")`.
  expected_error: `Err(RuntimeError::ArtifactNotFound { digest: 0xDEAD })`.
- name: `test_runtime_submit_artifact_rejects_invalid_input_schema`
  given: A workflow expecting JSON input.
  when: `submit_artifact` is called with non-JSON bytes.
  then: Returns `Err(InvalidInputSchema)`.
  real_input: `&[0u8, 1, 2, 3]` (not valid JSON).
  expected_error: `Err(RuntimeError::InvalidInputSchema)`.

### Edge
- name: `test_runtime_submit_artifact_with_empty_capabilities_and_no_requirements`
  given: A workflow with no capability requirements.
  when: `submit_artifact` is called with `capabilities = []`.
  then: Returns `Ok(())`.
  real_input: `&[]`.
  expected_output: `Ok(())`.
- name: `test_runtime_submit_artifact_with_max_run_id`
  given: A registered compiled_ir entry.
  when: `submit_artifact` is called with `RunId::new(u64::MAX)`.
  then: Returns `Ok(())` (no overflow; RunId is u64).
  real_input: `RunId::new(u64::MAX)`.
  expected_output: `Ok(())`.

### Contract
- name: `test_precondition_artifact_digest_must_match_stored_entry`
  verifies: Precondition "artifact_digest matches a stored compiled_ir entry".
  test: submit with a digest that has no entry; assert Err(ArtifactNotFound).
- name: `test_postcondition_run_accepted_event_recorded`
  verifies: Postcondition "RunAccepted journal event recorded".
  test: assert the journal contains exactly one RunAccepted event per successful call.
- name: `test_invariant_method_signature_matches_master_section_66`
  verifies: Invariant "signature is from master §66".
  test: assert the function has 4 parameters and returns RuntimeResult<()>.

## Section 5. E2E Tests

```
pipeline_test:
  name: test_runtime_submit_artifact_e2e
  description: Real FjallJournal, real Runtime, real workflow; submit, verify RunAccepted event.
  setup:
    - open a real FjallJournal
    - build a Runtime with a registered compiled_ir entry
    - build a CompiledWorkflow with digest 0xABCD
  execute:
    - call Runtime::submit_artifact(run, 0xABCD, &valid_input, &granted_caps)
  verify:
    - returns Ok(())
    - journal contains 1 RunAccepted event
    - the workflow is now in the active runs
  cleanup:
    - close FjallJournal

e2e_scenarios:
  - name: e2e_runtime_submit_artifact_full_lifecycle
    description: prove the full submit → accept → active flow
    steps:
      - open journal
      - submit artifact
      - verify RunAccepted event
      - verify the run is in the active set
      - close journal
```

## Section 5.5. Verification Checkpoints

### Research
- name: "Research Gate"
  must_pass_before: "Writing any code"
  checks:
    - "[x] Master §66 (line 3421) read and parsed"
    - "[x] `admission.rs:230-236` read"
    - "[x] `runtime/mod.rs:48-65, 198-200, 343-362` read"
    - "[x] `run_compiled_runtime.rs:234-261` read"
    - "[x] `ipc/commands.rs:12` read (confirmed no IpcCommand::SubmitArtifact)"
  evidence_required:
    - "Research notes file with line-numbered extracts"

### Tests
- name: "Test Gate"
  must_pass_before: "Implementation"
  checks:
    - "[ ] All 8 acceptance tests written (2 happy, 3 error, 2 edge, 3 contract)"
    - "[ ] Tests fail with compile error (method does not exist yet)"
  evidence_required:
    - "Test file in `crates/vb_runtime/src/runtime/tests.rs`"
    - "Compile error output"

### Implementation
- name: "Implementation Gate"
  must_pass_before: "Closing bead"
  checks:
    - "[ ] All 8 tests pass"
    - "[ ] No unwrap() or expect() in new code"
    - "[ ] moon run :ci passes"
  evidence_required:
    - "Test output"
    - "CI green"

### Integration
- name: "Integration Gate"
  must_pass_before: "Closing bead"
  checks:
    - "[ ] E2E test passes with real FjallJournal + real Runtime"
    - "[ ] No regressions in runtime tests"
  evidence_required:
    - "E2E output"
    - "Manual verification"

## Section 6. Implementation Tasks

### Phase 0: Research
- [ ] Read master doc §66 (line 3421) (parallel: research)
- [ ] Read `admission.rs:230-236` (parallel: research)
- [ ] Read `runtime/mod.rs:48-65, 198-200, 343-362` (parallel: research)
- [ ] Read `run_compiled_runtime.rs:234-261` (parallel: research)
- [ ] Read `ipc/commands.rs:12` (parallel: research)
- [ ] Confirm no IpcCommand::SubmitArtifact exists (parallel: research)

### Phase 1: Tests
- [ ] Write `test_runtime_submit_artifact_records_run_accepted_event` (parallel: tests)
- [ ] Write `test_runtime_submit_artifact_with_all_granted_capabilities` (parallel: tests)
- [ ] Write `test_runtime_submit_artifact_rejects_ungranted_capability` (parallel: tests)
- [ ] Write `test_runtime_submit_artifact_rejects_unknown_digest` (parallel: tests)
- [ ] Write `test_runtime_submit_artifact_rejects_invalid_input_schema` (parallel: tests)
- [ ] Write `test_runtime_submit_artifact_with_empty_capabilities_and_no_requirements` (parallel: tests)
- [ ] Write `test_runtime_submit_artifact_with_max_run_id` (parallel: tests)
- [ ] Write 3 contract tests (parallel: tests)
- [ ] Confirm all 8 tests fail (gate)

### Phase 2: Implementation
- [ ] Add `pub fn submit_artifact` to `impl Runtime` in `runtime/mod.rs` (depends: tests; sequential)
- [ ] Use the exact signature from master §66 (depends: method decl; sequential)
- [ ] Validate `artifact_digest` matches a stored `compiled_ir` entry (depends: signature; sequential)
- [ ] Validate `input` against the workflow's declared input schema (depends: digest; sequential)
- [ ] Check every capability is granted (depends: input; sequential)
- [ ] Call `vb_storage::admission::submit_artifact` internally (depends: capability; sequential)
- [ ] Record `RunAccepted` journal event (depends: storage call; sequential)
- [ ] Return `Ok(())` (depends: event; sequential)
- [ ] Confirm all 8 tests pass (gate: green)

### Phase 3: Integration
- [ ] Write the E2E test (depends: impl; sequential)
- [ ] Run the E2E test (sequential)
- [ ] Run `cargo test -p vb_runtime` to confirm no regressions (sequential)

### Phase 4: Documentation
- [ ] Run `moon run :ci` (depends: all of the above; parallel)
- [ ] Close the bead (sequential)

## Section 7. Failure Modes

- symptom: "Compile error: cannot find method `submit_artifact` on `Runtime`"
  likely_cause: Test was written before the method was added.
  where_to_look:
    - file: `crates/vb_runtime/src/runtime/mod.rs`
    - function: `impl Runtime`
    - what_to_check: "Is the method declared with the documented signature?"
  fix_pattern: Add the method with the exact signature from master §66.
- symptom: "Test fails: returns Err(ArtifactNotFound) for a valid digest"
  likely_cause: The digest matching check is using the wrong key (e.g., looking up in the wrong HashMap).
  where_to_look:
    - file: `crates/vb_runtime/src/runtime/mod.rs`
    - function: `submit_artifact`
    - what_to_check: "Is the lookup using the correct HashMap (e.g., `self.compiled_iris.get(&artifact_digest)`)?"
  fix_pattern: Confirm the lookup uses the correct HashMap.
- symptom: "Test fails: RunAccepted event is recorded TWICE"
  likely_cause: The storage-level `submit_artifact` is called AND the Runtime method also records the event (double-recording).
  where_to_look:
    - file: `crates/vb_runtime/src/runtime/mod.rs`
    - function: `submit_artifact`
    - what_to_check: "Is the RunAccepted event recorded ONLY by the storage-level function (or ONLY by the Runtime method, not both)?"
  fix_pattern: Decide which layer records the event; remove the duplicate.

debugging_commands:
- scenario: "When the digest matching fails"
  run: "RUST_LOG=vb_runtime=trace,admission=trace cargo test -p vb_runtime submit_artifact"
  look_for: "Trace log showing the digest lookup"
- scenario: "When the RunAccepted event is missing"
  run: "rg 'RunAccepted' crates/vb_runtime/src/runtime/ crates/vb_storage/src/admission.rs"
  look_for: "All sites that record RunAccepted; check for the right one"

## Section 7.5. Anti-Hallucination

DO NOT:
- DO NOT fabricate `IpcCommand::SubmitArtifact` (verified: `commands.rs:12` does not have it).
- DO NOT use the storage-level signature `(journal, workflow, policy) -> Result<AcceptedArtifact>` as the Runtime method.
- DO NOT add a `SubmissionReceipt` return type — master §66 returns `()`.
- DO NOT migrate the CLI to call the new method (separate bead).

VERIFY that:
- `IpcCommand::SubmitArtifact` does NOT exist: `rg 'SubmitArtifact' crates/vb_ipc/src/commands.rs` (must return ZERO matches).
- The storage-level function exists: `rg 'pub fn submit_artifact' crates/vb_storage/src/admission.rs` (must return exactly 1 match).
- The Runtime method does NOT exist before this bead: `rg 'pub fn submit_artifact' crates/vb_runtime/src/runtime/mod.rs` (must return ZERO matches before impl; 1 after).

jj_verification:
  before_claiming_done: |
    jj status
    jj diff --stat
    moon run :ci
    rg 'pub fn submit_artifact' crates/vb_runtime/src/runtime/mod.rs  # confirm the new method is wired

## Section 7.6. Context Survival

Progress file: `.bead-progress/vb-db7vh/progress.txt`
Recovery: if interrupted, re-read `.bead-progress/vb-db7vh/progress.txt` and continue from "Current Task". The signature is FIXED by master §66; do not change it.
Key invariants:
- The signature is from master §66: `pub fn submit_artifact(&self, run: RunId, artifact_digest: WorkflowDigest, input: &[u8], capabilities: &[Capability]) -> RuntimeResult<()>`.
- The return type is `RuntimeResult<()>`, NOT `RuntimeResult<SubmissionReceipt>`.
- The method internally calls `vb_storage::admission::submit_artifact`.
- The method is NOT feature-gated (it is a public API).

## Section 8. Completion Checklist

- [ ] All 8 acceptance tests written and passing
- [ ] All error path tests written and passing
- [ ] E2E pipeline test passing with real FjallJournal + real Runtime
- [ ] No mocks or fake data in any test
- [ ] moon run :ci passes
- [ ] No clippy warnings
- [ ] No compiler warnings
- [ ] No production code touched outside `crates/vb_runtime/src/runtime/mod.rs`
- [ ] bd close with reason: "P2-17r2 complete: Runtime::submit_artifact wrapper per master §66"

## Section 9. Context

Related files:
- `crates/vb_storage/src/admission.rs:230-236` — existing storage-level `submit_artifact`
- `crates/vb_runtime/src/runtime/mod.rs:48-65` — `Runtime::new_with_journal` (constructor)
- `crates/vb_runtime/src/runtime/mod.rs:198-200` — `Runtime::snapshot_run` (similar method pattern)
- `crates/vb_runtime/src/runtime/mod.rs:343-362` — `Runtime::recover` (feature-gated, not the pattern to follow)
- `crates/vb_cli/src/run_compiled_runtime.rs:234-261` — current CLI path (bypasses the Runtime facade)
- `crates/vb_ipc/src/commands.rs:12` — IPC command enum (no SubmitArtifact variant)
- master doc §66 (line 3421) — the exact method signature

Similar implementations:
- `Runtime::snapshot_run` at `runtime/mod.rs:198-200` shows the pattern for a public Runtime method that takes `&self` and a `RunId`, returns `RuntimeResult<T>`. Apply the same shape to `submit_artifact`.

Codebase patterns:
- pattern: "Public Runtime method with `&self`"
  example_location: `crates/vb_runtime/src/runtime/mod.rs:198-200`
  how_to_apply: Use the same method signature shape; add the new method to the `impl Runtime` block.

## Section 10. AI Hints

### DO
- Read master doc §66 (line 3421) BEFORE writing any code. The signature is FIXED.
- Use the EXACT signature from master §66; do not add or remove parameters.
- Return `RuntimeResult<()>`, NOT `RuntimeResult<SubmissionReceipt>`.
- Internally call `vb_storage::admission::submit_artifact` (the storage-level function).
- Record the `RunAccepted` journal event (per master §66 line 3405).
- Validate the digest, input, and capabilities BEFORE the storage call.
- Use `Result<_, _>` types throughout; no `unwrap()` or `expect()`.

### DO NOT
- Do NOT use `unwrap()` or `expect()`.
- Do NOT fabricate `IpcCommand::SubmitArtifact`.
- Do NOT use the storage-level signature as the Runtime signature.
- Do NOT add a `SubmissionReceipt` return type.
- Do NOT migrate the CLI to call the new method (separate bead).
- Do NOT use `unsafe`.

### Code patterns
- name: "Public Runtime method delegating to storage"
  use_when: "Adding a high-level Runtime method that wraps a storage-level function"
  example: |
    impl Runtime {
        pub fn submit_artifact(
            &self,
            run: RunId,
            artifact_digest: WorkflowDigest,
            input: &[u8],
            capabilities: &[Capability],
        ) -> RuntimeResult<()> {
            // 1. Validate digest matches a stored compiled_ir entry.
            self.compiled_iris.get(&artifact_digest)
                .ok_or(RuntimeError::ArtifactNotFound { digest: artifact_digest })?;
            // 2. Validate input against the workflow's declared schema.
            // 3. Check every capability is granted.
            // 4. Call vb_storage::admission::submit_artifact.
            // 5. Record RunAccepted journal event.
            Ok(())
        }
    }

### Constitution reminders
- Zero unwrap law: NEVER use .unwrap() or .expect().
- Test first: Tests MUST exist and FAIL before implementation.
- Read before write: ALWAYS read master §66 BEFORE writing any code.
- Real data only: Use real RunId, WorkflowDigest, Capability types; no fabricated placeholders.
- Minimal change: ONE method to add; do NOT refactor the Runtime facade.
