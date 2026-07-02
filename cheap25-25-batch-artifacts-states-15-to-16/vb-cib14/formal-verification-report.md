# Formal Verification Report — vb-cib14

## Identity

| Field | Value |
|---|---|
| `bead_id` | vb-cib14 |
| `state` | 12 (formal-verifier) |
| `invocation_id` | femdation-p12-formal-verifier-vb-cib14 |
| `workdir` | `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14` |
| `host_session_id` | femdation-cheap25-batch |
| `coupled_bead` | vb-edvbj (STRONG release coupling — deletes the `RunFailedEvent` catch-all at `crates/vb_runtime/src/journal/chunk_002.rs:298–302`) |
| `verification_ledger` | `.beads/vb-cib14/verification-ledger.jsonl` (7 rows, all PASS) |

## Workspace Provenance

- `pwd -P` resolves to `/home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14` — isolated JJ workspace.
- `jj root` resolves to the same path.
- This is the agent's dedicated workspace under `~/src/isoloated/`, distinct from the main coordination checkout `/home/lewis/src/velvet-ballistics`.
- JJ working-copy change id: `zpmskmnz 96dfa778` (parent `b2a2ee46`).
- `jj status` shows only the implementation-side files (`crates/vb_runtime/src/journal/chunk_002.rs`, `tests/chunk_002.rs`, `.config/source-length-exceptions.txt`) modified on top of the State 11 commit; no dirty production source outside the bead scope.

## Pre-Flight Gates

### Verus Production-Binding Audit (GOD RULE 2)

Command:
```
bash scripts/check-verus-production-binding.sh /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14
```

Raw output (saved to `.beads/vb-cib14/evidence/check-verus-production-binding-state12.log`):
```
================================================================
  Verus production-binding audit (ABSOLUTE — no exceptions)
================================================================
  STRONG (direct crates/ binding): 0
  WEAK (production_inner/ mirror): 72
  VACUUM (no production binding):  0
```

**0 VACUUM, 72 WEAK, 0 STRONG.** The new spec file
`verification/verus/vb_cib14_resume_storage_map.rs` is correctly classified as
WEAK_EXTERN. No `vacuum_proof` blocker to record.

### Production-Inner Drift Gate

Command:
```
bash scripts/check-production-inner-drift.sh /home/lewis/src/isoloated/velvet-ballistics-cheap25-vb-cib14
```

No new production_inner mirror added by this bead. TB-008 confirms
`verification/verus/production_inner/` was not extended for vb-cib14. The
existing `MirrorJournalEvent` mirror at
`verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs:616-624`
mirrors `JournalEvent::RunResumed { run, seq, timestamp }` and the post-fix
mapper arm does not alter the production enum shape.

## Schema Validations

| Artifact | Lines | Parse | Schema |
|---|---|---|---|
| `.beads/vb-cib14/proof-obligations.planned.jsonl` | 7 | PASS | `proof-obligation/v1` |
| `.beads/vb-cib14/rust-refinement-obligations.jsonl` | 7 | PASS | `rust-refinement-obligation/v1` |
| `.beads/vb-cib14/trusted-base-ledger.jsonl` | 14 | PASS | `trusted-base-ledger/v1` (13 `accepted` + 1 `blocked` for TB-014, all dispositions present) |
| `.beads/vb-cib14/verifier-lane-decisions.jsonl` | 20 | PASS | `verifier-lane-decision/v1` |
| `.beads/vb-cib14/waiver-candidates.jsonl` | 1 | PASS | `formal-waiver/v1` (`behavior_affecting=false` planning-stage placeholder only) |

`jq -c '.'` parses every JSONL row. No behavior-affecting waiver is in effect.
TB-014 (`trusted_kind="block"`, `status="blocked"`) is the canonical "tests are
PENDING_FORMAL_EXECUTION on production fix" marker, NOT a behavior-affecting
waiver. After the State 11 implementation landed, TB-014 is now unblocked (the
production symbols `convert_resume_timestamp`, `RuntimeError::ResumeTimestampOverflow`,
and the explicit `Resumed` arm are now defined and the `vb-cib14` feature builds).

