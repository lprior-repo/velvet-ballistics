// RETIRED: non_applicable — vacuum Kani proof.
// Obligation ID: POB-vb-vzcuf-014
// Verifier: kani
// Proof seed: vb-vzcuf-PS-004
// Contract clause: contract.md:C5
//
// === RETIRED IN BH-W0-S04 / FINDING-B-01 EXTENSION ===
// The Kani harnesses in this file claimed PRODUCTION BINDING to
// `JournalWriteBatch::staged_event_keys: HashSet<[u8; 17]>` — a production
// field that was dead code (no .insert()/.contains()/.remove() ever called)
// and was removed in commit 150e1489a (bead vb-u2psq).
//
// Per AGENTS.md GOD RULE 2 (No Vacuum Kani Proofs), Kani harnesses MUST
// bind to live production code. With the binding field removed, the
// harnesses became vacuous: they proved nothing about the actual
// `JournalWriteBatch::append_event` lifecycle in
// crates/vb_storage/src/batch/write_event.rs.
//
// === STATUS: non_applicable / retired ===
// No `#[kani::proof]` functions remain in this file. The Kani lane
// decision (LD-vb-vzcuf-014-kani) is registered as retired in
// .beads/vb-vzcuf/verifier-lane-decisions.jsonl.
//
// The domain claim (C5 — accumulated byte rejection leaves batch state
// unchanged) is now covered by:
//   - crates/vb_storage/src/batch/byte_accounting_tests.rs
//     (Rust behavior tests, red-queen v2)
//   - crates/vb_storage/src/batch/tests.rs (existing batch tests)
//
// Reference: .beads/vb-vzcuf/proof-obligations.planned.jsonl
// POB-vb-vzcuf-014 status will be updated to retired alongside this fix.
//
// Source: .beads/vb-vzcuf/proof-obligations.planned.jsonl POB-vb-vzcuf-014
