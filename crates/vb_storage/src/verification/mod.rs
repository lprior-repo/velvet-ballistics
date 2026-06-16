#![forbid(unsafe_code)]

#[cfg(any(test, flux))]
pub mod flux;

#[cfg(test)]
pub mod mrwe5_production_bridge;

#[cfg(kani)]
#[path = "vb-fn4vt/mod.rs"]
pub mod vb_fn4vt;

/// Verus proof artifacts: spec functions and exec production bridges
/// for recovery types, MRWE5 classification, and replay invariants.
pub mod verus;
