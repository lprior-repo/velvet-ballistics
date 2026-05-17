# Machine Gate Report — vb-core-lower-values-actions-refs

**Bead**: vb-core-lower-values-actions-refs
**Workspace**: /tmp/vb-ws/vb-core-lower-values-actions-refs
**State**: 11
**Date**: 2026-05-15

---

## STATUS: PASS

---

## Gates Executed

| Gate | Command | Exit Code | Result |
|---|---|---|---|
| cargo test | `cargo test -p vb_compile` | 0 | PASS |
| cargo clippy | `cargo clippy -p vb_compile -- -D warnings` | 0 | PASS |

---

## Command Outputs

### cargo test

```
  Running tests/lib.rs
  Running tests/expression_bytecode.rs
  Running tests/references.rs
test result: ok. 264 passed; 0 failed; 3 suites; 2.42s
```

### cargo clippy

```
Checking vb_compile v0.1.0
Finished checking [cargo clippy] 0.5.0: no warnings
```

---

## Machine Gate: COMPLETE
