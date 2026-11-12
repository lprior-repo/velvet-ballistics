// Verification artifacts: Kani harnesses for choose replay.
// Bead: vb-xi2f.13 | State: 5 (proof-writer)
// PO-KANI-009, PO-KANI-010
//
// GOD RULE 1: Uses kani::any() for exhaustive input generation.
// GOD RULE 2: Binds to production replay_choose_slot in super::mod.
// GOD RULE 4: Unwinding bounds are documented per harness.

#![cfg(kani)]

mod kani_choose_bool_condition;
mod kani_choose_no_otherwise;
