# vb-kyyf Black-Hat Review — State 12 Attempt 3

STATUS: APPROVED

Startup mandate satisfied: read `/home/lewis/.claude/skills/black-hat-reviewer/SKILL.md` and `/home/lewis/.agents/skills/black-hat-reviewer/SKILL.md`. Both copies match and require contract/bead parity first (`SKILL.md:12-16`), deterministic public-surface assertions (`SKILL.md:18-21`), Holzman/DDD discipline (`SKILL.md:23-33`), and clinical findings with exact citations (`SKILL.md:40-44`). No conflict observed; `/home/lewis/.agents/...` would win on conflict.

## Findings

No blocking defects found in the manifest-listed State 12 review surface.

## Attack checks

### BDD-KYYF-002 locked-writer / zero-event laundering

- Contract requires a persisted run, dropped/reopened store, repeated `events_for_run`, recovery summary/frame seed, and CLI `replay/events/inspect`; observations must be identical with contiguous monotonic sequence numbers (`contract.md:76-79`; POST-002 at `contract.md:36`; public surface list at `contract.md:8`).
- Current evidence now reports four durable storage events on both observations with `seq=0..3`: `RunAccepted`, `RunAdmission`, `StepStarted`, `RunFinished` (`.evidence/vb-kyyf/storage-replay-resume.md:12-13`).
- Recovery summary/frame seed aligns with those same terminal facts: first/last sequence `0..3`, one step started, terminal `Finished { result: SlotIdx(0) }`, and matching seeds across first/second reads (`.evidence/vb-kyyf/storage-replay-resume.md:14-17`).
- Prior locked-writer laundering is no longer present in the reviewed evidence: State 11 explicitly checked `! rg -q 'locked-writer|events=0'` and passed (`state11-cap-bdd-acceptance-report.md:29-33`), then recorded no `locked-writer` and no `events=0` marker (`state11-cap-bdd-acceptance-report.md:44-49`). The raw observation itself contains neither marker and reports `events=4` (`.evidence/vb-kyyf/storage-replay-resume.md:18-19`).

### CLI replay/events/inspect public-surface repeatability

- CLI public-surface reports are present for all three commands on both runs: `command_name: "replay"`, `"events"`, and `"inspect"` (`.evidence/vb-kyyf/storage-replay-resume.md:18-19`).
- Status/stdout/stderr are semantically aligned and exactly repeated: both `cli_first` and `cli_second` show `status_code: Some(0)` and empty stderr for each command; stdout contains the same run id `20002`, evidence path, normalized digest, and `events=4` marker (`.evidence/vb-kyyf/storage-replay-resume.md:18-19`).
- Replay stdout reports `recovered 4 event(s)` and sequence lines `seq=0` through `seq=3` ending in `terminal: RunFinished`; events stdout reports the same `seq=0..3` and `4 event(s) total`; inspect stdout reports `status=finished, events=4` (`.evidence/vb-kyyf/storage-replay-resume.md:18-19`). That now matches the durable journal events and terminal recovery facts (`.evidence/vb-kyyf/storage-replay-resume.md:12-17`).
- PO-002 command evidence passed after the cap-unblock rerun: `rtk cargo test -p vb_storage --test replay_resume` exit 0, three tests passed (`formal-verification-report.md:30-32`; `verification-ledger.jsonl:2`; `machine-gate-report.md:12-13`).

### Prior State 12 non-findings remain non-findings

- BDD-KYYF-001 static traceability/evidence markers remain present: bead id, scenario id, Given/When/Then, public surface, evidence path, digest, raw left/right observations, and `comparison=Ok` (`.evidence/vb-kyyf/bdd-cross-run-determinism.md:3-14`).
- BDD-KYYF-003 repeated blocked replay remains stable: first and second attempts return `ReplayPolicyBlocked`, with scheduled event count unchanged before/after (`.evidence/vb-kyyf/non-replay-safe-actions.md:12-16`).
- BDD-KYYF-004 corrupt/digest cases remain deterministic: eight cases report typed attempt1/attempt2 equality (`.evidence/vb-kyyf/recovery-bdd-errors.md:12-19`).
- BDD-KYYF-005 generated/IR parity evidence still carries matching semantic observation fields for IR and generated mode (`.evidence/vb-kyyf/generated-ir-parity.md:12-13`).
- BDD-KYYF-006 generated subset fail-closed remains typed as `CodegenError::UnsupportedIr` / `UnsupportedGeneratedSubset` (`.evidence/vb-kyyf/generated-subset-fail-closed.md:7-12`).
- BDD-KYYF-007 acceptance catalog rows still map every scenario to Given/When/Then, public surface, evidence path, and traceability (`.evidence/vb-kyyf/acceptance-catalog-traceability.md:5-13`).

### PO-010 deferred-global classification

- PO-010 permits `DEFERRED_GLOBAL` only after all BDD, TLA+, and Verus lanes pass and bead-local failures are not classified as global (`proof-obligations.planned.jsonl:10`).
- State 11 reports PO-001..PO-009 PASS and PO-010 `DEFERRED_GLOBAL` only after scoped obligations passed (`formal-verification-report.md:26-39`, `formal-verification-report.md:49-53`; `verification-ledger.jsonl:1-10`).
- The `moon ci` failures are identified as two out-of-scope `vb_cli` invalid-path/storage-error exit-code regressions plus a `mutants-smoke` disk-quota failure copying `.tlc-metadir`, not a vb-kyyf planned artifact failure (`regression-diff.md:18-29`; `machine-gate-report.md:21-23`).
- Classification is acceptable for this bead because no manifest-listed vb-kyyf scoped command failed after the cap-unblock evidence repaired BDD-KYYF-002 (`machine-gate-report.md:12-23`; `formal-verification-report.md:51-53`).

## Verdict

APPROVED. Attempt 3 fixes the prior lethal defect: CLI `replay/events/inspect` evidence is no longer a locked-writer/events=0 success stub and now repeats public-surface observations that match the durable journal sequence `0..3` and terminal `RunFinished` facts. PO-010 remains deferred global for unrelated workspace/global debt, not a hidden vb-kyyf failure.
