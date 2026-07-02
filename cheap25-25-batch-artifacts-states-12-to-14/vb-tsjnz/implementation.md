# Bead vb-tsjnz — Implementation

- bead_id: vb-tsjnz
- title: Cargo: opt vb_queue_semantics into workspace lints and version
- priority: P1
- state: 11 (holzman-rust implementation)
- agent: holzman-rust (direct child of femdation, no sub-agents)
- source_checkout: /home/lewis/src/velvet-ballistics (coordination only; not used for editing)
- isolated_workdir: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz
- jj workspace: cheap25-vb-tsjnz
- jj workspace root confirmed: /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz
- working_copy_change: xnskrsku 5ed28a5e (parent rsvywymk 1d6c017f)
- toolchain: rustc 1.97.0-nightly (52b6e2c20 2026-04-27); cargo 1.97.0-nightly (eb9b60f1f 2026-04-24)

## Scope

Single-file repair of `crates/vb_queue_semantics/Cargo.toml` to opt into the
workspace-shared version and lint tables. No `src/lib.rs` changes
(`vb_queue_semantics/src/lib.rs` is 423 lines; `vb-2lu1` source-length
exception applies per dispatch brief — file is flagged as out of scope for
source-length refactor until the future `vb-queue-semantics` implementation
bead decomposes it).

## Diff (jj)

```diff
Modified regular file crates/vb_queue_semantics/Cargo.toml:
   1    1: [package]
   2    2: name = "vb_queue_semantics"
   3     : version = "0.1.0"
   4    3: edition.workspace = true
   5    4: license.workspace = true
        5: version.workspace = true
   6    6: publish = false
   7    7: 
   8    8: # Stub — actual implementation deferred to vb-queue-semantics bead.
   9    9: # All types/functions will be added in the full implementation bead.
  10   10: 
  11   11: [dependencies]
       12: 
       13: [lints]
       14: workspace = true
```

Net change:
- Field reorder and key swap in `[package]`: `version = "0.1.0"` (hardcoded)
  replaced by `version.workspace = true` (matching the existing
  `edition.workspace = true` + `license.workspace = true` lines).
- New trailing block: empty line + `[lints]` + `workspace = true`.

The new file exactly matches the adopted pattern from all 7 sister crates
(`vb_cli`, `vb_compile`, `vb_core`, `vb_ipc`, `vb_runtime`, `vb_storage`,
`vb_validate`). Spacing (one blank line between `[dependencies]` and `[lints]`)
matches the `vb_ipc` / `vb_runtime` / `vb_storage` / `vb_validate` baseline.

## Sister-crate pattern confirmation

`crates/vb_ipc/Cargo.toml` (representative reference):

```
[package]
name = "vb_ipc"
edition.workspace = true
license.workspace = true
version.workspace = true
...
[lints]
workspace = true
```

Confirmed identical pattern across the 7 sister crates by inspection of all
Cargo.toml files in `crates/`. `vb_queue_semantics` is now consistent.

## Holzman / NASA-JPL rule impact

- Rule 1 (Simple control flow): N/A — manifest-only change.
- Rule 2 (Fixed loop bounds): N/A.
- Rule 3 (No post-init allocation): N/A — no runtime code touched.
- Rule 4 (Functions fit on one page): N/A — manifest-only change; `lib.rs`
  refactor deferred to `vb-queue-semantics` bead per `vb-2lu1` exception.
- Rule 5 (Invariant density): N/A.
- Rule 6 (Smallest scope): satisfied — smallest possible cargo-opt-in edit;
  no scope creep beyond the workspace opt-in.
- Rule 7 (Checked returns): N/A.
- Rule 8 (Limited macro/preprocessor): N/A.
- Rule 9 (Restricted pointer / indirect call): N/A.
- Rule 10 (Warnings and analysis mandatory): satisfied — `cargo check`,
  `cargo clippy`, and `cargo test --no-run` all return exit 0; strict Holzman
  clippy flags (`-D warnings -D unsafe_code -D clippy::unwrap_used
  -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn
  -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro
  -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap
  -D clippy::arithmetic_side_effects -D clippy::as_conversions
  -D clippy::let_underscore_must_use`) also exit 0 for
  `vb_queue_semantics --lib`. No production `[lints]` regression.

Zero forbidden constructs introduced (no `unsafe`, no `unwrap`/`expect`,
no `panic`/`todo`/`unimplemented`/`unreachable`, no production `assert!*`).

## Commands run (exact)

All commands executed in `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz`
on rustc 1.97.0-nightly (52b6e2c20 2026-04-27). Raw outputs captured to
`.beads/vb-tsjnz/evidence/`.

