# Implementation Plan: 17-Ready Beads Batch

**Generated:** 2026-06-11  
**Status:** IN IMPLEMENTATION — Phase 1 decision/quick fixes complete  
**Beads Covered:** vb-a408a, vb-6xh6c, vb-81urc, vb-9ckqp, vb-f2xk3, vb-benchbn01, vb-wstlsl01, vb-strecov01, vb-stortst01, vb-ybzsz, vb-dxi1k, vb-27jox, vb-jy6re, vb-e6xr7, vb-9zy8r, vb-p9owu, vb-32gwc  
**Total:** 17 beads (fewer than 20 requested — only 17 are currently `ready`)

---

## Executive Summary

This document is the authoritative implementation guide for the 17 ready beads in the velvet-ballistics repository. Each bead entry contains: the full description, current code state, required implementation actions, affected files, acceptance criteria, and recommended execution order.

### Implementation Evidence Log

- 2026-06-11: Phase 1 quick fixes complete.
  - `vb-9ckqp` closed. No persisted Red Queen config existed outside this progress directory; bead title/prose corrected to the clean-build expectation (`grep -c 'error\\['` exits `1` when no compiler errors are present). Evidence command printed `0` matches and recorded `vb_validate_grep_exit=1`.
  - `vb-f2xk3` closed with the same correction for `vb_core`. Evidence command printed `0` matches and recorded `vb_core_grep_exit=1`.
  - `vb-a408a` closed. Decision recorded as Option B: rewrite `vb-e5lfm` to the actual `vb_validate` Gate 8 accessor-validation scope. `ShardCommandQueue`/`ArrayQueue` Kani proof work is explicitly out of scope for vb_validate Gate 8 and must be a separate `vb_runtime` bead if needed.

### Phase Map

| Phase | Beads | Theme |
|-------|-------|-------|
| Phase 1 | vb-9ckqp, vb-f2xk3, vb-a408a | Quick fixes + decision |
| Phase 2 | vb-81urc, vb-ybzsz, vb-dxi1k | CI wiring + probes |
| Phase 3 | vb-27jox, vb-jy6re, vb-e6xr7 | File splits (medium) |
| Phase 4 | vb-9zy8r, vb-p9owu, vb-32gwc | File splits (large) |
| Phase 5 | vb-strecov01, vb-stortst01 | Storage tests |
| Phase 6 | vb-6xh6c, vb-benchbn01, vb-wstlsl01 | Kani + benchmarks + tests |

---

## vb-a408a — Reconcile Stale ArrayQueue Prose with Gate 8 Scope

**Priority:** P1  
**Status:** BLOCKED — requires decision before code change

### Description
> P1 Reconcile stale vb-e5lfm ArrayQueue prose with current Gate 8 scope

### Research Findings

**Critical Problem:** The bead vb-e5lfm (parent) references Kani harnesses that DO NOT EXIST in the codebase:
- `kani_gate_08_shard_command_queue_drain`
- `kani_gate_08_shard_command_queue_push`
- `kani_gate_08_shard_command_queue_bounded_invariant`
- `kani_gate_08_initial_state`

A grep for `kani_gate_08_shard` returns **zero results**.

**Actual Gate 8 Scope:**
- Gate 8 = accessor path segment validation
- Validates that `AccessorProgram` paths in `WorkflowParts` don't go out of bounds on `slot_count` or `symbols_count`
- Implementation: `validate_gate_08_accessor_path_segments` in `crates/vb_validate/src/gate_08_accessor.rs`
- **Does NOT include:** `ShardCommandQueue`, `ArrayQueue`, or any queue operations

**Actual Gate 8 Harnesses (21 total across 2 files):**
- `crates/vb_validate/src/kani_gate_08_accessor.rs` — 7 harnesses
- `crates/vb_validate/src/kani_gate_08_structural.rs` — 14 harnesses

**Moon CI runs only 4 of them:**
1. `kani_gate_08_valid_zero_accessors_pass`
2. `kani_gate_08_arbitrary_parts_valid_accessors_pass`
3. `kani_gate_08_arbitrary_parts_root_oob_rejected`
4. `kani_gate_08_arbitrary_parts_symbol_oob_rejected`

**The `ShardCommandQueue` actually lives in:**
- `crates/vb_runtime/src/shard/queue.rs` — a domain wrapper around `ArrayQueue<ShardCommand>`
- This is NOT part of vb_validate Gate 8 scope

### Required Decision (Must Resolve Before Code Change)

**Option A: Retarget to Runtime ShardCommandQueue/ArrayQueue Kani scope**
- Create new Kani harnesses in `crates/vb_runtime/src/verification/kani/` that verify ShardCommandQueue behavior
- Close vb-e5lfm as "wrong scope, redirected"
- vb-a408a becomes a coordination bead for this redirect

**Option B: Rewrite vb-e5lfm/vb-8o7p5 prose as current vb_validate Gate 8 work**
- Update vb-e5lfm to name exact crate/files (no ArrayQueue references)
- Update budget claim: bead says "120s" but moon kani.yml doesn't set 120s limit
- Address vb_validate Kani harness feature isolation

### Implementation Actions (Post-Decision)

1. Update vb-e5lfm prose to reflect actual current state
2. Remove all ArrayQueue/crossbeam_queue references from vb-e5lfm
3. Reconcile timeout budget claims
4. Update vb-a408a with reconciliation results

