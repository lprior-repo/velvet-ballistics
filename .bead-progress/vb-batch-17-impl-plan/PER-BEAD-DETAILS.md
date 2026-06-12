# Per-Bead Implementation Details: 17 Ready Beads

**Generated:** 2026-06-11
**Beads:** vb-a408a, vb-6xh6c, vb-81urc, vb-9ckqp, vb-f2xk3, vb-benchbn01, vb-wstlsl01, vb-strecov01, vb-stortst01, vb-ybzsz, vb-dxi1k, vb-27jox, vb-jy6re, vb-e6xr7, vb-9zy8r, vb-p9owu, vb-32gwc

---

## vb-a408a — Reconcile Stale ArrayQueue Prose with Gate 8 Scope

**ID:** vb-a408a · P1 · decision · OPEN
**Owner:** Lewis
**Created:** 2026-06-11

### Description
> BHR-E5LFM confirmed the bead prose points at crossbeam_queue::ArrayQueue::new / ShardCommandQueue, but current vb_validate Gate 8 scope has no ArrayQueue, crossbeam_queue, or ShardCommandQueue path.
>
> Decision needed:
> - Retarget parent/child work to the runtime ShardCommandQueue/ArrayQueue Kani scope, or rewrite vb-e5lfm/vb-8o7p5 prose as current vb_validate Gate 8 timeout and harness-correctness work.
> - Align the canonical Kani budget: bead text says 120s while .moon/tasks/kani.yml uses timeout 15m.
> - Decide whether verify-kani-vb-validate belongs in moon ci / .moon.yml and whether vb_validate Kani harness groups need feature isolation.

### Research Findings

**Critical Problem:** vb-e5lfm (parent) references Kani harnesses that DO NOT EXIST:
- `kani_gate_08_shard_command_queue_drain` — ZERO matches
- `kani_gate_08_shard_command_queue_push` — ZERO matches
- `kani_gate_08_shard_command_queue_bounded_invariant` — ZERO matches
- `kani_gate_08_initial_state` — ZERO matches

**Actual Gate 8 Scope (accessor path validation only):**
- `crates/vb_validate/src/kani_gate_08_accessor.rs` — 7 harnesses
- `crates/vb_validate/src/kani_gate_08_structural.rs` — 14 harnesses
- Moon CI runs only 4 of 21 total

**Actual ShardCommandQueue location:**
- `crates/vb_runtime/src/shard/queue.rs` — domain wrapper around `ArrayQueue<ShardCommand>`
- NOT part of vb_validate Gate 8 scope

### Decision Required

| Option | Action |
|--------|--------|
| **A: Retarget** | Create new Kani harnesses in `vb_runtime` for ShardCommandQueue/ArrayQueue |
| **B: Rewrite** | Update vb-e5lfm prose to match actual accessor-validation Gate 8 scope |

### Implementation Actions

1. Choose Option A or B
2. Update vb-e5lfm prose to reflect decision
3. Remove all ArrayQueue/crossbeam_queue references from parent bead prose
4. Reconcile budget claims (bead says 120s, moon uses 15m)
5. Close vb-a408a with reconciliation evidence

### Acceptance Criteria
- [ ] Scope decision documented and approved
- [ ] vb-e5lfm prose updated to match decision
- [ ] No stale ArrayQueue unwind-8 acceptance criterion used
- [ ] vb-a408a closed

### Files Affected
- `.beads/vb-e5lfm/` — prose update
- `crates/vb_validate/src/kani_gate_08_*.rs` — scope reference
- `crates/vb_runtime/src/shard/queue.rs` — potential new harness target

---

## vb-6xh6c — Repair Remaining Gate 8 Kani 120s Timeouts

**ID:** vb-6xh6c · P1 · chore · OPEN
**Owner:** Lewis
**Created:** 2026-06-11

