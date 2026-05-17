# Red Queen Report

## State 11 Rerun (Post-Refactor State 13)

### Adversarial QA Scope
- Contract: admission durability errors (POST-001, POST-002, INV-001)
- Error taxonomy verification: 5 admission variants
- Post-refactor touched files: `crates/vb_runtime/src/error/*`, `crates/velvet_ballastics/tests/admission_durability_code.rs`

### Command Evidence

**Core admission tests:**
```
rtk cargo test -p vb_runtime admission_durability --lib
=> 3 passed, 1322 filtered out

rtk cargo test -p vb_runtime admission_durability_error_variants_are_exhaustive
=> 1 passed

rtk cargo test -p vb_runtime admission_durability_errors_have_stable_codes_distinct_from_generic_storage
=> 1 passed
```

**Idempotency test:**
```
rtk cargo test -p vb_runtime duplicate_run_id_preserves_stable_diagnostic_code
=> 1 passed
```

**API envelope test:**
```
rtk cargo test -p velvet_ballastics --test admission_durability_code
=> 1 passed
```

**Full suite:**
```
rtk cargo test -p vb_runtime --lib => 1316 passed
rtk cargo test -p velvet_ballastics => 351 passed
moon run :quick => completed
```

### Contract Parity Verification

| Contract Clause | Status | Evidence |
|---|---|---|
| POST-001: admission rejection → stable variant + code | PASS | 5 variants with distinct codes (0x2011, 0x2012, 0x2014, 0x2015, 0x2004) |
| POST-002: API envelope exposes stable code without parsing | PASS | `runtime_code()` returns `Option<&'static str>`; integration test passes |
| INV-001: Display/diagnostic_code/runtime_code/PartialEq/source preserve cause | PASS | HeaderPersistenceFailed preserves `source: Arc<JournalError>`; artifact/capability variants preserve fields; generic storage path (0x2008) remains distinct |

### Error Taxonomy Coverage

| Variant | Diagnostic Code | Runtime Code | Source |
|---|---|---|---|
| `AdmissionArtifactNotFound` | 0x2011 | None | None (semantic cause) |
| `AdmissionArtifactInvalid` | 0x2014 | None | None (semantic cause) |
| `AdmissionCapabilityDenied` | 0x2012 | None | None (semantic cause) |
| `AdmissionHeaderPersistenceFailed` | 0x2015 | ADMISSION_DURABILITY_ERROR | Some(JournalError) |
| `RunAlreadyExists` | 0x2004 | None | None |

Note: INV-001 "source does not erase cause" is satisfied for HeaderPersistenceFailed (has storage source) and for semantic variants (cause preserved in variant fields, not applicable to Error::source).

### Determinism Check
- All tests use deterministic inputs (hardcoded enum variants, no randomness)
- No timing-dependent assertions
- No external I/O in tests
- **Result: DETERMINISTIC**

### Assertion Strength
- Strong exact-match assertions on diagnostic_code and runtime_code
- Distinctness assertions (admission vs generic storage, admission vs idempotency)
- Exhaustive match coverage via `diagnostic_codes_are_unique` (16 codes, 14 unique - 2 intentional collisions via Core)
- Mutation resistance: deleting any admission arm fails exhaustive match test
- **Result: STRONG**

### State-Space Coverage
- RuntimeError is a pure enum with no state machines
- No async/hconcurrency in error handling paths
- **Result: N/A (no state machine invariants to verify)**

### Flaky Behavior Check
- No timeouts, retries, or race conditions in tests
- No external service dependencies
- **Result: NO FLAKY BEHAVIOR**

### Nondeterminism Check
- No randomness in test inputs or execution
- **Result: NO NONDETERMINISM**

### Gap Analysis (Non-Blocking)
- INV-001: Error::source returns None for ArtifactNotFound, ArtifactInvalid, CapabilityDenied because these variants carry semantic causes (digest, action/required/granted) rather than underlying storage errors. This is correct behavior per error taxonomy design.

## Decision

**STATUS: APPROVED**

Post-refactor code passes all adversarial checks. The refactor extracted RuntimeError into `error/` modules while preserving:
- All 5 admission durability variants with stable diagnostic codes
- API envelope stability via `runtime_code()` / `diagnostic_code()`
- Source preservation for storage-propagated errors
- Distinctness from generic storage errors (0x2008 vs 0x2015)
- PartialEq field comparison integrity