### Affected Files
- `crates/vb_validate/src/kani_gate_08_structural.rs` (harness references)
- `crates/vb_validate/src/kani_gate_08_accessor.rs` (harness references)
- `crates/vb_runtime/src/shard/queue.rs` (actual ArrayQueue usage)
- `.beads/vb-e5lfm/` (prose update)

### Acceptance Criteria
- [ ] Scope decision documented and approved
- [ ] vb-e5lfm prose updated to match decision
- [ ] vb-a408a closed with reconciliation evidence

---

## vb-6xh6c — Repair Remaining Gate 8 Kani 120s Timeouts

**Priority:** P1  
**Status:** OPEN — requires raw log capture + harness repair

### Description
> P1 Repair remaining vb_validate Gate 8 Kani 120s timeouts with raw logs

### Research Findings

**Parent Issue (vb-8o7p5):** Identified 3-4 `kani_gate_08_*` harnesses timed out at 120s due to `crossbeam_queue::ArrayQueue::new` unwinding issues.

**Recent Fix (commit eabe95b17):**
- Changed from `kani::any()` for full `WorkflowParts` to `bounded_parts_with_valid_accessors()` helper functions
- Bumped many harnesses from unwind 3/5/10 to unwind 17

**Current Moon CI Gate 8 Harnesses:**
| Harness | Unwind | Status |
|---------|--------|--------|
| `kani_gate_08_valid_zero_accessors_pass` | — | No explicit unwind |
| `kani_gate_08_arbitrary_parts_valid_accessors_pass` | 5 | From structural.rs |
| `kani_gate_08_arbitrary_parts_root_oob_rejected` | 5 | From structural.rs |
| `kani_gate_08_arbitrary_parts_symbol_oob_rejected` | 5 | From structural.rs |

**No raw Kani output logs are archived** in `.evidence/`. The `.evidence/vb-8o7p5/` directory only contains an empty `diff-scope-audit.txt` file.

### Implementation Actions

1. **Archive full Gate 8 family per-harness logs** under canonical 120s budget:
   - Command run
   - Exit status
   - Duration
   - Verifier verdict
   - Store in `.evidence/vb-6xh6c/`

2. **Repair remaining solver bombs** by:
   - Using bounded `kani::any()` construction patterns (as recent commit started)
   - Splitting complex harnesses into simpler ones
   - Feature isolation to remove problematic code paths
   - **NOT** blindly raising unwind bounds

3. **Classify issues separately:**
   - Strict `RUSTFLAGS=-Dwarnings` blockers (separate issue type)
   - Actual verifier timeouts (Kani CBMC unwind/solver issues)

4. **Keep proofs bound** to `crate::gates::validate_gate_08_accessor_path_segments`

### Acceptance Criteria
- [ ] Per-harness raw Kani logs archived in `.evidence/vb-6xh6c/`
- [ ] Every in-scope harness verifies within 120s budget OR has child bead with blocker evidence
- [ ] No blind unwind bound increases
- [ ] `moon run :verify-kani-vb-validate` exits 0

---

## vb-81urc — Wire test-determinism into CI

**Priority:** P1  
**Status:** OPEN — requires moon config + script changes

### Description
> P1 moon: move test-determinism to runInCI:true; archive 1,088 findings as baseline

### Research Findings

**Task Location:** `.moon/tasks/all.yml` lines 182-194
```yaml
test-determinism:
  command: 'bash scripts/check-test-determinism.sh'
  inputs:
    - 'scripts/check-test-determinism.py'
    - 'scripts/check-test-determinism.sh'
    - '.moon/tasks/all.yml'
    - 'README.md'
    - 'crates/*/tests/**/*.rs'
    - 'crates/workspace_tests/src/**/*.rs'
  options:
    cache: false
    runInCI: false   # <-- FLIP THIS
```

**Current findings:** 1,093 total
| Category | Count |
|---|---|
| `SharedTempState` | 791 |
| `UncontrolledClock` | 254 |
| `UncontrolledRandom` | 31 |
| `GlobalMutableState` | 15 |
| `SleepAsSync` | 2 |

**Baseline status:** No `.evidence/test-determinism/baseline.txt` exists.

### Three-Child Decomposition

**T0 — Archive Baseline (3h)**
- Create `.evidence/test-determinism/baseline.txt` with current 1,093 findings
- Verify no duplicates
- Command: `bash scripts/check-test-determinism.sh > .evidence/test-determinism/baseline.txt 2>&1`

**T2-T3 — Modify check-test-determinism.py for CI/Dev Modes (5h)**
- Add `MOON_CI` env var detection or `--ci` flag
- CI mode: diff against baseline, exit 0 if findings match baseline
- Dev mode: show full diff, exit 0 only if zero NEW findings

**T4-T5 — Flip runInCI to true + iterate CI fixes (55h)**
- Change `.moon/tasks/all.yml` line 194: `runInCI: false` → `runInCI: true`
- Run `moon ci` locally
- Fix discovered determinism issues (likely HashMap iteration order, SystemTime::now() in fixtures)
- Push when green

### Files to Modify
1. `.evidence/test-determinism/baseline.txt` — create (T0)
2. `scripts/check-test-determinism.py` — add CI/dev mode logic (T2-T3)
3. `.moon/tasks/all.yml` line 194 — flip `runInCI` (T4-T5)

### Acceptance Criteria
- [ ] `.evidence/test-determinism/baseline.txt` exists with 1,093 findings
- [ ] `check-test-determinism.py` supports CI/dev modes
- [ ] `runInCI: true` in `.moon/tasks/all.yml`
- [ ] `moon ci` runs test-determinism task green

