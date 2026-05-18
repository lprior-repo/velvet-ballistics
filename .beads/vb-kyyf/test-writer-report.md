# vb-kyyf State 8 Test Writer Repair Report — Attempt 7

## Startup citations
- `/home/lewis/.claude/skills/test-writer/SKILL.md`: lines 49-67 require pre-flight over the test plan/source/infrastructure; lines 158-163 reject weak assertions while allowing deterministic helpers/table coverage; lines 313-360 define compile/test gates.
- `/home/lewis/.agents/skills/test-writer/SKILL.md`: same content observed; this copy wins on conflict. No conflict found.

## Files changed
- `crates/workspace_tests/tests/vb_kyyf_cross_run_determinism.rs`
  - DEFECT-003: expanded BDD-KYYF-004 into independent corruption/replay rows for `corrupt-snapshot`, `sequence-gap`, `duplicate-sequence`, `out-of-order-sequence`, `workflow-source-digest-mismatch`, `compiled-ir-digest-mismatch`, `action-abi-digest-mismatch`, and `policy-digest-mismatch`.
  - DEFECT-003: supported public surfaces now persist fixture state, drop/reopen, run two recovery/read attempts, and emit per-case `attempt1`, `attempt2`, and `expected_typed_error` evidence. Missing legal surfaces are recorded as exact `ScenarioSurfaceUnavailable` blockers instead of being hidden.
  - DEFECT-005: removed synthesized generated `PublicObservation` constants. Generated/IR parity now fails with exact `ScenarioSurfaceUnavailable { public_surface: "generated durable replay public surface" }` unless a public durable generated replay surface can emit/reload terminal result, taint, journal signature/payload digest, suspension, and typed errors.
  - DEFECT-005: BDD-KYYF-005 still writes durable evidence with the real IR observation and generated durable replay blocker before returning the typed diagnostic.
- `.beads/vb-kyyf/test-writer-report.md`
  - Replaced attempt-6 report with this attempt-7 evidence.

## Commands and raw outcomes

```text
$ pwd -P
exit: 0
stdout: /home/lewis/src/bd-vb-kyyf-bdd
```

```text
$ test "$(pwd -P)" = /home/lewis/src/bd-vb-kyyf-bdd && TMPDIR=/home/lewis/src/bd-vb-kyyf-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo fmt --check
exit: 1
result: RED formatting diff after test edits
```

```text
$ test "$(pwd -P)" = /home/lewis/src/bd-vb-kyyf-bdd && TMPDIR=/home/lewis/src/bd-vb-kyyf-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo fmt && TMPDIR=/home/lewis/src/bd-vb-kyyf-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo fmt --check
exit: 0
result: GREEN formatting after rustfmt
```

```text
$ test "$(pwd -P)" = /home/lewis/src/bd-vb-kyyf-bdd && TMPDIR=/home/lewis/src/bd-vb-kyyf-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo fmt --check
exit: 0
result: GREEN required format gate rerun
```

```text
$ test "$(pwd -P)" = /home/lewis/src/bd-vb-kyyf-bdd && TMPDIR=/home/lewis/src/bd-vb-kyyf-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo test -p velvet-ballastics-workspace-tests --test vb_kyyf_cross_run_determinism bdd_kyyf_001_to_006_require_executable_public_surfaces_not_catalog_bookkeeping_only -- --test-threads=1
exit: 101
result: RED / failing-first successful
failure summary:
- BDD-KYYF-002 remains `ScenarioSurfaceUnavailable` for `velvet-ballastics CLI replay/events/inspect`.
- BDD-KYYF-004 now reaches the repaired blocker: `ScenarioSurfaceUnavailable` for `action-abi or policy digest mismatch recovery public surface` after emitting all eight independent case rows.
- BDD-KYYF-005 now fails with exact `ScenarioSurfaceUnavailable` for `generated durable replay public surface` instead of a rustc harness compile blocker or synthesized generated constants.
```

