# STATE.md — vb-core-proof-15-gate

## Beacon
- **Bead**: vb-core-proof-15-gate — "Emit real 15-gate VerificationProof"
- **Workspace**: vb-core-proof-15-gate-fresh
- **Started**: 2026-05-17
- **Pipeline**: go-skill 15-state

## Gap Summary
`crates/vb_storage/src/admission.rs` line 86-99: `VerificationProof::new()` sets all proof flags (`bounded`, `taint_safe`, `retry_safe`, `replayable`) to `true` unconditionally. No per-gate validation occurs. Gate count is set to 15 but no actual verification gates run.

## States

### State 1: Explore
- [ ] Map relevant files
- [ ] Identify verification boundary
- [ ] Confirm gap location

### State 2: Map
- [ ] `crates/vb_storage/src/admission.rs` — submit_artifact, VerificationProof::new
- [ ] `crates/vb_core/src/` — CompiledWorkflow, RuntimePolicy
- [ ] `verification/` — existing verification artifacts

### State 3: Contract
- [ ] Define per-gate contracts
- [ ] Specify expected proof flag semantics

### State 4: Proof Planning
- [ ] Kani harness for proof flag gap
- [ ] Identify which flags should depend on actual verification

### State 5: Proof Writing
- [ ] Write Kani harness

### State 6: Proof Review
- [ ] Review harness correctness

### State 7: Test Planning
- [ ] Plan unit/integration tests

### State 8: Test Writing
- [ ] Write tests

### State 9: Test Review
- [ ] Review test quality

### State 10: Implementation
- [ ] Implement per-gate validation

### State 11: Formal Verification
- [ ] Run Kani
- [ ] Run Miri if applicable

### State 12: Black-Hat Review
- [ ] Adversarial review

### State 13: Evidence Packaging
- [ ] Package verification evidence

### State 14: Landing
- [ ] Push to remote
- [ ] Close bead

### State 15: Cleanup
- [ ] Clean up workspace
