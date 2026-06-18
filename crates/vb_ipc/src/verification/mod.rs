//! Verification artifacts for vb_ipc.
//!
//! These modules are compiled only for explicit Verus/tooling lanes. They are
//! not production proof evidence unless a proof artifact states and verifies a
//! production binding.

#[cfg(verus)]
#[path = "verus/vb_5iebh.rs"]
pub mod vb_5iebh;
