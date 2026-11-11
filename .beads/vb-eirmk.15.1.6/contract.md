# Contract: Public RunFrame Slot Failure Atomicity

## Scope

Qualify public `vb_core::RunFrame` slot and taint write failures using integration scenarios only; production code, APIs, schemas, dependencies, gates, settings, tracker state, and existing tests remain unchanged.

## Given

Each scenario independently constructs a populated three-step, three-slot frame with a nondefault run ID, program counter, step states, slot values, taints, executed count, and parallel-in-flight fields; slot 1 remains valid and uninitialized where required.

## When

Invoke `write_slot`, `write_slot_with_taint`, or `write_taint` on out-of-range slot 3, invoke `write_taint` on valid uninitialized slot 1, and separately read slot 1 and slot 3 through `read_slot` and `read_taint`.

## Then

Each of the four failing writes returns its exact current `CoreError` variant and slot (`SlotOutOfBounds { slot: SlotIdx::new(3) }` for the three slot-3 writes; `SlotUninitialized { slot: SlotIdx::new(1) }` for the slot-1 taint write), and full `RunFrame` equality with a pre-call clone proves failure atomicity.

`read_slot` and `read_taint` each distinguish `SlotUninitialized { slot: SlotIdx::new(1) }` from `SlotOutOfBounds { slot: SlotIdx::new(3) }`.

## Files and Verification

Add only `crates/workspace_tests/tests/vb_eirmk_frame_slots.rs` as the behavior-test artifact; preserve all existing and parent-provided files.

Run pinned file-only rustfmt with nightly `2026-04-28`, then run the scoped native nextest command `CARGO_BUILD_JOBS=2 RUSTC_WRAPPER= RUSTFLAGS=-Dwarnings rustup run nightly-2026-04-28 cargo nextest run --cargo-quiet -p velvet-ballistics-workspace-tests --all-features --test vb_eirmk_frame_slots`, capturing output under `target/core-build-20261110/evidence/native.log` and reporting the native process exit status.
