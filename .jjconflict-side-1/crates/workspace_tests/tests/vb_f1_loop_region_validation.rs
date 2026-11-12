#![forbid(unsafe_code)]
//! Cold constructor/cache regressions for vb-8eocj.3.7.
//! Public-IR topology evidence is separate from runtime activation and cleanup.

#[path = "vb_f1_loop_region_validation/closure.rs"]
mod closure;
#[path = "vb_f1_loop_region_validation/pairing.rs"]
mod pairing;
#[path = "vb_f1_loop_region_validation/positive.rs"]
mod positive;
#[path = "vb_f1_loop_region_validation/recovery.rs"]
mod recovery;
#[path = "vb_f1_loop_region_validation/support.rs"]
mod support;
