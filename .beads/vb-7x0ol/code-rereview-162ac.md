# Code Re-Review — vb-7x0ol (Independent Recovery + ABI Guard Review)

**Bead**: vb-7x0ol
**Reviewer**: DeepSeek v4 Pro via OMP (independent; authored none of the changes under review)
**Scope**: F1 guard wiring, F3 retained-input recovery, and the new typed ABI-error guard at frozen source.
**Model metadata**: `deepseek/deepseek-v4-pro`, thinking level high. No fallback model; no local-5090/3090 label.

## Frozen Source (reviewed)

| Role | SHA |
|------|-----|
| Review head (`@`) | `162ac607dc86657f060882a81dc8957e955e726c` |
| Source tree | `dda8ec066baf23eec0ded5057e41fb7cfd133e7e` |
| Parents | `1e7aeee2495f2cfca0e6c1e7719727dc7515ea0a` + `c789b1b35cb5d19a60cf2f786cbda5947109dcb1` |
| Source-identity twin | `82d381e334657329905e64d7c293885046bf5c12` (differs only in `.beads/vb-wlnl4/integration-manifest.md`) |

Reviewed blobs:

| File | Blob |
|------|------|
| `crates/vb_runtime/src/runtime.rs` | `ba6249b147fb6b72c8c131b2f4c833db86e45b00` |
| `crates/vb_runtime/src/shard/lifecycle/chunk_001.rs` | `ad5924f1513bc23dce7c54c36b99adeb6dbdb169` |
| `crates/vb_runtime/src/shard/lifecycle/execution_binding.rs` | `e8a63ffd286b6d8ae2573b96a8e9d1629b6506e2` |
| `crates/vb_runtime/src/runtime/registered_executor.rs` | `1b72d915230adb4594a28126eba88cd0688c9baa` |
| `crates/velvet_ballistics/tests/vb_md3w_retained_input_recovery.rs` | `d1782a971fcf4a217b236923bde81393f01dcd6d` |

## Gate Result

**STATUS: APPROVED (scoped) — F1 and F3 and the ABI guard are correct at this source.**
Not a full integration approval: two known worker-owned Fjall gaps remain red, and the registered-executor public surface still has no in-repo dispatch caller (assessed per the stated contract as a non-blocker; see §Known-Remaining).

---

## F1 — Recovery guard wiring (previously dead `validate_registered_recovery`)

**Delta vs prior rejection** (`18ef1b9c`): `runtime.rs:606-609` now calls
`self.validate_registered_recovery(&recovered.artifact, boundary)?` inside
`build_runtime_recovery_product`, and `chunk_001.rs:405-413` now derives
`registered_execution = ReconcileOnly` when `boundary.kind() == PendingAction`
(for the legacy recovery path; the retry-checkpoint path independently sets
`ReconcileOnly` in `RunCheckpointV1::restore` at `run_checkpoint.rs:213`).

**Call graph verified** (fresh, not inherited):
- `recover_and_resume` → `recover_product` → `build_runtime_recovery_product` → `validate_registered_recovery` (runtime.rs:609).
- `validate_registered_recovery` (registered_executor.rs:56-80): only fires when
  `boundary.pending_action_ticket()` is `Some`, the artifact evidence is `Packed`,
  and a pack exists for the pending action; it then requires
  `catalog.require_receipt_recovery(artifact, action)` (catalog.rs:123-130), which
  fails unless the pack's `recovery == RecoveryClass::ReconcileReceipt`.
- On failure it returns `RuntimeError::RecoveryCannotResume { reason: "registered_recovery_blocked" }`,
  which aborts the recovery product before any run state is admitted.

**No re-execution effect / no re-mint authority from deserialized metadata**:
- `reconcile_registered_action` (registered_executor.rs:44-54) calls only
  `adapter.reconcile` → `lookup` (register.rs:118-135), which reads the existing
  receipt, re-derives `receipt::record(grant, value, effect_ordinal)` and compares
  (`record == expected`), and `persist`; it never reaches `commit_effect`. The only
  target-writing path is `execute` → `commit_effect` (register.rs:60-75).
- `execute_registered_action` (registered_executor.rs:26-38) is gated on
  `context.mode == Live` before any adapter call; ReconcileOnly returns
  `ExecutorError::Unauthorized`.
- `registered_context` (registered_executor.rs:91-102) re-checks
  `grant.artifact_digest() == context.artifact.digest` and re-verifies the artifact
  through `catalog.verify_artifact` — so a grant issued for a different artifact or a
  forged/deserialized receipt cannot authorize a write.
- Receipt `validate_record` (receipt.rs:78-86) binds `input_digest == hash(value)`,
  `digest == hash(body)`, and `effect_ordinal != 0`; `lookup` additionally re-derives
  the expected record from the live grant+value. A tampered `effect_ordinal`/`value`
  with recomputed digest still fails because the re-derived `expected` uses the live
  `grant` fields (artifact, pack, scope, invocation) that the attacker cannot change.

