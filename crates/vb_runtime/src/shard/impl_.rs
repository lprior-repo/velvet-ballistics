#![forbid(unsafe_code)]
//! Shard construction, queue operations, and core tick processing.

include!("impl_parts/chunk_004.rs");
include!("impl_parts/chunk_001.rs");
include!("impl_parts/chunk_001/queue.rs");
include!("impl_parts/chunk_001/run_slots.rs");
include!("impl_parts/chunk_001/journal_append.rs");
include!("impl_parts/chunk_001/journal_sequences.rs");
include!("impl_parts/chunk_001/run_state.rs");
include!("impl_parts/chunk_001/run_admission.rs");
include!("impl_parts/chunk_001/pending.rs");
include!("impl_parts/chunk_001/tick_drive.rs");
include!("impl_parts/chunk_001/inspect_evidence.rs");
include!("impl_parts/chunk_002.rs");
include!("impl_parts/chunk_003.rs");
include!("impl_parts/policy_decision.rs");
include!("impl_parts/gate_decision.rs");
include!("impl_parts/gate_opened.rs");

#[cfg(test)]
mod tests {
    include!("impl_tests/chunk_001.rs");
    include!("impl_tests/chunk_002.rs");
}
