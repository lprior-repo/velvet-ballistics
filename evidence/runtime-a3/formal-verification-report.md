# vb-4969v Runtime A3 Verification Report

Workspace: `/home/lewis/src/isoloated/velvet-ballistics-w25-runtime-a3`

Last refreshed: 2026-07-09

## Result

- Non-Kani gates: **PASS**.
- Kani proof lane: **GOD RULE 1 repair landed; minimal aggregate-only harness added; BLOCKED_KANI_TIMEOUT**. No Kani proof is claimed.

## Kani inspection

- `kani_resume_state_machine.rs` now uses bounded symbolic workflow/run-frame generation for PO-vb282my-RS-KANI-001/002 guard setup instead of a fixed dummy `WorkflowParts` / `RunFrame` fixture.
- The former `kani_resume_non_resumable_guard` was split into `kani_resume_initial_guard` and `kani_resume_resuming_guard` so Initial and Resuming rejection are independently named.
- Guard setup bounds were reduced to steps 1..=2, slots 1..=2, empty constants, linear `Nop` prefixes ending in `Finish`, optional outputs, symbolic digest/name/step-name materialization. This remains meaningful for vb-4969v because the checked claim is aggregate/runtime-state membership and `handle_resume` rejects before workflow execution.
- The feature gate is now `kani-vb-4969v-runtime-a3`; the lane remains feature-isolated for resource control, not quarantined for GOD RULE 1.
- `KANI_FEATURES=kani-vb-4969v-runtime-a3 bash scripts/kani-list.sh vb_runtime` was rerun; raw evidence records `kani_resume_initial_guard`, `kani_resume_resuming_guard`, and aggregate invariant harnesses. The live `.evidence/kani-list/vb_runtime.json` was later overwritten by the aggregate-only feature inventory run.
- A targeted run of `kani_resume_initial_guard` was attempted with `timeout 10m` and `-j 1`; it timed out (`exit_status=124`) before a verifier success/failure summary. No proof claim is made.
- A targeted run of `kani_terminal_membership_cannot_coexist_with_runtime_state` was attempted with `timeout 10m` and `-j 1`; it timed out (`exit_status=124`) before a verifier success/failure summary. No proof claim is made.
- Added `kani_vb4969v_aggregate_invariant.rs` behind `kani-vb-4969v-aggregate-invariant` for PO-vb282my-RS-KANI-006. It calls only `RunAggregate` crate APIs (`checked_out_insert`, `runtime_state_insert`, `terminal_insert`, `runtime_state_get`, `terminal_contains`) and constructs no workflow/frame structures.
- `KANI_FEATURES=kani-vb-4969v-aggregate-invariant bash scripts/kani-list.sh vb_runtime` was rerun; feature inventory lists `kani_vb4969v_terminal_membership_excludes_runtime_state_minimal` in the live `.evidence/kani-list/vb_runtime.json`.
- Targeted resource-controlled runs of the minimal aggregate harness were attempted under `systemd-run --user --scope` with `MemoryHigh=8G`, `MemoryMax=12G`, `MemorySwapMax=0`, `timeout 10m`, and `-j 1`. Both timed out (`exit_status=124`) before `VERIFICATION:- SUCCESSFUL`; the drop-elided attempt still hit hashbrown `find_inner` unwinding at bound 16. No proof claim is made.

## Raw evidence

- Root/JJ checks: `evidence/runtime-a3/raw/root-checks.txt` — PASS.
- Format: `evidence/runtime-a3/raw/fmt.txt` — PASS.
- Check: `evidence/runtime-a3/raw/check.txt` — PASS.
- Clippy/source lint: `evidence/runtime-a3/raw/lint-src-clippy.txt` — PASS.
- Source length: `evidence/runtime-a3/raw/source-length.txt` — PASS.
- `vb_runtime` lib tests: `evidence/runtime-a3/raw/vb_runtime-lib-tests.txt` — PASS, 1850 passed.
- Kani GOD RULE 1 scan: `evidence/runtime-a3/raw/kani-god-rule1-scan.txt` — PASS, no old fixed-fixture patterns or unsplit legacy harness remain in the repaired resume harness.
- Kani inventory/status: `evidence/runtime-a3/raw/kani-list-vb_runtime.txt` — PASS for inventory only; cargo-kani 0.67.0; feature inventory contains split resume guard harnesses.
- Aggregate GOD RULE 1 scan: `evidence/runtime-a3/raw/kani-god-rule1-aggregate-scan.txt` — PASS for fixture scan only.
- Aggregate Kani inventory/status: `evidence/runtime-a3/raw/kani-list-vb_runtime-aggregate-invariant.txt` — PASS for inventory only; cargo-kani 0.67.0; feature inventory contains the minimal aggregate harness.
- Historical targeted Kani attempt: `evidence/runtime-a3/raw/kani-targeted-resume-non-resumable.txt` — TIMEOUT (`exit_status=124`), no proof claim.
- Targeted Kani attempt: `evidence/runtime-a3/raw/kani-targeted-resume-initial-guard-timeout.txt` — TIMEOUT (`exit_status=124`), no proof claim.
- Targeted Kani attempt: `evidence/runtime-a3/raw/kani-targeted-terminal-membership-timeout.txt` — TIMEOUT (`exit_status=124`), no proof claim.
- Targeted Kani attempt: `evidence/runtime-a3/raw/kani-targeted-aggregate-invariant-cgroup.txt` — TIMEOUT (`exit_status=124`), no proof claim.
- Targeted Kani attempt after drop elision: `evidence/runtime-a3/raw/kani-targeted-aggregate-invariant-cgroup-drop-elided.part1.txt`, `.part2.txt`, `.part3.txt` — TIMEOUT (`exit_status=124`), no proof claim.

## Residual blockers

1. Targeted Kani proof closure remains blocked by the 10-minute timeout / unwind-heavy verifier paths for split guard and aggregate invariant harnesses, including the new aggregate-only harness.
2. The repair records bounded symbolic workflow scope (1..=2 steps, 1..=2 slots, empty constants, linear Nop/Finish shapes); it is not a proof over arbitrary unbounded workflows.
3. The minimal aggregate harness records a bounded symbolic domain (run ids 0..=3, live states Initial/Running/Resumable/Resuming, active capacity 1) and drop-elision simplification; it is not evidence for Drop/allocator/destructor behavior.
4. Runtime-a3 Kani lane must not be presented as proof closure until a resource-controlled targeted run reaches `VERIFICATION:- SUCCESSFUL` for the required harnesses.

Review-ready status: GOD RULE 1 repair is review-ready with honest timeout evidence, not Kani proof closure.
