# Test Repair Guide — vb-qi37.10 Repair Attempt 1 Rejection

## Required Repair Loop

The suite is still rejected. Do not modify production code in this repair loop unless the next go-skill state explicitly allows implementation. Repair tests only.

## What Is Fixed

1. Required filters now select at least one test. Keep this property.
2. `generated_support_matrix_totality_requires_parity_owner_for_every_supported_family` is no longer just a non-empty-string table; it verifies that each owner name appears as `fn <owner>` in `tests.rs`. Keep this enforcement or strengthen it further.

## Mandatory Repairs

1. Fix expression/accessor fixture construction:
   - `expression_generated_parity_matches_append_value_order_and_taint` and `expression_generated_parity_matches_merge_field_precedence_and_taint` must build workflows that actually execute the named helper/accessor behavior.
   - Do not use a workflow that only declares an accessor in `WorkflowParts.accessors` while no node/expression references it.

2. Strengthen all generated-vs-runtime parity tests:
   - Repeat: compare generated and runtime/oracle terminal value/error, exact error variant/fields, final pc, slots, taints, step states, attempt counters, and normalized journal signature.
   - Reduce: compare accumulator initialization, item binding, iteration result, output slot, final value, taint, pc, step states, and journal signature.
   - Together: compare fanout/join semantics, branch result order, error policy, slots, taints, step states, and journal signature.
   - Collect: compare page state, duplicate/stale-page behavior, materialized order, capacity errors, slots, taints, step states, and journal signature.
   - Expression/accessor: compare exact `SlotValue`, order, type/error variant and fields, missing-path errors, and taint.
   - Taint: compare exact taint enum values and terminal values.
   - Journal: compare normalized semantic event kind/order/essential fields and exact mismatch dimensions.

3. Do not count static source-shape checks as semantic parity:
   - `compare_generated_to_ir` may remain an additional guard, but it is not enough.
   - Source substring assertions like `source.contains("repeat")`, `source.contains("journal")`, or `source.contains("read_taint")` are not generated-vs-runtime parity.

4. Preserve failing-first behavior correctly:
   - If a required family is unsupported, the failing-first test may fail at source emission with an exact unsupported feature.
   - Once emission exists, the same test must fail on exact runtime/oracle mismatch, not pass because a keyword appeared in generated source.

## Required Commands After Repair

Run and preserve raw output for:

1. `rtk cargo test -p vb_codegen generated_support_matrix_totality -- --nocapture`
2. `rtk cargo test -p vb_codegen repeat_generated_parity -- --nocapture`
3. `rtk cargo test -p vb_codegen reduce_generated_parity -- --nocapture`
4. `rtk cargo test -p vb_codegen together_generated_parity -- --nocapture`
5. `rtk cargo test -p vb_codegen collect_generated_parity -- --nocapture`
6. `rtk cargo test -p vb_codegen expression_generated_parity -- --nocapture`
7. `rtk cargo test -p vb_codegen generated_taint_parity -- --nocapture`
8. `rtk cargo test -p vb_codegen text_helper_generated_support_or_rejection -- --nocapture`
9. `rtk cargo test -p vb_codegen generated_source_contract -- --nocapture`
10. `rtk cargo test -p vb_codegen journal_signature_generated_parity -- --nocapture`
11. `rtk cargo test -p vb_codegen --test trybuild_tests`

Any required filter returning zero selected tests is automatic rejection. Any failing-first failure caused by an unused fixture, wrong workflow, or source-keyword assertion instead of a real product gap is automatic rejection.

---

# Post-State-10 Rerun Repair Section — STATUS: REJECTED

## Required Repair Target

State 10 holzman implementation repair.

## Mandatory State 10 Repairs

1. Accepted tests must not launder runtime-oracle failures as success:
   - `unsupported primitive: not_yet_implemented` from `vb_core::run_until_blocked` or the runtime oracle is not a passing parity result.
   - If the oracle cannot execute a supported final-IR family, the test must fail or the implementation must restore validation fail-closed behavior and record that the bead is non-closable without an approved blocker/scope decision.
   - A generated stdout prefix like `ok:` is not generated-vs-runtime parity.

2. Supported Repeat/Reduce/Together/Collect require executable parity evidence:
   - Repeat: executable comparison against the oracle for terminal value/error, exact typed error fields, pc, slots, taints, step states, attempt counters, and journal signature.
   - Reduce: executable comparison for accumulator initialization, item binding, iteration, output slot, final value, taint, pc, step states, and journal signature.
   - Together: executable comparison for fanout/join semantics, branch result order, error policy, slots, taints, step states, and journal signature.
   - Collect: executable comparison for page state, duplicate/stale-page behavior, materialized order, capacity errors, slots, taints, step states, and journal signature.
   - If any family cannot meet this, validation must fail closed before emission and the bead remains non-closable unless an approved contract/scope blocker decision changes `POST-002`.

3. Collect support status must not outrun semantics:
   - Do not mark `CollectStart`, `CollectPage`, `CollectNext`, or `CollectFinish` supported while implementation only handles first-page/minimal materialization.
   - Duplicate page, stale page, multi-page pagination, materialization order, capacity, taint, and journal parity must exist before Collect can be counted as supported for bead closure.

4. Source-owner checks are not parity evidence:
   - `include_str!("tests.rs")` plus `source.contains("fn <owner>")` may be a bookkeeping guard only.
   - It cannot satisfy Repeat/Reduce/Together/Collect runtime parity obligations.