### Description
> BHR-E5LFM found diagnostic Gate 8 Kani timeouts under a 120s per-harness budget. vb-e5lfm repaired two current-scope semantic harness defects and proved them with raw targeted logs, but it did not close the full family or parent blocker.
>
> Scope:
> - Re-run the full vb_validate Gate 8 accessor + structural family under the selected canonical budget.
> - Capture raw per-harness logs with command, exit status, duration, and verifier verdict.
> - Repair remaining solver bombs by harness design or feature isolation; do not blindly raise unwind bounds or timeouts.
> - Keep proofs bound to crate::gates::validate_gate_08_accessor_path_segments.
> - Classify strict RUSTFLAGS=-Dwarnings blockers separately from verifier timeouts.

### Moon CI Harnesses

| Harness | Unwind | File |
|---------|--------|------|
| `kani_gate_08_valid_zero_accessors_pass` | — | accessor.rs |
| `kani_gate_08_arbitrary_parts_valid_accessors_pass` | 5 | structural.rs |
| `kani_gate_08_arbitrary_parts_root_oob_rejected` | 5 | structural.rs |
| `kani_gate_08_arbitrary_parts_symbol_oob_rejected` | 5 | structural.rs |

### Implementation Actions

1. Archive per-harness raw Kani logs to `.evidence/vb-6xh6c/`
2. Repair solver bombs using bounded `kani::any()` construction
3. Use feature isolation for complex harnesses
4. **Do NOT** blindly raise unwind bounds
5. Classify RUSTFLAGS blockers separately

### Acceptance Criteria
- [ ] Per-harness raw logs archived
- [ ] Every in-scope harness verifies within 120s OR has child bead with blocker
- [ ] No blind unwind bound increases
- [ ] `moon run :verify-kani-vb-validate` exits 0

---

## vb-81urc — Wire test-determinism into CI

**ID:** vb-81urc · P1 · task · OPEN
**Owner:** Lewis
**Created:** 2026-06-08

### Description
> `.moon/tasks/all.yml` — the `test-determinism` task is currently NOT marked `runInCI: true`. The 1,088 findings from the prior test-determinism audit must be archived as the baseline before the task can run in CI without flooding the dashboard.
>
> 3-child decomposition:
> - **T0** (3h): archive the 1,088 findings as the baseline.
> - **T2-T3** (5h): make the task exit-0 on the archived baseline.
> - **T4-T5** (55h): set `runInCI: true` and verify CI stays green.

### Current Findings (1,093 total)

| Category | Count |
|----------|-------|
| SharedTempState | 791 |
| UncontrolledClock | 254 |
| UncontrolledRandom | 31 |
| GlobalMutableState | 15 |
| SleepAsSync | 2 |

### Three-Child Decomposition

**T0 — Archive Baseline (3h)**
- Create `.evidence/test-determinism/baseline.txt`
- Verify no duplicates

**T2-T3 — CI/Dev Mode (5h)**
- Add `MOON_CI` detection to `check-test-determinism.py`
- CI mode: diff against baseline, exit 0 if matches baseline
- Dev mode: show full diff, exit 0 only if zero NEW findings

**T4-T5 — Wire into CI (55h)**
- Flip `runInCI: false` → `runInCI: true` in `.moon/tasks/all.yml:194`
- Run `moon ci` locally
- Iterate fixes for determinism issues uncovered

### Files to Modify
1. `.evidence/test-determinism/baseline.txt` — create
2. `scripts/check-test-determinism.py` — add CI/dev modes
3. `.moon/tasks/all.yml:194` — flip runInCI

### Acceptance Criteria
- [ ] Baseline file exists with 1,088+ findings
- [ ] Script supports CI/dev modes
- [ ] `runInCI: true` in task config
- [ ] `moon ci` runs test-determinism green

---

## vb-9ckqp — Red Queen BLOCKER: vb_validate Pre-Existing-Build

**ID:** vb-9ckqp · P2 · bug · OPEN
**Owner:** Lewis
**Created:** 2026-06-10

