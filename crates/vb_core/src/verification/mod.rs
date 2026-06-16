//! Verification artifacts for vb_core.

#[cfg(kani)]
pub(crate) mod kani;

// Flux refinement modules (compiled with flux-rs; extern specs)
#[cfg(all(flux, feature = "vb-rxru0-flux-refinements"))]
pub mod flux {
    pub mod vb_rxru0_action_enums;
}

// Proptest modules (compiled with cargo test)
#[cfg(test)]
pub(crate) mod proptest {
    pub(crate) mod vb_rxru0_action_properties;
}

// Verus proof modules (compiled with verus toolchain)
#[cfg(verus)]
pub mod verus {
    pub mod run_frame_new_exec_proofs;
    pub mod step_state_absorbing_proofs;
    pub mod vb_rxru0_action_specs;
}