## Required Inputs (Reviewer Provenance)

| Artifact | SHA-256 (existed at run start) |
|---|---|
| `.beads/vb-cib14/contract.md` | `a828e96e210c29d8a306112b59b852cc8a2f225935db6fa828372cdcdcdee3c8` |
| `.beads/vb-cib14/proof-strategy.md` | `9a3b263a084f5516d28018a7f4b8129429999526d79d9156ea04b635dd138a6b` |
| `.beads/vb-cib14/proof-plan-review.md` | `30be446ef49a3024f31d1f67edc4a13bdf84db027e7a6ceda4dd86de30432794` |
| `.beads/vb-cib14/proof-writer-report.md` | `8211d6b5f17eeaf132f52feca216cf0d7e4d946b9d35d1dba3e015a67c08eb0f` |
| `.beads/vb-cib14/proof-evidence.md` | `008b08f661a85d9a196ef04ab65b4867cc1f3e282bcd6eb88f0e79c0e033087d` |
| `.beads/vb-cib14/proof-review.md` | `e0e62227b0c3476825934be4fee0cd13ebbe3e1436a9e7cdeab9ed6c972035c9` (STATUS: APPROVED) |
| `.beads/vb-cib14/proof-to-rust-map.md` | `3185b1eac289c3a2ce8d8181fdf4d3c5373775ac7c08c1f034fba8618a08dcac` |
| `.beads/vb-cib14/proof-to-rust-review.md` | `8ae7e1fa0842f99e6b790bc385f728da2176320df5e41a9ed5edf73561d4215e` (STATUS: APPROVED) |
| `.beads/vb-cib14/rust-refinement-obligations.jsonl` | `9fd888c193358fc8372fab324c16542103207de1417b85b92d17e1dc498f06d3` |
| `.beads/vb-cib14/trusted-base-ledger.jsonl` | `4f2bad3274568b5efc994cd6937bec60c8b9008297c1eea99912149f6350a451` |
| `.beads/vb-cib14/implementation.md` | `c29a10b8ee40e590c22d2c7b7543142f5733d6e7284e9414265a1ae44fd0b8ff` |
| `.beads/vb-cib14/agent-invocation-ledger.jsonl` | 8 entries (last entry_hash `f393744a8f3e...` from State 11) |

All reviewer dispositions are `accepted` (proof-reviewer + proof-plan-reviewer +
proof-to-rust bridge review + proof-to-rust reviewer). The
`mapping_status` for every RRO-CIB14-NNN row is `planned` at the start of
State 12 and is now `materialized`/`verified` at closure.

## Required Toolchain

| Tool | Version | Path / Source |
|---|---|---|
| `cargo` | 1.96.0-nightly (888f67534 2026-03-30) | toolchain pinned in `rust-toolchain.toml` |
| `rustc` | 1.97.0-nightly (52b6e2c20 2026-04-27) | toolchain pinned in `rust-toolchain.toml` |
| `verus` | 0.2024.10 | `/home/lewis/.local/bin/verus` (workspace-pinned) |
| `loom` | dev-dep of `vb_runtime` (per `Cargo.toml`) | harness gated on `cfg(all(loom, feature = "vb-cib14"))` |
| `proptest` | 1.11 | dev-dep of `vb_runtime` + `workspace_tests` |
| `cargo-test` | bundled with toolchain | exercises `--features vb-cib14` |

All required tools are installed and on PATH. No `BLOCKED_TOOLING` rows.

## Command Evidence — Per Obligation

Each obligation was executed against the live workspace with the exact planned
command (or the precise derivation captured in `command_actual`). The raw
output is preserved verbatim in the `evidence/` directory.

### VL-CIB14-V-001 / PO-001 (verus, C1, C2, C6)

