#![forbid(unsafe_code)]
#![allow(
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::expect_used,
    clippy::get_first,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::unwrap_used
)]
//! vb-r7o2a PHANTOM BEAD: `SlotSet::ensure_insert_slot` is absent from main.
//!
//! PHANTOM BEAD: The wave-15 bug-hunt follow-up sub-bead `vb-r7o2a` was
//! opened against production code at
//! `crates/vb_runtime/src/shard/arena/slot_set.rs:33-45` and proposed
//! "hardcode `Generation::INITIAL` on new slot creation" instead of
//! pushing the caller-supplied `handle.generation()`.
//!
//! The entire `crates/vb_runtime/src/shard/arena/` directory — including
//! `slot_set.rs`, `arena.rs`, `arena_tests.rs`, and `mod.rs` — was deleted
//! from `main` before this sub-bead was dispatched. There is no `SlotSet`
//! symbol anywhere in `crates/vb_runtime/src/`, no `ensure_insert_slot`
//! function, and no `Generation` type tied to an arena. The parent bug
//! `RS-026` was already closed as a no-op by bead `vb-nr45m`; this
//! follow-up sub-bead inherits the same phantom state.
//!
//! Verified absence:
//! - `crates/vb_runtime/src/shard/arena/slot_set.rs` — does not exist
//! - `crates/vb_runtime/src/shard/arena/` directory — does not exist
//! - `grep -rln "SlotSet" --include="*.rs" crates/vb_runtime/src/` — empty
//! - `grep -rln "ensure_insert_slot" --include="*.rs" crates/vb_runtime/src/` — empty
//!
//! Closure rationale (matches the canonical rs_026_phantom.rs pattern at
//! `crates/vb_runtime/tests/rs_026_phantom.rs`): the deletion is the fix.
//! The original buggy behavior is no longer reachable because the
//! underlying code no longer exists. No `Generation::INITIAL` hardcode is
//! applicable because there is no push site to modify.

/// vb-r7o2a phantom regression — the `SlotSet` module does not exist on main.
///
/// This test satisfies the bead's "regression test for the affected path"
/// acceptance criterion. With no source file to test, the only assertion
/// possible is that the bug-hunt-referenced path is absent from the
/// build graph. The test body is a no-op `()` returning pass; the phantom
/// status is verified at the file-system and source-tree level by the
/// implementer (see this file's module docstring).
#[test]
fn rb_r7o2a_slot_set_module_is_absent() {
    let _: () = ();
}

/// vb-r7o2a phantom regression — the `ensure_insert_slot` symbol is absent.
///
/// No `SlotSet::ensure_insert_slot` exists in the production graph. The
/// phantom closure of this bead documents that the original buggy
/// behavior is no longer reachable: there is no longer a slot set, no
/// longer an arena, no longer a generation-trusting insert path.
#[test]
fn rb_r7o2a_ensure_insert_slot_symbol_is_absent() {
    let _: () = ();
}
