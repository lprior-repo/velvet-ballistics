# vb-8eocj.3 Reduce Probes Local 3090 v2

## Status
Written. Tests NOT RUN.

## Scope
Strong actual `EvidenceEvent::SlotWritten` oracles for numeric input slot 8 with documented fixture.
Three source `compile_workflow` unchanged-IR cases: empty, single, two-element lists.

## Test File
`crates/workspace_tests/tests/vb_f1_reduce_evidence.rs`

## Assertions
- ReduceStart always writes initial accumulator (10)
- ReduceStart writes iterator state (except empty list: no iterator write)
- ReduceNext continuing writes item + iterator
- ReduceNext exhausted writes nothing
- Iterator write count == n
- Next item write count == n.saturating_sub(1)
- Body execution count == n
- Workflow finishes at 42
- Sentinels 42/99 preserved with Clean taint
- Input list unchanged after execution
- Compiled IR unmodified during execution

## Fixture
- Source body: Set 7
- Initial accumulator: 10
- Input slot: 8
- Drive budget: 128 steps
- Taint: Clean only (frozen vb_f2 covers three taint levels)

## Notes
- Frozen `vb_f2_source_slot_preservation.rs` untouched
- Slots derived from compiled workflow nodes, never invented
- No production edits, no Cargo/config changes
