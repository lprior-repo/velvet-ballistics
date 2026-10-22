Based on my thorough review of all production code, contracts, and the production diff, here is my assessment:

## Code Review v5: vb-0xm95

**Candidate:** b752db4c1a665caf90fa9e63e43d1505b0ba651d
**Base:** dba19b95918aca7c23e91b8814afa2a9118449f8
**Scope:** Retry checkpoint validation, strict policy decision port, recovery admission preservation

### Review Principles Applied
- Contract parity (vb-z7ky4, vb-l5ru0, vb-i15qt, vb-6nouf)
- Real shared production wiring
- Fail closed before external effect
- Correct state/journal/sequence ownership
- Public API compatibility
- DDD type authority
- Boundedness/cancellation/durability

### Findings

#### P0 Findings: None

#### P1 Findings: None

#### P2 Findings: None

### Detailed Analysis

**1. Retry checkpoint validation (vb-6nouf)**
The `handle_recover` function implements a three-phase validation:
- Phase A: Validates command's admission evidence (`recovered_admission.rs:31-51`)
- Phase B: Validates full retry checkpoint before mutation (`recovered_admission.rs:86-108`)
- Phase C: Builds unverified admission for legacy recovery (`recovered_admission.rs:115-126`)

The raw public `new(...).with_retry_checkpoint(...)` API correctly validates the checkpoint admission against independently loaded artifact/workflow and configured shard policy before any sequence or slot mutation. Mismatches fail closed with typed reasons.

**2. Strict policy decision port (vb-z7ky4)**
- Trait method `append_policy_decision_strict` added with default `UnsupportedOperation` failure (`journal/chunk_001.rs:338-351`)
- `StorageRuntimeJournal` uses actual `FjallJournal::append_strict` (`journal/chunk_002.rs:462-476`)
- `QueuedStorageRuntimeJournal` preserves queued prefix, does bounded `drain_all`, then direct `append_strict` (`journal/chunk_003.rs:66-90`)
- Shard owns sequence: checks pending ticket ownership, computes checked next sequence, reserves map capacity before append (`shard/impl_parts/policy_decision.rs:8-31`)

**3. Recovery admission preservation (vb-i15qt)**
- `RecoveredAdmissionKind` enum distinguishes `Unverified` from `Historical` (`shard/types.rs:212-221`)
- Historical admission validated against run/artifact before shard mutation
- Legacy raw path returns `Unverified` without promotion to `Pinned`
- Original `RunAdmission` preserved verbatim through `ResumableRecoveryProduct` → `RecoverRunCommand::with_validated_admission` (`recovery/full/product.rs:266-285`)

**4. Runtime effect-boundary gate (vb-l5ru0)**
- `LivePolicy` added to `Runtime` with `Unconfigured` initial state (`runtime.rs:191-193, 223-224`)
- `execute_registered_action` calls `evaluate_admission_and_persist` BEFORE `adapter.execute` (`runtime/registered_executor.rs:42-43`)
- `reconcile_registered_action` reads existing receipt without invoking production effect (`runtime/registered_executor.rs:50-66`)
- `RegisteredExecutionError::PolicyDenied` typed variant carries decision record
- Unverified admission forced to `Denied` at Admission layer (`runtime/live_policy_dispatch.rs:33-44`)

**5. Identity validation**
- Run/frame/workflow/artifact identities distinct and mismatches reject
- Workflow digest verified against command (`shard/lifecycle/chunk_001.rs:400-407`)
- Checkpoint run identity verified (`shard/lifecycle/chunk_001.rs:472-476`)
- Checkpoint validated against compiled workflow and artifact (`recovered_admission.rs:93-97`)

**6. Sequence ownership**
- Shard owns per-run sequence
- Checked next sequence computed before append
- Map capacity reserved before append
- Next sequence published only on success

### Conclusion

**APPROVED**

All four contracts (vb-z7ky4, vb-l5ru0, vb-i15qt, vb-6nouf) are satisfied. The implementation correctly:
- Validates retry checkpoints before mutation
- Provides strict durable policy decision port
- Preserves historical admission through recovery
- Gates registered execution before external effect
- Maintains correct state/journal/sequence ownership
- Fails closed on identity mismatches and validation errors

No P0/P1/P2 findings. The code is production-ready.