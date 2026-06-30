# vb-vt2f Blocker Report

## Status

- Status: `CLEARED_BY_OWNER_AUTHORIZED_KANI_PROOF_KERNEL`
- Sublane: `owner-authorized-unblock / kani-tractable-proof-kernel`
- Attempt: 1
- Workspace: `/home/lewis/src/bd-vb-vt2f-bdd`

## Cleared Rows

- `KANI-VT2F-RUNTIME-FACADE-001`: PASS by replacement proof kernel under exact harness name.
- `KANI-VT2F-SHARD-LOWER-001`: PASS by replacement proof kernel under exact harness name.

## Evidence

- `TMPDIR=/home/lewis/src/bd-vb-vt2f-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 cargo kani -p vb_runtime --harness vt2f_runtime_facade_semantics` => `VERIFICATION:- SUCCESSFUL`, 7/7 covers satisfied.
- `TMPDIR=/home/lewis/src/bd-vb-vt2f-bdd/.tmp RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 cargo kani -p vb_runtime --harness vt2f_shard_lower_semantics` => `VERIFICATION:- SUCCESSFUL`, 8/8 covers satisfied.
- `RUSTC_WRAPPER= SCCACHE_DISABLE=1 CARGO_INCREMENTAL=0 rtk cargo check -p vb_runtime` => PASS.

## Remaining Risk

The blocker is cleared only through the approved proof-kernel abstraction. Concrete full-runtime Kani remains intentionally avoided because it is not tractable for CBMC in this crate feature graph.
