# Machine Gate Report — vb-qi37.1.4

STATUS: PASS

## Gate Results

### 1. cargo clippy --workspace --lib --bins --examples --all-features -D warnings

```
cargo clippy: No issues found
Exit: 0
```

**Result: PASS**

### 2. cargo test -p vb_runtime -- recovery --nocapture

```
cargo test: 14 passed, 1430 filtered out (9 suites, 0.00s)
Exit: 0
```

**Result: PASS**

### 3. cargo test -p vb_storage --lib -- recovery --nocapture

```
cargo test: 129 passed, 798 filtered out (1 suite, 0.13s)
Exit: 0
```

**Result: PASS**

### 4. cargo test -p vb_storage --test recovery_integration -- --nocapture

```
cargo test: 16 passed (1 suite, 0.14s)
Exit: 0
```

**Result: PASS**

### 5. cargo kani --package vb_storage --no-default-features

```
error[E0277]: the trait bound `recovery::types::RecoveryFrameSeed: kani::Arbitrary` is not satisfied
   --> crates/vb_storage/src/kani_codec.rs:202:28
    |
202 |     let seed = kani::any::<RecoveryFrameSeed>();
    |                            ^^^^^^^^^^^^^^^^^ unsatisfied trait bound
error: could not compile `vb_storage` (lib) due to 1 previous error
Exit: 1
```

**Result: FAIL_LOCAL** — KANI-CODEC harness requires `kani::Arbitrary` implementation for `RecoveryFrameSeed`

## Tool Availability

| Tool | Status |
|------|--------|
| cargo | Available |
| clippy | Available (no issues) |
| cargo test | Available (152 recovery tests pass) |
| cargo kani | Available (0.67.0) — harness compilation error |
| rustc | Available |

## Residual Risk

- KANI-CODEC (VERUS-GAP3 related) fails due to missing `kani::Arbitrary` trait on `RecoveryFrameSeed`
- This is a bead-local issue requiring a harness fix, not a proof failure
- Waivers exist for VERUS-GAP3-001 and VERUS-GAP3-002 (WAIVER-GAP3-ABI)
