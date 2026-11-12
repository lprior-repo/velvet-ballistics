# vb-4969v Kani Status

Status: **GOD RULE 1 fixture violation repaired; minimal aggregate-only harness added; targeted Kani still timed out; BLOCKED_KANI_TIMEOUT; no Kani proof claim**.

`crates/vb_runtime/src/verification/kani/kani_resume_state_machine.rs` no longer uses the fixed `WorkflowParts` / `RunFrame` fixture. The PO-vb282my-RS-KANI-001/002 guard setup now builds bounded symbolic `WorkflowParts`, validates them through `CompiledWorkflow::try_from_parts`, derives `RunFrame` dimensions from the validated workflow, and preserves aggregate/run-state invariant assertions before `Shard::admit_run_state`.

The former monolithic `kani_resume_non_resumable_guard` was split into `kani_resume_initial_guard` and `kani_resume_resuming_guard` so each non-resumable reason/state is targeted independently. The shared setup asserts exact `ResumeError::NotResumable { run_id, current_state }` parity.

Feature gating was updated from the quarantine feature to `kani-vb-4969v-runtime-a3`. The lane remains feature-isolated because it is resource-heavy; it is no longer labeled as a GOD RULE quarantine.

This continuation added `crates/vb_runtime/src/verification/kani/kani_vb4969v_aggregate_invariant.rs` behind `kani-vb-4969v-aggregate-invariant` for PO-vb282my-RS-KANI-006 / vb-4969v. The harness targets only `RunAggregate` crate APIs and intentionally constructs no `WorkflowParts`, `CompiledWorkflow`, or `RunFrame`.

Raw evidence:
- Root/JJ checks: `evidence/runtime-a3/raw/root-checks.txt` — PASS.
- GOD RULE 1 scan: `evidence/runtime-a3/raw/kani-god-rule1-scan.txt` — PASS; no old hardcoded fixture pattern or unsplit legacy harness name remains in `kani_resume_state_machine.rs`.
- Feature inventory: `evidence/runtime-a3/raw/kani-list-vb_runtime.txt` — PASS; `KANI_FEATURES=kani-vb-4969v-runtime-a3 bash scripts/kani-list.sh vb_runtime` raw evidence records split harnesses `kani_resume_initial_guard` and `kani_resume_resuming_guard` (the live `.evidence/kani-list/vb_runtime.json` was later overwritten by the aggregate feature inventory run).
- Aggregate GOD RULE 1 scan: `evidence/runtime-a3/raw/kani-god-rule1-aggregate-scan.txt` — PASS; negative scan found no workflow/frame construction or fixed structural fixture patterns in the minimal aggregate harness, and positive scan found the feature gate, aggregate APIs, and explicit `core::mem::forget` drop-elision markers.
- Aggregate feature inventory: `evidence/runtime-a3/raw/kani-list-vb_runtime-aggregate-invariant.txt` — PASS; `KANI_FEATURES=kani-vb-4969v-aggregate-invariant bash scripts/kani-list.sh vb_runtime` lists `verification::kani::kani_vb4969v_aggregate_invariant::kani_vb4969v_terminal_membership_excludes_runtime_state_minimal` in the live `.evidence/kani-list/vb_runtime.json`.
- Targeted split guard attempt: `evidence/runtime-a3/raw/kani-targeted-resume-initial-guard-timeout.txt` — TIMEOUT (`exit_status=124`) for `kani_resume_initial_guard` with `timeout 10m` and `-j 1`; no `VERIFICATION:- SUCCESSFUL` line, so no proof claim.
- Targeted aggregate invariant attempt: `evidence/runtime-a3/raw/kani-targeted-terminal-membership-timeout.txt` — TIMEOUT (`exit_status=124`) for `kani_terminal_membership_cannot_coexist_with_runtime_state` with `timeout 10m` and `-j 1`; no `VERIFICATION:- SUCCESSFUL` line, so no proof claim.
- Minimal aggregate cgroup attempt: `evidence/runtime-a3/raw/kani-targeted-aggregate-invariant-cgroup.txt` — TIMEOUT (`exit_status=124`) under `systemd-run --user --scope` memory controls, `timeout 10m`, and `-j 1`; no `VERIFICATION:- SUCCESSFUL` line, so no proof claim.
- Minimal aggregate cgroup attempt after result/container drop elision: `evidence/runtime-a3/raw/kani-targeted-aggregate-invariant-cgroup-drop-elided.part1.txt`, `.part2.txt`, `.part3.txt` — TIMEOUT (`exit_status=124`) under `systemd-run --user --scope` memory controls, `timeout 10m`, and `-j 1`; raw log reaches repeated hashbrown `find_inner` unwinding at bound 16 and no verifier summary. **BLOCKED_KANI_TIMEOUT** for PO-vb282my-RS-KANI-006.

Recorded bounds/assumptions/trust boundaries:
- Symbolic workflow generator bounds for guard setup: step count 1..=2, slot count 1..=2, constants empty, linear `Nop` prefixes ending in `Finish`, optional outputs, symbolic digest/name/step-name materialization.
- Model simplification: expressions/accessors/constants/action contracts are empty because these resume guard harnesses do not drive workflow execution or inspect action contracts; this is meaningful for vb-4969v because the claim is aggregate/runtime-state membership and `handle_resume` rejects before workflow execution.
- Added domain-restatement `kani::assume` calls in `assume_guard_workflow_parts_domain` to state the same generated bounds explicitly before production `try_from_parts` validation; these assumptions are proof context and not proof closure.
- `any_live_runtime_state` keeps the pre-existing `variant < 4` assumption over live states.
- Minimal aggregate harness bounds: `RunId::new(u64::from(kani::any::<u8>() % 4))`; live runtime states are symbolic over Initial, Running, Resumable, and Resuming; active capacity is 1; `runs` is an empty `IndexMap` because the aggregate APIs are exercised through checked-out ownership rather than full `RunState` construction.
- Minimal aggregate model simplification: `core::mem::forget` elides local `Result`, `RuntimeError`, `RunAggregate`, and `IndexMap` drop glue after discriminant/invariant checks. This keeps the proof focused on membership mutation/rejection semantics and makes no claim about Drop/allocator/`RuntimeError` destructor behavior.
- Latest Kani execution was resource-controlled with `systemd-run --user --scope --collect`, `MemoryHigh=8G`, `MemoryMax=12G`, `MemorySwapMax=0`, `timeout 10m`, and solver parallelism `-j 1`.