### Description
> [Red Queen] BLOCKER: pre-existing-build: cargo check -p vb_validate 2>&1 | grep -c 'error\[' --expect_exit 0

### Problem

`grep -c` returns **exit 0 when matches found**, **exit 1 when no matches found**.
- vb_validate compiles clean → 0 errors → grep exits 1
- `--expect_exit 0` expects grep to find matches (exit 0)
- Test fails because grep exits 1 (no errors found)

### Fix

Change `--expect_exit 0` → `--expect_exit 1` in the Red Queen smoke test.

### Implementation
Find the Red Queen smoke test file using this command and update the flag.

### Acceptance Criteria
- [ ] `cargo check -p vb_validate 2>&1 | grep -c 'error\[' --expect_exit 1` passes

---

## vb-f2xk3 — Red Queen BLOCKER: vb_core Pre-Existing-Build

**ID:** vb-f2xk3 · P2 · bug · OPEN
**Owner:** Lewis
**Created:** 2026-06-10

### Description
> [Red Queen] BLOCKER: pre-existing-build: cargo check -p vb_core 2>&1 | grep -c 'error\[' --expect_exit 0

### Problem

Same issue as vb-9ckqp. vb_core compiles clean → grep exits 1 → pipeline fails.

### Fix

Change `--expect_exit 0` → `--expect_exit 1`.

### Acceptance Criteria
- [ ] `cargo check -p vb_core 2>&1 | grep -c 'error\[' --expect_exit 1` passes

---

## vb-benchbn01 — Add Missing Benchmark Groups

**ID:** vb-benchbn01 · P2 · bug · OPEN
**Owner:** Lewis
**Created:** 2026-06-08

### Description
> crates/vb_benchmark/benches/ has 20 of 22 master §39 bench groups registered. Missing: warm_throughput (measure warm-cache throughput), digest_computation (measure BLAKE3-256 + CRC32C computation throughput). Create crates/vb_benchmark/benches/warm_throughput.rs and digest_computation.rs; add to [[bench]] in vb_benchmark/Cargo.toml; verify cargo bench --no-run exits 0. Master §39 line 1796.

### Missing Benchmarks

| Group | Description |
|-------|-------------|
| `warm_throughput` | Warm-cache throughput (distinct from cold_start) |
| `digest_computation` | BLAKE3-256 + CRC32C computation throughput |

### Files to Create

**`crates/vb_benchmark/benches/warm_throughput.rs`**
- Follow pattern from `crates/workspace_tests/benches/cold_start.rs`
- `BENCH_METADATA` constant
- `c.benchmark_group("warm_throughput")`

**`crates/vb_benchmark/benches/digest_computation.rs`**
- Measure `blake3::hash()` throughput
- Measure `crc32c::crc32c()` throughput

### Cargo.toml Changes

```toml
[dev-dependencies]
blake3.workspace = true
crc32c.workspace = true

[[bench]]
name = "warm_throughput"
harness = false

[[bench]]
name = "digest_computation"
harness = false
```

### Acceptance Criteria
- [ ] Both benchmark files created
- [ ] Both wired in `[[bench]]`
- [ ] `cargo bench --no-run` exits 0

---

## vb-wstlsl01 — Delete Self-Laundering Tests

**ID:** vb-wstlsl01 · P2 · bug · OPEN
**Owner:** Lewis
**Created:** 2026-06-08

### Description
> crates/workspace_tests/tests/section17_runtime_code_reverse_parity.rs:35-50 hardcodes the 11 dead-letter codes in SECTION_17_UNMAPPED and asserts they must NOT appear in runtime_code() output — locking in the gap. crates/workspace_tests/tests/section17_runtime_code_coverage_report.rs:159-217 documents the gap as "Future: X not yet implemented." Delete the UNMAPPED / PARTIALLY_MAPPED sections; have the tests fail loudly when codes are missing. Master §17 contract obligation.

### 11 Missing Section 17 Codes

