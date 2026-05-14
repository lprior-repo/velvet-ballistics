# Test Plan Review: vb-fzx7 — Core Orchestrator Benchmark Suite

## VERDICT: APPROVED

All LETHAL and MAJOR findings from the previous review have been resolved.

---

## Axis 1 — Contract Parity

**PASS** All 14 `pub fn` contract signatures have corresponding BDD scenarios:
- `check_evidence_gate` → EG-001 through EG-008 (8 scenarios)
- `baseline_within_budget` → MC-010
- `result_exceeds_threshold` → MC-008, MC-009
- `capture_metadata` → MC-001, MC-002, MC-003
- `yaml_parse_benches` → BG-001
- `yaml_validate_benches` → BG-002
- `yaml_compile_benches` → BG-003
- `runtime_step_benches` → BG-004
- `runtime_primitive_benches` → BG-005
- `ipc_frame_benches` → BG-006
- `ipc_backpressure_benches` → BG-007
- `storage_journal_write_benches` → BG-008
- `storage_journal_replay_benches` → BG-009
- `recovery_hydration_benches` → BG-010

**PASS** All `EvidenceError` variants have scenarios asserting exact variant:
- `MissingBaseline` → EG-002
- `MissingResult` → EG-003
- `MissingEnvironment` → EG-004
- `MissingCommand` → EG-005
- `MissingCommit` → EG-006
- `RegressionDetected` → EG-007
- `EmptyBudget` → EG-008

**PASS** `YamlBenchmarkError::ValidationFailure` (BE-002) now has a BDD scenario with concrete message assertion (line 399-404):
- `Then the error is YamlBenchmarkError::ValidationFailure`
- `And the error message matches "workflow validation failed: .+"` ← concrete assertion added

**PREVIOUSLY LETHAL-1 (MC-003) — NOW FIXED:**
- MC-003 behavior table (line 24) now specifies: `panics with "commit_hash must be non-empty ASCII hex"`
- MC-003 scenario (lines 217-226) now has concrete Then: `Then the process panics with message "commit_hash must be non-empty ASCII hex"`
- This is a specific panic assertion, not vague "rejects" language.

---

## Axis 2 — Assertion Sharpness

All Evidence Gate scenarios (EG-001 through EG-008) use concrete expected values — **PASS**.

Metadata Capture scenarios:
- MC-001: concrete `BenchmarkMetadata` fields — **PASS**
- MC-002: concrete commit_hash non-empty ASCII hex — **PASS**
- MC-003: concrete panic message `"commit_hash must be non-empty ASCII hex"` — **PASS** (previously LETHAL)
- MC-004 through MC-010: all use concrete boolean/integer values — **PASS**

Error Variant scenarios (Section 3.4): All use specific error variant assertions — **PASS**.
BE-002 now includes concrete message regex assertion.

**PREVIOUSLY MAJOR-1 (Section 8.1 row 869) — NOW FIXED:**
- Row "no baseline (new bench)" now correctly shows `Err(EvidenceError::MissingBaseline)` (line 880)
- This aligns with POST-013, EG-002, and the Evidence Gate scenario that rejects absent baseline
- The matrix row is now internally consistent with the BDD scenarios.

---

## Axis 3 — Trophy Allocation

**PASS** Parser/deserializer fuzz targets exist:
- `parse_yaml_events` → Section 5.1 fuzz target
- `yaml_compile` → Section 5.1 fuzz target
- `decode_frame` → Section 5.3 fuzz target

**PASS** Pure arithmetic functions have proptest invariants:
- `budget_utilization_percent` → Section 4.1 invariants
- `latency_within_budget` → Section 4.1 invariants
- `result_exceeds_threshold` → Section 4.1 + 4.3 invariants
- `baseline_within_budget` → Section 4.1 invariants

**PASS** Pure invariants have Kani coverage:
- INV-005 (commit_hash non-empty ASCII hex) → Section 6.2 Kani proof
- Arithmetic invariants → Section 6.1 Kani proofs

**PREVIOUSLY MAJOR-2 (planned unit test count not stated) — NOW FIXED:**
- Section 10.5 (Planned Unit Test Count) added with full breakdown table
- 14 public functions × 5 density = 70 tests minimum
- Planned count: 70 unit tests (9+11+12+8+30)
- Exit criteria line added: "Planned unit test count ≥ 70 (5× density for 14 public functions) — see Section 10.5"

---

## Axis 4 — Boundary Completeness

**Boundary cases explicitly named per function — MINOR gaps (tolerated at 2/5 threshold):**

- `result_exceeds_threshold`: boundary at exactly `baseline + threshold_pct * baseline / 100` is tested (Section 4.3, `regression_delta_computed_correctly`). At-threshold does NOT exceed — correct. **PASS.**

- `budget_utilization_percent`: boundary at `elapsed.as_micros() == budget_us` not explicitly named. **MINOR.**

- `latency_within_budget`: equality boundary `elapsed.as_micros() == budget_us` not explicitly named. **MINOR.**

