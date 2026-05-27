#![forbid(unsafe_code)]

// State 5 proof-writer repair: make the 11 PO-vb-om21-*-proptest
// integration-test artifacts discoverable by Cargo/nextest while preserving
// each obligation-specific file and test name.

#[path = "proptest/vb_om21_big_endian_max_proptest.rs"]
mod vb_om21_big_endian_max_proptest;
#[path = "proptest/vb_om21_bounded_scan_proptest.rs"]
mod vb_om21_bounded_scan_proptest;
#[path = "proptest/vb_om21_key_parse_proptest.rs"]
mod vb_om21_key_parse_proptest;
#[path = "proptest/vb_om21_missing_journal_proptest.rs"]
mod vb_om21_missing_journal_proptest;
#[path = "proptest/vb_om21_prefix_bound_proptest.rs"]
mod vb_om21_prefix_bound_proptest;
#[path = "proptest/vb_om21_replay_parity_proptest.rs"]
mod vb_om21_replay_parity_proptest;
#[path = "proptest/vb_om21_single_event_tail_proptest.rs"]
mod vb_om21_single_event_tail_proptest;
#[path = "proptest/vb_om21_tail_mismatch_proptest.rs"]
mod vb_om21_tail_mismatch_proptest;
#[path = "proptest/vb_om21_tail_overflow_proptest.rs"]
mod vb_om21_tail_overflow_proptest;
#[path = "proptest/vb_om21_typed_errors_proptest.rs"]
mod vb_om21_typed_errors_proptest;
#[path = "proptest/vb_om21_zero_tail_query_proptest.rs"]
mod vb_om21_zero_tail_query_proptest;