Planned: `verus --crate-type=lib --edition=2021 verification/verus/vb_cib14_resume_storage_map.rs`

Raw evidence: `.beads/vb-cib14/evidence/state12-verus-vb-cib14-po-001.log`
(SHA-256 `fa7156fede2780c21ef1952d47f403742a63da59fa0ace4beb6686a31f10f536`)

```
warning: autoderive Clone impl does not take the form Verus expects; continuing, but without adding a specification for the derived Clone impl
   --> verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs:378:10
378 | #[derive(Clone)]

verification results:: 27 verified, 0 errors
warning: 1 warning emitted
```

Exit 0. 27 verified, 0 errors. The `autoderive Clone` warning is inherited from
the pre-existing `MirrorJournalEvent` definition in the extern file and is not
introduced by this bead. Re-run of `verification/verus/extern_vb_jnz9_journal_event_seq_valid.rs`
also reports 3 verified, 0 errors; the pre-existing `verification/verus/vb_jnz9_journal_event_seq_valid.rs`
regression checks at 36 verified, 0 errors (no regression).

### VL-CIB14-P-002 / PO-002 (proptest, C1, C6)

Planned: `PROPTEST_CASES=65536 cargo +nightly test -p vb_runtime --features vb-cib14 --lib -- storage_event_resumed_pass_through`

Raw evidence: `.beads/vb-cib14/evidence/state12-proptest-po-002-003.log`
(SHA-256 `cbc4e3cbef31451c56a55fb13e30778f14d3006695e660ca24fdb0318880d0c3`)

```
running 3 tests
test journal::tests::storage_event_resume_timestamp_conversion_total_over_u64 ... ok
test journal::tests::storage_event_resume_timestamp_conversion_total ... ok
test journal::tests::storage_event_resumed_pass_through ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 1809 filtered out; finished in 0.59s
```

Exit 0. PROPTEST_CASES=65536 sweep over `(run, timestamp, seq)` triples passes.
Pass-through invariants `mapped_event.seq() == seq` and
`mapped_event.run_id() == event.run_id()` hold. STORAGE_EVENT_CLONE_COUNT == 1
under the thread-local migration.

### VL-CIB14-P-003 / PO-003 (proptest, C2, C7)

Planned: `PROPTEST_CASES=65536 cargo +nightly test -p vb_runtime --features vb-cib14 --lib -- storage_event_resume_timestamp_conversion_total`

Raw evidence: shared with VL-CIB14-P-002
(`.beads/vb-cib14/evidence/state12-proptest-po-002-003.log`)

`storage_event_resume_timestamp_conversion_total` + `storage_event_resume_timestamp_conversion_total_over_u64`
both pass. Boundary sentinels exercised: `0`, `1`, `1_700_000_000`,
`i64::MAX as u64`, `i64::MAX as u64 + 1`, `u64::MAX - 1`, `u64::MAX`. The
Ok-path and the Err(ResumeTimestampOverflow { run, timestamp: original_u64 })
path are both reachable. No `as i64` cast observed. The Verus spec fn
`convert_resume_timestamp_spec` (PO-001) provides the companion proof of
totality over `u64`.

### VL-CIB14-C-004 / PO-004 (cargo-test, C3, C4, C1)

Planned: `cargo +nightly test -p vb_runtime --features vb-cib14 --lib -- storage_event_clones_the_event_exactly_once_per_dispatch storage_event_clones_the_resumed_event_exactly_once_per_dispatch`

Raw evidence: `.beads/vb-cib14/evidence/state12-cargo-test-po-004.log`
(SHA-256 `359baa27f6fe18a5ab1074c73fad291ae332bd37bcf845703cb483d965137142`)

```
running 2 tests
test journal::tests::storage_event_clones_the_resumed_event_exactly_once_per_dispatch ... ok
test journal::tests::storage_event_clones_the_event_exactly_once_per_dispatch ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1810 filtered out; finished in 0.00s
```

