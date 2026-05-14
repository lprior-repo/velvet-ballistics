# Machine Gate Report — vb-qi37.16.3 (State 8)

**Bead:** vb-qi37.16.3
**Phase:** State 8 machine gates after State 3/4 TLA repair and State 6 no-change validation
**Date:** 2026-05-11

---

## STATUS: PASS_WITH_DEFERRED_GLOBAL

## Commands

### Global format sensor

```bash
rtk cargo fmt -- --check
```

**Result:** FAIL. `rustfmt --check` reported formatting diffs in files outside the vb-qi37.16.3 delivery scope, including:

- `crates/vb_core/src/engine/expr_eval/kani_stack.rs`
- `crates/vb_core/src/ids/kani_id_bounds.rs`
- `crates/vb_core/src/kani_expr_bound.rs`
- `crates/vb_expr/src/lexer/miri_tests.rs`
- `crates/vb_expr/src/parser/miri_tests.rs`
- `crates/vb_proof_kernels/src/envelope_header.rs`
- `crates/vb_storage/src/codec_miri_tests.rs`
- `fuzz/fuzz_targets/decode_record.rs`
- `xtask/src/main.rs`
- `xtask/src/proof.rs`

Primary category: `FORMAT`.

### Bead-scoped retry red-phase suite

```bash
rtk cargo test -p vb_runtime --test durable_retry_red_phase
```

Result:

```text
cargo test: 9 passed (1 suite, 0.00s)
```

### Bead-scoped runtime library suite

```bash
rtk cargo test -p vb_runtime --lib
```

Result:

```text
cargo test: 1337 passed (1 suite, 0.31s)
```

### Quick sensor

```bash
moon run :quick
```

Result:

```text
velvet-ballastics:quick ... Tasks: 1 completed
Time: 41s 895ms
```

### Test sensor

```bash
moon run :test
```

Result:

```text
velvet-ballastics:test | Starting 9860 tests across 58 binaries
velvet-ballastics:test | Summary [  12.274s] 9860 tests run: 9860 passed, 0 skipped
Tasks: 4 completed (1 cached)
Time: 24s 98ms
```

## Classification

The bead-local and test sensors pass. The only red gate observed is global `FORMAT` in files outside the vb-qi37.16.3 delivery scope. Classified in `regression-diff.md` as `DEFERRED_GLOBAL`, not bead-local pass.
