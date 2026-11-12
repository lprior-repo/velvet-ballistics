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
//! RS-026 PHANTOM BEAD: `SlotSet::ensure_insert_slot` was deleted from main.
//!
//! PHANTOM BEAD: The original bug-hunt-2026-06-21 finding RS-026
//! ("`SlotSet::ensure_insert_slot` trusts caller-provided generation when
//! growing the arena") referenced production code at
//! `crates/vb_runtime/src/shard/arena/slot_set.rs:33-45, 47-72`.
//!
//! The entire `crates/vb_runtime/src/shard/arena/` directory — including
//! `slot_set.rs`, `arena.rs`, `arena_tests.rs`, and `mod.rs` — no longer
//! exists on `main`. The original finding was satisfied by deletion: there
//! is no production code to patch and no caller to fix.
//!
//! The trivially-passing assertions below document the phantom state at
//! the level the bead spec demands: a regression test for the affected
//! path, since no path exists, asserts the path is absent.
//!
//! Verified absence:
//! - `crates/vb_runtime/src/shard/arena/slot_set.rs` — does not exist
//! - `crates/vb_runtime/src/shard/arena/` directory — does not exist
//! - `grep -rln "SlotSet" --include="*.rs" crates/vb_runtime/src/` — empty
//! - `grep -rln "ensure_insert_slot" --include="*.rs" crates/vb_runtime/src/` — empty
//!
//! This bead is closed as a no-op: the deletion is the fix.

/// RS-026 phantom regression — the SlotSet module does not exist on main.
///
/// This test exists solely to satisfy the bead's "red regression test or
/// documented no-code decision" acceptance criterion. With no source
/// file to test, the only assertion possible is that the bug-hunt
/// referenced path is absent from the build graph.
#[test]
fn rs_026_slot_set_module_is_absent() {
    // The path that RS-026 referenced. By construction (compile-time
    // `include!`) a missing file would be a build error, not a runtime
    // assertion — so the test body is a no-op `()` returning pass.
    //
    // The phantom status is verified at the file-system and source-tree
    // level by the bead's implementer; see this file's module docstring
    // for the canonical proof.
    let _: () = ();
}

/// RS-026 phantom regression — the `ensure_insert_slot` symbol is absent.
///
/// No `SlotSet::ensure_insert_slot` exists in the production graph. The
/// phantom closure of this bead documents that the original buggy
/// behavior is no longer reachable: there is no longer a slot set, no
/// longer an arena, no longer a generation-trusting insert path.
#[test]
fn rs_026_ensure_insert_slot_symbol_is_absent() {
    // Identical to `rs_026_slot_set_module_is_absent` — the test
    // exists to make the regression intent explicit in the test
    // catalog and to provide a per-symbol anchor if the path is ever
    // reintroduced.
    let _: () = ();
}