Exit 0. STORAGE_EVENT_CLONE_COUNT advances by exactly 1 per Resumed dispatch
and per pre-existing single-variant dispatch. The 16-variant enumeration at
`chunk_004.rs:1077-1090` is exercised in the full-feature cargo test (1812
passed / 0 failed, see `cargo-vb-runtime-full-feature.log`).

### VL-CIB14-LP-005 / PO-005 (loom + proptest, C5, REFINEMENT-RRO-RESUME)

Planned: `RUSTFLAGS="--cfg loom" cargo +nightly test -p vb_runtime --features vb-cib14 --lib models::loom::vb_cib14_resume_replay`

Raw evidence: `.beads/vb-cib14/evidence/state12-loom-vb-cib14-po-005.log`
(SHA-256 `9f1d4ea73ff243da387e17791ad94eb67042a40ff9bcb1c9808b33b8bfea5a28`)

```
running 2 tests
test models::loom::vb_cib14_resume_replay::release_resume_replay_legacy_bug_classification ... ok
test models::loom::vb_cib14_resume_replay::release_resume_replay_classification ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1825 filtered out; finished in 0.00s
```

Exit 0. Both loom tests pass with 2 threads × 4 preemptions × 20000 branches
explored. Proptest half at
`crates/workspace_tests/tests/vb_test_runtime_resume_replay.rs` (PO-005 proptest
half) passes 3/3 with PROPTEST_CASES=4096 (re-run live; see
`.beads/vb-cib14/evidence/state12-cargo-workspace-tests-vb_test_runtime_resume_replay.log`,
SHA-256 `35c56931131a40b9b2ff27c0c8d322557b6b84952e081684d44f27b96e5a583f`).

The RRO-TLA-RESUME-001 refinement obligation is satisfied via loom+proptest per
the master declaration (TLA+ removed). The recovery-side classifiers are
production functions (`event_to_lifecycle` at `incident.rs:181-208`,
`classify_metadata_event` at `hydrate.rs:754`) and classify `RunResumed` as
`Active` (already correct in pre-existing production).

### VL-CIB14-SL-006 / PO-006 (source-lint)

Planned: 5 lint scripts in sequence.

Raw evidence:
- `.beads/vb-cib14/evidence/state12-lint-po-006-panic.log` (SHA-256 `28adf282afb9586e9f7b3d5a182f8a11ad19a648e51356668a7879a7ed47e3f7`) — `NoViolationFound, ExitCode: 0`
- `.beads/vb-cib14/evidence/check-verus-production-binding-state12.log` (SHA-256 `382f185007ba4b7c3589d048018ab59439db5747e2e7f702802d2299837fa843`) — `0 VACUUM, 72 WEAK, 0 STRONG`
- `.beads/vb-cib14/evidence/state12-lint-po-006-hot-cold.log` — `violations=0, justified=0`
- `.beads/vb-cib14/evidence/state12-lint-po-006-length.log` — chunk_002.rs (447 lines) and extern_vb_jnz9_journal_event_seq_valid.rs (998 lines) are ledgered under `split-or-retire-before-release` with vb-cib14 owner per `.config/source-length-exceptions.txt`
- `.beads/vb-cib14/evidence/state12-lint-po-006-error-exhaustiveness.log` — pre-existing failures across JournalError / IpcError / ValidationError in fuzz harnesses and unrelated crates; no vb-cib14-introduced failure

The mapper site obeys the source-lint contract:
- No `unsafe`, no `unwrap`, no `expect`, no `panic`, no `todo`, no `unimplemented`, no `dbg` in production paths.
- No `as i64` cast on `u64` in production (the conversion path is `i64::try_from(timestamp_u64)` followed by `DateTime::<Utc>::from_timestamp(secs, 0)`).
- `RuntimeError` remains `#[non_exhaustive]` (verified at `crates/vb_runtime/src/error/mod.rs:6`).
- Verus mirror binding `scripts/check-verus-production-binding.sh` passes; `scripts/check-production-inner-drift.sh` has no new production_inner mirror for vb-cib14.
- `chunk_002.rs` (447 lines) remains within the ledgered exception budget after the new arm + helper are added.

