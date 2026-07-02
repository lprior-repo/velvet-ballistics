# Verifier Lane Matrix — vb-n5k6v

Maps each proof seed to its assigned verifier lanes. Built
from the 15 proof seeds in
`.beads/vb-n5k6v/proof-seeds.jsonl` and the 7
verifier-lane-decision rows per seed in
`verifier-lane-decisions.jsonl` (105 rows total).

**bead_id:** vb-n5k6v
**isolated_workdir:** /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-n5k6v
**contract:** `.beads/vb-n5k6v/contract.md` (10 contract clauses, CC-WIRE-001..010)
**proof seeds:** 15 (PS-WIRE-DECL-001..PS-WIRE-QUEUE-015)
**obligations:** 3 (PO-WIRE-DECL-001, PO-WIRE-RUN-004, PO-WIRE-DELTA-005)
**lane profile:** `default-rust` (cargo test) only; all 6 other formal
verifiers (`kani`, `verus`, `flux-rs`, `loom`, `miri`, `cargo-fuzz`)
are `not_applicable` for this bead.

---

## Matrix

| Proof Seed ID | Description | proptest | kani | verus | flux-rs | loom | miri | cargo-fuzz |
|---|---|---|---|---|---|---|---|---|
| PS-WIRE-DECL-001 | 3-line mod declaration at lib.rs:182 | ✅ required | — | — | — | — | — | — |
| PS-WIRE-NOPROD-002 | 0 production-logic change outside 3-line insert | — (constraint) | — | — | — | — | — | — |
| PS-WIRE-NOCROSS-003 | 0 cross-crate change | — (constraint) | — | — | — | — | — | — |
| PS-WIRE-RUN-004 | 26 surfaced tests all pass | ✅ required | — | — | — | — | — | — |
| PS-WIRE-COUNT-005 | test count delta = +26 (924 → 950) | ✅ required | — | — | — | — | — | — |
| PS-WIRE-LINES-006 | file line count unchanged (637) | — (constraint) | — | — | — | — | — | — |
| PS-WIRE-LEDGER-007 | source-length exception preserved | — (constraint) | — | — | — | — | — | — |
| PS-WIRE-UNIQ-008 | 26 test fn names unique across workspace | — (constraint) | — | — | — | — | — | — |
| PS-WIRE-CARGO-009 | Cargo.toml byte-identical | — (constraint) | — | — | — | — | — | — |
| PS-WIRE-LINT-010 | new declaration passes clippy | ✅ folded into PO-WIRE-DECL-001 | — | — | — | — | — | — |
| PS-WIRE-CONC-011 | 4 concurrent tests under default-Rust threading | ✅ folded into PO-WIRE-RUN-004 | — | — | — | — | — | — |
| PS-WIRE-CODEC-012 | 5 record-boundary tests complement kani_record_* | ✅ folded into PO-WIRE-RUN-004 | — | — | — | — | — | — |
| PS-WIRE-PERSIST-013 | 11 persistence tests with per-test tempdir | ✅ folded into PO-WIRE-RUN-004 | — | — | — | — | — | — |
| PS-WIRE-BATCH-014 | 3 batch tests pin cross-batch duplicate detection | ✅ folded into PO-WIRE-RUN-004 | — | — | — | — | — | — |
| PS-WIRE-QUEUE-015 | 3 queue tests pin terminal-shutdown rejection | ✅ folded into PO-WIRE-RUN-004 | — | — | — | — | — | — |

**Legend:**
- ✅ required — `applicability: required`; the obligation binds the
  proof seed to a `proof-obligation/v1` row.
- ✅ folded — `applicability: required`; the obligation is shared with
  another seed (multiple seeds bound to a single PO row).
- — (constraint) — the proof seed is a static constraint, not a
  behavior proof. Tracked in `proof-coverage-matrix.md` and
  `trusted-base-plan.md` boundary sections. No `proof-obligation/v1`
  row is required because the seed carries no behavior surface.
- — (not_applicable) — the verifier was evaluated and rejected with
  concrete non-applicability evidence; see
  `verifier-lane-decisions.jsonl` for the per-seed rationale.

## Non-Applicable Lanes (per verifier)

The following 6 formal verifiers are `not_applicable` for **every**
proof seed in this bead. Per-seed rationale is in
`verifier-lane-decisions.jsonl`; the summary rationale is:

| Lane | Reason |
|---|---|
| `kani` | No bounded state/control-flow proof target introduced by the wire. The wire is a module declaration; the 26 tests are concrete-value behavior tests. Existing `kani_record_*.rs` harnesses already exhaust the codec's input domain. |
| `verus` | No production-bound exec fn to verify. The wire is a 3-line `#[cfg(test)] #[path = "..."] mod ...;` declaration; no `requires`/`ensures` seam exists. A Verus mirror-only proof would violate the no-vacuum-Verus rule (AGENTS.md, God Rule 2). |
| `flux-rs` | No refinement type target. The wire introduces no new type with `#[refined_by]`; the 26 tests are concrete-value behavior tests. |
| `loom` | The 4 concurrent tests use `std::thread::spawn` + `Arc<FjallJournal>` with `FjallJournal::append_*` taking `&self` (interior mutability) and `JournalWriterQueue` wrapping `Mutex<InnerState>` at `queue/writer.rs:33`. The existing pattern in `journal/tests.rs:2598+` and `recovery/tests.rs` uses default-Rust threading without Loom. A Loom permutation model would add redundant coverage given the `&self` + `Mutex` serialization. |
| `miri` | vb_storage is `#![forbid(unsafe_code)]`; the 26 tests use safe Rust APIs only. No raw pointers, FFI, `MaybeUninit`, provenance, or aliasing-sensitive paths exist in the wire or the 26 tests. |
| `cargo-fuzz` | No hostile-input surface. The 26 tests use specific concrete values (`u64::MAX`, `u32::MAX`, 128 KiB-1 MiB payloads, specific magic bytes, etc.); they are not a fuzz target. Existing `kani_record_*.rs` harnesses already exhaust the codec's input domain at the type level. |

## Lane Pairing Summary

| Verifier | Required rows | Not-applicable rows | Total |
|---|---|---|---|
| proptest | 9 (across PS-WIRE-DECL-001, RUN-004, COUNT-005, LINT-010, CONC-011, CODEC-012, PERSIST-013, BATCH-014, QUEUE-015) | 6 (constraint-only seeds: NOPROD, NOCROSS, LINES, LEDGER, UNIQ, CARGO) | 15 |
| kani | 0 | 15 | 15 |
| verus | 0 | 15 | 15 |
| flux-rs | 0 | 15 | 15 |
| loom | 0 | 15 | 15 |
| miri | 0 | 15 | 15 |
| cargo-fuzz | 0 | 15 | 15 |
| **Total** | **9** | **96** | **105** |

## Default-Rust Lane Analog

The contract and delivery-scope both name `default-rust`
(cargo test) as the required lane. The proof-planner
`ALLOWED_VERIFIERS` schema does not include `default-rust`,
so the analog `proptest` is used. This mapping is
documented in `proof-strategy.md` §3 and every required
row in `verifier-lane-decisions.jsonl`. The actual
verification command is `cargo test -p vb_storage --lib edge_case`
(PO-WIRE-RUN-004), which runs in the standard `cargo test`
mode that proptest also uses. The `proptest` analog is the
closest formal-verifier entry; proptest itself is a no-op
for the 26 concrete-value tests (the test bodies contain
specific values, not `proptest!` strategies).

## Lane Decisions Cross-Reference

For the per-seed, per-verifier rationale (12 fields per row),
see `verifier-lane-decisions.jsonl` (105 rows). The decision
rows include:

- `id` (e.g., `vld-vb-n5k6v-decl-001-proptest`)
- `requirement_id` (e.g., `REQ-WIRE-001`)
- `contract_clause` (e.g., `CC-WIRE-001`)
- `proof_seed_id` (e.g., `PS-WIRE-DECL-001`)
- `verifier` (one of the 7 allowed values)
- `risk_tags` (mirrored from `proof-seeds.jsonl`)
- `applicability` (`required` or `not_applicable`)
- `decision_reason` (concrete rationale)
- `required_obligation_ids` (only set for `required` rows)
- `non_applicability_evidence_refs` (set for `not_applicable` rows)
- `limitation_kind` (`surface_absent`, `superseded_by_other_lane_with_evidence`,
  or `external_dependency_unavoidable`; empty for `required`)
- `owner_state` (4 for all rows)
- `status` (`planned` for `required`, `not_applicable` for others)

## Waiver Candidates

This bead has **zero waiver candidates**. The skill rule
"Never emit behavior-affecting waiver-candidate" applies:
all 3 obligations are `behavior_affecting: false` and the
6 not-applicable verifiers are documented in
`verifier-lane-decisions.jsonl` (not in
`waiver-candidates.jsonl`). The constraint-only seeds
(PS-WIRE-NOPROD-002, NOCROSS-003, LINES-006, LEDGER-007,
UNIQ-008, CARGO-009) require no waiver because they are
not behavior proofs; they are tracked in
`proof-coverage-matrix.md` and `trusted-base-plan.md` as
boundary conditions.

END OF VERIFIER LANE MATRIX.
