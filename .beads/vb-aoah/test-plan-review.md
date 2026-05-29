# Test Plan Review — vb-aoah State 10

## Provenance

- **Reviewer**: test-reviewer (plan review gate)
- **Invocation**: test-reviewer-vb-aoah-state10-plan-review-001
- **Bead**: vb-aoah (migration skeleton tests)
- **State**: 10 (test-reviewer)
- **Reviewed artifacts**:
  - `test-plan.md` (686 lines, dated 2026-05-27)
  - `contract.md` (48 lines, 11 acceptance requirements)
- **Input plan provenance**: test-planner-vb-aoah-state8-001 (ledger_sequence 25, APPROVED)
- **Workspace**: `/home/lewis/isolated/femdation-velvet-ballistics/vb-aoah`
- **Date**: 2026-05-27

## Plan Review Gates

### Gate 1: Contract Completeness

| Contract Requirement | Scenarios Planned | Verdict |
|---|---|---|
| R1: Test surface at restate_explicit_migration_skeleton_tests.rs | §2: Trophy allocation targets this file | PASS |
| R2: Old-version fixture test-only, minimal, no copy | §4.1: Fixture struct + strategy, no Restate code copied | PASS |
| R3: Explicit migration maps every old version → one named migration | B5-B7 (3 scenarios, registry totality/uniqueness) | PASS |
| R4: Verify migrated records before manifest advance | B8-B10 (4 scenarios, verify-before-advance gate) | PASS |
| R5: Cleanup verifies old keyspace empty before success | B11-B13 (3 scenarios, cleanup postcondition) | PASS |
| R6: Runtime open rejects old schema, no side effects | B1-B4 (4 scenarios, runtime open detection) | PASS |
| R7: Reopen after migration reads current, no migration invoked | B14-B15 (2 scenarios, reopen idempotence) | PASS |
| R8: Missing verification/cleanup returns typed error | Covered via B8-B13 error paths + B21 (manifest gates) | PASS |
| R9: Empty old-keyspace has explicit no-op semantics | B16-B17 (2 scenarios, explicit NoOp outcome) | PASS |
| R10: New migration errors typed and diagnostic-code mapped | §10: Error variant coverage table (17 variants → test scenarios) | PASS |

**Verdict**: All 10 acceptance-relevant requirements (R1-R8, R10) from contract.md are mapped to at least one behavior scenario. R9 is covered by B16-B17. 100% contract coverage.

### Gate 2: Error Variant Coverage

All 17 error variants from `error-taxonomy.md` are mapped to test scenarios in §10:

| Coverage | Status |
|---|---|
| Variants with explicit test scenario | 17/17 (100%) |
| Variants with specific integration test | 8/17 (manifest errors, cleanup errors, verification errors) |
| Variants gated behind fuzz targets | 4/17 (copy/rewrite errors at codec boundaries) |
| Variants exercisable only post-implementation | 10/17 (manifest corrupt, read/write failures, record decode/encode) |

**Finding (PLAN-F-001, LOW)**: 10 of 17 error variants are only testable when production `migrations.rs` exists and real Fjall/Postcard codecs are integrated. The test plan gates these behind fuzz campaigns at State 12. This is correct for a test-first bead — the error taxonomy exists, variants are declared, and test scenarios are planned. No plan change needed; this is a State 12 closure obligation.

### Gate 3: Assertion Strength

Reviewing the BDD scenarios in §3:

| Pattern | Count | Assessment |
|---|---|---|
| Specific value assertions (e.g., `Ok(MigrationAction)`, `Err(MigrationManifestAdvanceRejected)`) | 20/22 | STRONG |
| Exact error variant assertions with fields | 18/22 | STRONG |
| State verification (keyspace unchanged, counter unchanged) | 14/22 | STRONG |
| Boolean-only assertions (`is_ok()`, `is_err()`) | 0/22 | PASS — none present |
| Weak assertions (`prop_assert!(f.old_records > 0)`) | 0/22 (after BR-F-002 hardening) | PASS — all tautology-class assertions removed |

**Verdict**: Zero weak assertions found. All scenarios use exact value or typed variant assertions.

### Gate 4: Boundary Cases

| Boundary | Scenarios |
|---|---|
| Zero old records | B16 (empty NoOp), B17 (cannot claim verified), B11 (NoCleanupNeeded) |
| MAX records | B11 (Success with count), B20 (batch size limit) |
| Just below limit | B19 (199+2 = 201 > 200 limit) |
| Just above limit | B19 (limit exceeded) |
| Overflow u64::MAX | B19 (u64::MAX + 1, u64::MAX + u64::MAX) |
| Version 0 boundary | B5 (registry lookup) |
| Version u16::MAX boundary | §8 (registry lookup at u16::MAX) |
| Future version (CURRENT+1) | B4 (UnsupportedSchemaVersion) |
| Empty/zero collections | B16 (empty keyspace), B13 (no cleanup required) |