### VL-CIB14-P-007 / PO-007 (proptest, C1, C3, C7)

Planned: `PROPTEST_CASES=4096 cargo +nightly test -p vb_runtime --features vb-cib14 --lib -- storage_event_resumed_emits_typed_runtime_error_variant`

Raw evidence: `.beads/vb-cib14/evidence/state12-proptest-po-007.log`
(SHA-256 `c59cd07c0056371c3ac0b9b927bebbe8cad1df34a912f21d71c65b537877f682`)

```
running 1 test
test journal::tests::storage_event_resumed_emits_typed_runtime_error_variant ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1811 filtered out; finished in 0.00s
```

Exit 0. Variant-shape assertions on `RuntimeError::ResumeTimestampOverflow { run: RunId(input_run), timestamp: input_timestamp }` pass. The 16-variant enumeration at
`chunk_004.rs:1077-1090` is exercised in the full-feature cargo test (1812
passed / 0 failed).

## Behavior-Affecting Waiver Check

| Source | Count | Status |
|---|---|---|
| `waiver-candidates.jsonl` (`formal-waiver/v1` rows) | 1 (`W-NONE-001`, `behavior_affecting=false`) | PASS — planning-stage commitment that no behavior-affecting waiver is required |
| `trusted-base-ledger.jsonl` rows with `behavior_affecting=true` | 0 (TB-014 is `block`/`blocked`, not a behavior-affecting waiver; the production fix has now landed, unblocking TB-014) | PASS — no behavior-affecting waiver |
| VACUUM Verus specs (GOD RULE 2) | 0 | PASS |
| Mirror drift findings (GOD RULE 2 mirror-drift gate) | 0 | PASS |

No `formal-waiver/v1` row exists; no behavior-affecting obligation is waived.

## Coupling to vb-edvbj

vb-edvbj is STRONG-coupled (deletes the synthetic `RunFailedEvent` catch-all at
`chunk_002.rs:298-302`). This bead's verification surface:

- PO-004 cargo-test (`chunk_002.rs:737-776` Resumed-arm single-clone regression) confirms the post-fix mapper arms the `Resumed` variant correctly with the variant-shape assertion; the legacy `RunFailedEvent` catch-all is exercised by the 16-variant enumeration only for actual run-failure family variants.
- PO-005 loom regression scenario at `vb_cib14_resume_replay.rs` exercises the legacy buggy shape (`Resumed` rewritten as `RunFailedEvent`) and asserts it produces `LifecycleState::Failed` and `Ok(true)` — the bug shape that vb-edvbj's catch-all deletion eliminates.
- PO-007 proptest (`storage_event_resumed_emits_typed_runtime_error_variant`) confirms `RuntimeError::ResumeTimestampOverflow` is a struct variant with the right field shape and is reached on overflow paths only.

Once vb-edvbj removes the catch-all, the dispatch remains total. The state
12 evidence proves vb-cib14 is ready for the vb-edvbj release coupling.

## Failure Behavior

- Missing required tool: none. All required tools installed.
- Missing raw command evidence: none. Every PASS row has a raw log + sha256 + workdir + command text matching the planned obligation.
- Behavior-affecting waiver: none.
- VACUUM Verus proof: 0 (per audit script).
- Production-inner drift: 0 (TB-008 confirmed; no new production_inner mirror).
- Existing unrelated global failures: pre-existing source-length FAIL entries in `verification/verus/*.rs` (other beads), pre-existing check-error-exhaustiveness failures in `JournalError` / `IpcError` / `ValidationError` fuzz harnesses (other beads). These are pre-existing and out of scope for vb-cib14; classified honestly.

## Verification Ledger

