// RETIRED: non_applicable — vacuum Flux refinement.
// Obligation ID: POB-vb-vzcuf-035
// Verifier: flux-rs
// Proof seed: vb-vzcuf-PS-009
// Contract clause: contract.md:C2 open product question
//
// === RETIRED IN BH-W0-S04 / FINDING-B-01 EXTENSION ===
// The previous version of this file already declared
// `PRODUCTION BINDING (REMOVED IN COMMIT 150e1489a)` — correctly noting
// that `JournalWriteBatch::staged_event_keys: HashSet<[u8; 17]>` was
// removed in commit 150e1489a (bead vb-u2psq). However, the file still
// contained a `#[flux_rs::sig]` annotation on a local helper
// `conservative_accounting(current: u64, encoded_len: u64) -> u64` plus
// four `#[test]` functions and three standalone assertion helpers
// (`test_conservative_always_increases`, `test_precise_duplicate_unchanged`,
// `test_precise_new_key_increases`, `test_policies_agree_for_new`,
// `test_staged_monotonic`).
//
// These `#[flux_rs::sig]` annotations and tests are bound to the local
// helpers in this standalone file, NOT to live production code in
// crates/vb_storage/src/batch/write.rs. The actual production duplicate
// accounting is enforced via `journal.events.contains_key(key)` in
// crates/vb_storage/src/batch/write_event.rs:19-25. None of the Flux
// refinements here bound to that production path.
//
// Per AGENTS.md GOD RULE 2 (No Vacuum Proofs), Flux refinements MUST
// mathematically bind to the actual Rust implementations in the
// production codebase. Refinements over local helpers in a standalone
// file are vacuous mathematical models, not production proofs.
//
// === STATUS: non_applicable / retired ===
// No `#[flux_rs::sig]` annotations, no `#[test]` functions, and no
// local helper code remain in this file. The Flux-rs lane decision
// (LD-vb-vzcuf-035-flux_rs) is registered as retired in
// .beads/vb-vzcuf/verifier-lane-decisions.jsonl.
//
// The domain claim (C2 — same-batch duplicate accounting follows the
// documented policy and preserves staged byte invariant) is now covered
// by:
//   - crates/vb_storage/src/batch/byte_accounting_tests.rs
//     (Rust behavior tests, red-queen v2)
//   - crates/vb_storage/src/batch/tests.rs (existing duplicate tests)
//   - crates/vb_storage/src/batch/write_event.rs::append_event
//     (contains_key guard at line 19)
//
// Source: .beads/vb-vzcuf/proof-obligations.planned.jsonl POB-vb-vzcuf-035