---

## vb-9ckqp — Red Queen BLOCKER: vb_validate Pre-Existing-Build

**Priority:** P2 (Bug — Red Queen Blocker)  
**Status:** OPEN — quick fix

### Description
> [bug] [Red Queen] BLOCKER: pre-existing-build: cargo check -p vb_validate 2>&1 | grep -c 'error\[' --expect_exit 0

### Research Findings

**Command semantics:**
- `cargo check -p vb_validate` — compile-check vb_validate crate
- `grep -c 'error\['` — count occurrences of `error[` (Rust compiler error format: `error[E123]:`)
- `grep -c` returns **exit 0 when matches ARE found**, **exit 1 when NO matches found**

**Current state:** vb_validate compiles cleanly (0 errors found → grep exits 1 → pipeline fails)

**The `--expect_exit 0` flag expects grep to find matches (exit 0), which happens when there ARE errors**

### Fix

Change `--expect_exit 0` to `--expect_exit 1` since the test should pass when the build is clean (no errors found = grep exits 1).

**OR** clarify the intended semantics:
- If Red Queen should baseline against a **clean build**: `--expect_exit 1`
- If Red Queen should verify **known errors persist**: `--expect_exit 0` (but vb_validate is healthy)

### Files to Modify
- Red Queen smoke test configuration (find the actual test file using this command)

### Acceptance Criteria
- [ ] `cargo check -p vb_validate 2>&1 | grep -c 'error\[' --expect_exit 1` passes (grep exits 1 = clean build)
- [ ] Red Queen pre-existing-build check for vb_validate green

---

## vb-f2xk3 — Red Queen BLOCKER: vb_core Pre-Existing-Build

**Priority:** P2 (Bug — Red Queen Blocker)  
**Status:** OPEN — quick fix

### Description
> [bug] [Red Queen] BLOCKER: pre-existing-build: cargo check -p vb_core 2>&1 | grep -c 'error\[' --expect_exit 0

### Research Findings

**Same issue as vb-9ckqp but for vb_core.**

**Current state:** vb_core compiles cleanly (0 errors found → grep exits 1 → pipeline fails)

**Historical context:** Per `landing-report.md`, vb_core previously had E0164 compile errors in `kani_step_harnesses.rs` that were tracked in bead `vb-yd9g0`. Those errors were fixed.

### Fix

Same as vb-9ckqp: Change `--expect_exit 0` to `--expect_exit 1`.

### Acceptance Criteria
- [ ] `cargo check -p vb_core 2>&1 | grep -c 'error\[' --expect_exit 1` passes (grep exits 1 = clean build)
- [ ] Red Queen pre-existing-build check for vb_core green

---

## vb-benchbn01 — Add Missing Benchmark Groups

**Priority:** P2  
**Status:** OPEN — requires new benchmark files

### Description
> [bug] vb_benchmark: add 2 missing bench groups (warm_throughput, digest_computation) per master §39

### Research Findings

**Master §39 requires 22 benchmark groups.** Currently missing:
1. `warm_throughput` — measures warm-cache throughput (distinct from cold_start)
2. `digest_computation` — measures BLAKE3-256 + CRC32C computation throughput

**Current vb_benchmark structure:**
- `src/lib.rs` (322 lines) — BenchmarkMetadata, evidence gate types, helpers
- `tests/benchmark_tests.rs` (609 lines) — unit tests
- **`benches/` directory does NOT exist** — needs to be created

**Dependencies needed:**
- `blake3` — BLAKE3-256 digests (already workspace dep)
- `crc32c` — CRC32C checksums (already workspace dep)

### Files to Create

**`crates/vb_benchmark/benches/warm_throughput.rs`**
- Criterion benchmark for warm-cache throughput
- Follow existing pattern from `crates/workspace_tests/benches/cold_start.rs`
- `BENCH_METADATA` constant with profile/tool/durability/mode/latency/allocations info
- `c.benchmark_group("warm_throughput")` with `bench_function()`

**`crates/vb_benchmark/benches/digest_computation.rs`**
- Criterion benchmark for BLAKE3-256 + CRC32C computation
- Measure throughput of `blake3::hash()` and `crc32c::crc32c()`

### Cargo.toml Changes

```toml
[dev-dependencies]
blake3.workspace = true   # ADD
crc32c.workspace = true   # ADD

[[bench]]
name = "warm_throughput"
harness = false

[[bench]]
name = "digest_computation"
harness = false
```

### Acceptance Criteria
- [ ] `crates/vb_benchmark/benches/warm_throughput.rs` exists
- [ ] `crates/vb_benchmark/benches/digest_computation.rs` exists
- [ ] Both benches wired in `[[bench]]` in Cargo.toml
- [ ] `cargo bench --no-run` exits 0

---

## vb-wstlsl01 — Delete Self-Laundering Tests

**Priority:** P2  
**Status:** OPEN — requires test deletion + renaming

### Description
> [bug] workspace_tests: delete self-laundering tests asserting the 11 missing Section 17 codes must NOT appear

### Research Findings

**Self-laundering tests** assert that missing functionality MUST NOT appear — encoding gaps as contractual requirements.

**Files affected:**
1. `crates/workspace_tests/tests/section17_runtime_code_reverse_parity.rs`
2. `crates/workspace_tests/tests/section17_runtime_code_coverage_report.rs`

**Tests to DELETE:**