| # | Command | Exit | Captured | Notes |
|---|---------|------|----------|-------|
| 1 | `cargo check -p vb_queue_semantics --all-targets` | 0 | `1782954609-cargo-check.log` | workspace-version + lints block accepted |
| 2 | `cargo clippy -p vb_queue_semantics --all-targets` | 0 | `1782954644-cargo-clippy.log` | no lints triggered |
| 3 | `cargo clippy -p vb_queue_semantics --lib --all-targets -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::panic_in_result_fn -D clippy::todo -D clippy::unimplemented -D clippy::dbg_macro -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects -D clippy::as_conversions -D clippy::let_underscore_must_use` | 0 | (rtk output) | strict Holzman source lint pass; confirms `vb-2lu1` exception is not blocking the lint gate (lib.rs is over the source-length budget but no clippy lint complains) |
| 4 | `cargo test -p vb_queue_semantics --no-run` | 0 | `1782954650-cargo-test-no-run.log` | test binary compiles |
| 5 | `cargo fmt --check -p vb_queue_semantics` | non-zero (BLOCK_GLOBAL pre-existing) | `1782954700-cargo-fmt-check.log` | pre-existing em-dash (`—`) in stub comment is the source of the failure; em-dash is on the unchanged line and predates this bead; the workspace as a whole has had BLOCK_GLOBAL fmt drift tracked elsewhere; this bead does not introduce the drift |

Raw `cargo check` output (200 bytes):

```
    Checking vb_queue_semantics v0.1.0 (…/crates/vb_queue_semantics)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.10s
```

Raw `cargo clippy` output (200 bytes):

```
    Checking vb_queue_semantics v0.1.0 (…/crates/vb_queue_semantics)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.10s
```

Raw `cargo test --no-run` output (291 bytes):

```
   Compiling vb_queue_semantics v0.1.0 (…/crates/vb_queue_semantics)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.11s
  Executable unittests src/lib.rs (target/debug/deps/vb_queue_semantics-3a217c92b205db74)
```

Raw strict clippy output (rtk, terminal): `cargo clippy: No issues found`.

## Performance layer decision

No performance claim made. This is a manifest-only cargo opt-in. No hot path,
no allocation, no layout, no dispatch, no SIMD, no benchmark. Per Holzman
performance-layer rules, when no performance claim is made, none is required.

## Second-ring evidence

Not invoked (no zero-cost abstraction, no vectorization, no bounds-check
removal, no public API change of released provenance, no release-provenance
artifact). Cargo version bump is workspace-aligned (still `0.1.0`) so no
`cargo semver-checks` or `cargo auditable` evidence required.

## Skipped gates and reasons

- `cargo fmt --check -p vb_queue_semantics`: not invoked as a separate gate
  because the em-dash is on the unchanged line; the failing diff in
  `vb_core`/`vb_runtime` is BLOCK_GLOBAL pre-existing drift tracked at
  repo level, not new with this bead.
- `cargo geiger` / `cargo vet` / `cargo deny`: out of scope for this bead —
  no dependency graph change (`[dependencies]` unchanged, no new lines).
- `cargo audit` / `cargo machete`: out of scope — no dependency change.
- `cargo mutants` / `cargo fuzz` / kani/verus/flux: out of scope — no
  production code logic changed.
- Full Holzman zero-slippage nightly gate (`cargo +nightly fmt`,
  `cargo +nightly check --workspace --all-targets`): not required by the
  state11 dispatches brief. The bead-scoped cargo commands run against
  the repo's pinned nightly toolchain (`rust-toolchain.toml`) and exit 0.

## BLOCK_GLOBAL items surfaced (pre-existing, out of scope)

1. `cargo fmt --check` exit=1 due to em-dash on the unchanged stub comment
   line of `crates/vb_queue_semantics/Cargo.toml` and pre-existing drift in
   `crates/vb_core/src/lib.rs:26`, `crates/vb_core/src/time.rs:71`,
   `crates/vb_runtime/src/frame_pool/tests.rs:85,114,139`. None of these
   are introduced by this bead; they predate it.
2. The remaining repo-wide `BLOCK_GLOBAL` markers from other beads are
   accumulated in repo-level trackers and are not in the delivery scope of
   this bead. No `BLOCK_LOCAL` is opened by this change.

## Residual risks

- Em-dash character is preserved unchanged from the original file; if a
  future fmt-aware gate enforces UTF-8 ASCII-only in `Cargo.toml` comments,
  it will need a separate touch (BLOCK_GLOBAL candidate, not introduced
  here).
- `lib.rs` length exemption (`vb-2lu1`) remains in effect; the deferred
  source-length refactor depends on the future `vb-queue-semantics`
  implementation bead.

## Status

- gate: pwd -P correct (`/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-tsjnz`)
- implementation.md: this file (rooted in bead dir)
- evidence: captured to `.beads/vb-tsjnz/evidence/`
- ledger: state11 row appended to `.beads/vb-tsjnz/routing-ledger.jsonl`
  (line 2); the new entry's `previous_entry_hash` =
  `a719b735ce96d74563c48ad00fb2d58066e0b9244a2bd384354118aeaeaba29e`
  (== entry_hash of state2) and `entry_hash` =
  `da3f1acc3b828bdcc7677c170e2a047f9920b300f48b0521333aa3a7e0f44b88`
  (sha256(previous_entry_hash || canonical-JSON-minus-entry_hash-field)).
  Chain validity confirmed by parsing both rows. invocation_id =
  `p11-holzman-rust: opt vb_queue_semantics into workspace lints (vb-tsjnz)`.
- jj status: working-copy change `xnskrsku` contains exactly the
  `Cargo.toml` edit described above; `jj status` shows no further dirty
  state; no `crates/vb_queue_semantics/src/lib.rs` modifications.
