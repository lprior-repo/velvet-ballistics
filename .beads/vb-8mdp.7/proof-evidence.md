# Proof Evidence — vb-8mdp.7 Invocation Supersession

**State**: 5 | **Date**: 2026-05-30

## Verus

```
$ verus --crate-type=lib verification/verus/collect_lowering.rs
verification results:: 6 verified, 0 errors
```

## Proptest — vb_core (budget)

```
$ cargo test -p vb_core budget_vb_8mdp_7
test result: ok. 16 passed; 0 failed; 0 ignored; 2575 filtered out
```

## Proptest — vb_runtime (admission)

```
$ cargo test -p vb_runtime admission_vb_8mdp_7
test result: ok. 22 passed; 0 failed; 0 ignored; 1915 filtered out
```

## Proptest — vb_compile (collect lowering)

```
$ cargo test -p vb_compile --test vb_8mdp_7_collect_lowering_props
test result: ok. 15 passed; 0 failed; 0 ignored
```

## Integration — workspace_tests (resource admission)

```
$ cargo test -p velvet-ballistics-workspace-tests --test vb_8mdp_7_resource_admission_props
test result: ok. 21 passed; 0 failed; 0 ignored
```

## Kani — BLOCKED_TOOLING

45 pre-existing compilation errors in crate-wired Kani harnesses. ICE at hooks.rs:158.

## Flux — BLOCKED_TOOLING

Package smoke passes but no per-function refinements exist for vb-8mdp.7 obligations.

## TLA+ — DROPPED

Per controller directive. Prior TLC evidence preserved in evidence/tlc-collect-body-model.log.
