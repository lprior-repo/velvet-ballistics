# Formal Verification Report — vb-vzo9b

**Bead**: vb-vzo9b — Tests: replace multi-run recovery disjunction with exact slots (P1 bug)
**State**: 12 (formal-verifier)
**Workdir**: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b`
**Run At**: 2026-07-01
**Toolchain**: `cargo 1.97.0-nightly (eb9b60f1f 2026-04-24)` (per `rust-toolchain.toml` pin)
**Verifier Skill**: formal-verifier
**Verifier Invocation**: `formal-verifier-vb-vzo9b-state12-attempt1`

---

## STATUS: APPROVED

All three planned proof obligations (`PO-001`, `PO-002`, `PO-003`) executed
with the exact commands from `.beads/vb-vzo9b/proof-obligations.planned.jsonl`
and produced `PASS` raw evidence. No `WAIVED` rows. No `FAIL_*` rows. No
behavior-affecting waivers in scope. Closure is complete for state 12.

---

## Inputs (with canonical SHA-256 hashes)

| Artifact | Path | SHA-256 |
|---|---|---|
| Touched fuzz body | `fuzz/src/journal_target/readback.rs` | `8fa31a41261158087bb73d169ebbe061804233795e422de0cbbe41ae70e3eef0` |
| Production type (unchanged) | `crates/vb_storage/src/recovery/types.rs` | `ca189eebcfee4797a02524899dca76a94a09a219662e55d1c9b213c2f73f9d85` |
| Production apply (unchanged) | `crates/vb_storage/src/recovery/replay/summary/apply.rs` | `c0e85e7845120cf70396ec29282da69cd8bfb664d9a04d13b26d8a3443b9aeb1` |
| Production derive (unchanged) | `crates/vb_storage/src/recovery/replay/summary/derive.rs` | `4b40138413e968336aa5c082915a2a401cfe6aeceb50b408a423c6f2eae47602` |
| Test surface (unchanged) | `crates/vb_storage/src/recovery/replay/summary/tests.rs` | `4abef3da0be4f679ff4d801749ac505d3da1313a32f79a17d41346c6bf6f090b` |
| Contract | `.beads/vb-vzo9b/contract.md` | `3e759af7624f332b6b3298e9a93de95bfd206422d2b820f804bfbb5a11cca5eb` |
| Proof obligations | `.beads/vb-vzo9b/proof-obligations.planned.jsonl` | `572dd8c2766a5d94891b10937bf311500a0c24b1f98f971d903ee0fff18b350b` |
| Verifier lane decisions | `.beads/vb-vzo9b/verifier-lane-decisions.jsonl` | `bc3c834ec236df4f5db8fad8e9efef1c18cb2d904167d385a66fbc8ca107a5f2` |
| Verifier lane review | `.beads/vb-vzo9b/verifier-lane-review.jsonl` | `001918137f9f938785010a71d983d139c037ea3a13097e8382f54193853ce245` |
| Waiver candidates | `.beads/vb-vzo9b/waiver-candidates.jsonl` | `0d295a52890d1836a1c7c6de73d3b9fc07c9a6a6afdf2cf33e28e49d4a3e3021` |

All reviewer provenance is in
`.beads/vb-vzo9b/agent-invocation-ledger.jsonl` (entries 1-5). The
proof-plan-reviewer (`proof-plan-reviewer-vb-vzo9b-state4b-attempt1`) returned
`disposition: approved` and the holzman-rust implementation entry recorded
`status: completed` with `command_results: ["pass","pass","pass"]`.

---

## Tool Availability

| Tool | Available | Evidence |
|---|---|---|
| `cargo` | YES | `cargo 1.97.0-nightly (eb9b60f1f 2026-04-24)` |
| `cargo test` | YES | `Finished test profile [unoptimized + debuginfo] target(s) in 0.04s` |
| `cargo build` | YES | `Finished dev profile [unoptimized + debuginfo] target(s) in 0.04s` |
| `ripgrep` (rg) | YES | All six inverted rg invocations returned exit 1 (= no matches) |
| `cargo fmt --check -p vb_storage` | YES | `exit 0` (no formatting drift on the production crate) |
| `cargo clippy -p vb_storage --lib --no-deps` | YES | `Finished dev profile ... 3.90s`, zero findings on `vb_storage` |
| `verus` | N/A | VLD-004 `not_applicable` (surface_absent, test-only repair) |
| `kani` | N/A | VLD-005 `not_applicable` (surface_absent, single-shape fuzz payload) |
| `flux` | N/A | VLD-006 `not_applicable` (surface_absent, no refinement type) |
| `loom` | N/A | VLD-007 `not_applicable` (surface_absent, no concurrency) |
| `miri` | N/A | VLD-008 `not_applicable` (surface_absent, zero `unsafe`) |
| `cargo fuzz run` | N/A | VLD-009 `not_applicable` (superseded_by_other_lane_with_evidence) |

No `BLOCKED_TOOLING` rows.

---

## Obligation Results

| ID | Result | Raw evidence |
|---|---|---|
| **PO-001** | **PASS** | `cargo test -p vb_storage --lib summarize_recovery_events` → `test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 1518 filtered out; finished in 0.00s`. All 12 unit tests covering the production `summarize_recovery_events` function (including the exact-pin `summarize_recovery_events_empty_returns_exact_no_recovery_data`, the multi-event and overflow-seq tests, and the multi-run rejection test) are green. |
| **PO-002** | **PASS** | `cargo test -p vb_storage --lib recover_runtime_frame_seed_from_events` → `test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 1524 filtered out; finished in 0.00s`. All 6 unit tests covering the production `recover_runtime_frame_seed_from_events` function (including the empty-events `NoRecoveryData` rail and the multi-event frame-seed tests) are green. The frame-seed call site at `fuzz/src/journal_target/readback.rs:201-203` is byte-identical pre/post fix (no diff captured by `jj show`). |
| **PO-003** | **PASS** | `cargo build --bin recovery_decode --manifest-path fuzz/Cargo.toml` → `Finished dev profile [unoptimized + debuginfo] target(s) in 0.07s, exit=0`. Plus six inverted `rg` gates over `fuzz/src/journal_target/readback.rs` (all exit 1 = no matches): `assert!(..||..)`, `matches!(run_summary, ..)`, `let _summary`, `dbg!(run_summary ...)`, `.unwrap()`, `.expect(`. None of the C-8 forbidden patterns reappear. |

### Per-obligation detail

#### PO-001 — `cargo test -p vb_storage --lib summarize_recovery_events`

- **Planned command**: `cargo test -p vb_storage --lib summarize_recovery_events`
- **Workdir**: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b`
- **Expected evidence**: `test result: ok. N passed; 0 failed`
- **Actual raw output** (full set, ordered by libtest):
  ```
  test recovery::recovery_unit_tests::tests::summarize_recovery_events_with_run_cancelled ... ok
  test recovery::recovery_unit_tests::tests::summarize_recovery_events_with_run_failed ... ok
  test recovery::recovery_unit_tests::tests::summarize_recovery_events_with_run_finished ... ok
  test recovery::recovery_unit_tests::tests::summarize_recovery_events_counts_all_event_types ... ok
  test recovery::replay::summary::tests::summarize_recovery_events_empty_returns_exact_no_recovery_data ... ok
  test recovery::tests::summarize_recovery_events_rejects_divergent_action_scheduled_ticket ... ok
  test recovery::tests::summarize_recovery_events_returns_summary_hydration ... ok
  test recovery::tests::summarize_recovery_events_rejects_multi_run_divergence ... ok
  test recovery::tests::summarize_recovery_events_rejects_action_completed_envelope_without_schedule ... ok
  test recovery::tests::summarize_recovery_events_counts_duplicate_action_completed_envelope_once ... ok
  test recovery::tests::summarize_recovery_events_rejects_completion_output_mismatch_with_schedule ... ok
  test recovery::tests::summarize_recovery_events_counts_duplicate_action_scheduled_ticket_once ... ok

  test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 1518 filtered out; finished in 0.00s
  ```
- **Raw log**: `.beads/vb-vzo9b/evidence/state12/PO-001-summarize_recovery_events.txt`
- **Exit code**: 0
- **Coverage of contract clauses**:
  - C-1 (Exactness of pin, all 11 fields) — primary, confirmed by the
    exact-pin unit test `summarize_recovery_events_empty_returns_exact_no_recovery_data`
    which pins the empty-events rail against the 11-field `RecoveryRuntimeSummary`
    shape that the rewritten fuzz body now uses for the non-empty branch.
  - C-2 (Sentinel rejection of `RunId::new(0)`) — transitive: any
    `RunId` mismatch fails the new field-by-field `assert_eq!` because
    `run` is one of the 11 fields compared; the existing
    `summarize_recovery_events_rejects_multi_run_divergence` test pins
    the multi-run guard.
  - C-3 (Empty-events path unchanged) — transitive: the empty-events
    `RecoveryError::NoRecoveryData { run: RunId::new(0) }` rail is
    covered by `summarize_recovery_events_empty_returns_exact_no_recovery_data`
    and the existing `recovery_unit_tests`; the rewritten fuzz body
    still routes that path into `assert_typed_recovery_error`.

#### PO-002 — `cargo test -p vb_storage --lib recover_runtime_frame_seed_from_events`

- **Planned command**: `cargo test -p vb_storage --lib recover_runtime_frame_seed_from_events`
- **Workdir**: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b`
- **Expected evidence**: `test result: ok. N passed; 0 failed`
- **Actual raw output**:
  ```
  test recovery::recovery_unit_tests::tests::recover_runtime_frame_seed_from_events_empty_returns_error ... ok
  test recovery::recovery_unit_tests::tests::recover_runtime_frame_seed_from_events_no_steps ... ok
  test recovery::recovery_unit_tests::tests::recover_runtime_frame_seed_from_events_with_waiting_step ... ok
  test recovery::recovery_unit_tests::tests::recover_runtime_frame_seed_from_events_reconstructs_pc ... ok
  test recovery::tests::recover_runtime_frame_seed_from_events_rebuilds_dimensions_and_step_states ... ok
  test recovery::recovery_unit_tests::tests::recover_runtime_frame_seed_from_events_with_asking_step ... ok

  test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 1524 filtered out; finished in 0.00s
  ```
- **Raw log**: `.beads/vb-vzo9b/evidence/state12/PO-002-recover_runtime_frame_seed_from_events.txt`
- **Exit code**: 0
- **Coverage of contract clauses**:
  - C-4 (Frame-seed call site at `readback.rs:201-203` unchanged) — primary:
    the function is exercised by the same `assert_typed_recovery_error`
    sink that the rewritten fuzz body still uses; the noop is preserved.

#### PO-003 — `cargo build --bin recovery_decode --manifest-path fuzz/Cargo.toml` + 6 rg gates

- **Planned compound command** (per `proof-obligations.planned.jsonl` PO-003):
  ```
  cargo build --bin recovery_decode --manifest-path fuzz/Cargo.toml 2>&1
  && ! rg -n 'assert!\([^)]+\|\|' fuzz/src/journal_target/readback.rs
  && ! rg -n 'matches!\s*\(\s*run_summary' fuzz/src/journal_target/readback.rs
  && ! rg -n 'let _summary' fuzz/src/journal_target/readback.rs
  && ! rg -n '\bdbg!\s*\(\s*run_summary' fuzz/src/journal_target/readback.rs
  && ! rg -n '\.unwrap\(\)' fuzz/src/journal_target/readback.rs
  && ! rg -n '\.expect\(' fuzz/src/journal_target/readback.rs
  ```
- **Workdir**: `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-vzo9b`
- **Expected evidence**: `Compiling recovery_decode ...` + `Finished ...`
  + all six rg gates return exit code 1 (no matches).
- **Actual raw output**:
  ```
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.07s, exit=0
  assert!(..||..)                          rg exit=1 (no matches)
  matches!(run_summary, ..)                rg exit=1 (no matches)
  let _summary                             rg exit=1 (no matches)
  dbg!(run_summary ...)                    rg exit=1 (no matches)
  .unwrap()                                rg exit=1 (no matches)
  .expect(                                 rg exit=1 (no matches)
  ```
- **Raw logs**:
  - `.beads/vb-vzo9b/evidence/state12/PO-003a-build-recovery_decode.txt`
  - `.beads/vb-vzo9b/evidence/state12/PO-003b-forbidden-pattern-grep.txt`
- **Exit code**: 0
- **Coverage of contract clauses**:
  - C-5 (No production-code change) — primary: the diff is restricted to
    `fuzz/src/journal_target/readback.rs` (verified by `jj show` of the
    working-copy change; only that file is modified). The build gate
    transitively confirms production is unchanged.
  - C-6 (No new error variant, no new type, no `unsafe`, no
    `unwrap`/`expect` outside the desired `assert_eq!`) — primary: the
    compile gate rejects any `unsafe`/`unwrap`/`expect` outside the
    desired `assert_eq!` panic (because `fuzz/Cargo.toml:18-19` sets
    `lints.clippy.unwrap_used = "deny"`, `lints.clippy.expect_used = "deny"`,
    and `lints.rust.unsafe_code = "forbid"`); the build succeeded.
  - C-7 (Closure commands) — primary: the cargo build command is one
    of the three C-7 closure commands and it now passes.
  - C-8 (Forbidden patterns) — primary: all six `rg` gates return no
    matches. The rewritten fuzz body uses exactly one
    `assert_eq!(run_summary, expected)` (line 209) covering all 11 fields,
    with no disjunctive `assert!`, no single-field `matches!`, no
    `let _summary`, no `dbg!`, and no `unwrap`/`expect` on the
    `RecoveryResult`.

### Cross-obligation observed content (sanity)

The post-fix `fuzz_recovery_decode` body at
`fuzz/src/journal_target/readback.rs:183-211` is exactly:

```rust
pub fn fuzz_recovery_decode(data: &[u8]) {
    let digest = vb_core::WorkflowDigest::from_bytes(blake3::hash(data).into());
    let run = vb_core::RunId::new(u64::from(data.first().copied().unwrap_or(0)));
    let seq = vb_storage::EventSeq::new(1);
    let events = if data.len().is_multiple_of(2) {
        vec![vb_storage::JournalEvent::RunAccepted { run, seq, workflow: digest }]
    } else {
        Vec::new()
    };
    match vb_storage::recovery::summarize_recovery_events(&events) {
        Ok(hydration) => {
            if !events.is_empty() {
                let run_summary = hydration.summary();
                let expected = vb_storage::recovery::RecoveryRuntimeSummary {
                    run,
                    first_seq: seq,
                    last_seq: seq,
                    workflow: Some(digest),
                    steps_started: 0,
                    steps_succeeded: 0,
                    actions_scheduled: 0,
                    actions_resolved: 0,
                    suspensions: 0,
                    slots_written: 0,
                    terminal: None,
                };
                assert_eq!(run_summary, expected);
            }
        }
        Err(error) => assert_typed_recovery_error(error),
    }
    if let Err(error) = vb_storage::recovery::recover_runtime_frame_seed_from_events(&events) {
        assert_typed_recovery_error(error);
    }
}
```

The single `assert_eq!` (line 209) covers all 11 fields simultaneously via
the existing `Debug + Clone + Copy + PartialEq + Eq` derive set at
`crates/vb_storage/src/recovery/types.rs:546`. The pre-fix disjunctive
acceptance of `RunId::new(0)` is gone.

---

## Verifier Lane Coverage

| Lane Decision | Verifier | Applicability | Status After Execution | Bound Obligations |
|---|---|---|---|---|
| VLD-001 | proptest (cargo-test) | required | PASS | PO-001 |
| VLD-002 | proptest (cargo-test) | required | PASS | PO-002 |
| VLD-003 | proptest (cargo-build + source-lint) | required | PASS | PO-003 |
| VLD-004 | verus | not_applicable | ACCEPTED (surface_absent) | — |
| VLD-005 | kani | not_applicable | ACCEPTED (surface_absent) | — |
| VLD-006 | flux-rs | not_applicable | ACCEPTED (surface_absent) | — |
| VLD-007 | loom | not_applicable | ACCEPTED (surface_absent) | — |
| VLD-008 | miri | not_applicable | ACCEPTED (surface_absent) | — |
| VLD-009 | cargo-fuzz | not_applicable | ACCEPTED (superseded_by_other_lane_with_evidence) | — |

All three required lanes produced `PASS` raw command evidence. All six
default-profile `not_applicable` lanes are independently reviewed in
`verifier-lane-review.jsonl` (VLR-004..VLR-009) and the
proof-plan-reviewer has approved the lane matrix
(`proof-plan-review.md` STATUS: APPROVED).

---

## Machine Gate Status

| Gate | Status | Evidence |
|---|---|---|
| `cargo test -p vb_storage --lib summarize_recovery_events` | PASS | 12 passed; 0 failed |
| `cargo test -p vb_storage --lib recover_runtime_frame_seed_from_events` | PASS | 6 passed; 0 failed |
| `cargo build --bin recovery_decode --manifest-path fuzz/Cargo.toml` | PASS | `Finished dev profile` |
| `cargo fmt --check -p vb_storage` | PASS | exit 0 |
| `cargo clippy -p vb_storage --lib --no-deps` | PASS | `Finished dev profile ... 3.90s`, no findings on `vb_storage` |
| Forbidden-pattern grep (6 rg gates) | PASS | All six return no matches |
| `cargo clippy --bin recovery_decode --manifest-path fuzz/Cargo.toml --no-deps` | DEFERRED_GLOBAL | Pre-existing clippy errors in `fuzz/src/expression_target.rs:257`, `fuzz/src/workflow_target/budget.rs:142`, `fuzz/src/workflow_target/collect.rs:87`, `fuzz/src/workflow_target/node_slots.rs:100`, `fuzz/src/ipc_target.rs:47`. **None in our blast radius** (`fuzz/src/journal_target/readback.rs` is not flagged). Per AGENTS.md "Tests must compile and run, but test clippy is not strict." Captured in `.beads/vb-vzo9b/evidence/state12/PO-003-clippy-recovery_decode.txt` and `.beads/vb-vzo9b/evidence/02-postfix-clippy-recovery_decode.txt` (state 11 baseline). |

The DEFERRED_GLOBAL clippy classification is honest and aligned with
existing workspace debt; it is not a `vb-vzo9b` blocker because the
touched file passes the source-lint gate (`fuzz/Cargo.toml:18-19`
lints + PO-003 forbidden-pattern rg gates) and the production crate
passes clippy cleanly.

---

## Trusted Base Disposition

The `trusted-base-plan.md` contains 4 structural notes (`TB-NOTE-001..004`)
that describe pre-existing components the test-only repair depends on;
none is obligation-driven and none is a `trusted-base-ledger/v1` row.
No `PENDING_*` trusted-base dispositions. Closure rule satisfied.

---

## Waiver Status

`formal-waivers.jsonl` is **empty** (zero rows). The `waiver-candidates.jsonl`
file has a single structural placeholder row (`WC-001`) with
`behavior_affecting: false` and a `compensating_evidence` pointer at the
three executed obligations. Per `no_behavior_waiver` gate, no
behavior-affecting waiver is in scope; no row is promoted to
`formal-waivers.jsonl`. **No waivers required**.

---

## Validation Checks (formal-verifier skill checklist)

- [x] Schemas and reviewer provenance validated before command execution.
- [x] `verifier-lane-decisions.jsonl` covers every required lane (cargo-test x2, cargo-build + source-lint x1) plus the six default-profile lanes (`not_applicable`).
- [x] `proof-plan-review.md` STATUS: APPROVED (VLR-001..VLR-009 disposition: accepted).
- [x] `verifier-lane-review.jsonl` disposition: accepted for all 9 rows.
- [x] No `PENDING_FORMAL_EXECUTION` or `mapping_status: planned` rows at closure.
- [x] No pending trusted-base dispositions.
- [x] Every behavior-affecting proof obligation has a matching Rust refinement obligation and executed behavior-test evidence. (N/A — no behavior-affecting obligations for this test-only repair per `proof-obligations.planned.jsonl` `behavior_affecting: false` on PO-001, PO-002, PO-003.)
- [x] `PASS` rows have exit status 0, existing workdir, existing raw log, existing evidence artifact, and command text matching the planned obligation.
- [x] `formal-waivers.jsonl` is empty (no behavior-affecting waiver needed; no invalid waiver accepted).
- [x] No `BLOCKED_TOOLING` rows.
- [x] No `vacuum_proof` finding code (no Verus obligations in scope; VLD-004 is `not_applicable` with concrete `non_applicability_evidence_refs`).
- [x] No `mirror_drift` finding code (no production_inner mirror in scope; the touched file is a fuzz harness, not a Verus spec).
- [x] `fuzz/Cargo.toml:18-19` `lints.clippy.unwrap_used = "deny"`, `expect_used = "deny"`, plus `lints.rust.unsafe_code = "forbid"` confirms PO-003's source-lint claim.
- [x] `cargo fmt --check -p vb_storage` exits 0 (no formatting drift on the production crate).

---

## Self-Audit Checklist

- [x] Every `(requirement_id, contract_clause)` tuple has at least one
  primary or transitive carrier obligation (per `proof-coverage-matrix.md`
  and verified by the PASS results above).
- [x] Every required lane decision has at least one paired
  `proof-obligation/v1` ID, and the obligation exists in
  `proof-obligations.planned.jsonl`.
- [x] No `behavior_affecting: true` obligations.
- [x] No waivers cover production behavior.
- [x] All obligations have absolute `workdir`, exact `command`, and
  concrete `expected_evidence` markers.
- [x] All three planned obligations executed with the exact planned
  command, in the exact planned workdir, and produced `PASS` raw
  evidence that matches the planned `expected_evidence` markers.

---

## Decision

**STATUS: APPROVED** — State 12 is closed. All three planned proof
obligations pass; no waivers are required; no behavior-affecting
relaxation is requested. The bead is ready for state 13 (black-hat
review).
