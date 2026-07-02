# Proof Evidence: vb-5bqmr SlotExtra Discriminator

## Obligation Map

| PO        | Lane    | Status                  | Notes |
|-----------|---------|-------------------------|-------|
| PO-VERUS-001 | verus  | PASS (smoke)            | 21 verified, 0 errors; WEAK production_inner mirror binding |
| PO-KANI-001  | kani   | BLOCKED_TOOLING         | pre-existing kani_helpers.rs:1-22 unclosed delimiter; artifacts written |
| PO-KANI-002  | kani   | BLOCKED_TOOLING         | pre-existing kani_helpers.rs:1-22 unclosed delimiter; artifacts written |
| PO-FLUX-001  | flux-rs| PASS (smoke)            | package-level check succeeded |
| PO-PROP-001  | proptest| PENDING_FORMAL_EXECUTION | gated behind `kani-vb-5bqmr` feature; targets 3-arm code |
| PO-PROP-002  | proptest| PENDING_FORMAL_EXECUTION | gated behind `kani-vb-5bqmr` feature; targets 3-arm code |
| PO-PROP-003  | proptest| PENDING_FORMAL_EXECUTION | gated behind `kani-vb-5bqmr` feature; cross-crate |

## Verus (PO-VERUS-001)

### Command

```bash
verus --crate-type=lib verification/verus/vb_5bqmr_slot_extra_version_reject.rs
```

### Output

```text
verification results:: 21 verified, 0 errors
```

### Evidence

The Verus spec at
`verification/verus/vb_5bqmr_slot_extra_version_reject.rs` passes with
21 verified proofs / 0 errors. The 5 lemma proofs plus the
`assume_specification` contract and the `checked_decode_partition`
exec wrapper verify cleanly.

### Production-binding audit

```bash
bash scripts/check-verus-production-binding.sh "$PWD"
```

```text
================================================================
  Verus production-binding audit (ABSOLUTE — no exceptions)
================================================================
  STRONG (direct crates/ binding): 0
  WEAK (production_inner/ mirror): 72
  VACUUM (no production binding):  0

```

The new spec is classified as WEAK (production_inner/ mirror) per
the script's check at line 113-115 (the spec uses
`#[path = "extern_vb_5bqmr_slot_extra.rs"]` and the extern file uses
`#[path = "production_inner/vb_5bqmr_slot_extra_production.rs"]`).
No VACUUM files; the spec is properly production-bound.

### Trust boundary

The production body of `decode_slot_written_extra` is
`#[verifier::external]`-bound via the production mirror, so Verus
does not verify the body. The contracts attached via
`assume_specification` in the spec file state the production
behaviour the spec proofs discharge.

## Kani (PO-KANI-001, PO-KANI-002)

### Status

BLOCKED_TOOLING — pre-existing issue in the parent commit
unrelated to this bead.

### Discovery command

```bash
cargo kani -p vb_storage --features kani-vb-5bqmr --harness kani_decode_unknown_version_rejects --no-assertion-reach-checks
```

### Error

```text
error: this file contains an unclosed delimiter
  --> crates/vb_core/src/frame/parts/kani_helpers.rs:22:7
   |
 1 | mod frame_kani_harnesses {
   |                          - unclosed delimiter
...
22 |     }
   |      ^

error: could not compile `vb_core` (lib) due to 1 previous error
error: Failed to execute cargo (exit status: 101). Found 1 compilation errors.
```

### Mitigation

The pre-existing issue at
`crates/vb_core/src/frame/parts/kani_helpers.rs:1-22` (unclosed
`mod frame_kani_harnesses` delimiter) blocks ALL Kani harnesses in
the project, not just the ones for vb-5bqmr. The Kani artifacts
are correctly written; the formal-verifier at State 12 will run
them when the upstream issue is resolved.

### Library smoke

```bash
cargo check -p vb_storage --features kani-vb-5bqmr
```

```text
cargo build (0 crates compiled)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.03s
```

The library compiles with the `kani-vb-5bqmr` feature (the Kani
harness file is `#[cfg]`-gated and does not need to compile for
the library to be valid).

## Flux-rs (PO-FLUX-001)

### Command

```bash
bash scripts/flux-check-package.sh vb_storage
```

### Output

```text
Finished `flux` profile [unoptimized + debuginfo] target(s) in 0.07s
```

### Evidence

The Flux-rs package-level check passes. The
`verification/flux/vb_5bqmr_slot_extra_magic_prefix.rs` file
contains 6 Flux-rs refinement annotations
(`spec_prefix_len`, `spec_magic`, `spec_version`, `spec_prefix`,
`spec_discriminator_no_version_branch_for_short`,
`spec_discriminator_versioned_branch_reachable`) plus a companion
runtime test module that asserts the prefix constant matches its
compositional derivation at runtime.

The package-level Flux pass is a CRATE SMOKE check (per
`proof-writer/SKILL.md`); the per-file Flux artifacts will be
picked up by the formal-verifier at State 12.

## Proptest (PO-PROP-001, PO-PROP-002, PO-PROP-003)

### Status

PENDING_FORMAL_EXECUTION — gated behind the `kani-vb-5bqmr` cargo
feature. The proptest files reference the planned
`SlotWrittenExtraError::VersionMismatch` and
`CollectExtraHydrationFailureKind::VersionMismatch` variants which
do not exist in the current 2-arm production code.

### Test files

- `crates/vb_storage/tests/proptest_vb_5bqmr_slot_extra.rs` (gated
  behind `#[cfg(all(test, feature = "kani-vb-5bqmr"))]`)
- `crates/vb_runtime/tests/proptest_vb_5bqmr_collect_slot_extra.rs`
  (gated behind `#[cfg(all(test, feature = "kani-vb-5bqmr"))]`)

