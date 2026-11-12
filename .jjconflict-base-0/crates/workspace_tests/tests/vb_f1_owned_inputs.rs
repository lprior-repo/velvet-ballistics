#![forbid(unsafe_code)]

#[path = "vb_f1_owned_inputs/atomic.rs"]
mod atomic;
#[path = "vb_f1_owned_inputs/binding.rs"]
mod binding;
#[path = "vb_f1_owned_inputs/corruption.rs"]
mod corruption;
#[path = "vb_f1_owned_inputs/roundtrip.rs"]
mod roundtrip;
#[path = "vb_f1_owned_inputs/scalar.rs"]
mod scalar;
#[path = "vb_f1_owned_inputs/support.rs"]
mod support;
#[path = "vb_f1_owned_inputs/wire.rs"]
mod wire;