| File | Lines | Item |
|------|-------|------|
| `section17_runtime_code_reverse_parity.rs` | 42-50 | `SECTION_17_UNMAPPED` constant (7 codes) |
| `section17_runtime_code_reverse_parity.rs` | 244-271 | `section17_reverse_parity_unmapped_codes_have_no_sources` test |
| `section17_runtime_code_coverage_report.rs` | 197-222 | `UNMAPPED_CODES_WITH_RATIONALE` constant (6 codes) |
| `section17_runtime_code_coverage_report.rs` | 224-227 | `PARTIALLY_MAPPED_CODES` constant (1 code) |
| `section17_runtime_code_coverage_report.rs` | 248-257 | `section17_coverage_report_unmapped_codes_stay_unmapped` test |

**Rename/Merge:**

| File | Old | New |
|------|-----|-----|
| `section17_runtime_code_reverse_parity.rs` | `SECTION_17_MAPPED` | `SECTION_17_GOLDEN` (33 codes) |
| `section17_runtime_code_reverse_parity.rs` | `section17_reverse_parity_mapped_codes_have_sources` | `section17_reverse_parity_every_golden_code_has_source` |
| `section17_runtime_code_coverage_report.rs` | `MAPPED_CODES` | `SECTION_17_GOLDEN` (33 codes) |

### Acceptance Criteria
- [ ] `rg "SECTION_17_UNMAPPED" crates/` returns zero matches
- [ ] `rg "UNMAPPED_CODES_WITH_RATIONALE" crates/` returns zero matches
- [ ] `rg "PARTIALLY_MAPPED_CODES" crates/` returns zero matches
- [ ] `cargo test -p workspace_tests section17` fails (waiting for 11 missing codes to be implemented)

---

## vb-strecov01 — Add error_recovery Test for Fuzz-Malformed Journal Records

**Priority:** P2  
**Status:** OPEN — requires new test file

### Description
> [bug] vb_storage: add error_recovery test for fuzz-malformed journal records (recovery::replay)

### Research Findings

**New file to create:** `crates/vb_storage/src/recovery/tests/error_recovery_tests.rs`
- Directory `recovery/tests/` does not exist yet — must be created
- 10 deterministic `#[test]` functions
- No `unsafe`, no `unwrap`, no `proptest`

**Test matrix:**

| # | Mutation | Corruption Applied | Expected `JournalError` |
|---|---------|--------------------|-----------------------|
| 1 | truncated payload | Slice encoded bytes short of `payload_len` | `UnexpectedEof` |
| 2 | swapped magic | Change bytes `[0..4]` to `MAGIC_SNAPSHOT` (0x5642_534E) | `BadMagic { found: 0x5642_534E }` |
| 3 | corrupted CRC32C | Flip bit in bytes `[56..60]` | `HeaderChecksumMismatch` |
| 4 | BLAKE3 digest mismatch | Corrupt one payload byte | `PayloadDigestMismatch` |
| 5 | payload_len overflow | Set `payload_len` > `MAX_JOURNAL_EVENT_PAYLOAD_BYTES` | `PayloadTooLarge { len, max }` |
| 6 | header_len mismatch | Set header_len field to ≠ 60 | `HeaderLengthMismatch { found }` |
| 7 | record_kind outside 1..=50 | Set record_kind to 99 | `UnknownRecordKind { kind: 99 }` |
| 8 | duplicate sequence number | Encode two records with same seq | `SequenceGap` |
| 9 | gap in sequence | Encode records with seq gap (1, 2, 4 — no 3) | `SequenceGap` |
| 10 | unknown record_kind family | MAGIC_JOURNAL_EVENT + record_kind=1 (workflow family) | `RecordKindFamilyMismatch` |

**Wire-format layout (60 bytes):**
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
- [ ] `crates/vb_storage/src/recovery/tests/error_recovery_tests.rs` created
- [ ] All 10 tests use `assert!(matches!(...))` assertions
- [ ] No `unsafe`, `unwrap`, `proptest`, or `arbitrary`
- [ ] `cargo test -p vb_storage error_recovery` passes

---

## vb-stortst01 — Split tests.rs Under 300-Line Cap

**Priority:** P2  
**Status:** OPEN — large mechanical refactor

### Description
> [bug] vb_storage: split tests.rs (8,091 LoC) under 300-line cap; violates master §3 300-line file cap

### Research Findings

**File:** `crates/vb_storage/src/tests.rs` — **8,215 lines**, 328 `#[test]` functions

**The rule:** No first-party Rust file may exceed 300 physical lines (master §3)

**Proposed split into ~30 files under `crates/vb_storage/src/tests/`:**

| File | Est. Lines | Contents |
|------|-----------|----------|
| `mod.rs` | ~20 | `mod` declarations + re-exports |
| `tests_key_encoding.rs` | ~250 | key encoder tests |
| `tests_envelope.rs` | ~250 | envelope round-trips |
| `tests_journal_ops.rs` | ~250 | journal append/batch ops |
| `tests_header_adversarial.rs` | ~250 | adversarial header decode |
| `tests_recovery.rs` | ~250 | recovery/replay tests |
| `tests_snapshot.rs` | ~250 | snapshot encode/decode |
| `tests_blob.rs` | ~250 | blob storage tests |
| `tests_index.rs` | ~250 | index keyspace tests |
| `tests_batch.rs` | ~250 | batch builder tests |
| `tests_durability.rs` | ~250 | durability profile tests |
| `tests_error_exhaustive.rs` | ~250 | error variant exhaustiveness |
| `tests_lock_enforcement.rs` | ~250 | process lock (vb-apn5) |
| `tests_recovery_stamp.rs` | ~250 | RecoveryStamp parity (vb-1cwhx) |
| ... | ... | ~30 files total |