## DEFECT-003 evidence summary

Written by the focused test to `crates/workspace_tests/.evidence/vb-kyyf/recovery-bdd-errors.md`:

```text
case=corrupt-snapshot,attempt1=CorruptSnapshot,attempt2=CorruptSnapshot,expected_typed_error=CorruptSnapshot
case=sequence-gap,attempt1=ScenarioSurfaceUnavailable,attempt2=ScenarioSurfaceUnavailable,expected_typed_error=ReplayDivergence
case=duplicate-sequence,attempt1=ScenarioSurfaceUnavailable,attempt2=ScenarioSurfaceUnavailable,expected_typed_error=ReplayDivergence
case=out-of-order-sequence,attempt1=ReplayDivergence,attempt2=ReplayDivergence,expected_typed_error=ReplayDivergence
case=workflow-source-digest-mismatch,attempt1=WorkflowSourceDigestMismatch,attempt2=WorkflowSourceDigestMismatch,expected_typed_error=WorkflowSourceDigestMismatch
case=compiled-ir-digest-mismatch,attempt1=CompiledIrDigestMismatch,attempt2=CompiledIrDigestMismatch,expected_typed_error=CompiledIrDigestMismatch
case=action-abi-digest-mismatch,attempt1=ScenarioSurfaceUnavailable,attempt2=ScenarioSurfaceUnavailable,expected_typed_error=ActionAbiMismatch
case=policy-digest-mismatch,attempt1=ScenarioSurfaceUnavailable,attempt2=ScenarioSurfaceUnavailable,expected_typed_error=PolicyDigestMismatch
```

## DEFECT-005 evidence summary

Written by the focused test to `crates/workspace_tests/.evidence/vb-kyyf/generated-ir-parity.md`:

```text
ir_observation:result=Ok,taint=Clean,event_signature=8,event_payload_signature=7,digest_status=workflow_source=true,compiled_ir=true,action_abi=true,policy=true,replay_policy_blocked=false,unsupported_generated_subset=false,semantic_slot_signature=42,semantic_action_signature=0,semantic_suspension=false,semantic_taint_signature=2
generated_durable_replay=ScenarioSurfaceUnavailable(public_surface=generated durable replay public surface)
```

## Red / green classification
- Overall State 8 attempt 7 classification: **RED / failing-first successful**.
- Format gate: **GREEN** after required rustfmt rerun.
- Hardened BDD public-surface suite: **RED by intended missing public surfaces/evidence**, not compile failure.
- DEFECT-003 status: repaired test coverage now names every required case, both attempts, and exact typed target/error/blocker.
- DEFECT-005 status: repaired test no longer permits synthesized generated observations; exact blocker is `ScenarioSurfaceUnavailable` for generated durable replay.

## Blockers for implementation/evidence state
- Public recovery surface for sequence-gap and duplicate-sequence currently does not yield stable typed `ReplayDivergence` through the persisted helper; evidence records exact `ScenarioSurfaceUnavailable` attempts.
- No public action-ABI or policy digest mismatch recovery verifier exists; evidence records exact `ScenarioSurfaceUnavailable` attempts against required `ActionAbiMismatch` and `PolicyDigestMismatch` typed errors.
- No public generated durable replay surface exists to reload generated terminal result, taint, journal signature/payload digest, suspension state, and typed errors.

## Next route
- Return to femdation for **State 9 test-reviewer** full suite review on attempt-7 hardened tests.

---

# State 8 Cap-Unblock Section — BDD-KYYF-002 CLI Hardening

## Authorization
- Owner authorization recorded: user explicitly authorized an explicit State 8 cap-unblock lane for BDD-KYYF-002 hardening in this conversation.
- Scope honored: bead `vb-kyyf` only; State 8 test-writer only; no production/runtime/storage/CLI behavior changes.

