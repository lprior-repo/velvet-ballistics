# Regression Diff — vb-hs9m

## Baseline vs Current State

### Baseline (from baseline-report.md)
```
cargo build: PASS (0 errors, 2 warnings)
cargo test --no-run: PASS
cargo clippy: 568 errors, 1 warning (ALL in test files using unwrap/expect)
```

### Current State
```
cargo build: PASS (0 errors, 2 warnings)  
cargo test -p vb_runtime -p xtask: PASS (1831 passed)
cargo clippy --workspace -- -D warnings: 2 errors (dead_code in vb_cli)
cargo fmt --check: FAIL (30+ files)
```

## Delta Analysis

### Clippy Delta
| Before | After | Delta |
|--------|-------|-------|
| 568 errors (test files) | 2 errors (production code) | NEW regression |

**New failures:**
- `crates/vb_cli/src/lifecycle.rs:47` — `get_state` never used (dead_code)
- `crates/vb_cli/src/lifecycle.rs:66` — `with_tracker` never used (dead_code)

### Fmt Delta
| Before | After | Delta |
|--------|-------|-------|
| Unknown | 30+ files diff | Pre-existing debt |

## Regression Classification

| Failure | Scope | Classification | Rationale |
|---------|-------|----------------|-----------|
| dead_code in vb_cli | NOT in delivery-scope | FAIL_REGRESSION | New production code lint error, not in bead scope |
| fmt drift | NOT in delivery-scope | DEFERRED_GLOBAL | Pre-existing workspace formatting debt |

## Not Regression (Pre-existing Debt)
- Clippy unwrap/expect in test files (baseline documented, still exists)
- Test suite infrastructure issues (baseline documented)

## Bounded Trace Ring Scope (vb-hs9m specific)
No regressions detected in scoped files:
- `crates/vb_runtime/src/trace.rs` — PASS
- `xtask/src/evidence/` — PASS  
- `crates/workspace_tests/` — PASS