**Implementation approach:**
1. Create `crates/vb_storage/src/tests/` directory
2. Extract test functions from `tests.rs` into thematic `*.rs` files (~250 LoC each)
3. Create `tests/mod.rs` that declares all submodules
4. Replace `tests.rs` with `tests/mod.rs` (or restructure the module tree)
5. Update `lib.rs` — change `pub mod tests;` declaration
6. Run `cargo test -p vb_storage` to confirm all 328 tests still pass
7. Run `moon :source-length` to confirm no file exceeds 300 lines

### Acceptance Criteria
- [ ] `crates/vb_storage/src/tests.rs` deleted or replaced
- [ ] All test functions moved to `tests/*.rs` files
- [ ] All files under 300 lines
- [ ] `cargo test -p vb_storage` passes with all 328 tests
- [ ] `moon :source-length` exits 0

---

## vb-ybzsz — Wire flux-check-package.sh and Loom Task into Moon

**Priority:** P2  
**Status:** OPEN — requires new moon task files

### Description
> P2 moon: wire flux-check-package.sh and a loom task

### Research Findings

**Two independent sub-tasks:**

**Sub-bead vb-ybzsz.1 — Flux wiring:**
- `scripts/flux-check-package.sh` already exists
- Packages with Flux obligations: `vb_compile` (6 obligations), `vb_runtime` (10 obligations)
- Need to create `.moon/tasks/flux.yml` with 2 tasks
- Wire into `.moon.yml` pipeline

**Sub-bead vb-ybzsz.2 — Loom wiring:**
- 5 loom models exist in `xtask/src/loom.rs:17-26`:
  1. `journal_writer_queue`
  2. `action_completion_cancel`
  3. `timer_fired_cancel`
  4. `shutdown_drain`
  5. `bounded_queue`
- Need to create `.moon/tasks/loom.yml` with 2 tasks:
  1. `loom-run` — loops over all 5 models
  2. `loom-list-smoke` — runs `bash scripts/loom-list.sh`
- `xtask` is already a workspace member (vb-lbg3h closed)

**Sub-bead vb-ybzsz.3 — Loom-list-smoke:**
- Add to `.moon/tasks/loom.yml`

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

### `.moon.yml` Pipeline Changes

Add to `.moon.yml` pipeline (after `verify-kani` or similar verification stage):
```yaml
- tasks: '.moon/tasks/flux.yml'
- tasks: '.moon/tasks/loom.yml'
```

### Acceptance Criteria
- [ ] `.moon/tasks/flux.yml` created with 2 tasks
- [ ] `.moon/tasks/loom.yml` created with 2 tasks
- [ ] Both included in `.moon.yml` pipeline
- [ ] `moon run :flux-check-vb-compile` exits 0
- [ ] `moon run :flux-check-vb-runtime` exits 0
- [ ] `moon run :loom-list-smoke` exits 0
- [ ] `moon run :loom-run` exits 0

---

## vb-dxi1k — Add verify-kani-vb-validate to Pipeline

**Priority:** P2  
**Status:** OPEN — single-line edit

### Description
> P2 moon: add verify-kani-vb-validate to .moon.yml pipeline

### Research Findings

**Task already exists** in `.moon/tasks/kani.yml:38-64` with `runInCI: true`.

**The task runs 4 harnesses:**
1. `kani_gate_08_valid_zero_accessors_pass`
2. `kani_gate_08_arbitrary_parts_valid_accessors_pass`
3. `kani_gate_08_arbitrary_parts_root_oob_rejected`
4. `kani_gate_08_arbitrary_parts_symbol_oob_rejected`

**Pipeline position:** `.moon.yml` line 12 — after `- 'verify-kani'` and before `- 'nightly-feature-gate'`

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

### Acceptance Criteria
- [ ] `.moon.yml` contains `verify-kani-vb-validate` after `verify-kani`
- [ ] `moon run :test` exits 0 (satisfies `deps: [test]`)
- [ ] `moon run :verify-kani-vb-validate` exits 0
- [ ] All 4 harnesses complete within 120s

---

## vb-27jox — Split output.rs Under 300-Line Cap

**Priority:** P2  
**Status:** PARTIALLY COMPLETE — Holzman substring matcher already deleted

### Description
> P2 vb_cli: split output.rs (303 LoC) under 300-line cap + delete Holzman substring matcher

### Research Findings

**`infer_legacy_json_error_code` function:** Already deleted from both `output.rs` and `output_utils.rs`. A grep for `message.contains(` in all `output*.rs` files returns **zero matches**.

**`output.rs` current state:** 280 lines (already under 300-line cap).

### Remaining Work

Split into `output/` directory following established pattern (`agent_context/`):

| File | Est. Lines | Contents |
|------|-----------|----------|
| `output/mod.rs` | ~12 | `mod` declarations + re-exports |
| `output/format.rs` | ~30 | `output_format_from_args`, `named_os_flag`, `parse_emit_output_format` |
| `output/io.rs` | ~90 | `write_structured_stderr`, `write_stderr_bytes`, `write_stderr_line_io`, `write_stdout_line*`, `write_stderr_line`, `write_stderr_best_effort` |
| `output/json.rs` | ~120 | `OutputError`, `json_out_exit`, `json_out`, `write_contract_error_json`, `json_error_with_code`, `write_diagnostic_message_stderr`, `write_yaml_diagnostic_stderr`, `write_typed_postcard_diagnostic_stderr`, encode functions |
| `output/compat.rs` | ~10 | `pub(crate) use crate::file_io::write_failure_message;` re-export |