## Startup citations
- `/home/lewis/.claude/skills/test-writer/SKILL.md`: lines 49-67 require pre-flight over plan/source/infrastructure; lines 158-163 reject weak assertions while allowing deterministic helpers/tables; lines 313-360 define verification gates.
- `/home/lewis/.agents/skills/test-writer/SKILL.md`: same content observed and wins on conflict; no conflict found.

## Changed files
- `crates/workspace_tests/tests/vb_kyyf_cross_run_determinism.rs`
  - BDD-KYYF-002 now drops the reopened `FjallJournal` before invoking CLI replay/events/inspect, so the CLI must read a real persisted store path rather than an active-writer handle.
  - Added CLI assertions that run replay/events/inspect twice, capture stdout/stderr/status via `CliReport`, require exact report equality, reject `storage is held by an active writer` / `writer_lock_held` / `events=0`, and require scenario id, command name, run id, evidence path, digest marker, nonzero `events=4`, and command-specific durable journal signatures matching seq `0..3` and terminal/status facts.
- `.beads/vb-kyyf/test-writer-report.md`
  - Added this owner-authorized cap-unblock report.

## Red / failing-first evidence
- Consumed failing artifact before this hardening: `.evidence/vb-kyyf/storage-replay-resume.md` showed durable storage had four events but CLI replay/events/inspect emitted `storage is held by an active writer` with `events=0` and `status_code: Some(0)`.
- State 12 red defect consumed: `.beads/vb-kyyf/black-hat-review.md` lines 9-14 and `.beads/vb-kyyf/defects.md` lines 5-12 require rejecting locked-writer zero-event success stubs.
- Attempted focused pre/post command with obsolete package alias:
  - Command: `TMPDIR=/home/lewis/src/bd-vb-kyyf-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo test -p workspace_tests --test vb_kyyf_cross_run_determinism bdd_kyyf_001_to_006_require_executable_public_surfaces_not_catalog_bookkeeping_only -- --test-threads=1`
  - Exit: nonzero; package alias `workspace_tests` did not match any package. Not counted as behavior evidence.

## Commands and outcomes

```text
$ pwd -P
exit: 0
stdout: /home/lewis/src/bd-vb-kyyf-bdd
```

```text
$ TMPDIR=/home/lewis/src/bd-vb-kyyf-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo fmt --check
exit: 0
result: GREEN
```

```text
$ TMPDIR=/home/lewis/src/bd-vb-kyyf-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo test -p velvet-ballastics-workspace-tests --test vb_kyyf_cross_run_determinism bdd_kyyf_001_to_006_require_executable_public_surfaces_not_catalog_bookkeeping_only -- --test-threads=1
exit: 0
result: GREEN; cargo test: 1 passed, 15 filtered out (1 suite, 1.15s)
```

## Green evidence after hardening
- `.evidence/vb-kyyf/storage-replay-resume.md` now records CLI replay/events/inspect twice with exact matching reports.
- CLI `replay`: `status_code: Some(0)`, `recovered 4 event(s) for run 20002`, seq `0..3`, `terminal: RunFinished`, and trace `BDD-KYYF-002 command=replay ... digest=normalized-replay events=4`.
- CLI `events`: `status_code: Some(0)`, seq `0..3`, `4 event(s) total`, and trace `BDD-KYYF-002 command=events ... digest=normalized-replay events=4`.
- CLI `inspect`: `status_code: Some(0)`, `run 20002: status=finished, events=4`, and trace `BDD-KYYF-002 command=inspect ... digest=normalized-replay events=4`.
- No locked-writer or zero-event stub is accepted by the hardened assertions.

## Red / green classification
- Format gate: GREEN.
- Focused BDD-KYYF-002 cap-unblock hardening: GREEN; the implementation honestly passes once the test closes/drops the writer before public CLI reads.
- No production behavior was changed.

## Next route
- Return to femdation for **State 9 test-reviewer** review of the cap-unblock BDD-KYYF-002 CLI hardening.
