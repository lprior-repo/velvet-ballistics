// RETIRED: non_applicable — vacuum Kani proof.
// Obligation ID: POB-vb-vzcuf-022
// Verifier: kani
// Proof seed: vb-vzcuf-PS-006
// Contract clause: contract.md:C1
//
// === RETIRED IN BH-W0-S04 / FINDING-B-01 EXTENSION ===
// The Kani harnesses in this file modeled a `byte_limit` value object
// on `JournalWriteBatch` that does not exist in production. The PRODUCTION
// BINDING header claimed "byte_limit field will be added per contract C1"
// — but the field was never added, and the production struct in
// crates/vb_storage/src/batch/write.rs has no `byte_limit` field.
//
// The harnesses instead bound symbolic witnesses to production constants
// (MAX_JOURNAL_EVENT_PAYLOAD_BYTES, RECORD_HEADER_LEN, MAX_BATCH_COUNT)
// and asserted "non-zero" — which is a constant-evaluation, not a
// behavior-affecting proof of the `append_event` byte-accounting policy.
//
// Per AGENTS.md GOD RULE 2 (No Vacuum Kani Proofs), Kani harnesses MUST
// mathematically bind to live production code. These harnesses bound to
// constants, not behavior; they prove nothing about the actual
// `JournalWriteBatch::append_event` byte-limit guard in
// crates/vb_storage/src/batch/write_event.rs:37-53.
//
// === STATUS: non_applicable / retired ===
// No `#[kani::proof]` functions remain in this file. The Kani lane
// decision (LD-vb-vzcuf-022-kani) is registered as retired in
// .beads/vb-vzcuf/verifier-lane-decisions.jsonl.
//
// The domain claim (C1 — every open JournalWriteBatch has a non-zero
// byte limit and cannot be constructed unbounded) is now covered by:
//   - crates/vb_storage/src/batch/types.rs (BatchByteLimit type)
//   - crates/vb_storage/src/batch/write.rs (DEFAULT_JOURNAL_BATCH_BYTE_LIMIT
//     + BatchByteLimit::bounded default in `new`)
//   - crates/vb_storage/src/batch/byte_accounting_tests.rs
//     (Rust behavior tests, red-queen v2)
//
// Source: .beads/vb-vzcuf/proof-obligations.planned.jsonl POB-vb-vzcuf-022
