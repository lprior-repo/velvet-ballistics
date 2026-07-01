# Proof Strategy — vb-n5k6v

**bead_id:** vb-n5k6v
**title:** Tests: wire orphaned `edge_case_tests` or delete stale file (P1 bug)
**isolated_workdir:** /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-n5k6v
**state:** 4 (proof-planner)
**owner_state:** 4
**rerun_from:** 4
**invocation_id:** cheap25-vb-n5k6v-p4-proof-planner
**captured_at:** 2026-07-01T15:25:00Z

---

## 1. Bead profile

This is a **TEST-ONLY** repair bead. The 637-line file
`crates/vb_storage/src/edge_case_tests.rs` is dormant in the
cargo test compile graph because `crates/vb_storage/src/lib.rs`
does not declare the `mod edge_case_tests;` module. The bead
allows two resolution branches (wire as `[[test]]` integration
entry, or delete the file). The contract at `.beads/vb-n5k6v/contract.md`
selects **WIRE** (in-source `#[cfg(test)] #[path = "..."] mod ...;`
declaration) because the file uses intra-crate `use crate::{...}`
imports and accesses `pub(crate)` methods.

The wire is a 3-line insertion at `crates/vb_storage/src/lib.rs:182`,
between the existing `mod snapshot_tests;` (line 180-181) and
`pub mod queue;` (line 183), that mirrors the canonical pattern
of the 16 sibling `#[path = "<name>_tests.rs"] mod <name>;`
declarations at `lib.rs:118-181`.

**Production API change:** none.
**Public API change:** none.
**Cross-crate change:** none.
**Cargo.toml change:** none.
**Existing test breakage:** none (the 26 tests were dormant, not
broken — they are now wired into the compile graph).
**New test surface:** 26 default-Rust behavior tests.

## 2. Risk classification (proof-planner domain model)

| Risk class | Where | Severity | Notes |
|---|---|---|---|
| `build_graph` | wire at `lib.rs:182` | low | static module-resolution construct |
| `module_resolution` | wire at `lib.rs:182` | low | matches canonical 16-sibling pattern byte-for-byte |
| `test_orchestration` | 26 tests at `edge_case_tests.rs:36-635` | low | concrete-value behavior tests |
| `diff_hygiene` | wire changes 1 file, +3 lines, -0 | low | `git diff --stat` invariant |
| `concurrency` | 4 tests at lines 84, 123, 163, 199 | medium | `std::thread::spawn` + `Arc<FjallJournal>`; FjallJournal append is `&self`; JournalWriterQueue wraps `Mutex<InnerState>` at `queue/writer.rs:33` |
| `persistence` | 11 tests | low | per-test `tempfile::tempdir()` isolation |
| `parser/codec` | 5 tests at lines 443-530 | low | concrete-value complement to existing Kani harnesses |
| `file_size` | 637-line file | low | on `.config/source-length-exceptions.txt:150` (split-or-retire tracked by `vb-jpq7.47`) |
| `dependency` | `tempfile`, `proptest`, `blake3`, `fjall` | none | all already in `crates/vb_storage/Cargo.toml` |
| `user-visible-behavior` | none | n/a | wiring only affects `#[cfg(test)]` visibility |
| `migration` | none | n/a | no schema/version change |

No `risk_tags` entry in the proof seed matches the
`DEFAULT_RISK_PROFILE` classes (`arithmetic_overflow`, `index_safety`,
`panic_freedom`, `rejection`, `bounded_transition`, `equality`,
`ordering`, `field_sensitivity`, `illegal_state`, `refinement`,
`concurrency_interleaving`, `cancellation_safety`,
`shutdown_drain`, `temporal_liveness`, `temporal_safety`,
`ub_safety`, `hostile_input`, `parse_canonicalization`). The
`check_risk_profile_coverage` validator gate therefore does not
fire any `E_LANE_DECISION_MISSING` finding for this bead.

## 3. Verifier lane profile (decision summary)

Per `delivery-scope.jsonl` row 57 and `contract.md` "Verifier Lane
Profile" section, the lane profile is:

| Lane | Applicability | Verifier analog in this plan | Required? | Rationale |
|---|---|---|---|---|
| `default-rust` (cargo test) | required | `proptest` (closest formal analog; cargo test runs in `cargo test` mode) | **yes** | 26 surfaced tests must all pass; tally delta must be +26; lint clean; module compiles |
| `kani` | not_applicable | n/a | no | existing `kani_record_*.rs` already cover codec invariants; the wire has no symbolic input domain |
| `verus` | not_applicable | n/a | no | no production-bound exec fn to verify; the wire is a module declaration |
| `flux-rs` | not_applicable | n/a | no | no refinement type target |
| `loom` | not_applicable | n/a | no | default-Rust threading precedent in `journal/tests.rs:2598+` and `recovery/tests.rs`; FjallJournal append is `&self`; Queue wraps `Mutex<InnerState>` |
| `fuzz` | not_applicable | n/a | no | no hostile-input surface; tests are concrete-value |
| `proptest` | (folded into `default-rust`) | — | (subsumed) | the 26 tests are deterministic concrete-value tests; proptest is the closest formal-verifier analog for `cargo test` |
| `tla+` | removed | n/a | no | removed from the skill per SKILL.md; temporal workflows use loom + proptest |

The contract explicitly states: "default-rust (cargo test) |
REQUIRED". The proof-planner schema's `ALLOWED_VERIFIERS` does
not include "default-rust", so the lane analog is mapped to
`proptest` (the closest formal-verifier entry that runs in
`cargo test` mode). This mapping is documented in every
`verifier-lane-decision/v1` row and is the only verifier
analog used for `required` lanes.

## 4. Proof obligations (3 planned)

| ID | Requirement | Contract clause | Verifier | Behavior-affecting? | Mode |
|---|---|---|---|---|---|
| `PO-WIRE-DECL-001` | REQ-WIRE-001 | CC-WIRE-001 + CC-WIRE-010 | `proptest` | no | `verify-smoke` |
| `PO-WIRE-RUN-004` | REQ-WIRE-004 | CC-WIRE-004 | `proptest` | no | `verify-smoke` |
| `PO-WIRE-DELTA-005` | REQ-WIRE-005 | CC-WIRE-005 | `proptest` | no | `verify-smoke` |

`PO-WIRE-DECL-001` covers the 3-line declaration insertion
(CC-WIRE-001) and lint hygiene (CC-WIRE-010); `PO-WIRE-RUN-004`
covers the 26 surfaced tests including the 4 concurrent
(PS-WIRE-CONC-011), 5 codec (PS-WIRE-CODEC-012), 11 persistence
(PS-WIRE-PERSIST-013), 3 batch (PS-WIRE-BATCH-014), and 3 queue
(PS-WIRE-QUEUE-015) subsets; `PO-WIRE-DELTA-005` covers the
tally delta. The remaining 6 constraint-only clauses
(CC-WIRE-002, CC-WIRE-003, CC-WIRE-006, CC-WIRE-007,
CC-WIRE-008, CC-WIRE-009) are tracked in
`proof-coverage-matrix.md` and `trusted-base-plan.md` rather
than as separate obligations, because they are static hygiene
invariants with no behavior surface.

All 3 obligations have `behavior_affecting: false` because the
wire is a build-graph declaration; no production logic is
touched. The 26 dormant tests are pre-existing behavior
tests; the wire only restores them to active CI coverage.

## 5. Production binding (N/A — no Verus obligations)

This bead plans **zero Verus obligations**. The
`production-binding` gate in SKILL.md is therefore vacuously
satisfied: there is no `proof-obligation/v1` row with
`verifier: verus` to bind. The wire is a 3-line module
declaration; no `exec fn` with non-trivial bound is
introduced. The 26 tests use existing `pub` and `pub(crate)`
APIs that are already production-bound by the crate's existing
`#![forbid(unsafe_code)]` and the `Holzman Rust` doctrine.

## 6. Strategy summary

- **Primary:** `cargo test -p vb_storage --lib edge_case` (26
  tests pass) — `PO-WIRE-RUN-004`.
- **Tally:** `cargo test -p vb_storage --lib 2>&1 | tail -5`
  reports 1556 passed (1530 pre-wire + 26 delta) —
  `PO-WIRE-DELTA-005`.
- **Build + lint:** `cargo check -p vb_storage --tests` +
  `cargo clippy -p vb_storage --tests -- -D warnings` —
  `PO-WIRE-DECL-001`.
- **Diff hygiene:** `git diff --stat` shows 1 file changed,
  3 insertions, 0 deletions (CC-WIRE-002, trusted-base-plan
  section 7).
