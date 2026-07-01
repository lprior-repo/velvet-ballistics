# Proof Coverage Matrix — vb-n5k6v

Maps each contract clause (from `.beads/vb-n5k6v/contract.md`)
to proof obligations, verifier lanes, and trusted-base boundary
references. This matrix is the traceability bridge between
the 10 contract clauses, the 15 proof seeds, the 3 planned
proof obligations, and the trusted-base assumptions.

**bead_id:** vb-n5k6v
**isolated_workdir:** /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-n5k6v
**contract clauses:** 10 (CC-WIRE-001..CC-WIRE-010)
**proof seeds:** 15 (PS-WIRE-DECL-001..PS-WIRE-QUEUE-015)
**planned obligations:** 3 (PO-WIRE-DECL-001, PO-WIRE-RUN-004, PO-WIRE-DELTA-005)

---

## 1. Clause → obligation mapping

| Contract clause | Description | Proof obligation(s) | Verifier | Mode | Status |
|---|---|---|---|---|---|
| **CC-WIRE-001** | 3-line mod declaration inserted | `PO-WIRE-DECL-001` (primary); also bounds PS-WIRE-DECL-001 | `proptest` | `verify-smoke` | planned |
| **CC-WIRE-002** | 0 production-logic change | (constraint) — `git diff --stat` shows 1 file, +3, -0; tracked in `trusted-base-plan.md` §7 | (none) | (n/a) | (constraint) |
| **CC-WIRE-003** | 0 cross-crate change | (constraint) — `cargo check --workspace` remains green; tracked in `trusted-base-plan.md` §8 | (none) | (n/a) | (constraint) |
| **CC-WIRE-004** | 26 surfaced tests all pass | `PO-WIRE-RUN-004` (primary); also bounds PS-WIRE-RUN-004, CONC-011, CODEC-012, PERSIST-013, BATCH-014, QUEUE-015 | `proptest` | `verify-smoke` | planned |
| **CC-WIRE-005** | test count delta = +26 (1530 → 1556) | `PO-WIRE-DELTA-005` (primary); also bounds PS-WIRE-COUNT-005 | `proptest` | `verify-smoke` | planned |
| **CC-WIRE-006** | file line count unchanged (637) | (constraint) — `rtk wc -l` returns 637; tracked in `trusted-base-plan.md` §3 | (none) | (n/a) | (constraint) |
| **CC-WIRE-007** | source-length exception preserved | (constraint) — `.config/source-length-exceptions.txt:150` byte-identical; tracked in `trusted-base-plan.md` §3 | (none) | (n/a) | (constraint) |
| **CC-WIRE-008** | 26 test fn names unique across workspace | (constraint) — `rtk rg` returns 26 hits, all in `edge_case_tests.rs`; tracked in `trusted-base-plan.md` §9 | (none) | (n/a) | (constraint) |
| **CC-WIRE-009** | Cargo.toml byte-identical | (constraint) — `git diff crates/vb_storage/Cargo.toml` empty; tracked in `trusted-base-plan.md` §10 | (none) | (n/a) | (constraint) |
| **CC-WIRE-010** | new declaration passes clippy | folded into `PO-WIRE-DECL-001` (also bounds PS-WIRE-LINT-010) | `proptest` | `verify-smoke` | planned |

**Legend:**
- Primary obligation: a `proof-obligation/v1` row whose
  `contract_clause` field equals the listed clause and whose
  `verifier` row is `required` in `verifier-lane-decisions.jsonl`.
- Constraint: a static-hygiene invariant verified by `git diff`,
  `rtk wc -l`, `rtk rg`, or `cargo check --workspace`. No
  proof-obligation row is required because the constraint carries
  no behavior surface; the constraint is tracked in
  `trusted-base-plan.md` instead.

## 2. Seed → obligation mapping

