# State 5 black-hat red test repair: vb-qi37.3

## Doctrine
- Read `/home/lewis/.claude/skills/test-writer/SKILL.md`: tests must prove observable behavior with exact assertions, assert exact error variants/values, and preserve state evidence.
- Read `/home/lewis/.agents/skills/test-writer/SKILL.md`: same content; no conflict. Per startup rule, agents copy would win if a future conflict appears.

## Files changed
- `crates/vb_runtime/src/collect_tests.rs`
  - Added `collect_next_immediate_duplicate_page_with_intervening_allocations_returns_duplicate_and_preserves_state` for DEFECT-001.
  - Added `collect_hydration_corrupt_slot_value_with_collect_extra_returns_decode_failed_and_no_state` for DEFECT-003.
- `crates/vb_runtime/src/engine/types.rs`
  - Adjusted capacity-one collect evidence test to fail closed with `EngineError::CollectEvidenceCapacityExceeded` and prove previous evidence remains unchanged for DEFECT-002.

## Formatting
- Ran narrow rustfmt only on edited test files:
  - `rustup run nightly-2026-04-28 rustfmt --edition 2024 "crates/vb_runtime/src/collect_tests.rs" "crates/vb_runtime/src/engine/types.rs"` — PASS.
- Initial `rustfmt` without explicit edition failed on existing Rust 2024 let-chain syntax in production-owned code; rerun used `--edition 2024` and did not run workspace-wide cargo fmt.

## Focused RED evidence

### DEFECT-001 duplicate page semantics with intervening allocations
Command:
```bash
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime -E 'test(collect_next_immediate_duplicate_page_with_intervening_allocations_returns_duplicate_and_preserves_state)'
```
Result: RED / test failure.
Key output:
```text
left: Err(CollectPageOrderViolation { kind: Stale, run_id: RunId(1), collector_slot: SlotIdx(1), expected_page: ListId(3), observed_page: ListId(1) })
right: Err(CollectPageOrderViolation { kind: Duplicate, run_id: RunId(1), collector_slot: SlotIdx(1), expected_page: ListId(3), observed_page: ListId(1) })
Summary [   0.025s] 1 test run: 0 passed, 1 failed, 1358 skipped
```

### DEFECT-002 capacity-one fail-closed evidence preservation
Command:
```bash
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime -E 'test(collect_slot_extra_capacity_one_returns_capacity_error_and_preserves_existing_evidence)'
```
Result: RED / test failure.
Key output:
```text
left: Ok(())
right: Err(CollectEvidenceCapacityExceeded { run_id: RunId(4103), slot: SlotIdx(1), capacity: 1, len: 1, required: "collect SlotWritten extra" })
Summary [   0.015s] 1 test run: 0 passed, 1 failed, 1358 skipped
```

### DEFECT-003 collect-bearing corrupt slot value hydration
Command:
```bash
rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime -E 'test(collect_hydration_corrupt_slot_value_with_collect_extra_returns_decode_failed_and_no_state)'
```
Result: RED / test failure.
Key output:
```text
left: Ok(())
right: Err(CollectExtraHydrationFailed { kind: DecodeFailed, run_id: RunId(3803), collector_slot: SlotIdx(1), event_seq: Some(EventSeq(6)) })
Summary [   0.009s] 1 test run: 0 passed, 1 failed, 1358 skipped
```

## Production code
- No production implementation code was edited.