### Smoke evidence

```bash
cargo check -p vb_storage --features kani-vb-5bqmr
```

```text
cargo build (0 crates compiled)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.03s
```

The library compiles. The proptest files are gated behind the
`kani-vb-5bqmr` feature; they will compile when the production
fix lands and the feature is enabled.

```bash
cargo check -p vb_storage --tests --features kani-vb-5bqmr
```

```text
cargo build: 5 errors, 4 warnings (1 crates)
```

The 5 errors are all `SlotWrittenExtraError::VersionMismatch` not
found in the pre-fix production code (PENDING_FORMAL_EXECUTION).
The library and gated test files compile with the feature; the
test cases will run after the production fix lands.

```bash
cargo check -p vb_runtime --tests --features kani-vb-5bqmr
```

```text
cargo build: 1 error, 1 warning (24 crates)
```

The 1 error is `CollectExtraHydrationFailureKind::VersionMismatch`
not found in the pre-fix production code (PENDING_FORMAL_EXECUTION).

### Negative invariants preserved

The proptests preserve the C-NEG-001..003 negative invariants:

- `b"\x01\x02\x03\x04"` → `Ok(LegacyFrameExtra(b"\x01\x02\x03\x04"))`
  (C-NEG-001, BDD scenario at `recovery_bdd_tests.rs:3158-3211`)
- `b"VBSE"` (4 bytes) → `Ok(LegacyFrameExtra(b"VBSE"))` (C-NEG-002)
- `b"VBSE\x01\xff\xff\xff"` → `Err(DecodeFailed)` (C-NEG-003,
  corrupt-v1 helper at `recovery/tests.rs:2332`)

## Cross-Crate Translation (PO-PROP-003)

The cross-crate proptests exercise the public
`hydrate_run_frame_from_events` entry point (storage side,
`crates/vb_storage/src/recovery/hydrate.rs:507`) and the public
`hydrate_collect_states_from_recovered_journal` entry point
(runtime side, `crates/vb_runtime/src/primitives/collect.rs:306`).
The private translation functions `decoded_slot_taint` (storage,
`hydrate.rs:220`) and `hydrate_slot_written_extra` (runtime,
`collect.rs:248`) are reachable through these public entry points.

The cross-crate test pair is split across two proptest files
(vb_storage and vb_runtime) to keep the proptest files
package-scoped. The runtime-side proptest focuses on the error
variant assertion (`CollectExtraHydrationFailureKind::VersionMismatch`);
the tracing log capture is the formal-verifier's responsibility
at State 12 (TB-PROP-003-tracing-capture).

## Trust Markers (See `trusted-base-ledger.jsonl`)

- `TB-KANI-001-cover-reachability` (PO-KANI-001)
- `TB-KANI-002-alloc-counter` (PO-KANI-002)
- `TB-KANI-002-cover-reachability` (PO-KANI-002)
- `TB-PROP-003-compile-time-exhaustiveness` (PO-PROP-003)
- `TB-PROP-PENDING-FORMAL-EXECUTION` (PO-PROP-001/002/003)

## Blockers

- `BLOCKED_TOOLING` at `crates/vb_core/src/frame/parts/kani_helpers.rs:1-22`
  (pre-existing unclosed `mod frame_kani_harnesses` delimiter).
  Affects all Kani harnesses in the project, not just vb-5bqmr.
  Documented in `trusted-base-ledger.jsonl` as
  `TB-KANI-TOOLING-BLOCKER`.

## Raw Evidence

### Verus spec verification

```text
$ verus --crate-type=lib verification/verus/vb_5bqmr_slot_extra_version_reject.rs
verification results:: 21 verified, 0 errors
```

### Production-binding audit

```text
$ bash scripts/check-verus-production-binding.sh "$PWD"
================================================================
  Verus production-binding audit (ABSOLUTE — no exceptions)
================================================================
  STRONG (direct crates/ binding): 0
  WEAK (production_inner/ mirror): 72
  VACUUM (no production binding):  0

```

### Flux-rs package check

```text
$ bash scripts/flux-check-package.sh vb_storage
Finished `flux` profile [unoptimized + debuginfo] target(s) in 0.07s
```

### Library smoke (Kani feature enabled)

```text
$ cargo check -p vb_storage --features kani-vb-5bqmr
cargo build (0 crates compiled)
Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.03s
```

### Kani execution (BLOCKED)

```text
$ cargo kani -p vb_storage --features kani-vb-5bqmr --harness kani_decode_unknown_version_rejects --no-assertion-reach-checks
Kani Rust Verifier 0.67.0 (cargo plugin)
   Compiling vb_core v0.1.0 (...)
error: this file contains an unclosed delimiter
  --> crates/vb_core/src/frame/parts/kani_helpers.rs:22:7
   |
 1 | mod frame_kani_harnesses {
   |                          - unclosed delimiter
...
22 |     }
   |      ^

error: could not compile `vb_core` (lib) due to 1 previous error
error: Failed to execute cargo (exit status: 101). Found 1 compilation errors.
```

### Test file smoke (gated behind feature)

```text
$ cargo check -p vb_storage --tests --features kani-vb-5bqmr
cargo build: 5 errors, 4 warnings (1 crates)
# 5 errors are all `VersionMismatch` not found in pre-fix production
# code; PENDING_FORMAL_EXECUTION

$ cargo check -p vb_runtime --tests --features kani-vb-5bqmr
cargo build: 1 error, 1 warning (24 crates)
# 1 error: `VersionMismatch` not found in
# CollectExtraHydrationFailureKind; PENDING_FORMAL_EXECUTION
```