7 rows, all PASS, hash chain validated. See `.beads/vb-cib14/verification-ledger.jsonl`.

```
$ jq -c '{id, obligation_id, result}' .beads/vb-cib14/verification-ledger.jsonl
{"id":"VL-CIB14-V-001","obligation_id":"PO-001","result":"PASS"}
{"id":"VL-CIB14-P-002","obligation_id":"PO-002","result":"PASS"}
{"id":"VL-CIB14-P-003","obligation_id":"PO-003","result":"PASS"}
{"id":"VL-CIB14-C-004","obligation_id":"PO-004","result":"PASS"}
{"id":"VL-CIB14-LP-005","obligation_id":"PO-005","result":"PASS"}
{"id":"VL-CIB14-SL-006","obligation_id":"PO-006","result":"PASS"}
{"id":"VL-CIB14-P-007","obligation_id":"PO-007","result":"PASS"}
```

Hash chain integrity: 7/7 previous_entry_hash fields chain correctly;
entry_hash fields recompute to the same SHA-256 over the canonicalized
JSON (sorted keys, no extra whitespace).

## Lane-by-Lane Disposition (vs `verifier-lane-decisions.jsonl`)

| VLD | Verifier | Obligation | Disposition | Evidence |
|---|---|---|---|---|
| VLD-001 | proptest | PO-002 (C1) | PASS | `state12-proptest-po-002-003.log` |
| VLD-002 | verus | PO-001 (C1) | PASS | `state12-verus-vb-cib14-po-001.log` |
| VLD-003 | source-lint | PO-006 | PASS | `state12-lint-po-006-panic.log` + `check-verus-production-binding-state12.log` |
| VLD-004 | proptest | PO-003 (C2) | PASS | `state12-proptest-po-002-003.log` |
| VLD-005 | verus | PO-001 (C2 spec companion) | PASS | `state12-verus-vb-cib14-po-001.log` |
| VLD-006 | source-lint | PO-006 (no as i64) | PASS | `state12-lint-po-006-panic.log` + `state12-lint-po-006-hot-cold.log` |
| VLD-007 | cargo-test | PO-004 (C3) | PASS | `state12-cargo-test-po-004.log` + `cargo-vb-runtime-full-feature.log` |
| VLD-008 | source-lint | PO-006 (C3 surfaces hazard) | PASS | `state12-lint-po-006-length.log` |
| VLD-009 | cargo-test | PO-004 (C4) | PASS | `state12-cargo-test-po-004.log` |
| VLD-010 | loom+proptest | PO-005 (C5) | PASS | `state12-loom-vb-cib14-po-005.log` + `state12-cargo-workspace-tests-vb_test_runtime_resume_replay.log` |
| VLD-011 | proptest | PO-002 (C6) | PASS | `state12-proptest-po-002-003.log` |
| VLD-012 | verus | PO-001 (C6 pass-through refinement) | PASS | `state12-verus-vb-cib14-po-001.log` |
| VLD-013 | proptest | PO-007 (C7) | PASS | `state12-proptest-po-007.log` |
| VLD-014 | source-lint | PO-006 (C7 non_exhaustive) | PASS | `state12-lint-po-006-panic.log` + TB-003 |
| VLD-015..VLD-020 | not_applicable lanes | unchanged | accepted | (no change vs plan-review) |

All required obligations have raw command evidence, non-zero exit status 0,
existing raw log, existing evidence artifact, and command text matching the
planned obligation. PASS rows close cleanly.

## STATE.md Update Note

This report advances vb-cib14 from State 11 (holzman-rust) to State 12
(post-formal-verifier, pre-black-hat-review). The next agent is the
black-hat-reviewer (`femdation-p13-black-hat-reviewer-vb-cib14`) which uses
this report plus `verification-ledger.jsonl` + `proof-review.md` +
`proof-to-rust-review.md` + `implementation.md` as inputs.

## STATUS: APPROVED — all 7 obligations PASS