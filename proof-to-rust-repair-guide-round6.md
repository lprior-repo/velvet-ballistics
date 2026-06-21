# Proof-To-Rust Bridge Repair Guide — Round 6

**Source review:** `proof-to-rust-review-round6.md` (STATUS: REJECTED workspace-wide)
**Source findings:** `.beads/vb-dzibx/proof-findings-round6.jsonl` (PROOFS W3-01, W3-02, BR-1, BR-2, BR-3)

This guide specifies the contract for closing the bridge gap. Implementation is the proof-writer's job.

---

## Priority 1: Bridge the 22 TLA+ specs (BR-2)

**Per TLA spec, add a row to `proof-to-rust-map.md` (or per-bead map) with:**
- Spec file path
- Spec variables → production type mappings
- Spec actions → production function mappings
- Spec invariants → production invariant mappings
- Verifier command (`tlc -config <cfg> <tla>`) with raw output capture

**22 specs to bridge:**
1. `verification/tla/AcceptedCliAdmission.tla`
2. `verification/tla/AtomicAcceptedRunAdmission.tla`
3. `verification/tla/CapabilityLifecycle.tla`
4. `verification/tla/collect_body_model.tla`
5. `verification/tla/ConcurrencyControl.tla`
6. `verification/tla/EngineYamlAdmission.tla`
7. `verification/tla/EngineYamlIngress.tla`
8. `verification/tla/EngineYamlRecovery.tla`
9. `verification/tla/EngineYamlRunLifecycle.tla`
10. `verification/tla/IdempotencySafety.tla`
11. `verification/tla/IpcSyncEvidence.tla`
12. `verification/tla/LifecycleJournal.tla`
13. `verification/tla/RecoveryCrashRestart.tla`
14. `verification/tla/RecoveryHydration.tla`
15. `verification/tla/StepBudgetSuspension.tla`
16. `verification/tla/TimerWheel.tla`
17. `verification/tla/V1PrimitiveLowering.tla`
18. `verification/tla/VbKyyfReplayDeterminism.tla`
19. `verification/tla/Vt2fRuntimeLifecycle.tla`
20. `verification/tla/Vt2fStrictAdmission.tla`
21. `verification/tla/WorkflowBoundedAdmission.tla`
22. `verification/tla/YamlE2eChain.tla`

**Resolve 5 stale refs:**
- `specs/AskAnswerLifecycle.tla` — recreate or remove matrix entry
- `specs/RetryFSM.tla` — recreate or remove matrix entry
- `specs/RetryJournal.tla` — recreate or remove matrix entry
- `specs/ResumeStateMachine.tla` — recreate or remove matrix entry
- `specs/admission_header_before_ack.tla` — recreate or remove matrix entry

---

## Priority 2: Bridge 125 Verus files (BR-1)

**Per Verus file, add a row to `proof-to-rust-map.md` with:**
- Verus file path
- Spec enums/structs → production type mappings
- Spec functions → production function mappings (cite production file:line)
- `extern_spec` or `verifier::external_body` or `assume_specification` declaration
- Verifier command (`verus --crate-type=lib <file>`) with raw output

**For 11 self-admitted blockers, either:**
- (a) Build the missing bridge (10 `vb_ajc40_*` need a postcard/Serde wire-format Verus model; 1 `taint_lattice.rs` is admitted retired)
- (b) Delete the file and document the obligation as unprovable in Verus

**For 5 trust-marker files, add a per-file bridge row documenting:**
- The production function being modeled
- Why the function is `#[verifier::external_body]` (and the production-side bound)

---

## Priority 3: Wire Flux annotations (BR-3)

**Two options:**

1. **Inline the Flux annotations into production source.** Use `#[flux_rs::spec]` and `#[flux_rs::sig]` decorators. This is invasive but tractable.

2. **Add a `scripts/flux-check-annotations.sh` that runs `flux <file>.flux` for each .flux file in `verification/flux/`.** Capture raw output. Add the script to moon ci.

**Either option must:**
- Be runnable on a clean checkout.
- Produce raw output that can be cited as evidence.
- Be invoked from `moon ci` so the bridge check is part of CI.

---

## Acceptance Criteria

**Workspace-wide bridge closure:**
- 22 missing TLA bridge rows added.
- 5 stale TLA bridge refs resolved.
- 125 Verus bridge rows added (or files deleted).
- 41 Flux trusted markers audited per-instance.
- 132 hardcoded Kani files converted to symbolic.

**Per-bead bridge closure (already approved for vb-xi2f.24):**
- Bridge matrix complete.
- Evidence commands run with raw output captured.
- Test/harness refs separate.

**Reviewer re-verification:**
- `proof-findings-round6.jsonl` findings 0/29 open.
- `tla-gaps-report.jsonl` regenerated with 0 NO_BRIDGE_REF and 0 STALE_BRIDGE_REF.
- `verification/trusted-base-ledger.jsonl` updated with `disposition: fixed_with_evidence` for all entries.

---

STATUS: GUIDE (not a review verdict)
