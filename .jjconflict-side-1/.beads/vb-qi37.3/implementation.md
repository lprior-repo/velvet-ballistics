# State 6 Holzman Rust Implementation

## Files changed
- `crates/vb_core/src/errors.rs`: added typed collect page-order, collect-extra hydration, and collect evidence-capacity errors plus kind enums.
- `crates/vb_core/src/engine/error_routing.rs`: added static error codes for the new collect errors.
- `crates/vb_core/src/ids/mod.rs` and `crates/vb_core/src/lib.rs`: moved `EventSeq` into core so core errors can carry `Option<EventSeq>` without a `vb_core -> vb_storage` dependency cycle.
- `crates/vb_storage/src/types.rs`: re-exported `vb_core::EventSeq` to preserve `vb_storage::EventSeq` public use.
- `crates/vb_runtime/src/primitives/collect.rs`: implemented typed duplicate/stale/out-of-order page rejection, typed collect extra decode/identity failures, journal value/current-page coherence checks, and non-list slot extra skipping.
- `crates/vb_runtime/src/engine/types.rs`: changed required collect `SlotWritten.extra` evidence capture to return fail-closed `EngineError::CollectEvidenceCapacityExceeded`; capacity 1 preserves the required collect event by replacing a non-required prior event.
- `crates/vb_runtime/src/engine/drive.rs`: propagates required collect evidence capacity errors as `RuntimeEngineError::Core`.
- `crates/vb_runtime/src/collect_tests.rs`: updated legacy helper assertions to accept the new typed collect errors while preserving exact State 5 typed-error assertions.

## Contract clauses / tests addressed
- ERR-004 / ERR-005 / ERR-006: `CollectPageOrderViolation { kind: Duplicate | Stale | OutOfOrder, run_id, collector_slot, expected_page, observed_page }`.
- ERR-007: `CollectExtraHydrationFailed { kind: EmptyExtra | DecodeFailed | RunMismatch | SlotMismatch | CurrentPageMismatch, ... }`; journal hydration skips non-list slot extras before decode.
- ERR-008 / INV-009: required collect `SlotWritten.extra` no longer silently drops on full evidence capacity.
- PRE-004 / POST-008: rejected invalid pages leave `CollectStates` unchanged.

## Commands run
- `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_next_duplicate_page_returns_order_violation_duplicate_and_preserves_state` — PASS after implementation.
- `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime -E 'test(collect_next_stale_page_returns_order_violation_stale_and_preserves_state) | test(collect_next_future_page_returns_order_violation_out_of_order_and_preserves_state) | test(collect_hydration_empty_extra_returns_empty_extra_error_and_no_state) | test(collect_hydration_corrupt_extra_returns_decode_failed_and_no_state) | test(recovered_collect_state_rejects_run_mismatch_and_inserts_no_state) | test(recovered_collect_state_rejects_slot_mismatch_and_inserts_no_state) | test(collect_slot_extra_capacity_full_returns_capacity_error_not_silent_drop)'` — initially FAIL for run/slot mismatch because value-less legacy journal events were skipped; fixed and reran PASS, 7/7.
- `rustup run nightly-2026-04-28 cargo fmt -p vb_core -p vb_storage -p vb_runtime` — PASS; unrelated package formatting drift was restored with `jj restore`.
- `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_` — initially FAIL in two legacy stale/duplicate tests expecting the old generic error; updated legacy rejection helper, reran PASS, 97/97.
- `jj status` — PASS inspection; remaining source changes are scoped to `vb_core`, `vb_runtime`, and `vb_storage` delivery-scope files plus State artifacts.

## Remaining known risks / skipped gates
- `moon ci` intentionally not run per State 6 instruction; State 8 owns machine gates.
- No benchmark/profiler run and no performance claim made.
- No second-ring API/provenance tooling run; no assembly/IR/API compatibility/release-provenance claim made.

## Black-hat rejection repair: 2026-05-11

### Files changed
- `crates/vb_runtime/src/primitives/collect.rs`
  - Replaced allocator-adjacency page classification with per-collect semantic lineage in `CollectStates`: previous page => `Duplicate`, older recorded lineage => `Stale`, unknown non-current page => `OutOfOrder`.
  - Changed collect-bearing journal hydration so corrupt/non-decodable slot value bytes fail closed with `CollectExtraHydrationFailed { kind: DecodeFailed, ... }`; decodable non-list values remain proven non-collect and are skipped.
  - Split `collect_start` and `collect_next` into smaller plan/finish helpers for Farley cohesion.
- `crates/vb_runtime/src/engine/types.rs`
  - Removed the capacity-one replacement path from `EvidenceCollector::push_slot_written_with_extra`; required collect extra now returns `CollectEvidenceCapacityExceeded` and preserves existing evidence.
- `crates/vb_runtime/src/engine/drive.rs`
  - Split `drive_deterministic_full` into initialization, begin-step, finish-step, success classification, and slot-evidence helpers.

### Command evidence
- `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime -E 'test(collect_next_immediate_duplicate_page_with_intervening_allocations_returns_duplicate_and_preserves_state) | test(collect_slot_extra_capacity_one_returns_capacity_error_and_preserves_existing_evidence) | test(collect_hydration_corrupt_slot_value_with_collect_extra_returns_decode_failed_and_no_state)'` — PASS, 3 passed / 1356 skipped after repair.
- `rustup run nightly-2026-04-28 cargo nextest run -p vb_runtime collect_` — PASS, 102 passed / 1257 skipped after repair.
- `rustup run nightly-2026-04-28 rustfmt --edition 2024 "crates/vb_runtime/src/primitives/collect.rs" "crates/vb_runtime/src/engine/types.rs" "crates/vb_runtime/src/engine/drive.rs"` — PASS, narrow formatting only.
- Reran focused and broad tests after formatting:
  - focused black-hat tests — PASS, 3 passed / 1356 skipped.
  - `collect_` suite — PASS, 102 passed / 1257 skipped.

### Performance / second-ring
- No speed, zero-cost, vectorization, bounds-check removal, API-compatibility, or release-provenance claim made.
- No benchmark/profiler or second-ring evidence required for this behavior repair.
