# Proof Evidence: vb-zioy

## PO-003 Evidence
```
cargo test -p vb_compile --test v1_primitive_lowering compile_workflow_rejects_multi_step_body_in_scoped_primitives
running 1 test
test compile_workflow_rejects_multi_step_body_in_scoped_primitives ... ok
test result: ok. 1 passed
```

## PO-004 Evidence
```
cargo test -p vb_compile --test v1_primitive_lowering
running 20 tests
... all passed ...
test result: ok. 20 passed
```

## PO-005 Evidence
```
cargo check -p vb_compile
    Finished dev [unoptimized + debuginfo] target(s) in 0.44s
grep -n 'emit_single_body_set' crates/vb_compile/src/mod_compile_lowering/*.rs
part_02.rs:192
part_03.rs:135
part_03.rs:195
part_04.rs:52
part_04.rs:118
part_04.rs:221
```

## PO-001/PO-002 Blocked
Proptest modules `proptest_body_dispatcher.rs` and `proptest_error_parity.rs` exist but are not declared in `lib.rs`.
Compensating evidence: integration tests cover same error paths.
