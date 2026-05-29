# Implementation Contract — Fuzz Hardening (vb-hbav)

## Contract Metadata

| Field | Value |
|-------|-------|
| Contract ID | `contract-vb-hbav-001` |
| Schema Version | `contract/v1` |
| Bead ID | `vb-hbav` |
| Domain | Fuzz hardening (Phase 1 of Red Queen Campaign) |
| Status | Open — awaiting implementation |
| Upstream | `RED_QUEEN_MASTER_PLAN.md`, `EXECUTE.md` |
| Dependency | Phase 0 must be complete (all targets compile, smoke, ASAN smoke) |

## Contract Objectives

1. Harden 21 weak fuzz functions from CoverageOnly to at least TypedError strength
2. Fix C.25: Implement `fuzz_collect_page_pagination` in `fuzz/src/lib.rs`
3. Verify C.21-C.24 fixes with 1-hour ASAN campaign
4. Create seed corpora for all targets lacking them
5. Refactor stdin boilerplate to shared module (`fuzz/src/bin_common.rs`)
6. Run 1-hour ASAN campaign on all hardened targets

## Requirement Clauses

### RQ-01: Harden 21 Weak Functions

**Description**: Each of the 21 CoverageOnly fuzz functions listed in `RED_QUEEN_MASTER_PLAN.md` §5.1 must be upgraded to at least TypedError assertion strength.

**Spec Ref**: RED_QUEEN_MASTER_PLAN.md §5.1 lines 338-362, EXECUTE.md §1.1

**Implementation Contract**:

