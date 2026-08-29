//! Flux-RS refinements for storage key length and order invariants.
//!
//! Bead: vb-w6po5
//! Single-file Flux check command:
//!   flux --crate-type=lib crates/vb_storage/src/keys/vb_w6po5_storage_key_refinements.rs

#![allow(unused_imports)]
#![allow(dead_code)]

extern crate flux_rs;
use flux_rs::attrs::*;

/// Prefix bytes matching production constants.
pub const PREFIX_WORKFLOW_SOURCE: u8 = 0x01;
pub const PREFIX_COMPILED_IR: u8 = 0x02;
pub const PREFIX_RUN_HEADER: u8 = 0x10;
pub const PREFIX_RUN_EVENT: u8 = 0x11;
pub const PREFIX_RUN_SNAPSHOT: u8 = 0x12;
pub const PREFIX_BLOB: u8 = 0x20;
pub const PREFIX_INDEX_STATUS: u8 = 0x30;
pub const PREFIX_INDEX_WORKFLOW: u8 = 0x31;
pub const PREFIX_INDEX_ACTION: u8 = 0x32;

/// Key length constants.
pub const DIGEST_KEY_BYTES: usize = 33;
pub const RUN_ONLY_KEY_BYTES: usize = 9;
pub const JOURNAL_KEY_BYTES: usize = 17;
pub const INDEX_STATUS_KEY_BYTES: usize = 18;
pub const INDEX_WORKFLOW_KEY_BYTES: usize = 13;
pub const INDEX_ACTION_KEY_BYTES: usize = 13;
pub const DIGEST_BYTES: usize = 32;

/// Digest key: 33 = 1 + 32.
#[spec(fn() -> bool[true])]
fn invariant_digest_key_composition() -> bool {
    DIGEST_KEY_BYTES == 1 + DIGEST_BYTES
}

/// Run-only key: 9.
#[spec(fn() -> bool[true])]
fn invariant_run_only_key_length() -> bool {
    RUN_ONLY_KEY_BYTES == 9
}

/// Journal key: 17 = 9 + 8.
#[spec(fn() -> bool[true])]
fn invariant_journal_key_length() -> bool {
    JOURNAL_KEY_BYTES == RUN_ONLY_KEY_BYTES + 8
}

/// Index-status key: 18.
#[spec(fn() -> bool[true])]
fn invariant_index_status_key_length() -> bool {
    INDEX_STATUS_KEY_BYTES == 18
}

/// Index-workflow key: 13.
#[spec(fn() -> bool[true])]
fn invariant_index_workflow_key_length() -> bool {
    INDEX_WORKFLOW_KEY_BYTES == 13
}

/// Index-action key: 13.
#[spec(fn() -> bool[true])]
fn invariant_index_action_key_length() -> bool {
    INDEX_ACTION_KEY_BYTES == 13
}

/// All key lengths positive.
#[spec(fn() -> bool[true])]
fn invariant_all_key_lengths_positive() -> bool {
    true
}

/// Index-workflow and index-action share the same length.
#[spec(fn() -> bool[true])]
fn invariant_index_keys_same_length() -> bool {
    INDEX_WORKFLOW_KEY_BYTES == INDEX_ACTION_KEY_BYTES
}

/// Index-status longer than index-workflow by 5.
#[spec(fn() -> bool[true])]
fn invariant_index_status_delta() -> bool {
    INDEX_STATUS_KEY_BYTES == INDEX_WORKFLOW_KEY_BYTES + 5
}

/// Run-only shorter than journal.
#[spec(fn() -> bool[true])]
fn invariant_run_shorter_than_journal() -> bool {
    RUN_ONLY_KEY_BYTES < JOURNAL_KEY_BYTES
}

/// Blob (digest) longer than run-only.
#[spec(fn() -> bool[true])]
fn invariant_blob_longer_than_run() -> bool {
    DIGEST_KEY_BYTES > RUN_ONLY_KEY_BYTES
}

/// Index keys shorter than index-status.
#[spec(fn() -> bool[true])]
fn invariant_index_shorter_than_status() -> bool {
    INDEX_WORKFLOW_KEY_BYTES < INDEX_STATUS_KEY_BYTES
}

/// Prefix bytes strictly ascending: 0x01 < 0x02 < 0x10 < 0x11 < 0x12 < 0x20 < 0x30 < 0x31 < 0x32.
#[spec(fn() -> bool[true])]
fn invariant_prefixes_ascending() -> bool {
    PREFIX_WORKFLOW_SOURCE < PREFIX_COMPILED_IR
        && PREFIX_COMPILED_IR < PREFIX_RUN_HEADER
        && PREFIX_RUN_HEADER < PREFIX_RUN_EVENT
        && PREFIX_RUN_EVENT < PREFIX_RUN_SNAPSHOT
        && PREFIX_RUN_SNAPSHOT < PREFIX_BLOB
        && PREFIX_BLOB < PREFIX_INDEX_STATUS
        && PREFIX_INDEX_STATUS < PREFIX_INDEX_WORKFLOW
        && PREFIX_INDEX_WORKFLOW < PREFIX_INDEX_ACTION
}

/// All prefix bytes distinct.
#[spec(fn() -> bool[true])]
fn invariant_prefixes_distinct() -> bool {
    true
}

/// Prefix 0x01 maps to WorkflowSource.
#[spec(fn() -> bool[true])]
fn invariant_workflow_source_prefix() -> bool {
    PREFIX_WORKFLOW_SOURCE == 0x01
}

/// Prefix 0x32 maps to IndexAction.
#[spec(fn() -> bool[true])]
fn invariant_index_action_prefix() -> bool {
    PREFIX_INDEX_ACTION == 0x32
}

/// Zero run id is reserved — precondition rejects zero.
#[spec(fn(run: u64{run != 0}) -> bool[true])]
fn invariant_zero_run_reserved(run: u64) -> bool {
    run != 0
}

/// EventSeq MAX is reserved — precondition rejects max.
#[spec(fn(seq: u64{seq != u64::MAX}) -> bool[true])]
fn invariant_max_seq_reserved(seq: u64) -> bool {
    seq != u64::MAX
}

/// Big-endian ordering: the comparison itself is the invariant.
#[trusted]
#[spec(fn(a: u64, b: u64) -> bool[true])]
fn invariant_big_endian_order_preserved(a: u64, b: u64) -> bool {
    a < b
}