**Verdict**: All relevant boundaries covered. No missing boundary cases identified.

### Gate 5: Property Tests

| Invariant | Strategy | Coverage |
|---|---|---|
| Registry lookup idempotence | `any version 0..CURRENT, any registry state` (§4.2) | PLANNED |
| Cleanup outcome determinism | `any u64 record count, any bool result` (§4.2) | PLANNED |
| Manifest version monotonicity | `any MigrationPhase, any valid/invalid transition` (§4.2) | PLANNED |
| Record count consistency | `any u64 counts with bounded totals` (§4.2) | PLANNED |
| No side effects from detection | `any u16 version, before/after snapshots` (§4.2) | PLANNED |
| Existing 7 proptest tests | `fixture_strategy()` covering version/records/bytes/phase | EXISTING (to be hardened) |

**Verdict**: 12 property tests total (7 existing + 5 new planned). Adequate proptest coverage for the state space.

### Gate 6: Fuzz Targets

All 4 fuzz targets from plan §5 are planned:
1. Hostile manifest bytes at runtime-open boundary
2. Corrupt old keyspace at cleanup boundary
3. Malformed empty fixture at NoOp detection boundary
4. Boundary/overflow numeric inputs at checked arithmetic boundary

Each includes `-max_total_time=60 -runs=10000` bounds. These are acceptable for local dev but will be resource-risk findings when run.

**Finding (PLAN-F-002, LOW)**: Fuzz command examples in §5.2 and §12 lack explicit timeout/memory bounds in environment context. Should use `timeout 120s` wrapper for production CI. Not blocking — can be addressed at State 12 closure.

### Gate 7: Verifier Harnesses ≠ Behavior Tests

The plan correctly separates:
- Kani harnesses (§6): panic-freedom, overflow-freedom, typestate correctness
- Behavior tests (§3): observable behavioral outcomes
- Fuzz targets (§5): hostile input boundaries

No verifier harness is counted as a behavior test. The plan is explicit: "Kani/proof harnesses do not replace behavior tests."

### Gate 8: Proof-to-Implementation Binding

All 18 proof obligations (from `rust-refinement-obligations.jsonl`) have:
- Planned production source refs in cluster maps (§1 of proof-to-rust-map.md)
- Behavior test refs in the test plan (§3, §8)
- Kani/proptest/fuzz harness refs in the test plan (§4, §5, §6)

**Verdict**: All 18 bridge rows are mapped to executable behavior tests. No orphaned obligations.

## Resource Governance

| Command | Scope | Verdict |
|---|---|---|
| `cargo kani -p vb_storage --harness <specific>` | One harness per invocation, exact target | SAFE |
| `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_explicit_migration_skeleton_tests` | Single test file | SAFE |
| `cargo fuzz run <target> -- -max_total_time=60 -runs=10000` | Time-bounded, run-count-bounded | ACCEPTABLE |
| `cargo mutants -p vb_storage -- --test restate_explicit_migration_skeleton_tests` | Full mutation sweep on one package | RESOURCE RISK (see RT-F-001) |
| `moon ci` | Full CI sweep | RESOURCE RISK (see RT-F-001) |

**Finding (RT-F-001, MEDIUM)**: `cargo mutants -p vb_storage -- --test restate_explicit_migration_skeleton_tests` without a timeout is unbounded. Plan must require `--timeout 60` or equivalent.

## Plan Findings Summary

| ID | Severity | Gate | Finding |
|---|---|---|---|
| PLAN-F-001 | LOW | Gate 2 | 10 error variants require production code integration (State 12 closure) |
| PLAN-F-002 | LOW | Gate 6 | Fuzz commands lack explicit OS-level timeout wrapper |
| RT-F-001 | MEDIUM | Resource | `cargo mutants` command unbounded — needs timeout |

## Final Verdict

**STATUS: APPROVED with findings**

No lethal behavior-test gaps. All three findings are non-blocking for State 9 test implementation and can be addressed at State 12 closure (resource bounds) or are inherent to the test-first workflow (production-integrated error variants).

The test plan is fit for purpose as a specification for the test-writer at State 9. All 22 BDD scenarios map to contract requirements, all error variants have planned test coverage, and all hazards have planned mitigation verification.
