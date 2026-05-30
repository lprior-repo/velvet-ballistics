# Architectural Drift Report: `vb_runtime_recovery_hydration_tests.rs`

## File Overview

| Attribute | Value |
|-----------|-------|
| **File** | `crates/vb_runtime/tests/recovery_hydration_tests.rs` |
| **Total Lines** | 2087 |
| **Test Functions** | 41 `#[test]` |
| **Kani Proofs** | 12 `#[kani::proof]` (inside `#[cfg(kani)]` module) |
| **Total Test Items** | 53 |
| **Location Category** | `tests/` — correctly placed integration test file |

---

## Drift Assessment

### ✅ Passes: Location Contract
- File is correctly located at `crates/vb_runtime/tests/` (integration test directory)
- Not in repository root or improperly nested

### ❌ FAILS: Size Gate (300 Line Limit)

| Gate | Limit | Actual | Status |
|------|-------|--------|--------|
| Max lines per `.rs` file | 300 | 2087 | **VIOLATION (+1787)** |

The file is **6.96× over** the maximum line count.

---

## Structural Analysis

### Test Coverage (13 Sections)

| Section | Topic | Approx. Lines | Tests |
|---------|-------|---------------|-------|
| 1 | Clean shutdown recovery | 134–237 | 3 |
| 2 | Crash (partial journal) recovery | 239–361 | 3 |
| 3 | Hydration from journal events | 363–577 | 5 |
| 4 | Hydration with missing events | 579–660 | 3 |
| 5 | Corrupted snapshot handling | 662–757 | 3 |
| 6 | Checkpoint create/restore | 759–854 | 2 |
| 7 | Incremental recovery | 856–933 | 2 |
| 8 | Recovery idempotency | 935–1046 | 3 |
| 9 | Max-size journal recovery | 1048–1156 | 2 |
| 10 | Additional combinatorial coverage | 1158–1414 | 7 |
| 11 | Runtime boundary recovery | 1662–1763 | 3 |
| 12 | Advanced hydration scenarios | 1784–1923 | 4 |
| 13 | Kani verification proofs | 1924–2087 | 12 |

---

## DDD Cohesion Assessment

The file exhibits **good cohesion** within sections — each section tests a distinct recovery/hydration concern. However, the **file-level cohesion is poor** due to size.

**Good patterns observed:**
- Helper functions (`test_digest`, `open_journal`, `write_events_strict`, `build_two_step_finished_run`, `test_admission_event`) are properly isolated at top
- Clear `SECTION` comment markers for navigation
- BDD-style test names: `Given/When/Then` in doc comments

---

## Recommendations

### Primary: Split the File

The file should be split into **minimum 7–13 files** based on the existing section boundaries:

```
crates/vb_runtime/tests/recovery_hydration/
├── mod.rs                              # Re-exports helpers
├── clean_shutdown_tests.rs             # Section 1 (3 tests)
├── crash_partial_journal_tests.rs      # Section 2 (3 tests)
├── hydration_from_events_tests.rs      # Section 3 (5 tests)
├── missing_events_tests.rs             # Section 4 (3 tests)
├── corrupted_snapshot_tests.rs         # Section 5 (3 tests)
├── checkpoint_tests.rs                  # Section 6 (2 tests)
├── incremental_recovery_tests.rs        # Section 7 (2 tests)
├── idempotency_tests.rs                # Section 8 (3 tests)
├── max_size_journal_tests.rs           # Section 9 (2 tests)
├── combinatorial_coverage_tests.rs      # Section 10 (7 tests)
├── runtime_boundary_tests.rs            # Section 11 (3 tests)
├── advanced_hydration_tests.rs          # Section 12 (4 tests)
└── kani_recovery_tests.rs              # Section 13 (12 proofs, behind cfg)
```

### Secondary: Module Organization

After splitting:
1. Create `crates/vb_runtime/tests/recovery_hydration/mod.rs` that re-exports the shared helpers
2. Update `crates/vb_runtime/tests/mod.rs` to reference the new module tree
3. Ensure `cargo test --package vb_runtime` still discovers all tests

---

## Verdict

| Check | Result |
|-------|--------|
| Location contract | ✅ PASS |
| Size gate (<300 lines) | ❌ **FAIL** — 2087 lines |
| DDD cohesion | ⚠️  Needs splitting |
| Test organization | ✅ Well-structured sections |

**STATUS: REQUIRES REFACTOR** — File must be split before landing.
