# Proof Coverage Matrix — vb-pg2wq duplicate-event test exact-contract repair

STATUS: PLANNED. No proof closure is claimed. All obligations are at `owner_state: 4, status: planned`.

## Coverage roll-up

| Requirement | Seed | Required behavior lanes/obligations | Non-applicable lanes recorded | Behavior tests cited |
|---|---|---|---|---|
| `O1-exact-tuple-pin-and-variant-discriminant` (ps001) | `vb-pg2wq-seed-ps001` | proptest `PO-vb-pg2wq-001` | verus, kani, flux-rs, loom, miri, cargo-fuzz | `ps001_duplicate_rejected` (`crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs:69-79`); field-bound guard pins run/seq against proptest inputs |
| `O1-exact-tuple-pin-and-variant-discriminant` (ps003) | `vb-pg2wq-seed-ps003` | proptest `PO-vb-pg2wq-001` | verus, kani, flux-rs, loom, miri, cargo-fuzz | `ps003_dup_fields` (`crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs:55-65`); field-bound guard delivers on the function name's promise |
| `O1-exact-tuple-pin-and-variant-discriminant` (ps004a) | `vb-pg2wq-seed-ps004a` | proptest `PO-vb-pg2wq-002` | verus, kani, flux-rs, loom, miri, cargo-fuzz | `ps004_no_persist` (`crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs:39-54`); field-bound guard + secondary invariants (b2.is_aborted, BatchAborted, events.len()==1) |
| `O1-exact-tuple-pin-and-variant-discriminant` (ps004b) | `vb-pg2wq-seed-ps004b` | proptest `PO-vb-pg2wq-002` | verus, kani, flux-rs, loom, miri, cargo-fuzz | `ps004_empty_commit_after_rej` (`crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs:84-98`); field-bound guard + secondary invariants (b2.is_aborted, BatchAborted) |
| `O1-exact-tuple-pin-and-variant-discriminant` (ps008) | `vb-pg2wq-seed-ps008` | proptest `PO-vb-pg2wq-001` | verus, kani, flux-rs, loom, miri, cargo-fuzz | `ps008_dup_before_queue` (`crates/vb_storage/tests/proptest_vb_vzcuf_PS_008.rs:27-36`); field-bound guard |
| `O1-exact-tuple-pin-and-variant-discriminant` (ps009) | `vb-pg2wq-seed-ps009` | proptest `PO-vb-pg2wq-001` | verus, kani, flux-rs, loom, miri, cargo-fuzz | `ps009_dup_rejected` (`crates/vb_storage/tests/proptest_vb_vzcuf_PS_009.rs:27-37`); field-bound guard; corresponding fuzz target `fuzz/fuzz_targets/vb_vzcuf_PS_009.rs` is OUT OF SCOPE |
| `O5-preserve-proptest-strategy` (class discipline) | `vb-pg2wq-seed-class-no-regression` | proptest `PO-vb-pg2wq-003` (source-lint scan) | verus, kani, flux-rs, loom, miri, cargo-fuzz | cross-cutting pattern-discipline scan: zero remaining weak `matches!(.., JournalError::DuplicateEvent { .. })` in `crates/vb_storage/tests/proptest_vb_vzcuf_PS_001/003/004/008/009.rs` |
| `O6-no-production-change` (binding strengthened) | `vb-pg2wq-seed-kani-binding-strengthened` | (no new obligation) | proptest, verus, kani, flux-rs, loom, miri, cargo-fuzz | existing Kani harness at `crates/vb_storage/src/kani_vb_vzcuf_ps004.rs:48-59` models DuplicateEvent { run, seq } with field-bound guard; runtime↔Kani alignment strengthened |

## Behavior tests cited as planned evidence

The planned evidence for PO-vb-pg2wq-001/002/003 is the workspace `cargo test -p vb_storage --test proptest_vb_vzcuf_PS_001/003/004/008/009` suite plus the source-lint scan.

| Test function | File | Lines | Obligation |
|---------------|------|-------|------------|
| `ps001_duplicate_rejected` | `crates/vb_storage/tests/proptest_vb_vzcuf_PS_001.rs` | 69-79 | `PO-vb-pg2wq-001` |
| `ps003_dup_fields` | `crates/vb_storage/tests/proptest_vb_vzcuf_PS_003.rs` | 55-65 | `PO-vb-pg2wq-001` |
| `ps004_no_persist` | `crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs` | 39-54 | `PO-vb-pg2wq-002` |
| `ps004_empty_commit_after_rej` | `crates/vb_storage/tests/proptest_vb_vzcuf_PS_004.rs` | 84-98 | `PO-vb-pg2wq-002` |
| `ps008_dup_before_queue` | `crates/vb_storage/tests/proptest_vb_vzcuf_PS_008.rs` | 27-36 | `PO-vb-pg2wq-001` |
| `ps009_dup_rejected` | `crates/vb_storage/tests/proptest_vb_vzcuf_PS_009.rs` | 27-37 | `PO-vb-pg2wq-001` |

All 6 functions are rewritten from weak to strong assertion (per contract.md §Per-File Change Specification); all are expected to pass under the strengthened assertion because the production contract at `crates/vb_storage/src/batch/append_event.rs:61-67` already returns the typed `run: event.run_id(), seq: event.seq()` tuple.

## Behavior-affecting status

Every obligation in this plan is `behavior_affecting: false`:

- The test rewrite is the only code change; production source is untouched (per contract.md §Obligation 6, codebase-map.md lines 312-326).
- The strengthened assertion pins the existing production contract; it does not modify production behavior.
- The Kani harness at `crates/vb_storage/src/kani_vb_vzcuf_ps004.rs:48-59` is unchanged; the runtime↔Kani binding is strengthened but not new.

`waiver-candidates.jsonl` is empty by design (no behavior-affecting waiver is made).

## Cross-reference

- `proof-strategy.md` — high-level strategy
- `verifier-lane-decisions.jsonl` — 56 rows of `(requirement_id, contract_clause, proof_seed_id, verifier)` decisions
- `verifier-lane-matrix.md` — rolled-up lane matrix
- `proof-obligations.planned.jsonl` — 3 planned obligations
- `trusted-base-plan.md` — trusted/abstracted surfaces
- `waiver-candidates.jsonl` — empty (no waivers planned)
- `contract.md` — bead contract (source of truth)
- `codebase-map.md` — codebase scout report
- `proof-seeds.jsonl` — proof seeds (8 rows)
- `traceability-matrix.jsonl` — per-row traceability (8 rows)
- `delivery-scope.jsonl` — delivery scope (26 rows)