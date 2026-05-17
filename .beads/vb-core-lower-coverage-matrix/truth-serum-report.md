# Truth-Serum Report — vb-core-lower-coverage-matrix

## Audit Context
- **Bead**: vb-core-lower-coverage-matrix
- **Audit Date**: 2026-05-17
- **Auditor**: Truth-Serum

## Raw Evidence Commands

### Cargo Test Evidence
```bash
cd /home/lewis/src/velvet-ballistics && rtk cargo test -p vb_compile
```
**Output**: `cargo test: 294 passed (5 suites, 12.26s)`
**Verification**: VERIFIED

### Verus Verification Evidence
```bash
cd /home/lewis/src/velvet-ballistics && verus verification/verus/v1_primitive_lowering.rs
```
**Output**: `verification results:: 15 verified, 0 errors`
**Verification**: VERIFIED

### Compilation Fix Evidence
```bash
cd /home/lewis/src/velvet-ballistics && rtk cargo build -p vb_compile
```
**Output**: `cargo build (1 crates compiled) Finished dev profile`
**Verification**: VERIFIED

## Anti-Hallucination Check
- [x] No command output invented
- [x] No test counts fabricated
- [x] No verifier status faked
- [x] No reviewer approval claimed without evidence
- [x] No commit IDs invented
- [x] No paths fabricated
- [x] No waiver decisions faked

## Findings
- All raw evidence is authentic and traceable
- Test counts match actual command output
- Verus results match actual command output
- No hallucinated claims detected

**STATUS**: PASS