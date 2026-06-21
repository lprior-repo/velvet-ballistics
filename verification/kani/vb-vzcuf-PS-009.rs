// RETIRED: non_applicable — vacuum Kani proof.
// Obligation ID: POB-vb-vzcuf-034
// Verifier: kani
// Proof seed: vb-vzcuf-PS-009
// Contract clause: contract.md:C2 open product question
//
// === RETIRED IN BH-W0-S04 / FINDING-B-01 EXTENSION ===
// The Kani harnesses in this file claimed PRODUCTION BINDING to
// `JournalWriteBatch::append_event` for "duplicate accounting" — but
// the duplicate-accounting policy was enforced via the now-removed
// `staged_event_keys: HashSet<[u8; 17]>` field. That field was dead
// code (no .insert()/.contains()/.remove() ever called) and was removed
// in commit 150e1489a (bead vb-u2psq).
//
// The earlier annotation `=== REMOVED IN COMMIT 150e1489a ===` correctly
// noted the binding was gone, but the file still declared four
// `#[kani::proof]` harnesses that bound symbolic witnesses to production
// constants and proved only:
//   - encode_record is deterministic (a constant-level property, not a
//     behavior-affecting claim about append_event),
//   - arithmetically valid bounds on staged bytes arithmetic,
//   - the `JOURNAL_KEY_BYTES` constant is non-zero and <= 256.
//
// Per AGENTS.md GOD RULE 2 (No Vacuum Kani Proofs), Kani harnesses MUST
// mathematically bind to live production code. The actual production
// duplicate accounting is enforced via `journal.events.contains_key(key)`
// in crates/vb_storage/src/batch/write_event.rs:19-25 — none of these
// harnesses bound to that path.
//
// === STATUS: non_applicable / retired ===
// No `#[kani::proof]` functions remain in this file. The Kani lane
// decision (LD-vb-vzcuf-034-kani) is registered as retired in
// .beads/vb-vzcuf/verifier-lane-decisions.jsonl.
//
// The domain claim (C2 — same-batch duplicate accounting follows the
// documented policy) is now covered by:
//   - crates/vb_storage/src/batch/byte_accounting_tests.rs
//     (Rust behavior tests, red-queen v2)
//   - crates/vb_storage/src/batch/tests.rs (existing duplicate tests)
//   - crates/vb_storage/src/batch/write_event.rs::append_event
//     (contains_key guard at line 19)
//
// Source: .beads/vb-vzcuf/proof-obligations.planned.jsonl POB-vb-vzcuf-034
