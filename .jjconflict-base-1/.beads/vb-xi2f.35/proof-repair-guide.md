# Proof Repair Guide: ResourceContract Digest Proofs — REPAIR-4

## Bead

`vb-xi2f.35` — P1: digest covers resource contract semantics

## Review Result (R2)

**STATUS: REJECTED** — 18 of 25 active proof obligations rejected (3 CRITICAL findings).

## Core Progress (ACKNOWLEDGED)

REPAIR-3 correctly fixed the root architectural defect: all proof artifacts now test production code through the shared `vb_core::contract_encoding::encode_contract_bytes()` function. The production `canonical_digest(source, contract)` accepts a `ResourceContract` parameter in both compilation paths. This architecture should be preserved.

## Remaining Issues (3 CRITICAL)

### CRITICAL #1: Stale YAML Strings (PF-VB-012, PF-VB-013)

**Files to fix:**

Proptest (4 files):
- `crates/vb_compile/tests/proptest_entry_point_contract.rs`
- `crates/vb_compile/tests/proptest_dual_path_equivalence.rs`
- `crates/vb_compile/tests/proptest_digest_determinism.rs`
- `crates/vb_compile/tests/proptest_with_default_equivalence.rs`

Kani (5 files):
- `crates/vb_compile/src/kani_resource_contract_digest_determinism.rs`
- `crates/vb_compile/src/kani_resource_contract_migration_digest.rs`
- `crates/vb_compile/src/kani_resource_contract_entry_point.rs`
- `crates/vb_compile/src/kani_resource_contract_dual_path_equivalence.rs`
- `crates/vb_compile/src/kani_resource_contract_digest_field_sensitivity.rs`

**Problem:** All embedded YAML strings are missing the `when` field required by the current `vb_yaml::parse_workflow_source` parser. Tests panic with `MissingField { field: "when" }`.

**Fix:** Check what the working `when` field format looks like (search for valid YAML in working tests or the `crates/vb_compile/tests/proptest_contract_field_sensitivity.rs` file that actually works — it tests `encode_contract_bytes` directly without YAML, so it bypasses the parser issue). Look at existing integration tests (`crates/workspace_tests/`) for valid YAML examples. Update all 9 files to include the required `when` field.

**Validation:** After fixing, run `cargo test` on each proptest file to confirm it parses correctly:
```bash
cargo test -p vb_compile --test proptest_entry_point_contract -- --nocapture
cargo test -p vb_compile --test proptest_dual_path_equivalence -- --nocapture
cargo test -p vb_compile --test proptest_digest_determinism -- --nocapture
cargo test -p vb_compile --test proptest_with_default_equivalence -- --nocapture
```

### CRITICAL #2: Execute All Kani Harnesses

After fixing YAML, run all 14 `cargo kani` commands with `#[cfg(kani)]` enabled:

```bash
# Workdir: crates/vb_compile
RUSTFLAGS='--cfg kani' cargo kani --harness prove_digest_determinism --unwind 3
RUSTFLAGS='--cfg kani' cargo kani --harness prove_single_field_changes_digest --unwind 3
RUSTFLAGS='--cfg kani' cargo kani --harness prove_no_cross_field_collision --unwind 2
RUSTFLAGS='--cfg kani' cargo kani --harness prove_migration_digest_relationship --unwind 2
RUSTFLAGS='--cfg kani' cargo kani --harness prove_contract_survives_compilation --unwind 3
RUSTFLAGS='--cfg kani' cargo kani --harness prove_secret_results_changes_digest --unwind 2
RUSTFLAGS='--cfg kani' cargo kani --harness prove_dual_path_digest_equivalence --unwind 3
RUSTFLAGS='--cfg kani' cargo kani --harness prove_canonical_policy_digest_agree_on_identity --unwind 2

# Workdir: crates/vb_core
RUSTFLAGS='--cfg kani' cargo kani --harness prove_canonical_contract_has_17_fields --unwind 1
RUSTFLAGS='--cfg kani' cargo kani --harness prove_type_identity_across_paths --unwind 1
RUSTFLAGS='--cfg kani' cargo kani --harness prove_validation_covers_all_17_fields --unwind 3
RUSTFLAGS='--cfg kani' cargo kani --harness prove_encoding_no_collision --unwind 2

# Workdir: crates/vb_runtime
RUSTFLAGS='--cfg kani' cargo kani --harness prove_secret_result_not_allowed_enforcement --unwind 3
```

Capture ALL raw output to evidence files.

### CRITICAL #3: Fix Verus Vacuous Proof (for vb-xi2f.36)

When vb-xi2f.36 resumes Verus work, the `verification/verus/vb_compile/digest_contract_binding.rs` file needs `default_contract_encoding()` and `non_default_contract_encoding()` to return actual different encodings, not both return `Seq::empty()`. Currently the `requires` clause is always false, making the proof vacuously true.

## Secondary Repairs

### Replace kani::cover!(true) (MEDIUM)

10+ instances of `kani::cover!(true)` remain across 9 files. Replace each with a meaningful cover:
- `kani::cover!(base != modified)` — verifies mutation reachable
- `kani::cover!(field_idx < 17)` — verifies field enumeration
- `kani::cover!(val_a != val_b)` — verifies different values reachable

Every harness should have at least one meaningful cover.

### Update Trust Ledger (LOW)

Two stale entries in `trusted-base-ledger.jsonl`:
- **T5-IMPL-PREREQUISITE** (line 21): Delete or update — prerequisites ARE now completed
- **T5-KANI-HARNESS-INTEGRATION** (line 19): Delete or update — module declarations now exist in lib.rs

### Log Proof-Writer Invocation (MEDIUM)

Append to `agent-invocation-ledger.jsonl`:
```json
{"timestamp":"2026-05-25T...","agent":"proof-writer","bead_id":"vb-xi2f.35","state":5,"action":"repair","repair_attempt":3,"artifacts":["contract_encoding.rs","part_05.rs","part_01.rs",...],"summary":"REPAIR-3: Production code fixed; harnesses rewritten to call production functions"}
```

## Dependency Order

```
STEP 1: Fix YAML strings (all 9 files) ← BLOCKS ALL ELSE
    │
    ├── STEP 2: Run all 6 proptest suites → capture evidence
    ├── STEP 3: Execute all 14 Kani commands → capture evidence
    │
    ├── STEP 4: Replace kani::cover!(true) with meaningful covers
    ├── STEP 5: Clean up trust ledger
    ├── STEP 6: Log proof-writer invocation
    │
    └── STEP 7: Re-submit to proof-reviewer (State 6)
```

## Estimated Effort

| Step | Effort | Blocker? |
|------|--------|----------|
| Fix YAML strings | 30 min | YES (blocks all execution) |
| Run proptest suites | 15 min | YES |
| Run Kani harnesses | 60-120 min | YES (requires Kani toolchain) |
| Replace kani::cover!(true) | 30 min | No |
| Trust ledger cleanup | 10 min | No |
| Log invocation | 5 min | No |

## Fallback Position

If Kani toolchain is unavailable and cannot be installed, file a `TOOL_UNAVAILABLE` waiver for all Kani obligations with the rationale that:
- Proptest suites provide statistical defense-in-depth for the same properties
- Production code is architecturally correct (shared encoding, dual-path consistency)
- Verus proofs will provide unbounded verification when toolchain is available (vb-xi2f.36)