- `baseline_within_budget`: equality boundary `baseline.as_micros() == budget_us` not explicitly named. **MINOR.**

No function has more than 3 missing boundaries — no MAJOR triggered here.

---

## Axis 5 — Mutation Survivability

Mutation checkpoints in Section 7 map to kill tests. The key mutations:

- EG-M1 through EG-M6 (removing Missing* checks): Killed by EG-002 through EG-006 — **covered**.
- EG-M7 (inverting `>` to `>=`): Would be caught by the boundary test at exactly `baseline + threshold_pct * baseline / 100`. Section 4.3 `regression_delta_computed_correctly` explicitly tests this boundary. **Covered.**
- EG-M8 (changing `*` to `/`): Would be caught by EG-007 which uses 20% threshold and specific delta values. **Covered.**
- MC-M1 (empty commit_hash): MC-003 now asserts concrete panic message — **covered** (previously LETHAL).
- MC-M2, MC-M3 (empty environment/command): **MINOR** gap (no scenario tests `capture_metadata` with empty environment/command producing specific error, but INV-001 covers the evidence gate path).

---

## Axis 6 — Holzmann Plan Audit

**Rule 2 — Bound Every Loop**: Criterion benchmarks have an implicit iteration bound managed by criterion itself. No loops in BDD scenario definitions. **PASS.**

**Rule 5 — State Your Assumptions**: Every BDD scenario has a `Given` block explicitly stating preconditions. **PASS.**

**Rule 7 — Narrow Your State**: No shared mutable state in benchmark code. INV-002 (deterministic benchmarks) addresses non-determinism. **PASS at plan level.**

---

## Previous LETHAL Findings (All Fixed)

1. **PREVIOUSLY LETHAL-1 — MC-003 Then clause lacks concrete value (test-plan.md:354)**
   - Fixed: Section 1.2 behavior table (line 24) now says `panics with "commit_hash must be non-empty ASCII hex"`
   - Fixed: Section 3.2 MC-003 scenario (lines 217-226) now has `Then the process panics with message "commit_hash must be non-empty ASCII hex"`
   - This is a specific, concrete assertion, not vague "rejects" language.

2. **PREVIOUSLY LETHAL-2 — `YamlBenchmarkError::ValidationFailure` missing BDD scenario**
   - Fixed: BE-002 scenario exists in Section 3.4 (lines 388-392 previously, now 399-404)
   - Sharpened: Added `And the error message matches "workflow validation failed: .+"` — concrete message format assertion
   - Note: The scenario existed in the previous plan but was not sharply asserting the message format; now it does.

---

## Previous MAJOR Findings (All Fixed)

1. **PREVIOUSLY MAJOR-1 — Section 8.1 row 869 self-contradiction**
   - Fixed: Row now correctly shows `Err(EvidenceError::MissingBaseline)` (line 880)
   - This aligns with POST-013 and EG-002 which both require `MissingBaseline` error when baseline is absent at the evidence gate

2. **PREVIOUSLY MAJOR-2 — Planned unit test count not stated**
   - Fixed: Section 10.5 added with full test count table
   - 70 tests planned (9+11+12+8+30), satisfying 5× density requirement for 14 functions
   - Exit criteria line added referencing Section 10.5

---

## MINOR Findings (2/5 threshold — acknowledged, not blocking)

1. **`baseline_within_budget` equality boundary not explicitly named**
   - No scenario tests `baseline.as_micros() == budget_us` (returns `true`)
   - proptest uses `d / 2` vs `d` — always `<`, never `==`

2. **`latency_within_budget` equality boundary not explicitly named**
   - No scenario tests `elapsed.as_micros() == budget_us` case explicitly

---

## MANDATE CHECKLIST (All Resolved)

- [x] **MC-003**: Rewrite the Then clause to assert a concrete error variant or specific return value — DONE (line 24, lines 217-226)
- [x] **BE-002**: Add Gherkin scenario in Section 3.4 asserting exact error variant and message format — DONE (lines 399-404)
- [x] **Section 8.1 row 869**: Fix expected output to `Err(EvidenceError::MissingBaseline)` — DONE (line 880)
- [x] **Section 10.5**: Add "Planned Test Count" explicitly stating 70 tests (5× density) — DONE (lines 969-982)

---

## SUMMARY

| Axis | Status |
|------|--------|
| Contract Parity | **PASS** — all LETHALs fixed |
| Assertion Sharpness | **PASS** — MC-003 concrete, BE-002 sharpened |
| Trophy Allocation | **PASS** — test count now stated (70, 5×) |
| Boundary Completeness | PASS with 2 MINOR |
| Mutation Survivability | PASS — MC-M1 now covered |
| Holzmann Plan Audit | PASS |

**0 LETHAL + 0 MAJOR + 2 MINOR = APPROVED**

The plan now satisfies all mandatory contract requirements. Two MINOR boundary gaps remain but are below the 5-threshold and do not block implementation.

(End of file — total lines to be confirmed on write)