### Files to Modify/Create
1. Create `crates/vb_cli/src/output/` directory
2. Create `output/format.rs`
3. Create `output/io.rs`
4. Create `output/json.rs`
5. Create `output/compat.rs`
6. Create `output/mod.rs`
7. Update `crates/vb_cli/src/lib.rs` imports if needed

### Acceptance Criteria
- [ ] `output/` directory with 5 files under 300 lines each
- [ ] All `pub use` re-exports preserved
- [ ] `cargo build -p vb_cli` exits 0
- [ ] `moon :lint-src` exits 0

---

## vb-jy6re — Split cli_postcard/types.rs Under 300-Line Cap

**Priority:** P2  
**Status:** OPEN — mechanical refactor

### Description
> P2 vb_cli: split cli_postcard/types.rs (530 LoC) under 300-line cap

### Research Findings

**File:** `crates/vb_cli/src/cli_postcard/types.rs` — **628 lines** (not 530 as bead states)

**Current structure:**
| File | LoC |
|---|---|
| `classify.rs` | 250 |
| `types.rs` | **628** ← violates |
| `types_more.rs` | 116 |
| `validation.rs` | 87 |
| `error.rs` | 50 |
| `codec.rs` | 57 |
| `mod.rs` | 50 |

**27 re-exports in `mod.rs:29-35`** that must be preserved:

```rust
pub(crate) use types::{
    CLI_MAGIC, CLI_POSTCARD_KIND, CLI_SCHEMA_VERSION, CliPostcardKind, CliPostcardPayload,
    DiagnosticReport, DiffEntry, DiffReport, EnvelopeSchemaVersion, EventEntry, EventsReport,
    ExplainErrorEntry, ExplainReport, GenericPayload, HEADER_SIZE, HEADER_SIZE_U32, MAX_PAYLOAD,
    MAX_PAYLOAD_U32, PostcardHeader, ReplayReport, TraceEntry, TraceReport, ValidateReport,
    VerifyArtifactSection, VerifyDurabilitySection, VerifyReplaySection, VerifyReport,
};
```

### Split Plan

**Step 1:** `git mv types.rs types/mod.rs` (convert file to directory+mod)

**Step 2:** Create 7 new files:

| File | Symbols | Est. LoC |
|---|---|---|
| `types/header_payload.rs` | `PostcardHeader` | ~49 |
| `types/diagnostic.rs` | `DiagnosticReport`, `ValidateReport` | ~39 |
| `types/verify.rs` | `VerifyReport`, `VerifyArtifactSection`, `VerifyDurabilitySection`, `VerifyReplaySection` | ~56 |
| `types/explain.rs` | `ExplainReport`, `ExplainErrorEntry`, `ExplainArtifactSection` | ~40 |
| `types/events.rs` | `EventsReport`, `EventEntry` | ~25 |
| `types/trace.rs` | `TraceReport`, `TraceEntry` | ~20 |
| `types/replay_diff.rs` | `ReplayReport`, `DiffReport`, `DiffEntry` | ~45 |

**Step 3:** Update `cli_postcard/mod.rs` — `mod types;` remains valid (Rust finds `types/mod.rs`)

### Acceptance Criteria
- [ ] `types.rs` → `types/mod.rs`
- [ ] 7 new files under `types/`, all under 300 lines
- [ ] All 27 re-exports preserved
- [ ] `cargo build -p vb_cli` exits 0
- [ ] `cargo test -p vb_cli` exits 0

---

## vb-e6xr7 — Split errors.rs Under 300-Line Cap

**Priority:** P2  
**Status:** OPEN — mechanical refactor

### Description
> P2 vb_core: split errors.rs (738 LoC) under 300-line cap

### Research Findings

**File:** `crates/vb_core/src/errors.rs` — **738 lines**

**Stale ledger entry at `.config/source-length-exceptions.txt:85`:**
```
crates/vb_core/src/errors.rs|lewis|vb-jpq7.47|split-or-retire-before-release|Pre-existing over-300-line Rust source baseline (2038 lines)
```
This must be deleted or corrected (file is 738 lines, not 2038).

**Four logical families:**

| Family | Contents | Est. LoC |
|--------|----------|----------|
| Collection Errors | `CollectPageOrderViolationKind`, `CollectExtraHydrationFailureKind`, `CollectEvidenceCapacityExceeded` | ~100 |
| Lifecycle Errors | `LifecycleStorageUnavailable`, `LifecycleDuplicateRequest`, `LifecycleStaleRequest`, `LifecycleInvalidTransition` | ~120 |
| Journal/Replay Errors | `JournalWriteFailure`, `ReplayCorruption` | ~50 |
| Core Error Enum | `CoreError` enum + `CONST_*` constants + `diagnostic_code()` + `runtime_code()` | ~220 |

**Test file:** `crates/vb_core/src/errors/tests.rs` — **1,319 lines** (already separate)

### Split Plan

```
crates/vb_core/src/errors/
├── mod.rs           # type aliases, pub use re-exports, #[path] for tests
├── core.rs          # CoreError enum definition only (~200)
├── collect.rs       # Collection error types + re-exports (~100)
├── lifecycle.rs     # Lifecycle error types + re-exports (~120)
├── journal_replay.rs # Journal/Replay error types + re-exports (~50)
└── tests.rs         # already exists at this path
```

