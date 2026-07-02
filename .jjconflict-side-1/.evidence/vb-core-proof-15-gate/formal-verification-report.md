# Formal Verification Report: vb-core-proof-15-gate

**STATUS: STALE — UNSUPPORTED PASS CLAIMS REMOVED (vb-5kow2)**

## Gap: VerificationProof::new() Always Sets Proof Flags to True

### Verification Objective
Identify whether `VerificationProof::new()` in `crates/vb_storage/src/admission.rs` sets all proof flags (`bounded`, `taint_safe`, `retry_safe`, `replayable`) to `true` unconditionally, without performing actual per-gate validation.

### Verification Method
**Kani Bounded Model Checking (claim withdrawn; raw evidence missing)**

### Harness Status — UNVERIFIED
The following harness results were previously recorded as PASS but the referenced harness file (`crates/vb_storage/src/kani_proof_flags_gap.rs`) does not exist in the current tree, no raw Kani log is attached to this report, and no exit status was captured. Per bead vb-5kow2 and the master "no unsupported PASS without raw logs" rule, these rows are downgraded to UNVERIFIED:

| Harness | Property Investigated | Previous Result | Current Status |
|---------|------------------------|-----------------|----------------|
| VB-STORAGE-GAP-001 | `bounded == true` for any input | PASS | UNVERIFIED (no harness file, no log) |
| VB-STORAGE-GAP-002 | `taint_safe == true` for any input | PASS | UNVERIFIED (no harness file, no log) |
| VB-STORAGE-GAP-003 | `retry_safe == true` for any input | PASS | UNVERIFIED (no harness file, no log) |
| VB-STORAGE-GAP-004 | `replayable == true` for any input | PASS | UNVERIFIED (no harness file, no log) |
| VB-STORAGE-GAP-005 | All flags true simultaneously | PASS | UNVERIFIED (no harness file, no log) |
| VB-STORAGE-GAP-006 | Flags true even with gate_count=0 | PASS | UNVERIFIED (no harness file, no log) |

### Verification Evidence
None retained. The previously recorded inline block (`Verification Time: 0.38827655s`, `Complete - 6 successfully verified harnesses, 0 failures, 6 total.`) had no companion `.log` artifact and the harness file is absent, so it is not admissible as evidence.

### Code Location
- **Suspected gap**: `crates/vb_storage/src/admission.rs:86-99` (source location at time of writing; re-verify before acting on this finding)
- **Missing harness**: `crates/vb_storage/src/kani_proof_flags_gap.rs` (file absent)
- **Runtime Validation**: `crates/vb_runtime/src/admission.rs:318-333` (source location at time of writing; re-verify before acting on this finding)

### Conclusion
The PASS rows above have been removed because they were unsupported by raw logs. The conceptual gap (proof flags asserted without per-gate validation) remains a candidate concern but is **not** formally proven by this report. Any follow-up must (a) reintroduce the harness file with a `#[path = ".../crates/..."]` production binding or `Arbitrary` generators, (b) attach a raw Kani log with command, exit status, and timing, and (c) re-stamp this report only after those artifacts exist.

### Impact (unchanged, but not formally proven)
Any CompiledWorkflow (valid or invalid) producing proof with all flags=true would allow runtime admission to validate flags without verifying the underlying safety properties. Until a re-run produces raw evidence, this is a hypothesis, not a finding.
