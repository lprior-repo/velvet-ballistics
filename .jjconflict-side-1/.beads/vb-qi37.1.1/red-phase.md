# Red Phase Report: vb-qi37.1.1

## Files changed

- `Cargo.toml` — adds root `proptest` dev-dependency for property invariants.
- `tests/vb_qi37_1_1_red_recovery_contract_test.rs` — executable red integration/property tests for taint-preserving slot recovery, no-output semantics, hydration, and drain report conservation.
- `fuzz/src/bin/recover_runtime_frame_seed_contract.rs` — compileable fuzz-entry scaffold for recovery-event decoding.
- `benches/vb_qi37_1_1_recovery.rs` — Criterion scaffold for no-output recovery path.
- `.beads/vb-qi37.1.1/red-phase.md` — this report.

## Intended failing test commands

- `cargo nextest run --test vb_qi37_1_1_red_recovery_contract_test`
- `PROPTEST_CASES=1000 cargo nextest run --test vb_qi37_1_1_red_recovery_contract_test proptest`
- `cargo test --test vb_qi37_1_1_red_recovery_contract_test --no-run`
- `cargo bench --bench vb_qi37_1_1_recovery --no-run`

## Why failures are expected before implementation

- Current `RuntimeJournalEvent::SlotWritten` and `JournalEvent::SlotWrittenEvent` do not carry durable `Taint`, so event-only recovery cannot reconstruct `RecoveredSlotEntry.taint == Secret` or `DerivedFromSecret` and currently defaults recovered event slots to `Taint::Clean` while marking taint unsupported.
- Current `StepSucceeded` stores a mandatory `SlotIdx`, so no-output deterministic steps are represented via `SlotIdx::ZERO`; recovery therefore inflates slot dimensions instead of preserving `output: None` semantics.
- Complete event-only slot recovery cannot hydrate a live frame yet because taint is unsupported even when value bytes decode successfully.
- The drain conservation red assertion documents the required `drained == written` success report for a fully persisted three-event drain before the queue/storage implementation is wired into this bead’s acceptance path.
