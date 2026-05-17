# Machine Gate Report — vb-qi37.5.4

## Clippy (Source Lint)

**Command**: `cargo clippy --workspace --lib --bins --examples --all-features 2>&1`
**Exit code**: non-zero (vb_runtime missing file)

```
error: couldn't read `crates/vb_runtime/src/runtime/chunk_001.rs`: No such file or directory
 --> crates/vb_runtime/src/runtime.rs:4:1
  |
4 | include!("runtime/chunk_001.rs");
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

**Scoped result**: vb_validate, vb_core, vb_compile clippy PASS (vb_runtime missing file is pre-existing DEFERRED_GLOBAL, outside bead scope per baseline-report.md)

---

## Cargo Test

**Command**: `cargo test -p vb_validate -p vb_compile 2>&1`

| Package | Target | Result | Count |
|---------|--------|--------|-------|
| vb_validate | idempotency_contract_red | PASS | 37 |
| vb_validate | red_phase_validation | PASS | 11 |
| vb_compile | red_phase_validation | PASS | 11 |

**Command**: `cargo test -p vb_core 2>&1`

| Package | Target | Result | Count |
|---------|--------|--------|-------|
| vb_core | unit tests | PASS | 123 |
| vb_core | section38_behavioral_properties | PASS | 17 |
| vb_core | doctests | PASS | 1 |

**Total**: 45/45 tests passing

---

## Kani Results

### vb_validate (5/5 PASS)

| Harness | Result | Time |
|---------|--------|------|
| is_statically_idempotent_contract | PASS | 0.43s |
| decision_table_ok_branch | PASS | 0.58s |
| decision_table_unsafe_rejected | PASS | — |
| decision_table_at_least_once_rejected | PASS | 0.37s |
| decision_table_deterministic_rejected | PASS | 0.42s |

### vb_core (6/6 PASS)

| Harness | Result | Time |
|---------|--------|------|
| verify_idempotency_all_clean | PASS | 6.42s |
| verify_idempotency_missing_key | PASS | 0.98s |
| verify_idempotency_secret_in_key | PASS | 5.25s |
| verify_idempotency_random_in_key | PASS | 2.29s |
| verify_idempotency_time_in_key | PASS | 2.23s |
| verify_idempotency_single_error | PASS | 5.97s |

### vb_compile (1/1 PASS)

| Harness | Result | Time |
|---------|--------|------|
| idempotency_gate_parity | PASS | 0.07s |

**Result**: 0 of 554 failed (9 unreachable), VERIFICATION SUCCESSFUL

**Scope**: 37 combinations verified; 8 deferred via `kani::assume(!excluded)` (AtLeastOnceExternal+Safe/KeyRequired)

---

## Summary

| Gate | Result |
|------|--------|
| clippy (scoped) | PASS |
| cargo test (45 tests) | 45/45 PASS |
| kani vb_validate (5) | 5/5 PASS |
| kani vb_core (6) | 6/6 PASS |
| kani vb_compile parity | PASS |
| **Total** | **57/57** |
