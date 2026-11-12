# machine-gate-report.md

bead_id: vb-core-lower-control-primitives
bead_title: "compiler: Lower v1 control primitives from YAML AST"
phase: 11
updated_at: 2026-05-15T00:00:00Z
attempt: 1

## Canonical Machine Gate Evidence

### verify-standard lane

```
cargo clippy -p vb_compile -- -D warnings
  → No issues found

cargo test -p vb_compile
  → 297 passed (3 suites, 2.32s)
```

### verify-deep / verify-proof lane (DISCOVERY_BLOCKED)

All deeper verification obligations (Kani, Miri, Verus, TLA+) are blocked by vb-f04l not being landed. Classified as DEFERRED_GLOBAL.

### Cargo Test Suites

| Suite | Result |
|---|---|
| vb_compile lib tests | 297 passed |
| Total | **PASS** |

### Clippy

| Check | Result |
|---|---|
| vb_compile | **PASS** — No issues found |

## Regression vs Baseline

- **Clippy**: baseline clean → still clean (no new warnings)
- **Tests**: baseline 256 pass → 297 pass (+42 new tests added by this bead)
- **DISCOVERY_BLOCKED**: pre-existing, not introduced by this bead
