//! Journal replay and storage fuzzing targets.
#![allow(clippy::indexing_slicing)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::let_underscore_must_use)]
#![allow(clippy::as_conversions)]
#![allow(clippy::arithmetic_side_effects)]
#![allow(clippy::len_zero)]

mod admission;
mod errors;
mod event;
mod persisted;
mod readback;

pub use admission::{
    fuzz_admission_flow, fuzz_admission_fuzz, fuzz_digest_coherence, fuzz_strict_artifact_decoder,
};
pub use event::{
    fuzz_action_tracker, fuzz_extract_terminal, fuzz_journal_event, fuzz_replay_events,
};
pub use persisted::{
    fuzz_accepted_artifact_envelope_qi37_4_2, fuzz_binary_payload_boundary,
    fuzz_storage_envelope_boundary, fuzz_vb_qi37_12_persisted_payload_decode,
};
pub use readback::{
    fuzz_accepted_artifact_decode, fuzz_admission_input_surface, fuzz_readback_family_set,
    fuzz_recovery_decode,
};

const MAX_FUZZ_PAYLOAD: u32 = 4096;
