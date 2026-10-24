#![forbid(unsafe_code)]
//! Bounded component evidence; public durable cold recovery remains Root-held.
#[path = "vb_f1_owned_continuation_transport/arenas.rs"]
mod arenas;
#[path = "vb_f1_owned_continuation_transport/evidence.rs"]
mod evidence;
#[path = "vb_f1_owned_continuation_transport/golden.rs"]
mod golden;
#[path = "vb_f1_owned_continuation_transport/hostile.rs"]
mod hostile;
#[path = "vb_f1_owned_continuation_transport/receipt.rs"]
mod receipt;
#[path = "vb_f1_owned_continuation_transport/support.rs"]
mod support;

#[path = "vb_f1_owned_continuation_transport/layout.rs"]
mod layout;

#[path = "vb_f1_owned_continuation_transport/contracts.rs"]
mod contracts;

#[path = "vb_f1_owned_continuation_transport/maximum.rs"]
mod maximum;

#[path = "vb_f1_owned_continuation_transport/resume.rs"]
mod resume;
#[path = "vb_f1_owned_continuation_transport/store_bounds.rs"]
mod store_bounds;

#[path = "vb_f1_owned_continuation_transport/action_evidence.rs"]
mod action_evidence;
#[path = "vb_f1_owned_continuation_transport/action_fixture.rs"]
mod action_fixture;

/// Additive cold-carrier fixture bridge; original 19 test bodies/helpers are unchanged.
pub fn carrier_fixture(
    after: bool,
    blob_bytes: usize,
) -> Result<
    (
        vb_runtime::shard::types::RunState,
        vb_runtime::run_continuation::ContinuationAuthority,
    ),
    Box<dyn std::error::Error>,
> {
    let fixture = support::at_cut_with_blob(after, blob_bytes)?;
    let authority = fixture.authority()?;
    Ok((fixture.state, authority))
}