**Probes** (real public `Runtime` + `ExecutorCatalog` + `FjallRegister`, run via
`python3 /tmp/vb-run-gate.py`; full logs under `evidence-rereview/`):

| Probe | Observed | Conclusion |
|-------|----------|------------|
| Blocked pack pending action → `recover_and_resume` | `Err(RecoveryCannotResume { reason: "registered_recovery_blocked" })` | guard fires; recovery of non-reconciliable registered action fails closed |
| ReconcileReceipt pack pending action → `recover_product` | `Resumable { run_id, boundary_kind: PendingAction }` | guard permits reconciliable recovery; boundary classification correct |
| `execute_registered_action` (Live) | 1 effect, `value==42`, run completes | authentic execute writes exactly one effect |
| `reconcile_registered_action` after execute | same receipt, `effect_count` still 1 | reconcile never adds an effect |
| `execute_registered_action` with grant for a different artifact | `Err(Executor(Unauthorized))`, `effect_count==0` | mismatched grant cannot write |

**F1 verdict: FIXED and sound.**

---

## ABI guard — `compare_action_abi_digests` (execution_binding.rs:70-92)

**Delta**: inserted before the full `contracts != artifact.action_contracts` equality in
`compare_execution_tuple` so a same-id divergent submitter contract surfaces as
`AdmissionActionAbiDigestMismatch { action, expected, submitted }` (canonical
`blake3(postcard(contract))` digests) instead of generic `AdmissionArtifactInvalid`.

**Fail-closed verification (source)**: the `else { continue; }` branch for a submitted
contract absent from the accepted set does NOT relax the guard — the full slice equality
`contracts != artifact.action_contracts.as_ref()` still runs afterwards and rejects
missing/extra/duplicate/order differences with `AdmissionArtifactInvalid`. The digest
check is strictly a diagnostic refinement over the same rejection boundary, not a
relaxation. This matches the observed red test
`runtime_submit_drive_without_action_abi_authority_fails_before_ticket_persist`, whose
submit now fails at Gate12 with `ACTION_CONTRACT_MISSING` (stricter, not weaker).

**Observation (non-blocking, LOW)**: `compare_action_abi_digests` propagates
`action_contract_abi_digest`, which maps a `postcard` failure to
`RuntimeError::RecoveryCannotResume { reason: "action_abi_digests_missing" }`. In the
submit path this error kind is semantically misleading, but `postcard::to_allocvec` of an
`ActionContract` (no floats, bounded integers) cannot fail in practice. Cosmetic only.

**ABI verdict: APPROVED.** No relaxed-guard regression.

---

## Catalog / grant / receipt binding (executor surface)

`ExecutorCatalog::run_registered_conformance` is a trusted built-in selector (catalog.rs:12-14);
`ExecutedConformance` cannot be minted from a deserialized `ConformanceReport`. The
receipt body binds invocation + artifact + pack + scope + input digest + value + ordinal,
and `check_metadata` (register.rs:110-135) ties `meta.effects == meta.count == observed
receipt key count` on open. Combined with the Live/ReconcileOnly mode gate and
`grant.artifact_digest` re-check, there is no reachable re-execution or re-mint path from
deserialized metadata. Verified by the 5 passing executor-conformance tests and probes 3-5.

---

## Findings summary (new, this review)

- **N-1 (LOW, cosmetic)**: submit-path ABI digest error kind is `RecoveryCannotResume`
  (execution_binding.rs:84 via `action_contract_abi_digest`); unreachable in practice.
  Fix (optional): map to `AdmissionArtifactInvalid` instead.
- No CRITICAL/HIGH/MEDIUM new defects found in F1/F3/ABI-guard source.

## Known-remaining (worker-owned; not rediscovered here)

1. **vb-r5c1c** CLI durable-retry unavailable — dedicated author active
   (`/home/lewis/src/isoloated/velvet-ballistics-vb-r5c1c-deepseek-retry`). Excluded; unreviewed.
2. `runtime_submit_drive_without_action_abi_authority_fails_before_ticket_persist` red —
   Gate12 strict submit now rejects `ACTION_CONTRACT_MISSING` before the test's old
   "submit-ok + ticket-missing-ABI" expectation. Keep Gate12 strict; final integration
   author updates the expectation. No guard weakening.
3. `runtime_submit_drive_persists_recoverable_fjall_pending_action_abi_digest` red —
   pending ticket not rehydrated after cold reopen (`expected recovered pending action ticket`).
   Owned by `/home/lewis/src/isoloated/velvet-ballistics-vb-s1rry-reviewed-merge` (vb-s1rry).
4. `digest_duplicate_parity` obsolete mirrors (4 red) — out of scope; source-identity
   F4/F5/F6 already approved at `175e5739`; digest review not duplicated.
