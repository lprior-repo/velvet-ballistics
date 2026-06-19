# vb-11ti1: runtime: fix 5 lint-src failures (as_conversions and arithmetic_side_effects in vb_runtime)

## Scope (per bead prompt)
- ONLY touch `crates/vb_runtime/src/`.
- Fix `clippy::as_conversions` and `clippy::arithmetic_side_effects` violations.

## Reference files read
- `/home/lewis/.opencode/skill/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/SKILL.md`
- `/home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md`
- `/home/lewis/.agents/skills/holzman-rust/references/zero-cost-abstractions.md`

## Sites found (from `clippy -p vb_runtime --all-features --lib --bins --examples -- -D warnings -D clippy::as_conversions -D clippy::arithmetic_side_effects`)
Total 10 errors split across 2 source files.

### `crates/vb_runtime/src/shard/lru_ring.rs`
| Site | Lint | Description |
|------|------|-------------|
| 187:31 | `as_conversions` | `now.get() as i128` |
| 187:53 | `as_conversions` | `self.ttl_ticks as i128` |
| 190:13 | `as_conversions` | `(now.get() - self.ttl_ticks) as i128` |
| 190:13 | `arithmetic_side_effects` | `now.get() - self.ttl_ticks` |
| 193:16 | `as_conversions` | `ts.get() as i128` |

### `crates/vb_runtime/src/shutdown_cas.rs`
| Site | Lint | Description |
|------|------|-------------|
| 81:34 | `as_conversions` | `ShutdownPhase::Idle as u8` |
| 103:13 | `as_conversions` | `ShutdownPhase::Idle as u8` |
| 104:13 | `as_conversions` | `ShutdownPhase::ShuttingDown as u8` |
| 127:17 | `as_conversions` | `ShutdownPhase::ShuttingDown as u8` |
| 128:17 | `as_conversions` | `ShutdownPhase::Shutdown as u8` |

Note: bead description quoted "4 as_conversions and 1 arithmetic_side_effects"
but the current state was 9 as_conversions + 1 arithmetic_side_effects.
Bead title "5 lint-src failures" matches the LRU ring alone; the
shutdown_cas.rs sites are within scope (still `as_conversions`/`arithmetic_side_effects`,
still in `crates/vb_runtime/src/`). All 10 fixed to make the `lint-src`
gate exit 0.

## Fix applied

### `crates/vb_runtime/src/shutdown_cas.rs`
Use the existing safe accessor `ShutdownPhase::as_u8()` instead of `Variant as u8`.

```rust
// Before                              // After
ShutdownPhase::Idle as u8              ShutdownPhase::Idle.as_u8()
ShutdownPhase::ShuttingDown as u8      ShutdownPhase::ShuttingDown.as_u8()
ShutdownPhase::Shutdown as u8          ShutdownPhase::Shutdown.as_u8()
```

### `crates/vb_runtime/src/shard/lru_ring.rs`
Rewrite `sweep_expired` boundary check to use `u64::checked_sub` and
lossless widening `i128::from(u64)` `From` conversion. The conversion is
provably lossless because `i128::MAX >= u64::MAX`.

```rust
// Before                                          // After
let cutoff: i128 = if (now.get() as i128)          let cutoff: i128 = match
                  < (self.ttl_ticks as i128) {        now.get().checked_sub(self.ttl_ticks) {
    -1                                              Some(value) => i128::from(value),
} else {                                            None => -1,
    (now.get() - self.ttl_ticks) as i128        };
};
if (ts.get() as i128) <= cutoff {                 if i128::from(ts.get()) <= cutoff {
```

`i128::from(u64)` is a `From` trait impl, not an `as` cast, so it does
not trigger `clippy::as_conversions`. The `checked_sub` returns `None`
exactly when `now.get() < self.ttl_ticks`, so the previous semantics are
preserved exactly.

## Commands run

| Command | Exit code |
|---------|-----------|
| `cargo clippy -p vb_runtime --all-features --all-targets -- -D warnings -D clippy::as_conversions -D clippy::arithmetic_side_effects` (initial enumeration) | 101 (10 errors in scope, 7 out of scope in proptest files) |
| `cargo check -p vb_runtime --all-features --all-targets` (after first edit) | 0 |
| `cargo clippy ...` (after first edit) | 101 (1 remaining `arithmetic_side_effects` for the unchecked subtract) |
| `cargo check -p vb_runtime --all-features --all-targets` (after `checked_sub` edit) | 0 |
| `cargo clippy -p vb_runtime --all-features --lib --bins --examples -- -D warnings -D clippy::as_conversions -D clippy::arithmetic_side_effects` (lint-src style — exact match to `.moon/tasks/all.yml` except workspace flag) | 0 (No issues found) |
| `cargo clippy -p vb_runtime --all-features --lib --bins --examples -- [full lint-src flags from .moon/tasks/all.yml]` (exact lint-src invocation) | 0 (No issues found) |
| `cargo test -p vb_runtime --all-features --no-run` | 0 |
| `cargo check --workspace --all-targets --all-features` | 0 (11 crates compiled) |
| `cargo test -p vb_runtime --all-features --lib lru_ring` | 0 (3 passed) |
| `cargo test -p vb_runtime --all-features --lib shutdown` | 0 (47 passed) |

## Note on `--all-targets` vs `--lib --bins --examples`
The actual `lint-src` task in `.moon/tasks/all.yml` uses
`cargo clippy --workspace --lib --bins --examples` (no `--tests`).
With that scope, all `as_conversions` and `arithmetic_side_effects`
errors in `vb_runtime/src/` are eliminated by the fixes above.

`--all-targets` additionally lints the proptest harnesses in
`crates/vb_runtime/src/verification/proptest/`, which fail on
`clippy::redundant_pattern_matching`, `clippy::absurd_extreme_comparisons`,
`clippy::explicit_counter_loop`, and `clippy::needless_range_loop`.
None of these are `as_conversions` or `arithmetic_side_effects`, and
they are out of the bead scope (different lints, different files).
Verified pre-existing by stashing the patch and re-running the test —
the same 4 `chunk_012/020/021` test failures appear, but they are in
`shard::tests::*` test files, not in the production files I touched.
The proptest lint failures predate this bead.

## Final status
PASS — `clippy::as_conversions` and `clippy::arithmetic_side_effects`
are eliminated from `crates/vb_runtime/src/`. The `lint-src` gate
(`.moon/tasks/all.yml` invocation) exits 0 with "No issues found".

## Residual risk
1. The `--all-targets` lint run still fails on 7 errors in
   `crates/vb_runtime/src/verification/proptest/{proptest_attempt_fence.rs,mod.rs}`
   that are different lints (`redundant_pattern_matching`,
   `absurd_extreme_comparisons`, `explicit_counter_loop`,
   `needless_range_loop`). They predate this bead and are out of scope.
   Follow-up: a separate bead for proptest harness cleanup.
2. The `cargo test -p vb_runtime` run has 4 pre-existing test
   failures in `shard::tests::{chunk_012,chunk_020,chunk_021}` that
   expect `max_terminal_runs: 16` but the runtime returns
   `100000` (`DEFAULT_MAX_TERMINAL_RUNS`). These predate this bead
   (verified by `git stash`) and are out of scope.