| Code | Status |
|------|--------|
| REFERENCE_MISSING | UNMAPPED |
| STEP_SKIPPED_REFERENCE | UNMAPPED |
| RETRY_EXHAUSTED | UNMAPPED |
| RESULT_REFERENCE_MISSING | UNMAPPED |
| PAYLOAD_TOO_LARGE | UNMAPPED |
| REPLAY_DIVERGED | UNMAPPED |
| SECRET_UNAVAILABLE | PARTIALLY_MAPPED |
| WAIT_TIMEOUT | UNMAPPED |
| ASK_TIMEOUT | UNMAPPED |
| FOR_EACH_ITEM_FAILED | UNMAPPED |
| TOGETHER_BRANCH_FAILED | UNMAPPED |

### Delete

| File | Lines | Item |
|------|-------|------|
| section17_runtime_code_reverse_parity.rs | 42-50 | `SECTION_17_UNMAPPED` constant |
| section17_runtime_code_reverse_parity.rs | 244-271 | `section17_reverse_parity_unmapped_codes_have_no_sources` |
| section17_runtime_code_coverage_report.rs | 197-222 | `UNMAPPED_CODES_WITH_RATIONALE` |
| section17_runtime_code_coverage_report.rs | 224-227 | `PARTIALLY_MAPPED_CODES` |
| section17_runtime_code_coverage_report.rs | 248-257 | `section17_coverage_report_unmapped_codes_stay_unmapped` |

### Rename

| File | Old | New |
|------|-----|-----|
| section17_runtime_code_reverse_parity.rs | `SECTION_17_MAPPED` | `SECTION_17_GOLDEN` (33 codes) |
| section17_runtime_code_reverse_parity.rs | mapped test | `section17_reverse_parity_every_golden_code_has_source` |
| section17_runtime_code_coverage_report.rs | `MAPPED_CODES` | `SECTION_17_GOLDEN` (33 codes) |

### Acceptance Criteria
- [ ] `rg "SECTION_17_UNMAPPED" crates/` returns 0
- [ ] `rg "UNMAPPED_CODES_WITH_RATIONALE" crates/` returns 0
- [ ] `rg "PARTIALLY_MAPPED_CODES" crates/` returns 0
- [ ] `cargo test -p workspace_tests section17` FAILS (waiting for 11 missing codes)

---

## vb-strecov01 — Add error_recovery Test for Fuzz-Malformed Journal Records

**ID:** vb-strecov01 · P2 · bug · OPEN
**Owner:** Lewis
**Created:** 2026-06-08

### Description
> Add 10 unit tests at crates/vb_storage/src/recovery/tests/error_recovery_tests.rs covering: truncated payload, swapped magic, corrupted CRC32C, BLAKE3 digest mismatch, payload_len overflow, header_len mismatch, record_kind outside 1..=50, duplicate sequence number, gap in sequence, unknown record_kind family. Each test asserts replay returns a typed StorageError matching the mutation class.

### Test Matrix

| # | Mutation | Expected JournalError |
|---|---------|----------------------|
| 1 | truncated payload | `UnexpectedEof` |
| 2 | swapped magic | `BadMagic { found: 0x5642_534E }` |
| 3 | corrupted CRC32C | `HeaderChecksumMismatch` |
| 4 | BLAKE3 digest mismatch | `PayloadDigestMismatch` |
| 5 | payload_len overflow | `PayloadTooLarge { len, max }` |
| 6 | header_len mismatch | `HeaderLengthMismatch { found }` |
| 7 | record_kind outside 1..=50 | `UnknownRecordKind { kind: 99 }` |
| 8 | duplicate sequence | `SequenceGap` |
| 9 | gap in sequence | `SequenceGap` |
| 10 | unknown record_kind family | `RecordKindFamilyMismatch` |

### Wire Format (60 bytes)

