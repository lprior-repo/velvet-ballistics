# Proof-to-Rust Review

Provenance: proof-reviewer pass over repaired TLA+ models and bridge artifacts on `wip/tla-spec-audit-fixes` after direct TLC and targeted Rust behavior-test execution. Updated on bead vb-b69gz to reflect populated refinement_harness_refs. Second audit pass resolved source_ref hallucination findings. Third audit pass established honest partial closure with proportional waivers.

## Verdict

**STATUS: MIXED — 5 BRIDGE CLOSED, 2 PARTIAL CLOSURE**

5 rows are BRIDGE CLOSED (CHOOSE-LOWERING, CHOOSE-REPLAY, RETRY-FSM, RESUME, ADMISSION). 2 rows are PARTIAL CLOSURE with proportional waivers (ASK-ANSWER, RETRY-JOURNAL).

## Third-Pass Truth Serum Findings (honest scope audit)

### RRO-TLA-ASK-ANSWER-001: PARTIAL CLOSURE

**HARNESS LIMITATION**: `kani_ask_answer_lifecycle.rs` tests `apply(AwaitTimer)` state transitions and `append_journal_event` stub behavior. It does NOT call `await_timer` directly because `await_timer` requires a valid `RunState` with a workflow node at the current PC in an `Ask` variant with `timeout_slot`. The harness uses `minimal_workflow()` with empty nodes.

**Provenance**: Harness comment header now honestly documents:
- PROVABLE SCOPE: `apply(AwaitTimer)` state transitions, `append_journal_event` stub Ok/Err reachability, journal sequence monotonicity, `PendingTimerKind` enum
- TRUST BOUNDARY: `handle_ask_answer` control flow, `await_timer` append-then-insert ordering — proven by integration tests

**STUB FIX**: `journal_helpers.rs` `append_journal_event` stub was `Ok(())` always — fixed to `kani::any()` so both Ok and Err paths are reachable in Kani.

**Proportional Waiver**: ASK-ANSWER harness proves the Kani-provable subset. Integration tests cover `handle_ask_answer` and `await_timer` control flow. Waiver is proportional: harness proves state machine transitions and error isolation; integration tests prove the full control flow.

### RRO-TLA-RETRY-JOURNAL-001: PARTIAL CLOSURE

**HARNESS LIMITATION**: `kani_journal_duplicate.rs` tests `run_event_key` encoding injectivity. It does NOT call `append_unpersisted` or `append_queued_unpersisted` because they require `FjallJournal` with file-backed LSM keyspace and mutex — impossible to model in Kani.

**Provenance**: Harness comment header (lines 155-173) documents this explicitly: "BLOCKED: FjallJournal requires file-backed LSM tree + Mutex<()> which Kani cannot model directly."

**Proportional Waiver**: RETRY-JOURNAL harness proves key encoding guarantees (distinct pairs → distinct keys). Fjall Keyspace uniqueness (TB-vb282my-storage-fjall-001) is a documented trust boundary. Integration tests cover the full `append_unpersisted`/`append_queued_unpersisted` duplicate rejection. Waiver is proportional: harness proves the encoding foundation; integration tests prove the storage behavior.

## Corrected Source Refs (second-pass truth-serum audit)

| RRO | Was | Now |
|-----|-----|-----|
| CHOOSE-LOWERING-001 | `part_02.rs:216-293::lower_canonical_choose` (wrong file) | `part_14.rs:8::lower_canonical_choose` |
| CHOOSE-REPLAY-001 | `choose.rs:12-58` (non-existent file) | `choose/mod.rs:12-58` |
| RETRY-FSM-001 | `helpers.rs:300` (file only 31 lines) | `helpers/retry.rs:87::record_retry_attempt` |
| RETRY-FSM-001 | `chunk_001.rs::action failure handling` (non-existent file) | `chunk_001_action.rs:71::handle_action_failure` |
| RESUME-001 | `types.rs:722-733::RuntimeState` (file only 73 lines) | `run_state.rs:70::RuntimeState` |
| RESUME-001 | `transitions.rs:36-60::apply` (apply at line 50) | `transitions.rs:50::apply` |
| CHOOSE-LOWERING-001 | `tests.rs:524-608` (REDUCE tests, not CHOOSE) | `tests.rs:806-1600::lower_canonical_choose_*` |

## Resolved Findings

- `TLA-BRIDGE-REFINEMENT-HARNESS-GAP`: **RESOLVED** — All 7 RRO rows now have populated `refinement_harness_refs`.
- Hallucinated source_ref paths: **RESOLVED** — Corrected all source_refs to actual file:line locations.
- ASK-ANSWER overclaim: **RESOLVED** — Claim scoped to Kani-provable subset; trust boundary documented; proportional waiver approved.
- RETRY-JOURNAL overclaim: **RESOLVED** — Claim scoped to key encoding injectivity; Fjall trust boundary documented; proportional waiver approved.
- All other rows (CHOOSE-LOWERING, CHOOSE-REPLAY, RETRY-FSM, RESUME, ADMISSION): FULL BRIDGE CLOSED.

## Approved Bridge Subset

| RRO | Status | Proviso |
|-----|--------|---------|
| CHOOSE-LOWERING-001 | BRIDGE CLOSED | — |
| CHOOSE-REPLAY-001 | BRIDGE CLOSED | — |
| ASK-ANSWER-001 | PARTIAL CLOSURE | Proportional waiver for handle_ask_answer/await_timer trust boundary |
| RETRY-FSM-001 | BRIDGE CLOSED | — |
| RETRY-JOURNAL-001 | PARTIAL CLOSURE | Proportional waiver for Fjall Keyspace trust boundary |
| RESUME-001 | BRIDGE CLOSED | — |
| ADMISSION-001 | BRIDGE CLOSED | — |

## Proportional Waiver Standard

Each PARTIAL CLOSURE row must document:
1. Exact Kani-provable subset (what the harness actually calls)
2. Trust boundary (what cannot be modeled in Kani)
3. Alternative evidence (integration tests, proptest, etc. that cover the trust boundary)
4. Proportionality rationale (why the Kani subset + integration tests is sufficient)

(End of file - total 102 lines)
