# Proof → Implementation Bridge Input — vb-bc33k

This is the planner handoff for the proof-to-implementation bridge and the
implementation engineer. It maps each planned Verus/Kani/proptest claim to
the exact production Rust source it must be bound to.

## Rust Source Anchors

| Claim | Source File | Lines | Function |
|---|---|---|---|
| exec_expect_bool     | crates/vb_expr/src/eval/type_enforcers.rs | 9-17  | expect_bool |
| exec_expect_i64      | crates/vb_expr/src/eval/type_enforcers.rs | 19-27 | expect_i64 |
| exec_expect_symbol   | crates/vb_expr/src/eval/type_enforcers.rs | 29-37 | expect_symbol |
| exec_expect_list     | crates/vb_expr/src/eval/type_enforcers.rs | 39-47 | expect_list |
| exec_expect_object   | crates/vb_expr/src/eval/type_enforcers.rs | 49-57 | expect_object |

## Independent Behavior Tests

| Test | File | Type | Cases |
|---|---|---|---|
| proptest_expect_bool_iff_bool     | crates/vb_expr/tests/proptest_type_enforcer.rs | proptest | 4096 |
| proptest_expect_i64_iff_i64       | crates/vb_expr/tests/proptest_type_enforcer.rs | proptest | 4096 |
| proptest_expect_symbol_iff_symbol | crates/vb_expr/tests/proptest_type_enforcer.rs | proptest | 4096 |
| proptest_expect_list_iff_list     | crates/vb_expr/tests/proptest_type_enforcer.rs | proptest | 4096 |
| proptest_expect_object_iff_object | crates/vb_expr/tests/proptest_type_enforcer.rs | proptest | 4096 |
| proptest_partition_cover_all      | crates/vb_expr/tests/proptest_type_enforcer.rs | proptest | 16384 |

## Kani Harness References

| Harness | File | Spec Function |
|---|---|---|
| kani_exec_expect_bool_iff_bool     | crates/vb_expr/src/verification/kani/type_enforcer_arbitrary.rs | spec_expect_bool |
| kani_exec_expect_i64_iff_i64       | crates/vb_expr/src/verification/kani/type_enforcer_arbitrary.rs | spec_expect_i64 |
| kani_exec_expect_symbol_iff_symbol | crates/vb_expr/src/verification/kani/type_enforcer_arbitrary.rs | spec_expect_symbol |
| kani_exec_expect_list_iff_list     | crates/vb_expr/src/verification/kani/type_enforcer_arbitrary.rs | spec_expect_list |
| kani_exec_expect_object_iff_object | crates/vb_expr/src/verification/kani/type_enforcer_arbitrary.rs | spec_expect_object |
| kani_exec_partition_cover_all      | crates/vb_expr/src/verification/kani/type_enforcer_arbitrary.rs | spec_slot_value_partition |

## Required Evidence Commands

```
bash scripts/verify-verus.sh
bash scripts/kani-list.sh vb_expr
cargo kani --harness kani_exec_expect_bool_iff_bool -p vb_expr --features kani-type-enforcer
cargo kani --harness kani_exec_expect_i64_iff_i64 -p vb_expr --features kani-type-enforcer
cargo kani --harness kani_exec_expect_symbol_iff_symbol -p vb_expr --features kani-type-enforcer
cargo kani --harness kani_exec_expect_list_iff_list -p vb_expr --features kani-type-enforcer
cargo kani --harness kani_exec_expect_object_iff_object -p vb_expr --features kani-type-enforcer
cargo kani --harness kani_exec_partition_cover_all -p vb_expr --features kani-type-enforcer
cargo nextest run -p vb_expr type_enforcer
```

## Implementation Rule

The implementation engineer MUST NOT add `#[verifier::external_body]`,
`assume(...)`, or `axiom` to the new `exec_expect_*` bridges. Each bridge
must carry the actual match arms from the production `expect_*` function.
If a lemma cannot be proven with this restriction, the production code must
be fixed (GOD RULE 4).