```
[0..4]   magic          u32
[4..6]   schema_version u16
[6..8]   record_kind    u16
[8..12]  header_len     u32  (must be 60)
[12..16] payload_len    u32
[16..24] sequence       u64
[24..56] payload_digest BLAKE3 (32 bytes)
[56..60] header_checksum CRC32C
```

### Acceptance Criteria
- [ ] File created at correct path
- [ ] 10 deterministic tests
- [ ] No `unsafe`, `unwrap`, `proptest`
- [ ] `cargo test -p vb_storage error_recovery` passes

---

## vb-stortst01 — Split tests.rs Under 300-Line Cap

**ID:** vb-stortst01 · P2 · bug · OPEN
**Owner:** Lewis
**Created:** 2026-06-08

### Description
> crates/vb_storage/src/tests.rs:1-8091 is a single integration test file with 1,200+ #[test] functions. Violates the 300-line cap. Split into ~30 test modules of ~250 LoC each.

### Current State
- **File:** `crates/vb_storage/src/tests.rs`
- **Size:** 8,215 lines
- **Tests:** 328 `#[test]` functions
- **Declared in:** `lib.rs:211` as `pub mod tests;`

### Proposed Split

```
crates/vb_storage/src/tests/
├── mod.rs                    # ~20 lines — mod declarations
├── tests_key_encoding.rs      # ~250 lines
├── tests_envelope.rs         # ~250 lines
├── tests_journal_ops.rs       # ~250 lines
├── tests_header_adversarial.rs # ~250 lines
├── tests_recovery.rs         # ~250 lines
├── tests_snapshot.rs         # ~250 lines
├── tests_blob.rs             # ~250 lines
├── tests_index.rs            # ~250 lines
├── tests_batch.rs            # ~250 lines
├── tests_durability.rs       # ~250 lines
├── tests_error_exhaustive.rs # ~250 lines
├── tests_lock_enforcement.rs # ~250 lines (vb-apn5)
├── tests_recovery_stamp.rs   # ~250 lines (vb-1cwhx)
└── ... (~30 files total)
```

### Implementation Actions
1. Create `tests/` directory
2. Extract test functions into thematic files
3. Create `tests/mod.rs` with `mod` declarations
4. Update `lib.rs` — change `pub mod tests;` to use `#[path = "tests/mod.rs"]` or restructure
5. Run `cargo test -p vb_storage` — all 328 tests must pass

### Acceptance Criteria
- [ ] `tests.rs` replaced with `tests/` directory
- [ ] All files under 300 lines
- [ ] All 328 tests still pass
- [ ] `moon :source-length` exits 0

---

## vb-ybzsz — Wire flux-check-package.sh and Loom Task into Moon

**ID:** vb-ybzsz · P2 · chore · OPEN
**Owner:** Lewis
**Created:** 2026-06-08

### Description
> `scripts/flux-check-package.sh` and `scripts/loom-list.sh` exist but no moon tasks wrap them. Flux refinements and Loom models are verified by ad-hoc invocation, not in CI.

### 3 Sub-Bead Decomposition

**vb-ybzsz.1 — Flux (1h)**
- Create `.moon/tasks/flux.yml`
- Tasks: `flux-check-vb-compile`, `flux-check-vb-runtime`
- Wire into `.moon.yml` pipeline

**vb-ybzsz.2 — Loom-run (2h)** ⚠️ Depends on vb-lbg3h
- Create `.moon/tasks/loom.yml`
- Task: `loom-run` — loops 5 models
- Requires xtask in workspace

**vb-ybzsz.3 — Loom-list-smoke (0.5h)**
- Task: `loom-list-smoke`
- Runs `bash scripts/loom-list.sh`

### 5 Loom Models

1. `journal_writer_queue`
2. `action_completion_cancel`
3. `timer_fired_cancel`
4. `shutdown_drain`
5. `bounded_queue`

### Files to Create

