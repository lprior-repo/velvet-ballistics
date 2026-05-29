# Proof Evidence — vb-e5l9n

## Commands run

### Initial focused Kani command after first repair attempt

Command:

```bash
TMPDIR="$PWD/target/kani-tmp" env -u RUSTC_WRAPPER RUSTFLAGS="-Dwarnings" timeout 5m rustup run nightly-2026-04-28 cargo kani --lib -p vb_core --all-features --harness kani_budget_add_dim_zero --quiet
```

Result: FAIL, exit status 101.

Output summary:

```text
Compiling vb_core v0.1.0 (/home/lewis/src/velvet-ballistics/crates/vb_core)
error: unused import: `CodeEntry`
  --> crates/vb_core/src/kani/kani_registry_bijection.rs:11:59
   |
11 | use super::kani_symbolic_code_validation::{CODE_REGISTRY, CodeEntry};
   |                                                           ^^^^^^^^^
   |
   = note: `-D unused-imports` implied by `-D warnings`
error: could not compile `vb_core` (lib) due to 1 previous error
error: Failed to execute cargo (exit status: 101). Found 1 compilation errors.
```

Classification: local Kani harness compile warning promoted to error. Fixed by removing the unused import.

### Focused Kani command after unused import repair

Command:

```bash
TMPDIR="$PWD/target/kani-tmp" env -u RUSTC_WRAPPER RUSTFLAGS="-Dwarnings" timeout 5m rustup run nightly-2026-04-28 cargo kani --lib -p vb_core --all-features --harness kani_budget_add_dim_zero --quiet
```

Result: PASS, exit status 0.

Output summary:

```text
Compiling vb_core v0.1.0 (/home/lewis/src/velvet-ballistics/crates/vb_core)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.75s
```

### Canonical Moon Kani task before formatting

Command:

```bash
moon run velvet-ballistics:verify-kani
```

Result: PASS, exit status 0.

Output summary:

```text
velvet-ballistics:verify-kani (6s 709ms, 419fb5c7)
Tasks: 1 completed
Time: 6s 731ms
```

### Formatting

Command:

```bash
rtk cargo fmt --package vb_core
```

Result: PASS, exit status 0.

Output: no output.

### Final focused Kani command plus canonical Moon Kani task

Command:

```bash
TMPDIR="$PWD/target/kani-tmp" env -u RUSTC_WRAPPER RUSTFLAGS="-Dwarnings" timeout 5m rustup run nightly-2026-04-28 cargo kani --lib -p vb_core --all-features --harness kani_budget_add_dim_zero --quiet && moon run velvet-ballistics:verify-kani
```

Result: PASS, exit status 0.

Output summary:

```text
Compiling vb_core v0.1.0 (/home/lewis/src/velvet-ballistics/crates/vb_core)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.81s
velvet-ballistics:verify-kani (7s 390ms, cb2cb0f3)
Tasks: 1 completed
Time: 7s 410ms
```

## Assumptions / bounds / trust notes

- Existing harness unwind annotations were preserved.
- No Kani stubs or contracts were added.
- No `unsafe` was added.
- No hardcoded dummy proof shape was introduced; constructor changes use registry-sourced static symbolic strings.
- The serde mirror now scans `CODE_REGISTRY` to recover the registered static string before constructing `SymbolicCode`, avoiding borrowed data escaping.
- Evidence here proves the reported diagnostic harness compilation blockers are cleared for the commands run; it is not a full per-harness CBMC proof-success claim.