### Acceptance Criteria
- [ ] `errors.rs` → `errors/mod.rs`
- [ ] 4 new family files, all under 300 lines
- [ ] Stale exception entry deleted from `.config/source-length-exceptions.txt`
- [ ] All `pub use` re-exports preserved
- [ ] `cargo build -p vb_core` exits 0
- [ ] `cargo test -p vb_core` exits 0

---

## vb-9zy8r — Split frame.rs Under 300-Line Cap

**Priority:** P2  
**Status:** OPEN — mechanical refactor + kani isolation

### Description
> P2 vb_core: split frame.rs (1,254 LoC) under 300-line cap

### Research Findings

**File:** `crates/vb_core/src/frame.rs` — **1,254 lines**

**Contents:**

| Symbol | Type | Lines |
|--------|------|-------|
| `StepState` | enum (8 variants) | 10-29 |
| `is_valid_step_state_transition` | pub fn | 31-63 |
| `RunFrame` | struct + ~30 impl methods | 65-475 |
| Kani harnesses | `#[cfg(kani)]` module | 491-1254 |

**Orphaned reference file:** `crates/vb_core/src/frame/tests_and_verification.rs` (1,913 lines) — contains tests from prior unlanded split attempt.

### Split Plan

```
crates/vb_core/src/frame/
├── mod.rs              # re-exports, ~10 lines
├── state.rs            # StepState + is_valid_step_state_transition (~63)
├── transitions.rs      # validate_transition, validate_pending_admission (~20)
├── frame_struct.rs     # RunFrame struct + all impl methods (~410)
├── kani_harnesses.rs  # Kani harnesses behind #[cfg(kani)] + kani-frame feature (~763)
└── tests_and_verification.rs  # orphaned reference file
```

### Feature Flag Required

Add to `crates/vb_core/Cargo.toml`:
```toml
[features]
kani-frame = []
```

Kani harnesses isolated with:
```rust
#[cfg(all(kani, feature = "kani-frame"))]
mod frame_kani_harnesses;
```

### Acceptance Criteria
- [ ] `frame.rs` → `frame/mod.rs`
- [ ] 4 new files, all under 300 lines
- [ ] Kani harnesses behind `#[cfg(all(kani, feature = "kani-frame"))]`
- [ ] All 58+ importers compile unchanged
- [ ] `cargo test -p vb_core` exits 0

---

## vb-p9owu — Split diagnostic.rs Under 300-Line Cap

**Priority:** P2  
**Status:** OPEN — mechanical refactor

### Description
> P2 vb_core: split diagnostic.rs (2,070 LoC) under 300-line cap

### Research Findings

**File:** `crates/vb_core/src/diagnostic.rs` — **2,143 lines**

**Three distinct concerns (stated in file header):**

| Concern | Contents | Lines |
|---------|----------|-------|
| (a) Symbolic Registry | `CodeCategory`, `CodeEntry`, `CODE_REGISTRY` | ~17-1684 |
| (b) Numeric Projection | `SymbolicCode`, `DiagnosticCode`, conversions | ~1686-2120 |
| (c) User-facing Record | `Severity`, `Diagnostic`, `HasSymbolicCode` | ~1928-2044 |

**Critical constraint:** `CODE_REGISTRY` (lines 118–1632) is a **single `const` slice** that MUST remain unsplit.

**Rejected 2-way split:** `diagnostic_codes.rs + diagnostic_render.rs` — "render" doesn't exist in this file; it belongs to `vb_validate/diag_render.rs`.

### Split Plan

```
crates/vb_core/src/diagnostic/
├── mod.rs              # re-exports, ~20 lines
├── codes.rs            # CodeCategory, CodeEntry, CODE_REGISTRY (single const), lookup fns (~500)
├── numeric.rs          # SymbolicCode, DiagnosticCode, parse errors, conversions (~400)
├── record.rs           # Severity, Diagnostic, HasSymbolicCode (~120)
└── tests_and_verification.rs  # existing tests
```

### Acceptance Criteria
- [ ] `diagnostic.rs` → `diagnostic/mod.rs`
- [ ] 3 new files, all under 300 lines
- [ ] `CODE_REGISTRY` remains single `const` in `codes.rs`
- [ ] All importers compile unchanged
- [ ] `cargo test -p vb_core` exits 0

---

## vb-32gwc — Split budget.rs Under 300-Line Cap

**Priority:** P2  
**Status:** OPEN — mechanical refactor

### Description
> P2 vb_core: split budget.rs (2,393 LoC) under 300-line cap

### Research Findings

**File:** `crates/vb_core/src/budget.rs` — **2,394 lines**

**Orphan `budget/` directory already exists** at `crates/vb_core/src/budget/` containing:
- `tests.rs` — 229 KB (~7,339 lines)
- `tests_and_verification.rs` — 14 KB (~331 lines)
- `vb_qi37_2_4_state8_tests.rs` — 56 KB (~614 lines)
- **No `mod.rs`** — directory was scaffolded but never wired up

**10 public items:**

| Line | Public Item | Kind |
|------|-------------|------|
| 12 | `WholeWorkflowBudget` | struct |
| 326 | `BoundednessPolicy` | struct |
| 526 | `BudgetError` | enum |
| 563 | `AggregateResourceBudget` | struct |
| 591 | `AggregateResourceCapacity` | struct |
| 615 | `AggregateResourceUsage` | struct |
| 639 | `AggregateReservation` | struct |
| 647 | `AggregateBudgetError` | enum |
| 1101 | `validate_aggregate_budget` | pub fn |
| 1204 | `validate_step_ceilings` | pub fn |