**`.moon/tasks/flux.yml`**
```yaml
tasks:
  flux-check-vb-compile:
    command: 'bash scripts/flux-check-package.sh vb_compile'
    toolchains: [rust]
    runInCI: true
  flux-check-vb-runtime:
    command: 'bash scripts/flux-check-package.sh vb_runtime'
    toolchains: [rust]
    runInCI: true
```

**`.moon/tasks/loom.yml`**
```yaml
tasks:
  loom-list-smoke:
    command: 'bash scripts/loom-list.sh'
    toolchains: [rust]
    runInCI: true
  loom-run:
    command: |
      for model in journal_writer_queue action_completion_cancel timer_fired_cancel shutdown_drain bounded_queue; do
        cargo xtask loom --model "$model"
      done
    toolchains: [rust]
    runInCI: true
```

### Acceptance Criteria
- [ ] `.moon/tasks/flux.yml` created with 2 tasks
- [ ] `.moon/tasks/loom.yml` created with 2 tasks
- [ ] Both wired into `.moon.yml` pipeline
- [ ] All 4 tasks exit 0

---

## vb-dxi1k — Add verify-kani-vb-validate to Pipeline

**ID:** vb-dxi1k · P2 · chore · OPEN
**Owner:** Lewis
**Created:** 2026-06-08

### Description
> `.moon.yml:12` lists `- verify-kani`. The sibling task `verify-kani-vb-validate` is defined in `.moon/tasks/kani.yml:37-61` with `runInCI: true` but is NOT in the pipeline. The 4 `kani_gate_08_*` harnesses execute ZERO times in CI.

### One-Line Fix

Insert `- verify-kani-vb-validate` after `- verify-kani` in `.moon.yml:12`.

```yaml
# Before
  - 'verify-kani'
  - 'nightly-feature-gate'

# After
  - 'verify-kani'
  - 'verify-kani-vb-validate'
  - 'nightly-feature-gate'
```

### ⚠️ Dependency

Depends on `vb-e5lfm` — Kani unwinds 5→8 must be in place before landing.

### Acceptance Criteria
- [ ] `.moon.yml` contains the new entry
- [ ] `moon run :test` exits 0 first
- [ ] `moon run :verify-kani-vb-validate` exits 0
- [ ] All 4 harnesses complete

---

## vb-27jox — Split output.rs Under 300-Line Cap

**ID:** vb-27jox · P2 · chore · OPEN
**Owner:** Lewis
**Created:** 2026-06-08

### Description
> (a) `crates/vb_cli/src/output.rs` is 303 lines, exceeding the 300-line cap. (b) `infer_legacy_json_error_code` is a 12-clause `if message.contains(...)` chain that HOLZMAN §3 FORBIDS.

### Status Update

**`infer_legacy_json_error_code`:** Already deleted from both `output.rs` and `output_utils.rs`.

**`output.rs`:** Currently 280 lines (already under cap).

### Remaining Work

Split into `output/` directory:

| File | Est. Lines | Contents |
|------|-----------|----------|
| `output/format.rs` | ~30 | format helpers |
| `output/io.rs` | ~90 | write_* functions |
| `output/json.rs` | ~120 | JSON/serialization |
| `output/compat.rs` | ~10 | re-exports |

### Acceptance Criteria
- [ ] `output/` directory with 4 files
- [ ] All files under 300 lines
- [ ] `cargo build -p vb_cli` exits 0
- [ ] `moon :lint-src` exits 0

---

## vb-jy6re — Split cli_postcard/types.rs Under 300-Line Cap

**ID:** vb-jy6re · P2 · chore · OPEN
**Owner:** Lewis
**Created:** 2026-06-08

### Description
> `crates/vb_cli/src/cli_postcard/types.rs` is 530 lines. Master §3 cap is 300.

### Current State
- **File:** `crates/vb_cli/src/cli_postcard/types.rs`
- **Size:** 628 lines (not 530)

### Critical Constraint
27 re-exports in `cli_postcard/mod.rs:29-35` must be preserved.

### Split Steps

