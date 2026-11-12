#![forbid(unsafe_code)]
//! Component evidence only: durable cut activation remains a separate Root wire.

#[path = "vb_f1_owned_continuation_common/arenas.rs"]
mod arenas;
#[path = "vb_f1_owned_continuation/authority.rs"]
mod authority;
#[path = "vb_f1_owned_continuation/corruption.rs"]
mod corruption;
#[path = "vb_f1_owned_continuation/cuts.rs"]
mod cuts;
#[path = "vb_f1_owned_continuation_common/evidence.rs"]
mod evidence;
#[path = "vb_f1_owned_continuation/graph.rs"]
mod graph;
#[path = "vb_f1_owned_continuation_common/receipt.rs"]
mod receipt;
#[path = "vb_f1_owned_continuation/refusals.rs"]
mod refusals;
#[path = "vb_f1_owned_continuation_common/support.rs"]
mod support;
