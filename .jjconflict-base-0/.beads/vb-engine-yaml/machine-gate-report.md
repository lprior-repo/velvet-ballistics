# Machine Gate Report: vb-engine-yaml

STATUS: PASS

## Machine Gate Summary

Bead: `vb-engine-yaml`
State: 11 attempt 1
Workspace: `/home/lewis/src/vb-go-skill/p0-wave-20260515/vb-engine-yaml`

## Canonical Machine Gates

### Compile Gate
```
cargo check -p vb_yaml -p vb_validate -p vb_core
exit=0
Finished `dev` profile (12 crates compiled)
```
**Result**: PASS

### Test Gate: vb_yaml
```
cargo test -p vb_yaml --lib
exit=0
cargo test: 204 passed (1 suite, 0.10s)
```
**Result**: PASS

### Test Gate: vb_validate
```
cargo test -p vb_validate --lib
exit=0
cargo test: 927 passed (1 suite, 0.26s)
```
**Result**: PASS

### Test Gate: vb_core
```
cargo test -p vb_core --lib
exit=0
cargo test: 1521 passed (1 suite, 4.93s)
```
**Result**: PASS

### Formal Verification Gate
See `formal-verification-report.md` for detailed results.
- TLA+: PO-002 through PO-006 PASS
- Verus: PO-007 through PO-010 PASS
- Kani: PO-011A PASS, PO-012 PASS
- Loom: PO-013 PASS
- Waivers: PO-011B, PO-022, PO-023

## Regression Diff

No production code changes were made by this bead. All changes are verification-only (`#[cfg(kani)]`, `#[cfg(loom)]`, TLA+ models, Verus proofs).

## Decision

- **STATUS: PASS**
- All machine gates passed
- No regressions introduced
- Verification artifacts provide coverage for contract clauses