1. `git mv types.rs types/mod.rs`
2. Create 7 new files:

| File | Symbols |
|------|---------|
| `types/header_payload.rs` | `PostcardHeader` |
| `types/diagnostic.rs` | `DiagnosticReport`, `ValidateReport` |
| `types/verify.rs` | `VerifyReport`, `VerifyArtifactSection`, `VerifyDurabilitySection`, `VerifyReplaySection` |
| `types/explain.rs` | `ExplainReport`, `ExplainErrorEntry`, `ExplainArtifactSection` |
| `types/events.rs` | `EventsReport`, `EventEntry` |
| `types/trace.rs` | `TraceReport`, `TraceEntry` |
| `types/replay_diff.rs` | `ReplayReport`, `DiffReport`, `DiffEntry` |

### Acceptance Criteria
- [ ] `types.rs` → `types/mod.rs`
- [ ] 7 new files, all under 300 lines
- [ ] All 27 re-exports preserved
- [ ] `cargo test -p vb_cli` exits 0

---

## vb-e6xr7 — Split errors.rs Under 300-Line Cap

**ID:** vb-e6xr7 · P2 · chore · OPEN
**Owner:** Lewis
**Created:** 2026-06-08

### Description
> `crates/vb_core/src/errors.rs` is 738 lines. Master §3 cap is 300. Stale ledger entry says "2038 lines" — actual is 738.

### ⚠️ Stale Ledger Entry

`.config/source-length-exceptions.txt:85` — must be deleted or corrected.

### Split Plan

```
crates/vb_core/src/errors/
├── mod.rs              # type aliases, pub use re-exports
├── core.rs             # CoreError enum (~200)
├── collect.rs          # Collection error types (~100)
├── lifecycle.rs        # Lifecycle error types (~120)
├── journal_replay.rs   # Journal/Replay errors (~50)
└── tests.rs            # already exists (1,319 lines)
```

### 10 Public Items to Re-Export

| Item | Kind |
|------|------|
| `CoreResult<T>` | type alias |
| `EngineError` | type alias |
| `CoreError` | enum |
| `CollectPageOrderViolationKind` | enum |
| `CollectExtraHydrationFailureKind` | enum |
| `CollectEvidenceCapacityExceeded` | struct |
| `LifecycleStorageUnavailable` | struct |
| `LifecycleDuplicateRequest` | struct |
| `LifecycleStaleRequest` | struct |
| `LifecycleInvalidTransition` | struct |
| `JournalWriteFailure` | struct |
| `ReplayCorruption` | struct |

### Acceptance Criteria
- [ ] `errors.rs` → `errors/mod.rs`
- [ ] 4 family files, all under 300 lines
- [ ] Stale exception deleted
- [ ] `cargo build -p vb_core` exits 0
- [ ] `cargo test -p vb_core` exits 0

---

## vb-9zy8r — Split frame.rs Under 300-Line Cap

**ID:** vb-9zy8r · P2 · chore · OPEN
**Owner:** Lewis
**Created:** 2026-06-08

### Description
> `crates/vb_core/src/frame.rs` is 1,254 lines. Master §3 cap is 300.

### ⚠️ Previous Proposal Was Wrong

The proposed `frame/types.rs + frame/resolution.rs + frame/serialization.rs` is wrong — NO serialization concern exists in this file.

### Correct 3-Way Split

```
crates/vb_core/src/frame/
├── mod.rs              # re-exports
├── state.rs            # StepState + transition predicate (~63)
├── transitions.rs      # private validators (~20)
├── frame_struct.rs     # RunFrame + impl (~410)
├── kani_harnesses.rs  # Kani behind #[cfg(kani)] + kani-frame feature
└── tests_and_verification.rs  # orphaned reference
```

### Feature Flag Required

```toml
[features]
kani-frame = []
```

