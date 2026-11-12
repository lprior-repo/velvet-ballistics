# Proof Repair Guide: vb-qi37.2 State 6 Attempt 3

## Required Repairs

1. Add exact required Kani harnesses for `PO-010`, `PO-011`, and `PO-012`.
2. Rerun and capture successful raw output for:
   - `cargo kani -p vb_core --harness aggregate_usage_try_add_budget_rejects_overflow_and_sums_fields`
   - `cargo kani -p vb_core --harness aggregate_usage_fits_within_rejects_over_capacity_fields`
   - `cargo kani -p vb_core --harness value_store_cap_rejects_insert_with_budget_exceeded_max_slots`
3. Repair fuzz tooling/configuration for `PO-014`, `PO-015`, and `PO-016`, then rerun each required `cargo fuzz run ... -- -runs=1000` command with raw run-count evidence.
4. Repair the selected nightly Miri environment or provide an approved exact equivalent for `PO-017`; rerun `cargo +nightly miri test -p vb_core value_store -- --nocapture` with no UB diagnostics.
5. Resolve `PO-019` ResourceContract parity by documenting active/legacy status and field/diagnostic parity for `compiled_workflow.rs` versus `workflow/mod.rs`, or by routing to production repair before proof review.
6. Repair or formally classify the `moon ci` blockers for `PO-018`; a failed canonical gate cannot be converted to proof approval by prose.

## Preserve Passing Work

- Keep the repaired TLA model unless future code/proof edits invalidate it. The rerun passed with deadlock checking enabled and no `CHECK_DEADLOCK FALSE` in the config.
- Keep the executed Verus rows unless proof artifacts or mapped source semantics change.
- Keep existing add/sub Kani helper evidence, but do not treat it as coverage for aggregate admission or ValueStore typed-error parity.

## Next Review Entry Criteria

- `proof-evidence.md` must map every required obligation to raw command output, an accepted waiver, or a precisely classified blocker that is allowed at State 6.
- `proof-writer-report.md` must not list any required State 5-owned obligation as absent or `BLOCKED_REVIEW` without a downstream owner and acceptable waiver.
- `proof-findings.jsonl` from the next review must remain valid JSONL and non-empty.