- **Cross-crate stability:** `cargo check --workspace` remains
  green (CC-WIRE-003, trusted-base-plan section 8).
- **Ledger preservation:** `.config/source-length-exceptions.txt:150`
  remains byte-identical (CC-WIRE-007, trusted-base-plan
  section 3).
- **Test name uniqueness:** `rtk rg` returns exactly 26 hits
  in `edge_case_tests.rs` (CC-WIRE-008, trusted-base-plan
  section 9).
- **Cargo.toml unchanged:** `git diff crates/vb_storage/Cargo.toml`
  returns empty (CC-WIRE-009, trusted-base-plan section 10).
- **File line count:** `rtk wc -l crates/vb_storage/src/edge_case_tests.rs`
  returns 637 (CC-WIRE-006, trusted-base-plan section 3).

## 7. Out-of-scope (downstream)

- **Splitting the 637-line file** is tracked by bead
  `vb-jpq7.47` (split-or-retire-before-release). The wire does
  not touch file size; the exception at
  `.config/source-length-exceptions.txt:150` is preserved.
- **Loom permutation model** for the 4 concurrent tests is
  optional and not required. The contract recommends
  default-Rust threading; the planner agrees based on
  `FjallJournal::append_*` taking `&self` and
  `JournalWriterQueue` wrapping `Mutex<InnerState>` at
  `queue/writer.rs:33`. The 4 concurrent tests follow the
  same pattern as `journal/tests.rs:2598+` and
  `recovery/tests.rs`.
- **Wiring the other 8 dormant `vb_storage` test files** in
  `to-fix/wave3/agent-09-verus.md:19,45` is out of scope. All
  8 are already wired at `lib.rs:123-180`; only
  `edge_case_tests.rs` remains unwired.
- **`to-fix/wave3/agent-07-test-reviewer.md:23`** clippy flag
  (E0453 on file-level `#![allow(...)]` blocks) is pre-existing
  and unrelated to the wire fix.

## 8. Handoff

- **State 4b (proof-plan-reviewer):** the
  `proof-plan-reviewer` skill dispositions each lane decision
  in `verifier-lane-decisions.jsonl` and writes
  `verifier-lane-review.jsonl` + `proof-plan-review.md`.
- **State 5 (proof-writer):** this bead has **no proof-writer
  work** because all 3 obligations are `verify-smoke` mode
  with `verifier: proptest` and the verification artifacts are
  pre-existing (`edge_case_tests.rs` is owned and
  ready-to-use; no new harness, spec, or model is required).
  The handoff note for proof-writer is "no work; the
  verification artifacts are the existing 26 tests in
  `edge_case_tests.rs` and the existing dev-deps in
  `crates/vb_storage/Cargo.toml`".
- **State 7 (proof-to-implementation):** the
  `proof-to-implementation` skill maps the 3 obligations to
  the 3-line `mod edge_case_tests;` insertion at
  `lib.rs:182` and the lint/run/tally gate commands.
- **State 12 (formal-verifier):** the
  `formal-verifier` skill executes the 3 obligations and
  closes the verification ledger. This is a `default-rust`
  (cargo test) closure; no Verus/Kani/Flux/Loom/Miri/Fuzz
  execution is required.

## 9. Forbidden actions (downstream enforcement)

The bead description and contract explicitly forbid:

- Modifying `crates/vb_storage/Cargo.toml`.
- Modifying any other module in `crates/vb_storage/src/`.
- Modifying any other crate (`vb_core`, `vb_runtime`, `vb_cli`,
  `vb_validate`, etc.).
- Modifying `.config/source-length-exceptions.txt:150` (the
  exception entry for `edge_case_tests.rs` must remain
  byte-identical).
- Modifying any file in `to-fix/wave3/`.

These forbidden actions are tracked in
`proof-coverage-matrix.md` §6 (boundary conditions) and
`trusted-base-plan.md` §7 (diff-hygiene boundary).

## 10. Strategy verdict

The plan is **lowest-blast-radius, highest-confidence**:

- 3 lines of code added.
- 1 file changed.
- 0 lines removed.
- 0 production-logic change.
- 0 cross-crate change.
- 0 Cargo.toml change.
- 26 dormant behavior tests restored to CI coverage.
- 0 new test-budget leak.
- 0 waiver needed.
- 0 formal-verifier execution required.

END OF PROOF STRATEGY.