### Acceptance Criteria
- [ ] `frame.rs` → `frame/mod.rs`
- [ ] 4 new files, all under 300 lines
- [ ] Kani harnesses behind `#[cfg(all(kani, feature = "kani-frame"))]`
- [ ] All 58 importers compile unchanged
- [ ] `cargo test -p vb_core` exits 0

---

## vb-p9owu — Split diagnostic.rs Under 300-Line Cap

**ID:** vb-p9owu · P2 · chore · OPEN
**Owner:** Lewis
**Created:** 2026-06-08

### Description
> `crates/vb_core/src/diagnostic.rs` is 2,070 lines. Master §3 cap is 300.

### ⚠️ Previous Proposal Was Wrong

The proposed 2-way split (`diagnostic_codes.rs + diagnostic_render.rs`) is wrong — NO render layer exists in this file.

### 3 Distinct Concerns (per file header)

| Concern | Contents | Lines |
|---------|----------|-------|
| (a) Symbolic Registry | `CodeCategory`, `CodeEntry`, `CODE_REGISTRY` | ~17-1684 |
| (b) Numeric Projection | `SymbolicCode`, `DiagnosticCode`, conversions | ~1686-2120 |
| (c) User-facing Record | `Severity`, `Diagnostic`, `HasSymbolicCode` | ~1928-2044 |

### Critical Constraint

`CODE_REGISTRY` (lines 118–1632) MUST remain a **single `const` slice** in `diagnostic/codes.rs`.

### Split Plan

```
crates/vb_core/src/diagnostic/
├── mod.rs              # re-exports
├── codes.rs            # CODE_REGISTRY + lookup fns (~500)
├── numeric.rs          # SymbolicCode, DiagnosticCode (~400)
├── record.rs           # Severity, Diagnostic, HasSymbolicCode (~120)
└── tests_and_verification.rs
```

### Acceptance Criteria
- [ ] `diagnostic.rs` → `diagnostic/mod.rs`
- [ ] 3 new files, all under 300 lines
- [ ] `CODE_REGISTRY` single const preserved
- [ ] All importers compile unchanged
- [ ] `cargo test -p vb_core` exits 0

---

## vb-32gwc — Split budget.rs Under 300-Line Cap

**ID:** vb-32gwc · P2 · chore · OPEN
**Owner:** Lewis
**Created:** 2026-06-08

### Description
> `crates/vb_core/src/budget.rs` is 2,393 lines. Master §3 cap is 300.

### ⚠️ Orphan `budget/` Directory Already Exists

Contains:
- `tests.rs` — 229 KB (~7,339 lines)
- `tests_and_verification.rs` — 14 KB (~331 lines)
- `vb_qi37_2_4_state8_tests.rs` — 56 KB (~614 lines)
- **No `mod.rs`** — never wired up

### Split Plan (Recommended Path B)

```
crates/vb_core/src/budget/
├── mod.rs              # re-exports only (~50)
├── policy.rs           # BoundednessPolicy + BudgetError (~700)
├── compute.rs          # WholeWorkflowBudget + traversal (~900)
├── validation.rs       # aggregate types + validation (~700)
├── tests.rs            # existing (use #[path])
├── tests_and_verification.rs
└── vb_qi37_2_4_state8_tests.rs
```

### 10 Public Items

| Item | Kind |
|------|------|
| `WholeWorkflowBudget` | struct |
| `BoundednessPolicy` | struct |
| `BudgetError` | enum |
| `AggregateResourceBudget` | struct |
| `AggregateResourceCapacity` | struct |
| `AggregateResourceUsage` | struct |
| `AggregateReservation` | struct |
| `AggregateBudgetError` | enum |
| `validate_aggregate_budget` | pub fn |
| `validate_step_ceilings` | pub fn |

### Acceptance Criteria
- [ ] `budget.rs` → `budget/mod.rs`
- [ ] 3 family files, all under 300 lines
- [ ] `#[path = "tests.rs"] mod tests;` for large test file
- [ ] All 11 call sites compile unchanged
- [ ] `cargo test -p vb_core` exits 0
