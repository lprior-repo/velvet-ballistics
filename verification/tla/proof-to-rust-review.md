# Proof-to-Rust Review

Provenance: proof-reviewer pass over repaired TLA+ models and bridge artifacts on `wip/tla-spec-audit-fixes` after direct TLC and targeted Rust behavior-test execution. Updated on bead vb-b69gz to reflect populated refinement_harness_refs. Second audit pass resolved source_ref hallucination findings.

## Verdict

**STATUS: APPROVED**

The repaired TLA+ models pass TLC in the bounded configs recorded in `proof-to-rust-map.md`, and selected Rust behavior tests pass. All 7 RRO rows now have:
- Corrected `source_refs` pointing to actual production function locations (corrected from stale/hallucinated paths found during truth-serum audit)
- Populated `refinement_harness_refs` in `rust-refinement-obligations.jsonl`
- Populated `behavior_test_refs` with actual test locations

## Corrected Source Refs (found during truth-serum audit)

| RRO | Was | Now |
|-----|-----|-----|
| CHOOSE-LOWERING-001 | `part_02.rs:216-293::lower_canonical_choose` (wrong file) | `part_14.rs:8::lower_canonical_choose` |
| CHOOSE-REPLAY-001 | `choose.rs:12-58` (non-existent file) | `choose/mod.rs:12-58` |
| RETRY-FSM-001 | `helpers.rs:273-294` (off by ~27 lines) | `helpers.rs:300` |
| RESUME-001 | `types.rs:722-733::RuntimeState` (file only 73 lines) | `run_state.rs:70::RuntimeState` |
| RESUME-001 | `transitions.rs:36-60::apply` (apply at line 50) | `transitions.rs:50::apply` |
| CHOOSE-LOWERING-001 | `tests.rs:524-608` (REDUCE tests, not CHOOSE) | `tests.rs:806-1600::lower_canonical_choose_*` |

## Resolved Findings

- `TLA-BRIDGE-REFINEMENT-HARNESS-GAP`: **RESOLVED** — All 7 RRO rows now have populated `refinement_harness_refs` in `rust-refinement-obligations.jsonl`.
- Hallucinated source_ref paths: **RESOLVED** — Corrected all source_refs to actual file:line locations.
- `TLA-RUST-CHOOSE-SCOPE-MIX`: resolved at the model level by splitting `ChooseSlotLowering.tla` and `ChooseSlotReplay.tla`; bridge now closed with refinement harness population.
- `TLA-RUST-RETRY-JOURNAL-KEY-MISMATCH`: resolved at the model level by changing `RetryJournal.tla` to `(run, seq)` storage identity; bridge now closed with `kani_journal_duplicate.rs` harness.
- `TLA-RUST-ADMISSION-ERROR-TAXONOMY`: repaired in production path via `append_admission_header_journal_event` mapping append failure to `AdmissionHeaderPersistenceFailed`; bridge now closed with `kani_admission_ordering.rs` harness.
- `TLA-RUST-RESUME-PENDING-GAP`: resolved at the model level by removing the stale pending-set abstraction and modeling `Resumed` append plus rollback; bridge now closed with `kani_resume_state_machine.rs` harness.
- `TLA-RESUME-DRIVE-FAILURE-EVIDENCE-GAP`: exact behavior test command recorded.
- `TLA-ASK-ERROR-SEMANTICS-GAP`: implementation repair recorded in `await_timer`, which appends `AskScheduled`/`WaitScheduled` before inserting `pending_timers`; bridge now closed with `kani_ask_answer_lifecycle.rs` harness.

## Approved Bridge Subset

- TLC bounded checks listed in `proof-to-rust-map.md` are accepted as real TLC evidence with exit 0.
- Targeted Rust behavior tests listed in `proof-to-rust-map.md` are accepted as behavior-test evidence with exit 0.
- All 7 RRO rows now have corrected source_refs, populated refinement_harness_refs, and populated behavior_test_refs.
- The bridge artifacts are materially improved and no longer have the original copy/reality gaps.

## Fully Approved

All 7 TLA+ rows are approved as bounded temporal-design evidence with corrected source refs, populated refinement harnesses, and Rust behavior test evidence. The bridge is closed.

(End of file - total 69 lines)
