# Formal Verification Report

**Bead ID:** vb-ypnk
**State:** 11 - formal-verifier verify (pass2)
**Sublane:** formal-verifier
**Date:** 2026-06-05
**Source Checkout:** /home/lewis/src/velvet-ballistics

---

## Execution Summary

| Verifier | Command | Result | Classification |
|----------|---------|--------|----------------|
| Miri | `cargo +nightly miri test -p xtask --lib -- miri_postcard_roundtrip_no_ub` | **PASS** | Test passed |
| Kani | `cargo kani --lib -p xtask --only-codegen` | **PASS** | Codegen succeeded |

---

## 1. Miri Test: PASS

### Command
```bash
cargo +nightly miri test -p xtask --lib -- miri_postcard_roundtrip_no_ub
```

### Output
```
WARNING: Ignoring `RUSTC_WRAPPER` environment variable, Miri does not support wrapping.
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running unittests src/lib.rs (target/miri/x86_64-unknown-linux-gnu/debug/deps/xtask-9458513ccb192eb7)

running 1 test
test evidence::miri_tests::miri_postcard_roundtrip_no_ub ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 67 filtered out; finished in 0.50s
```

### Analysis
Test passed after type annotation fix. The `postcard::from_bytes::<EvidenceBundlePostcard>(&bytes)` annotation allows the compiler to infer the generic type.

### Classification: PASS

---

## 2. Kani Codegen: PASS

### Command
```bash
cargo kani --lib -p xtask --only-codegen
```

### Output
```
Kani Rust Verifier 0.67.0 (cargo plugin)
warning: unused variable: `step_raw`
   --> crates/vb_core/src/replay/kani_harnesses.rs:305:13
    |
305 |         let step_raw: u8 = kani::any();
    |             ^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_step_raw`
    |
warning: Found the following unsupported constructs:
             - TerminatorKind::InlineAsm (3)
             - caller_location (1)
             - catch_unwind (1)
             - foreign function (19)
             - simd_cast (3)
             - simd_reduce_all (1)

warning: Kani currently does not support concurrency. The following constructs will be treated as sequential operations:
             - thread local (replaced by static variable) (8)
             - atomic_xchg (5)
             - atomic_cxchg (45)
             - atomic_cxchgweak (30)
             - atomic_xadd (5)
             - atomic_store (15)
             - atomic_fence (4)
             - atomic_xsub (5)
             - atomic_load (24)

    Finished `dev` profile [unoptimized + debuginfo] target(s) in 10.42s
```

### Analysis
Codegen succeeded. The warnings about unsupported constructs and atomic operations are expected for this codebase and do not prevent verification. No verification was run (--only-codegen), but compilation passed.

### Classification: PASS
Exit status 0, compilation successful.

---

## Ledger Closure

| Obligation | Status |
|------------|--------|
| Miri postcard roundtrip UB test | PASS |
| Kani xtask codegen | PASS |

---

## Verification Status

**APPROVED** - All verification obligations passed.

- Miri test: 1/1 passed (no UB detected)
- Kani codegen: compilation successful

---

## Evidence Artifacts

- Miri test log: captured above
- Kani codegen log: captured above
