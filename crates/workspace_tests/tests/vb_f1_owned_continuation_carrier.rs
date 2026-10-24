#![forbid(unsafe_code)]
//! Cold wire evidence only. The included original 19 tests are repeated reuse.
#[path = "vb_f1_owned_continuation_carrier/bounds.rs"]
mod bounds;
#[path = "vb_f1_owned_continuation_carrier/compatibility.rs"]
mod compatibility;
#[path = "vb_f1_owned_continuation_transport.rs"]
pub mod component_fixture;
#[path = "vb_f1_owned_continuation_carrier/context.rs"]
mod context;
#[path = "vb_f1_owned_continuation_carrier/hydration.rs"]
mod hydration;
#[path = "vb_f1_owned_continuation_carrier/malformed.rs"]
mod malformed;
#[path = "vb_f1_owned_continuation_carrier/neutral.rs"]
mod neutral;
#[path = "vb_f1_owned_continuation_carrier/roundtrip.rs"]
mod roundtrip;
#[path = "vb_f1_owned_continuation_carrier/support.rs"]
mod support;
