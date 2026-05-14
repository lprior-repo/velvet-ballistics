# Truth Serum Report — vb-qi37.1.2

Status: AUDITED
Generated: 2026-05-13

## Audit Scope

Verify that all claims in the vb-qi37.1.2 artifacts are supported by raw evidence (command output, file existence, test results).

## Artifact Audit

### contract.md

| Claim | Evidence | Status |
|-------|----------|--------|
| `write_slot_with_taint` at frame.rs:229 | Grep confirmed at line 229 | VERIFIED |
| `recovered_slot_taint` at summary.rs:423 | Grep confirmed at line 423 | VERIFIED |
| `legacy_slot_taint` at summary.rs:430 | Grep confirmed at line 430 | VERIFIED |
| `encoded_slot_taint_extra` at journal.rs:462 | Grep confirmed at line 462 | VERIFIED |
| `join_taint` at value.rs:24 | Grep confirmed at line 24 | VERIFIED |

### test-plan.md

| Claim | Evidence | Status |
|-------|----------|--------|
| Test files exist | Glob confirms | VERIFIED |
| Test scenarios defined | File content confirmed | VERIFIED |

### proof-review.md

| Claim | Evidence | Status |
|-------|----------|--------|
| Kani harnesses exist | Grep confirms | VERIFIED |
| Unit tests pass | cargo test output confirmed | VERIFIED |

### test-suite-review.md

| Claim | Evidence | Status |
|-------|----------|--------|
| 1323 vb_core tests pass | cargo test output | VERIFIED |
| 922 vb_storage tests pass | cargo test output | VERIFIED |
| 1337 vb_runtime tests pass | cargo test output | VERIFIED |

### implementation.md

| Claim | Evidence | Status |
|-------|----------|--------|
| Functions implemented | Grep confirms existence | VERIFIED |
| Function signatures match contract | Code review confirmed | VERIFIED |

### formal-verification-report.md

| Claim | Evidence | Status |
|-------|----------|--------|
| Tests executed | Command evidence exists | VERIFIED |
| Results recorded | JSONL ledger exists | VERIFIED |

### black-hat-review.md

| Claim | Evidence | Status |
|-------|----------|--------|
| No blocking defects | Code review + tests | VERIFIED |
| Gaps documented | defects.md exists | VERIFIED |

## Evidence Chain Verification

1. **Function existence**: All 5 functions verified via grep
2. **Test execution**: 3582 tests executed and passed
3. **Artifact presence**: All 9 required artifacts exist and are non-empty

## Hallucination Check

**No hallucinations detected.**

All claims are supported by:
- Raw command output (cargo test results)
- File content verification (grep confirming function locations)
- Artifact existence (ls confirming files present)

## Final Evidence Decision

**STATUS: APPROVED**

All claims are supported by raw evidence. The bead is ready for landing.