| Proof seed | Contract clause | Risk tags | Required? | Binds to obligation | Verifier-lane-decision row |
|---|---|---|---|---|---|
| `PS-WIRE-DECL-001` | CC-WIRE-001 | build_graph, module_resolution, test_orchestration | yes | `PO-WIRE-DECL-001` | `vld-vb-n5k6v-decl-001-proptest` |
| `PS-WIRE-NOPROD-002` | CC-WIRE-002 | blast_radius, diff_hygiene | no (constraint) | (none) | (no required row) |
| `PS-WIRE-NOCROSS-003` | CC-WIRE-003 | cross_crate, api_stability | no (constraint) | (none) | (no required row) |
| `PS-WIRE-RUN-004` | CC-WIRE-004 | test_orchestration, build_graph | yes | `PO-WIRE-RUN-004` | `vld-vb-n5k6v-run-004-proptest` |
| `PS-WIRE-COUNT-005` | CC-WIRE-005 | test_orchestration, evidence | yes | `PO-WIRE-DELTA-005` | `vld-vb-n5k6v-count-005-proptest` |
| `PS-WIRE-LINES-006` | CC-WIRE-006 | file_size, source_length | no (constraint) | (none) | (no required row) |
| `PS-WIRE-LEDGER-007` | CC-WIRE-007 | source_length, ledger_preservation | no (constraint) | (none) | (no required row) |
| `PS-WIRE-UNIQ-008` | CC-WIRE-008 | test_orchestration, name_uniqueness | no (constraint) | (none) | (no required row) |
| `PS-WIRE-CARGO-009` | CC-WIRE-009 | cargo_manifest, dependency_stability | no (constraint) | (none) | (no required row) |
| `PS-WIRE-LINT-010` | CC-WIRE-010 | lint_hygiene, clippy | yes (folded) | `PO-WIRE-DECL-001` | `vld-vb-n5k6v-lint-010-proptest` |
| `PS-WIRE-CONC-011` | CC-WIRE-004 | concurrency, threading, mutex_serialization | yes (folded) | `PO-WIRE-RUN-004` | `vld-vb-n5k6v-conc-011-proptest` |
| `PS-WIRE-CODEC-012` | CC-WIRE-004 | parser/codec, magic_kind_family, payload_bounds | yes (folded) | `PO-WIRE-RUN-004` | `vld-vb-n5k6v-codec-012-proptest` |
| `PS-WIRE-PERSIST-013` | CC-WIRE-004 | persistence, fjall_keyspace, tempdir_isolation | yes (folded) | `PO-WIRE-RUN-004` | `vld-vb-n5k6v-persist-013-proptest` |
| `PS-WIRE-BATCH-014` | CC-WIRE-004 | batch_builder, duplicate_event_detection | yes (folded) | `PO-WIRE-RUN-004` | `vld-vb-n5k6v-batch-014-proptest` |
| `PS-WIRE-QUEUE-015` | CC-WIRE-004 | writer_queue, shutdown_terminal_state | yes (folded) | `PO-WIRE-RUN-004` | `vld-vb-n5k6v-queue-015-proptest` |

**Total:** 15 proof seeds. 9 bind to a required obligation
(3 obligations total, with 6 of the seeds folded into the
2 behavior obligations). 6 are constraint-only and bind to
no obligation.

## 3. Test surface inventory (CC-WIRE-004)

