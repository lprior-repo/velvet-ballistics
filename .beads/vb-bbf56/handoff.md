# vb-bbf56 Repair Handoff — DeepSeek faithful (13/13 drift findings)

## Outcome

`scripts/check-production-inner-drift.sh` went from **13 findings / exit 1** to
**0 findings / exit 0** (through `/tmp/vb-run-gate.py`, evidence under
`.beads/vb-bbf56/evidence/faithful-drift-final.*`). Production-binding gate
**exit 0** (`faithful-binding-final.*`). Affected bounded Verus obligations all
**verified, 0 errors** (evidence `faithful-verus-*.stdout`).

## What was repaired (all edits in `verification/verus/` only)

### 5 production_inner mirrors (semantic drift)

1. `production_inner/_exit_code_production.rs` — added `LifecycleError = 9`
   variant + `CliExitCode::LifecycleError => 9` match arm; corrected section
   claim ranges (`5-34`, `36-40`, `42-57`, `59-64`, `66-71`, `73-181`) and
   "9→10 variants"/"9→10 match arms" prose.
2. `production_inner/_workflow_error_production.rs` — added `InputSlotKind`
   local stub (9 variants) and the 3 production `WorkflowError` variants
   (`InputSlotOutOfBounds`, `DuplicateInputSlot`,
   `NonCanonicalInputSlotOrder`); DRIFT POLICY/claim `321-452` → `394-549`,
   "20→23 variants".
3. `production_inner/replay_invariants_production.rs` — added `RunInputBinding`
   local stub (`LegacyUnbound`/`Digest(WorkflowDigest)`), added `input_digest`
   field to `RunAdmission`, added `RetryCheckpoint {run,seq,attempt,step,
   payload_digest:[u8;32],payload}` variant, added `RetryCheckpoint` to
   `attempt()`; claim ranges → `events/event.rs:37-361` / `events.rs:175-204`.
4. `production_inner/try_from_parts_production.rs` — added `ExprBranch`,
   `SlotBranch`, `InputSlotKind` (9 variants incl. `Any`), `CompiledInputSlot`;
   added `semantic_source_identity` + `input_slots` to `WorkflowParts` and
   `CompiledWorkflow`; added the 3 input-slot `WorkflowError` variants;
   `try_from_parts` body now mirrors production (`validate_input_slots` call +
   both new fields); added `validate_input_slots` stub; claim ranges →
   `337-370`, `37-62`, `394-549`.
5. `production_inner/vb_xi2f_compile_source_production.rs` — re-exported
   `InputSlotKind`/`CompiledInputSlot`/`validate_input_slots` from the
   try_from_parts mirror; added `SlotCompiler.input_slots` field +
   `record_input_slot`; `compile_source_production` now populates
   `semantic_source_identity` + `input_slots`; CanonicalStepLayout claim
   `62-202` → `116-255`.

### 8 extern BINDING LEDGER files (stale line ranges)

`extern_budget_monotonic.rs`, `extern_ipc_runtime_transitions.rs`,
`extern_recovery_verification.rs`, `extern_storage_kind_family.rs`,
`extern_try_from_parts.rs`, `extern_vb_core_replay_step.rs`,
`extern_vb_jnz9_journal_event_seq_valid.rs`,
`extern_vb_jpq724_events_for_run_production.rs` — updated every `<- crates/`
ledger range to the current production line of the named identifier (e.g.
`workflow/mod.rs` 321/563/585/753/779/834/945/768/273/454/299/254 →
394/660/682/850/908/963/1074/1532/337/553/374/282; `shard/types.rs`
818/843/853/794/810/718/745/572/550 → 1246/1274/1283/1222/1241/1146/1173/703/721;
`codec/mod.rs` 153/160/149 → 176/183; `events.rs` 332/366/514 → 24/60/216;
`recovery/types.rs` 792/812 → 806/826; `recovery.rs` 194 → 220; etc.).

## Verification evidence (all exercised, none fabricated)

| Gate / obligation | Result |
|---|---|
| `check-production-inner-drift.sh` (via vb-run-gate) | exit 0, 0 findings |
| `check-verus-production-binding.sh` (via vb-run-gate) | exit 0 |
| `verus --crate-type=lib try_from_parts.rs` | 66 verified, 0 errors |
| `verus --crate-type=lib diagnostic_envelope_verus.rs` | 11 verified, 0 errors |
| `verus --crate-type=lib vb_xi2f_error_mapping.rs` | 13 verified, 0 errors |
| `verus --crate-type=lib vb_rpch_replay_invariants.rs` | 18 verified, 0 errors |
| `verus --crate-type=lib vb_xi2f_compile_source.rs` | 15 verified, 0 errors |
| `verus --crate-type=lib vb_rpch_seed_dimensions.rs` | 12 verified, 0 errors |

## Residual / out-of-scope notes (honest, no self-approval)

- **`extern_vb_xi2f_error_mapping.rs` inline `WorkflowError` enum is 20-variant**
  while production (and the now-fixed `_workflow_error_production.rs`) is
  23-variant. The companion proof `workflow_error_discriminant` (PO-005) covers
  only the 20 legacy variants. This is a **pre-existing gap** (not one of the 13
  drift findings), and closing it requires editing a proof match (proof claims
  are out of my allowed edit surface). The 3 input-slot variants are carried
  correctly by the `#[path]`-bound `prod_src` mirror; the Verus-mode inline
  enum remains at 20. Flagging for the independent reviewer.
- **Known Verus ICE** `recovery_verification.rs::production::CANNOT_RESUME_REASONS`
  (reported by root-owned canonical CI, coordinator message) is unrelated to
  these drift changes and is owned by root, not this workspace.
- Pre-existing modified files `crates/vb_core/src/kani_idempotency_gates.rs` and
  `crates/vb_core/src/replay/kani_harnesses.rs` (d14 Kani field repairs) were
  left untouched; production Rust not edited.

Bead left **OPEN**; root assigns fresh independent review.
