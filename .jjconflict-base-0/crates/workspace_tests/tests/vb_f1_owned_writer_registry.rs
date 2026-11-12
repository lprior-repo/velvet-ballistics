#![forbid(unsafe_code)]
//! Registry primitive behavior only; no real append or public recovery is guarded.
#[path = "vb_f1_owned_writer_registry/ownership.rs"]
mod ownership;
#[path = "vb_f1_owned_writer_registry/quarantine.rs"]
mod quarantine;
#[path = "vb_f1_owned_writer_registry/race.rs"]
mod race;
#[path = "vb_f1_owned_writer_registry/support.rs"]
mod support;