The 26 tests in `crates/vb_storage/src/edge_case_tests.rs`,
grouped by topic bucket (per the file's section comments):

| Bucket | Test fn | Line | Folds into obligation |
|---|---|---|---|
| Disk full | `persist_strict_handles_simulated_failure` | 36 | `PO-WIRE-RUN-004` (via PS-WIRE-PERSIST-013) |
| Disk full | `persist_strict_recovers_after_simulated_failure` | 58 | `PO-WIRE-RUN-004` (via PS-WIRE-PERSIST-013) |
| Concurrent | `multiple_threads_append_to_different_runs` | 84 | `PO-WIRE-RUN-004` (via PS-WIRE-CONC-011) |
| Concurrent | `concurrent_enqueue_to_writer_queue` | 123 | `PO-WIRE-RUN-004` (via PS-WIRE-CONC-011) |
| Concurrent | `concurrent_batch_writes_from_multiple_threads` | 163 | `PO-WIRE-RUN-004` (via PS-WIRE-CONC-011) |
| Concurrent | `concurrent_read_while_another_writes` | 199 | `PO-WIRE-RUN-004` (via PS-WIRE-CONC-011) |
| Very large | `very_large_blob_payload` | 249 | `PO-WIRE-RUN-004` (via PS-WIRE-PERSIST-013) |
| Very large | `very_large_compiled_ir_payload` | 263 | `PO-WIRE-RUN-004` (via PS-WIRE-PERSIST-013) |
| Very large | `very_large_workflow_source_payload` | 277 | `PO-WIRE-RUN-004` (via PS-WIRE-PERSIST-013) |
| Very large | `very_large_snapshot_with_many_slots` | 291 | `PO-WIRE-RUN-004` (via PS-WIRE-PERSIST-013) |
| Very large | `very_large_run_header_values` | 313 | `PO-WIRE-RUN-004` (via PS-WIRE-PERSIST-013) |
| Very large | `many_events_per_run` | 331 | `PO-WIRE-RUN-004` (via PS-WIRE-PERSIST-013) |
| Open/close | `rapid_open_close_cycles_preserve_data` | 358 | `PO-WIRE-RUN-004` (via PS-WIRE-PERSIST-013) |
| Open/close | `rapid_open_close_without_writes` | 385 | `PO-WIRE-RUN-004` (via PS-WIRE-PERSIST-013) |
| Open/close | `open_append_close_reopen_verify` | 400 | `PO-WIRE-RUN-004` (via PS-WIRE-PERSIST-013) |
| Record boundary | `encode_rejects_unknown_magic` | 443 | `PO-WIRE-RUN-004` (via PS-WIRE-CODEC-012) |
| Record boundary | `encode_accepts_run_header_with_index_magic` | 462 | `PO-WIRE-RUN-004` (via PS-WIRE-CODEC-012) |
| Record boundary | `encode_accepts_index_update_with_index_magic` | 481 | `PO-WIRE-RUN-004` (via PS-WIRE-CODEC-012) |
| Record boundary | `decode_rejects_zero_max_payload_with_nonzero_payload` | 500 | `PO-WIRE-RUN-004` (via PS-WIRE-CODEC-012) |
| Record boundary | `encode_rejects_zero_length_payload_serialization` | 523 | `PO-WIRE-RUN-004` (via PS-WIRE-CODEC-012) |
| Batch | `batch_commit_then_second_batch_with_same_run_seq_rejected` | 537 | `PO-WIRE-RUN-004` (via PS-WIRE-BATCH-014) |
| Batch | `batch_len_zero_after_digest_mismatch_abort` | 560 | `PO-WIRE-RUN-004` (via PS-WIRE-BATCH-014) |
| Batch | `empty_batch_strict_commits_successfully` | 575 | `PO-WIRE-RUN-004` (via PS-WIRE-BATCH-014) |
| Queue | `queue_capacity_one_single_enqueue_dequeue` | 588 | `PO-WIRE-RUN-004` (via PS-WIRE-QUEUE-015) |
| Queue | `queue_drain_all_with_large_batch_relative_to_capacity` | 601 | `PO-WIRE-RUN-004` (via PS-WIRE-QUEUE-015) |
| Queue | `queue_rejects_all_writes_after_shutdown` | 616 | `PO-WIRE-RUN-004` (via PS-WIRE-QUEUE-015) |

## 4. Verifier lane coverage summary

| Lane | Required | Not-applicable | Total seeds covered |
|---|---|---|---|
| `proptest` | 9 | 6 | 15 (all 15) |
| `kani` | 0 | 15 | 15 |
| `verus` | 0 | 15 | 15 |
| `flux-rs` | 0 | 15 | 15 |
| `loom` | 0 | 15 | 15 |
| `miri` | 0 | 15 | 15 |
| `cargo-fuzz` | 0 | 15 | 15 |

## 5. Behavior-affecting vs. constraint-only

| Obligation | Behavior-affecting? | Why |
|---|---|---|
| `PO-WIRE-DECL-001` | **no** | The 3-line mod declaration is a build-graph construct; no production logic changes. |
| `PO-WIRE-RUN-004` | **no** | The 26 tests are pre-existing dormant tests; the wire only restores them to active CI coverage. The tests' pass/fail behavior was already defined when they were written in 2026-05-23 (commit `a95354665`). |
| `PO-WIRE-DELTA-005` | **no** | Tally is a static property of the cargo test result summary; no runtime behavior change. |

All 3 obligations are `behavior_affecting: false`. This
satisfies the bead's "Behavior: false" requirement and
the skill's "Never emit behavior-affecting waiver-candidate"
rule (no waivers needed because all required obligations
are non-behavior-affecting).

## 6. Boundary conditions (constraints tracked but not as obligations)

These constraints are tracked in `trusted-base-plan.md`
boundary sections rather than as `proof-obligation/v1` rows,
because they are static hygiene invariants with no behavior
surface:

| Constraint | Verification command | Trusted-base ref |
|---|---|---|
| CC-WIRE-002 (0 production-logic change) | `git diff --stat` shows 1 file, +3, -0 | `trusted-base-plan.md#section-7` |
| CC-WIRE-003 (0 cross-crate change) | `cargo check --workspace` remains green | `trusted-base-plan.md#section-8` |
| CC-WIRE-006 (file line count = 637) | `rtk wc -l crates/vb_storage/src/edge_case_tests.rs` returns 637 | `trusted-base-plan.md#section-3` |
| CC-WIRE-007 (source-length exception preserved) | `.config/source-length-exceptions.txt:150` byte-identical | `trusted-base-plan.md#section-3` |
| CC-WIRE-008 (test name uniqueness) | `rtk rg` returns 26 hits, all in `edge_case_tests.rs` | `trusted-base-plan.md#section-9` |
| CC-WIRE-009 (Cargo.toml unchanged) | `git diff crates/vb_storage/Cargo.toml` empty | `trusted-base-plan.md#section-10` |

## 7. Forbidden actions (downstream enforcement)

The bead description and contract explicitly forbid:

- **Modifying `crates/vb_storage/Cargo.toml`** — covered by
  CC-WIRE-009; verified by `git diff`.
- **Modifying any other module in `crates/vb_storage/src/`** —
  covered by CC-WIRE-002; verified by `git diff --stat`.
- **Modifying `.config/source-length-exceptions.txt:150`** —
  covered by CC-WIRE-007; verified by `rtk rg`.

These are tracked in `trusted-base-plan.md` §11 (forbidden
actions boundary).

## 8. Pre-wire baseline evidence

| Evidence | Source | Value |
|---|---|---|
| Pre-wire `cargo test -p vb_storage --lib` tally (current) | `PROPTEST_CASES=1 cargo test -p vb_storage --lib 2>&1 | tail -3` from isolated workdir on 2026-07-01 | 1530 tests |
| Pre-wire `cargo test -p vb_storage --lib` tally (`historic_2026_05_baseline`) | `.beads/vb-2bok/qa-report.md:5` | 924 tests (May 2026 capture; not current) |
| Pre-wire `cargo test -p vb_storage --lib` tally (`historic_2026_05_baseline`, alt capture) | `.beads/vb-core-atomic-admission/STATE.md:1349` | 924 tests (May 2026 capture; not current) |
| Pre-wave-3 dormant-test audit | `to-fix/wave3/agent-09-verus.md:19,45` | 9 dormant files in `vb_storage`; 8 already wired at `lib.rs:123-180`; only `edge_case_tests.rs` remains |
| Sibling pattern (16 wired declarations) | `crates/vb_storage/src/lib.rs:118-181` | matches byte-for-byte |
| File line count (pre-wire) | `rtk wc -l crates/vb_storage/src/edge_case_tests.rs` | 637 lines |
| Source-length exception (pre-wire) | `.config/source-length-exceptions.txt:150` | present, owner `lewis`, removal plan `vb-jpq7.47` |
| Test name uniqueness (pre-wire) | `rtk rg` | 26 hits, all in `edge_case_tests.rs` |
| Cargo.toml (pre-wire) | `crates/vb_storage/Cargo.toml` | 32 lines, dev-deps `proptest` + `tempfile` |

## 9. Coverage summary

| Category | Count | Notes |
|---|---|---|
| Contract clauses | 10 | CC-WIRE-001..CC-WIRE-010 |
| Proof seeds | 15 | PS-WIRE-DECL-001..PS-WIRE-QUEUE-015 |
| Planned obligations | 3 | PO-WIRE-DECL-001, PO-WIRE-RUN-004, PO-WIRE-DELTA-005 |
| Behavior-affecting obligations | 0 | all 3 are `behavior_affecting: false` |
| Default-rust (cargo test) lanes required | 3 (one per obligation) | CC-WIRE-001+010, CC-WIRE-004, CC-WIRE-005 |
| Constraint-only seeds | 6 | NOPROD, NOCROSS, LINES, LEDGER, UNIQ, CARGO |
| Verifier-lane-decision rows | 105 | 15 seeds × 7 verifiers |
| Waiver candidates | 0 | no behavior-affecting waiver needed |

END OF PROOF COVERAGE MATRIX.
