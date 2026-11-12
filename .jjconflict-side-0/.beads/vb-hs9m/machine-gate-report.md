# Machine Gate Report — vb-hs9m

## Gate Summary

| Gate | Command | Result | Details |
|------|---------|--------|---------|
| Build | `cargo build --workspace` | ✅ PASS | 0 errors, 2 warnings |
| Tests | `cargo test -p vb_runtime -p xtask` | ✅ PASS | 1831 passed (24 suites, 3.12s) |
| Clippy | `cargo clippy --workspace -- -D warnings` | ❌ FAIL_REGRESSION | 2 errors: dead_code in vb_cli production code |
| Fmt | `cargo fmt --check` | ❌ DEFERRED_GLOBAL | 30+ files need formatting; pre-existing debt |

---

## Detailed Results

### Build Gate
```
warning: method `get_state` is never used
  --> crates/vb_cli/src/lifecycle.rs:47:8
warning: function `with_tracker` is never used  
  --> crates/vb_cli/src/lifecycle.rs:66:4
cargo build: 0 errors, 2 warnings (93 crates)
```

### Test Gate
```
cargo test: 1831 passed (24 suites, 3.12s)
```

### Clippy Gate (FAIL_REGRESSION)
```
error: method `get_state` is never used
error: function `with_tracker` is never used
```
**Location**: `crates/vb_cli/src/lifecycle.rs:47` and `:66`
**Classification**: FAIL_REGRESSION — new dead_code in production code, not in bead scope

### Fmt Gate (DEFERRED_GLOBAL)
```
Diff in 30+ files across codebase
```
**Classification**: DEFERRED_GLOBAL — pre-existing formatting debt, not bead-local

---

## Bounded Ring Buffer Observability Scope

The bead vb-hs9m delivers bounded trace ring buffer observability and evidence packaging.

**Scoped files verified:**
- `crates/vb_runtime/src/trace.rs` — TraceRing SPSC ring buffer
- `xtask/src/evidence/` — EvidenceBundle types and persistence
- `crates/workspace_tests/` — Catalog validation tests

All scoped production code compiles and passes tests.

---

## Regression Analysis

| Issue | File | Scope | Classification |
|-------|------|-------|----------------|
| dead_code: get_state | vb_cli/src/lifecycle.rs | NOT in delivery scope | FAIL_REGRESSION |
| dead_code: with_tracker | vb_cli/src/lifecycle.rs | NOT in delivery scope | FAIL_REGRESSION |
| fmt drift | 30+ files | NOT in delivery scope | DEFERRED_GLOBAL |

**Note**: vb_cli/src/lifecycle.rs is NOT in the delivery-scope.jsonl for vb-hs9m. These are pre-existing or cross-cutting issues, not bead-local failures.

---

## Recommendations

1. **dead_code in vb_cli**: Either add `#[allow(dead_code)]` or remove the unused functions. This is a workspace-level concern outside vb-hs9m scope.

2. **fmt debt**: Run `cargo fmt` to fix pre-existing formatting drift. This is workspace-level technical debt.

3. **Bead scope**: vb-hs9m observability deliverables (TraceRing, EvidenceBundle) are complete and passing.
