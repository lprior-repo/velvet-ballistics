//! Retired Verus artifact for `vb_ajc40_flux` Flux-contract obligations.
//!
//! The previous revision of this file defined local copies of admission-kernel
//! types, local copies of `validate_admission_summary`, local Flux contract
//! models, and local exec functions that only mirrored production behavior in
//! comments. Its proof lemmas did not call or otherwise mechanically bind to the
//! production functions in `crates/vb_core` or `crates/vb_ajc40_flux`.
//!
//! Keeping those lemmas would preserve a vacuum-proof claim: Verus would prove
//! local models about local models while comments described production binding.
//! This retired artifact intentionally retains no validation specs, proof
//! declarations, mirror exec functions, trusted assumptions, or external bodies.
//!
//! Current status:
//!
//! - no production-bound deductive proof claim is made here;
//! - `verification/verus/vb_ajc40_admission_kernel_scalar.rs` remains the
//!   separate generated scalar-kernel artifact for the admission kernel;
//! - future `vb_ajc40_flux` L4 evidence must use an auditable binding route,
//!   such as a shared production proof kernel, direct production
//!   `requires`/`ensures`, or another reviewed bridge that actually checks the
//!   production implementation rather than a copy.
//!
//! Until then, `verus --crate-type=lib` over this file is only a syntax/trust
//! regression check for the retired artifact; it is not production proof
//! evidence for PO-013, PO-015, PO-017, PO-019, PO-021, PO-025, PO-031, or
//! PO-033.
use vstd::prelude::*;

verus! {


} // verus!