### Split Plan

```
crates/vb_core/src/budget/
├── mod.rs           # re-exports only, ~50 lines
├── policy.rs        # BoundednessPolicy + BudgetError + DEFAULT (~700)
├── compute.rs       # WholeWorkflowBudget + all traversal functions (~900)
├── validation.rs    # aggregate types + aggregate validation fns (~700)
├── tests.rs         # already exists (use #[path])
├── tests_and_verification.rs  # orphaned
└── vb_qi37_2_4_state8_tests.rs  # orphaned
```

### Acceptance Criteria
- [ ] `budget.rs` → `budget/mod.rs`
- [ ] 3 new files, all under 300 lines
- [ ] `#[path = "tests.rs"] mod tests;` in mod.rs to avoid moving large test file
- [ ] All 11 call sites compile unchanged
- [ ] `cargo test -p vb_core` exits 0

---

## Implementation Execution Order

### Phase 1: Quick Fixes + Decision (Start Here)
| Order | Bead | Reason |
|-------|------|--------|
| 1 | vb-9ckqp | 5-minute fix — change `--expect_exit 0` → `--expect_exit 1` |
| 2 | vb-f2xk3 | 5-minute fix — same pattern for vb_core |
| 3 | vb-a408a | Decision bead — no code until scope resolved |

### Phase 2: CI Wiring + Probes
| Order | Bead | Reason |
|-------|------|--------|
| 4 | vb-dxi1k | 1-line edit — wire verify-kani-vb-validate into pipeline |
| 5 | vb-ybzsz | Create 2 new moon task files + pipeline includes |
| 6 | vb-81urc | T0 archive baseline (3h) + T2-T3 script modification |

### Phase 3: File Splits (Medium)
| Order | Bead | File Size | Risk |
|-------|------|-----------|------|
| 7 | vb-27jox | 280 lines | LOW — already under cap, pure refactor |
| 8 | vb-jy6re | 628 lines | MEDIUM — 27 re-exports must be preserved |
| 9 | vb-e6xr7 | 738 lines | MEDIUM — 10 public items + stale ledger entry |

### Phase 4: File Splits (Large)
| Order | Bead | File Size | Risk |
|-------|------|-----------|------|
| 10 | vb-9zy8r | 1,254 lines | MEDIUM — Kani harness isolation needed |
| 11 | vb-p9owu | 2,143 lines | HIGH — CODE_REGISTRY must stay single const |
| 12 | vb-32gwc | 2,394 lines | HIGH — budget/ dir already exists with orphaned files |

### Phase 5: Storage Tests
| Order | Bead | Reason |
|-------|------|--------|
| 13 | vb-strecov01 | New test file + directory creation |
| 14 | vb-stortst01 | Large mechanical refactor (328 tests) |

### Phase 6: Kani + Benchmarks + Tests
| Order | Bead | Reason |
|-------|------|--------|
| 15 | vb-6xh6c | Complex — raw log capture + harness repair strategy |
| 16 | vb-benchbn01 | New benchmark files + Cargo.toml deps |
| 17 | vb-wstlsl01 | Test deletion + renaming (will break tests until codes implemented) |

---

## Risk Register

| Bead | Risk | Mitigation |
|------|------|------------|
| vb-a408a | Scope decision required before code | Block until decision |
| vb-6xh6c | Kani timeouts may indicate fundamental harness design issue | Use bounded kani::any(), feature isolation |
| vb-81urc | 791 SharedTempState findings may require extensive test fixes | Archive baseline, run CI iteratively |
| vb-stortst01 | 328 tests sharing imports — split is mechanically risky | Extract with preserved imports, run tests after each file |
| vb-p9owu | CODE_REGISTRY must remain single const | Explicit constraint in split plan |
| vb-32gwc | Orphaned budget/ directory may have stale state | Inspect before use, prefer fresh split |
| vb-9zy8r | 58+ importers rely on direct module paths | Use pub use re-exports in mod.rs |

---

## Appendix: Source-Length Exception Ledger Edits Required

| Bead | File | Action | Stale Entry |
|------|------|--------|-------------|
| vb-e6xr7 | `errors.rs` → `errors/` | Delete stale row | `.config/source-length-exceptions.txt:85` |
| vb-stortst01 | `tests.rs` → `tests/` | No entry needed after split | None |
| vb-27jox | `output.rs` → `output/` | No entry needed | None |
| vb-jy6re | `types.rs` → `types/` | No entry needed | None |
| vb-9zy8r | `frame.rs` → `frame/` | No entry needed | None |
| vb-p9owu | `diagnostic.rs` → `diagnostic/` | No entry needed | None |
| vb-32gwc | `budget.rs` → `budget/` | No entry needed | None |

---

## Appendix: Moon Pipeline Positions

Current `.moon.yml` pipeline (relevant entries):
```yaml
- 'fmt'
- 'lint-src'
- 'check'
- 'sanitizer-address-check'
- 'verify-kani'
- 'nightly-feature-gate'    # vb-dxi1k inserts after verify-kani
- 'source-length'
- 'check-no-dead-ir-duplicates'
- 'check-test_density'
- 'check-bench-registration'
- 'check-kani-shape-vacuity'
- 'supply-chain'
- 'feature-powerset'
- 'hardened-build'
- 'test'
```

vb-ybzsz inserts new tasks after `verify-kani` block or as separate includes.
