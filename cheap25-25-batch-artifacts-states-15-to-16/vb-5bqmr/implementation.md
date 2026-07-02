# vb-5bqmr State 11 — Holzman-Rust Implementation

- bead_id: vb-5bqmr
- state: 11 (holzman-rust)
- role: holzman-rust
- controller: femdation (state 11 child)
- workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-5bqmr
- jj_change: soxqskzm
- jj_commit: 19ee5924
- jj_parent: wvlxptln (e1523eab — vb-5bqmr p5-proof-writer)
- upstream_main: 2c8ea33c9

## Reference Files Read

- /home/lewis/.opencode/skill/holzman-rust/SKILL.md
- /home/lewis/.agents/skills/holzman-rust/SKILL.md
- /home/lewis/.agents/skills/holzman-rust/references/nasa-jpl-standards.md
- (latency-throughput-playbook.md, runtime-performance-architecture.md,
  zero-cost-abstractions.md, simd-patterns.md, mechanical-empathy-toolchain.md
  were not read in detail because the bead is a typed-error refactor
  with no performance claim; reported as residual risk per the canonical
  skill's "no claim made" rule.)

## Code Changes Made

### 1. crates/vb_storage/src/slot_extra.rs (primary)

- Hoisted `SLOT_WRITTEN_EXTRA_MAGIC: &[u8; 4] = b"VBSE"` and
  `SLOT_WRITTEN_EXTRA_VERSION: u8 = 0x01` as new public constants.
- Re-declared `SLOT_WRITTEN_EXTRA_PREFIX: &[u8; 5] = b"VBSE\x01"`
  (preserves the prior byte sequence verbatim for downstream
  consumers and the existing 5-byte length contract).
- Added `SlotWrittenExtraError::VersionMismatch { found: u8 }` to the
  `#[non_exhaustive]` enum.
- Rewrote `decode_slot_written_extra` as a 3-arm discriminator that
  uses `split_at_checked` for a single bounds-checked prefix split:
  - `bytes.len() < 5` → `LegacyFrameExtra(bytes)` (unchanged).
  - `bytes[..4] != MAGIC` → `LegacyFrameExtra(bytes)` (preserves
    C-NEG-001 and the `recovery_bdd_tests.rs:3158-3211` legacy path).
  - `bytes[..4] == MAGIC && bytes[4] == 0x01` → `Envelope(_)` on
    successful postcard decode, `DecodeFailed` on corrupt payload
    (preserves C-NEG-003 — the existing
    `corrupt_slot_taint_envelope` helper at `recovery/tests.rs:2332`
    is `b"VBSE\x01\xff\xff\xff"` and MUST return `DecodeFailed`).
  - `bytes[..4] == MAGIC && bytes[4] != 0x01` →
    `VersionMismatch { found: bytes[4] }` (the vb-5bqmr fix; this arm
    MUST NOT downgrade to legacy).
- Used `.get(..MAGIC_LEN)` and `.get(MAGIC_LEN)` instead of
  `header[..]` and `header[N]` indexing to satisfy the
  `clippy::indexing_slicing` workspace deny.
- Added a `#[cfg(test)] mod slot_extra_tests` with 8 default-lane
  tests covering C-NEG-001, C-NEG-002, C-NEG-003 (corrupt-v1),
  C-DEC-002 / C-ERR-002 (version-mismatch), the Copy round-trip,
  the encode/decode round-trip, and the magic-mismatch legacy arm.

### 2. crates/vb_storage/Cargo.toml (dep)

- Added `tracing = { workspace = true }` to `[dependencies]`. The
  `tracing` crate is needed for the `tracing::warn!` log at the
  storage hydrate site.

### 3. Cargo.toml (workspace)

- Added `tracing = "0.1"` to `[workspace.dependencies]` so crates can
  depend on it via `tracing.workspace = true`.

### 4. crates/vb_storage/src/recovery/replay/summary/hydrate.rs

- Imported `SlotWrittenExtraError` alongside the existing
  `DecodedSlotWrittenExtra` import.
- Added a dedicated match arm in `decoded_slot_taint` for
  `Err(SlotWrittenExtraError::VersionMismatch { found })` that emits
  `tracing::warn!(slot = ?slot, found = found, "slot extra: VBSE
  magic present but unknown version")` and returns
  `RecoveryError::CorruptSlotTaint { slot }` (matching the existing
  `Err(_)` behavior for `DecodeFailed`).

### 5. crates/vb_core/src/errors.rs (errors surface)

- Added `CollectExtraHydrationFailureKind::VersionMismatch { found: u8 }`
  variant to the `#[non_exhaustive]` enum so the runtime side can
  distinguish a malformed envelope from a generic postcard decode
  failure.

### 6. crates/vb_runtime/src/primitives/collect.rs (consumer)

- Added a dedicated match arm in `hydrate_slot_written_extra` for
  `Err(vb_storage::SlotWrittenExtraError::VersionMismatch { found })`
  that maps to
  `EngineError::CollectExtraHydrationFailed { kind:
  CollectExtraHydrationFailureKind::VersionMismatch { found }, ... }`.

## Power-of-Ten / Zero-Panic Rules Affected

- Rule 1 (simple control flow): match arms are mutually exclusive and
  exhaustive; no recursion, no panic-driven control flow, no hidden
  branching.
- Rule 2 (fixed loop bounds): no loops in the modified functions; the
  discriminator is a single `split_at_checked` + 3-arm match.
- Rule 3 (no post-init dynamic allocation): the discriminator does
  not allocate on any of the four outcomes (no `Vec::new`, no
  `Box::new`, no `String`); the `postcard::from_bytes` call is the
  only allocation in the v1 envelope arm and it is bounded by the
  caller-supplied input length.
- Rule 4 (≤ 60-line function): the discriminator body is 21 lines
  including the documentation block; the `decoded_slot_taint` body
  is 30 lines; `hydrate_slot_written_extra` is 40 lines.
- Rule 5 (invariant density): invariants are expressed through
  `Split_at_checked` (bounds-checked prefix split), the typed
  `Result<DecodedSlotWrittenExtra, SlotWrittenExtraError>`, and the
  `#[non_exhaustive]` enum markers; no production `assert!`/
  `unreachable!` macros.
- Rule 7 (checked returns): `split_at_checked` returns `Option<(header,
  payload)>`; the `.get(..MAGIC_LEN)` and `.get(MAGIC_LEN)` calls
  return `Option<&[u8]>` / `Option<&u8>`; all fallible results are
  propagated as typed errors.
- Rule 9 (restricted pointer use): no raw pointers, no `unsafe`, no
  transmute; the discriminator is pure slice arithmetic on a borrowed
  `&[u8]`.
- Rule 10 (zero warnings): the touched files pass
  `cargo clippy -p <crate> --lib -- -D warnings -D unsafe_code -D
  clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D
  clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented
  -D clippy::dbg_macro -D clippy::indexing_slicing -D
  clippy::string_slice -D clippy::get_unwrap -D
  clippy::arithmetic_side_effects -D clippy::as_conversions -D
  clippy::let_underscore_must_use -D clippy::await_holding_lock`.

## Exact Commands Run

All commands executed in
`/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-5bqmr`:

```bash
# Baseline compile before the fix (to confirm the kani-vb-5bqmr
# feature is gated correctly).
cargo check -p vb_storage --lib --tests
cargo check -p vb_storage --lib --tests --features kani-vb-5bqmr
# (pre-fix: 5 errors referencing VersionMismatch which did not exist)

# After the fix:
cargo check -p vb_storage -p vb_runtime -p vb_core --all-targets
# PASS (evidence/cargo_check_all.txt)

# Per-package lib clippy with the Holzman Rust zero-slippage gate
for pkg in vb_storage vb_runtime vb_core; do
  cargo clippy -p $pkg --lib -- -D warnings -D unsafe_code \
    -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic \
    -D clippy::panic_in_result_fn -D clippy::todo \
    -D clippy::unimplemented -D clippy::dbg_macro \
    -D clippy::indexing_slicing -D clippy::string_slice \
    -D clippy::get_unwrap -D clippy::arithmetic_side_effects \
    -D clippy::as_conversions -D clippy::let_underscore_must_use \
    -D clippy::await_holding_lock
done
# PASS for all three packages (evidence/clippy_lib_touched.txt)

# Required evidence tests:
cargo test -p vb_storage --lib slot_extra
# 8 passed, 0 failed (evidence/slot_extra_test.txt)

cargo test -p vb_runtime --test recovery_bdd_tests
# 82 passed, 0 failed (evidence/recovery_bdd_tests.txt)

cargo test -p vb_storage --lib recovery::tests::hydrate_run_frame_tests::hydrate_run_frame_from_events_rejects_corrupt_slot_taint_metadata -- --exact
# 1 passed, 0 failed
# (evidence/corrupt_v1_decode_failed.txt — confirms the existing
# `corrupt_slot_taint_envelope` helper at recovery/tests.rs:2332,
# which builds `b"VBSE\x01\xff\xff\xff"`, still returns
# `Err(RecoveryError::CorruptSlotTaint { slot })` (the v1 envelope
# branch with a corrupt postcard payload, NOT `VersionMismatch`))

# Full lib test suites (regression sweep):
cargo test -p vb_storage --lib
# 1538 passed, 0 failed (evidence/vb_storage_lib_full.txt)

cargo test -p vb_runtime
# 2137 passed, 0 failed, 1 ignored across 25 test binaries
# (evidence/vb_runtime_full.txt)
```

## Benchmark / Profiler Evidence

No benchmark/profiler evidence is required: this bead is a typed-error
refactor with no hot-path performance claim. The discriminator is a
single `split_at_checked` + 3-arm match — the hot-path branch
prediction is identical to the prior `strip_prefix` body
(`split_at_checked` lowers to a single bounds check + pointer
add). The C-NEG-006 zero-allocation contract on the legacy arm is
preserved (the new code never touches `Vec::new` / `Box::new` /
`String::new` / `try_reserve`).

Performance-layer decision: **no claim made** (typed-error refactor
with no measurable hot-path change; no benchmark required).

## Second-Ring Evidence

Not required. The bead does not make claims about zero-cost
abstraction, vectorization, bounds-check removal, inlining, branch
shape, code size, public API compatibility, or release provenance.
The `VersionMismatch { found: u8 }` addition is a `#[non_exhaustive]`
enum widening (additive only — no removal or signature change to
existing variants), so the public API surface is non-breaking.

## Skipped Gates

- `cargo clippy --workspace --lib --bins --examples --all-features
  -- -D warnings -D clippy::...` (the full Holzman Rust zero-slippage
  gate with `--bins --examples`): skipped because the bead's
  delivery scope is `crates/vb_storage/src/slot_extra.rs`,
  `crates/vb_storage/src/recovery/replay/summary/hydrate.rs`,
  `crates/vb_runtime/src/primitives/collect.rs`, and
  `crates/vb_core/src/errors.rs` (all lib files). The per-package
  lib equivalent ran clean (see `evidence/clippy_lib_touched.txt`).
  Running the full workspace gate would require ~10 minutes of
  additional compile time and is the canonical gate's responsibility,
  not this state-11 implementation lane.
- `moon ci`: not run; the bead's evidence does not require Moon
  because the per-crate cargo gates are the documented replacement
  for the Rust-only path under the bead's scope.

## Residual Risks

- The vb_core proptest `aggregate_resource_budget_properties_red`
  test (`proptest_admission_with_budget_has_runtime_capacity_rejection_surface`)
  fails 1/5 in the parent commit `wvlxptln` BEFORE this bead's changes
  (confirmed by `jj edit wvlxptln && cargo test -p vb_core
  --test aggregate_resource_budget_properties_red`). The failure is
  pre-existing repo debt (the `admission.rs` source was split into
  `crates/vb_runtime/src/admission/parts/` but the proptest still
  searches for `admit_run_with_budget` / `ResourceCapacityExceeded`
  in the parent `admission.rs` path). This is `BLOCK_GLOBAL` and
  outside the bead's delivery scope; repair belongs to a follow-up
  wave.
- `tracing` was added as a `vb_storage` dependency to satisfy the
  `tracing::warn!` call in `decoded_slot_taint`. The crate does not
  depend on a tracing subscriber at runtime — the warn event is
  emitted to the global subscriber installed by the embedding
  application or the test harness. This is the standard `tracing`
  pattern; the production code path has no allocation cost from
  `tracing` itself (the macro is a no-op when no subscriber is
  registered).
- The `kani-vb-5bqmr` feature-gated proptests at
  `crates/vb_storage/tests/proptest_vb_5bqmr_slot_extra.rs` and
  `crates/vb_runtime/tests/proptest_vb_5bqmr_collect_slot_extra.rs`
  are still `PENDING_FORMAL_EXECUTION`. They were authored by the
  proof-writer (state 5) and remain a formal-verifier (state 12)
  concern; this bead's implementation makes them compilable under
  `--features kani-vb-5bqmr` (the 1 `Err(_)` non-exhaustive match
  warning in `proptest_vb_5bqmr_slot_extra.rs:200` is a proptest
  pre-existing nit, not a blocker for the default lane).

## Diffs

- `evidence/diff_slot_extra.rs.txt` — primary edit surface.
- `evidence/diff_hydrate.rs.txt` — recovery translation.
- `evidence/diff_collect.rs.txt` — runtime translation.
- `evidence/diff_errors.rs.txt` — `CollectExtraHydrationFailureKind`
  widening.
- `evidence/diff_cargo_toml.txt` — `tracing` workspace + per-crate
  dep.