| # | Target Function | Current Strength | Required Strength | Required Assertions |
|---|----------------|-----------------|-------------------|---------------------|
| 1 | `fuzz_yaml_events` | CoverageOnly | Structural | `!events.is_empty()` for non-empty input, source_map entries ≥ 0, match YamlError variants |
| 2 | `fuzz_replay_events` | CoverageOnly | Structural | `replayed.len() <= events.len()`, ActionReplayTracker state invariants |
| 3 | `fuzz_extract_terminal` | CoverageOnly | Structural | `terminal.children().is_empty()`, terminal is valid node kind |
| 4 | `fuzz_action_tracker` | CoverageOnly | Structural | `is_resolved` deterministic, state transitions correct |
| 5 | `fuzz_accepted_artifact_envelope_qi37_4_2` | CoverageOnly | Structural | `gate_count > 0`, `accepted_at_seq >= 1`, `required_capabilities.len()` matches |
| 6 | `fuzz_expr_bytecode` | CoverageOnly | Structural | `result.type_name()` is known type, stack depth ≤ max, no silent Null |
| 7 | `fuzz_verifier_gates` | CoverageOnly | TypedError | Each gate returns `ValidationError` variants, assert gate-specific invariants |
| 8 | `fuzz_budget_compute` | CoverageOnly | Structural | All budget components non-negative, `max_total_steps > 0` for non-empty, `max_fanout` bounded |
| 9 | `fuzz_admission_flow` | CoverageOnly | TypedError | Match `AdmissionError` variants, artifact exists in store on success |
| 10 | `fuzz_expr_eval` | (already Structural) | Verify | Assertions are mutation-resistant, no silent Null on Ok |
| 11 | `fuzz_accessor_traversal` | CoverageOnly | Structural | Path depth ≤ `FUZZ_MAX_ACCESSOR_DEPTH`, slot reference validity on success |
| 12 | `fuzz_admission_fuzz` | CoverageOnly | Structural | Parts has ≥ 1 node on success, match `AdmissionError` variants |
| 13 | `fuzz_digest_coherence` | CoverageOnly | Equivalence | `blake3::hash(data) == verify_digest_match(data)` when both succeed |
| 14 | `fuzz_admission_input_surface` | CoverageOnly | Equivalence | Strict and relaxed policies agree on success/failure for identical inputs |
| 15 | `fuzz_readback_family_set` | CoverageOnly | Structural | Classification ∈ `{Full, Partial, Absent, Unreadable}`, no `Unreadable` when all readable |
| 16 | `fuzz_accepted_artifact_decode` | CoverageOnly | Structural | Decoded artifact has `accepted_at_seq > 0`, `gate_count` matches |
| 17 | `fuzz_recovery_decode` | CoverageOnly | Structural | Seed has non-zero fields when events non-empty, match `RecoveryError` variants |
| 18 | `fuzz_collect_page_pagination` | MISSING | Structural | **IMPLEMENT**: page_count = ceil(list_len/page_size), each page item count ≤ page_size, page_size=0 → error, empty list → empty, non-list → error |
| 19 | `fuzz_action_tracker` (src/bin) | CoverageOnly | REMOVE | Remove duplicate, reference shared `fuzz_lib::fuzz_action_tracker` |
| 20 | `decode_record` (fuzz_targets) | CoverageOnly | TypedError | Replace `.ok()` with `match`, exhaustive `JournalError` match, `is_valid()` on Ok |
| 21 | `expr_eval` (fuzz_targets) | CoverageOnly | Verify | Delegates to `fuzz_lib::fuzz_expr_eval` (#10) |

**Acceptance**:
- [ ] All 21 targets have assertion strength ≥ TypedError
- [ ] All error matches are exhaustive over currently-defined variants
- [ ] Zero `let _ = result.ok()` patterns remain in any harness
- [ ] Each harness has at least one domain-invariant assertion on success

### RQ-02: Fix C.25 — Implement collect_page Pagination

**Description**: The `fuzz_collect_page_pagination` function called by `src/bin/collect_page_pagination.rs` does not exist in `fuzz/src/lib.rs`. It must be implemented.

**Spec Ref**: EXECUTE.md §1.2, RED_QUEEN_MASTER_PLAN.md L7

**Implementation Contract**:

```rust
// In fuzz/src/lib.rs, add a hardened version:
pub fn fuzz_collect_page_pagination(data: &[u8]) {
    // Must handle:
    // - Empty data → return early
    // - Derive list items from fuzz bytes
    // - Derive page_size from fuzz bytes
    // - Call collect_page with list and page_size
    // - Assert: page_count == ceil(list_len / page_size)
    // - Assert: each page item_count <= page_size
    // - Assert: page_size == 0 → error (not panic)
    // - Assert: empty list → empty result (not error)
    // - Assert: non-list slot → typed error
    // - Assert: collect_page never panics
}
```

**Acceptance**:
- [ ] Function `fuzz_collect_page_pagination` exists in `fuzz/src/lib.rs`
- [ ] All 6 pagination invariants are asserted
- [ ] Function never panics on any `&[u8]` input
- [ ] 10-second smoke passes with zero crashes
- [ ] 10-second ASAN smoke passes with zero crashes

### RQ-03: Verify C.21-C.24 Fixes

**Description**: Four targets (`generated_compare`, `compiled_ir`, `ipc_frame`, `expression`) were claimed fixed in prior beads. They must be verified with 1-hour ASAN campaign.

**Spec Ref**: EXECUTE.md §1.3

**Implementation Contract**:

| Target | Fixed In | Assertion Strength | Required Invariant |
|--------|----------|-------------------|-------------------|
| `generated_compare` | C.21 | Equivalence | validation and workflow construction agree; independent decode produces identical digest/node/slot count |
| `compiled_ir` | C.22 | Structural | slot bounds checked for all 34+ node kind variants; digest preserved; node/slot count match |
| `ipc_frame` | C.23 | Structural+Roundtrip | header re-encode matches original; payload decode typed error on mismatch; IpcPayload variant coverage |
| `expression` | C.24 | Structural | type_name non-empty on successful eval; eval result is valid SlotValue type |

**Acceptance**:
- [ ] All 4 targets pass `cargo fuzz run TARGET -- -max_total_time=3600 -rss_limit_mb=2048 -detect_leaks=1 -print_final_stats=1`
- [ ] Zero crashes in any target
- [ ] Zero leaks in any target
- [ ] All 4 targets show `total_execs > 0`
- [ ] Campaign logs saved to `/tmp/fuzz-verify-C21-C24-*.log`

### RQ-04: Create Seed Corpora

**Description**: At least 24 targets lack seed corpora. Every target must have ≥ 1 seed; structure-aware targets need ≥ 5.

**Spec Ref**: EXECUTE.md §1.4

**Implementation Contract**:

For each target T without a corpus:
```
mkdir -p fuzz/corpus/T/

# Minimum seed set:
seed_empty.bin           → 0 bytes
seed_single_00.bin       → [0x00]
seed_single_ff.bin       → [0xFF]
seed_single_7f.bin       → [0x7F]

# For format-aware targets (Parser, Roundtrip, StructureAware):
seed_valid.bin           → known-valid input (from integration test fixtures)
seed_edge.bin            → boundary input (max values, min values)
seed_corrupt.bin         → bit-flipped from valid
```

**Current corpora** (7 existing):
- `decode_record/`, `expr_eval/`, `lex_expr/`, `vb_f04l_yaml_compiler_compile/`, `wait_digest_exhaustive_collision/`, `wait_digest_sensitivity/`, `wait_sentinel_collision/`

**Targets needing corpora** (24+):
All other declared targets without a `fuzz/corpus/<name>/` directory.

**Acceptance**:
- [ ] At least 28 corpus directories exist (28 = 24 new + 7 existing, minus 3+ deferred)
- [ ] Every corpus directory contains ≥ 1 seed file
- [ ] Structure-aware targets contain ≥ 5 seeds
- [ ] No seed file is empty unless it's `seed_empty.bin`
- [ ] No seed file causes harness panic on load

### RQ-05: Refactor Stdin Boilerplate

**Description**: Extract duplicated `run_with_stdin`/`write_stderr`/`main` boilerplate from 38 `src/bin/*.rs` files into a shared `fuzz/src/bin_common.rs` module.

**Spec Ref**: EXECUTE.md §1.5, RED_QUEEN_MASTER_PLAN.md M10

**Implementation Contract**:

1. Create `fuzz/src/bin_common.rs`:
```rust
//! Shared stdin boilerplate for fuzz bin targets.

use std::io::Read;
use std::process::ExitCode;

pub fn run_with_stdin(target: fn(&[u8])) -> ExitCode {
    let mut input = Vec::new();
    match std::io::stdin().read_to_end(&mut input) {
        Ok(_) => {
            target(&input);
            ExitCode::SUCCESS
        }
        Err(error) => write_stderr(error),
    }
}

fn write_stderr(error: std::io::Error) -> ExitCode {
    use std::io::Write;
    let stderr = std::io::stderr();
    let mut handle = stderr.lock();
    match write!(handle, "stdin read error: {error}\n") {
        Ok(()) | Err(_) => {}
    }
    ExitCode::FAILURE
}
```

2. Update each `src/bin/*.rs` file to:
```rust
//! Fuzz target: TARGET_NAME.

#[cfg(feature = "fuzz")]
fn main() -> std::process::ExitCode {
    fuzz_lib::bin_common::run_with_stdin(fuzz_lib::fuzz_TARGET_NAME)
}

#[cfg(not(feature = "fuzz"))]
fn main() {}
```

**Acceptance**:
- [ ] `fuzz/src/bin_common.rs` exists with exactly one `run_with_stdin` and one `write_stderr`
- [ ] All 38 `src/bin/*.rs` files use the shared module (no local duplicates)
- [ ] `cargo check --features fuzz` succeeds for all bin targets
- [ ] Net line removal: ≥ 800 lines
- [ ] No behavior change in any target

### RQ-06: Run 1-Hour ASAN Campaign

**Description**: All hardened targets must pass a 1-hour ASAN campaign with zero crashes and zero leaks.

**Spec Ref**: EXECUTE.md §1.6

**Implementation Contract**:

```bash
for target in $(cargo fuzz list); do
    cargo fuzz run "$target" -- \
        -max_total_time=3600 \
        -rss_limit_mb=4096 \
        -print_final_stats=1 \
        -detect_leaks=1 \
        2>&1 | tee "/tmp/fuzz-1hr-$target.log"
done
```

**Acceptance**:
- [ ] `cargo fuzz list` shows all declared targets (≥ 42)
- [ ] Every target passes with zero crashes
- [ ] Every target passes with zero leaks
- [ ] Total executions per target > 0
- [ ] Logs saved for all targets
- [ ] Post-campaign corpus minimization: `cargo fuzz cmin` for each target

## Global Acceptance Gates

All of the following must be true before this bead can be closed:

- [ ] G1: 21 functions hardened (RQ-01)
- [ ] G2: C.25 collect_page implemented (RQ-02)
- [ ] G3: C.21-C.24 verified with 1-hour ASAN (RQ-03)
- [ ] G4: Seed corpora exist for all targets (RQ-04)
- [ ] G5: Stdin boilerplate refactored (RQ-05)
- [ ] G6: All targets pass 1-hour ASAN campaign (RQ-06)
- [ ] G7: Zero crashes, zero leaks across all targets
- [ ] G8: `cargo fuzz build` succeeds for all targets
- [ ] G9: All `fuzz_targets/*.rs` have `[[bin]]` entries in `fuzz/Cargo.toml`
- [ ] G10: No `let _ = result.ok()` patterns in any fuzz harness
- [ ] G11: No `unwrap`, `expect`, `panic`, `todo`, `unimplemented` in any fuzz file
- [ ] G12: Unsafe C ABI stubs in `fuzz_targets.rs` are removed or properly documented

## Non-Requirements (Out of Scope)

These items are explicitly NOT covered by this contract:

1. New P0 harnesses for unfuzzed crates (vb_boundary_inventory, vb_codegen, vb_proof_kernels) — separate beads
2. AFL++ or honggfuzz integration — Phase 6, deferred to FUTURE.md
3. Mutation testing on harnesses — Phase 6
4. Coverage reporting or dashboards — Phase 6
5. CI integration — separate bead
6. vb_5xs4_* targets — separate bead for API compatibility analysis
7. vb_ui_model_postcard_decode — crate not in workspace, disabled
8. Performance optimization of expensive admission targets — Phase 2+
9. In-memory journal stubs — Phase 2+
10. Dictionary files for AFL++ — Phase 6

## Contract Dependencies

- **Requires**: Phase 0 exit gate must be green (all targets compile, smoke, ASAN smoke)
- **Required By**: Phase 2 (new P0 harnesses), Phase 6 (CI integration, AFL++, mutation testing)
- **Parallel**: Can be done concurrently with non-fuzz beads that don't change production crate APIs

## Contract Change Log

| Date | Change | Author |
|------|--------|--------|
| 2026-05-29 | Initial contract from domain model | rust-contract agent |
