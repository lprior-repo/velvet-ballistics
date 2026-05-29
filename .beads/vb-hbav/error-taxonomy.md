# Error Taxonomy — Fuzz Hardening (vb-hbav)

## Taxonomy Overview

All fuzz failures are classified into 5 axes: **Phase**, **Severity**, **Category**, **Recoverability**, and **ProductionImpact**. Each error variant maps to a unique error code.

## Axis 1: Phase

| Phase | Code | Description |
|-------|------|-------------|
| `BUILD` | `E-FUZZ-BLD` | Failures during `cargo fuzz build` |
| `INSTRUMENT` | `E-FUZZ-INS` | Failures verifying libfuzzer instrumentation |
| `SMOKE` | `E-FUZZ-SMK` | Failures during 10-second unsanitized smoke |
| `ASAN-SMOKE` | `E-FUZZ-ASM` | Failures during 10-second ASAN smoke |
| `CAMPAIGN` | `E-FUZZ-CMP` | Failures during 1-hour deep campaign |
| `HARDEN` | `E-FUZZ-HDN` | Failures during assertion hardening |
| `CORPUS` | `E-FUZZ-CRP` | Failures during seed corpus creation |
| `REFACTOR` | `E-FUZZ-REF` | Failures during stdin boilerplate refactor |

## Axis 2: Severity

| Severity | Description | Gates Blocked |
|----------|-------------|---------------|
| `LETHAL` | Program crashes, memory corruption, undefined behavior | ALL gates |
| `CRITICAL` | ASAN finding (use-after-free, buffer overflow, leak) | Phase 1 exit gate |
| `MAJOR` | Missing function, orphan target, uncompileable target | Phase 0 exit gate |
| `MINOR` | Coverage-only assertion after hardening deadline | Phase 1 exit gate (soft) |
| `COSMETIC` | Duplicate boilerplate, missing seed corpus | Phase 1 exit gate (soft) |

## Axis 3: Category

| Category | Code | Example |
|----------|------|---------|
| `BUILD_FAILURE` | `CAT-BLD` | Missing crate dependency, edition mismatch, missing rust-src |
| `INSTRUMENTATION_MISSING` | `CAT-INS` | No LLVMFuzzer symbols in libfuzzer binary |
| `PANIC_IN_HARNESS` | `CAT-PAN` | `.unwrap()` or index-out-of-bounds in fuzz harness body |
| `ASAN_CRASH` | `CAT-ASN` | ASAN-detected heap-buffer-overflow, use-after-free |
| `ASAN_LEAK` | `CAT-LEK` | Memory leak detected (LSan) |
| `WEAK_ASSERTIONS` | `CAT-WEAK` | Harness at CoverageOnly strength; no behavioral assertions |
| `MISSING_FUNCTION` | `CAT-MIS` | `fuzz_lib::fuzz_*` function referenced but not defined |
| `ORPHAN_TARGET` | `CAT-ORP` | .rs file exists in fuzz_targets/ but no [[bin]] entry |
| `NAME_COLLISION` | `CAT-NAM` | Two [[bin]] entries with same `name` field |
| `DUPLICATE_BOILERPLATE` | `CAT-DUP` | Multiple copies of `run_with_stdin`/`write_stderr` |
| `EMPTY_CORPUS` | `CAT-COR` | Seed corpus directory exists but contains zero files |
| `NO_CORPUS_GROWTH` | `CAT-NCG` | Deep campaign found zero new edges / corpus did not grow |
| `CORRUPT_CORPUS` | `CAT-CCR` | Seed file causes harness panic (should be benign) |
| `ERROR_NOT_EXHAUSTIVE` | `CAT-ENX` | Error match misses a production error variant |

## Axis 4: Recoverability

| Level | Description |
|-------|-------------|
| `FATAL` | Target cannot run; requires code change |
| `RETRYABLE` | Transient failure (filesystem, memory pressure); retry may succeed |
| `CONFIG_ONLY` | Fixable via Cargo.toml or feature flag change |
| `DATA_ONLY` | Fixable by regenerating corpus or fixing seed content |

## Axis 5: ProductionImpact

| Impact | Description |
|--------|-------------|
| `NONE` | Build/infra error; no production code impact |
| `POTENTIAL_BUG` | Fuzz harness found an input that causes unexpected behavior |
| `CONFIRMED_BUG` | Fuzz campaign found a reproducible crash in production code |
| `UB_DETECTED` | ASAN/UBSAN detected undefined behavior (must fix immediately) |

## Complete Error Variant Catalog

### E-FUZZ-BLD (Build Phase)

| Code | Variant | Severity | Recoverability | ProductionImpact |
|------|---------|----------|----------------|-----------------|
| `E-FUZZ-BLD-001` | `CargoFuzzNotInstalled` | LETHAL | CONFIG_ONLY | NONE |
| `E-FUZZ-BLD-002` | `RustSrcNotInstalled` | LETHAL | CONFIG_ONLY | NONE |
| `E-FUZZ-BLD-003` | `NightlyToolchainMissing` | LETHAL | CONFIG_ONLY | NONE |
| `E-FUZZ-BLD-004` | `CrateCompilationFailed { crate: CrateName, errors: Vec<String> }` | MAJOR | FATAL | NONE |
| `E-FUZZ-BLD-005` | `MissingDependency { dep: String }` | MAJOR | CONFIG_ONLY | NONE |
| `E-FUZZ-BLD-006` | `EditionMismatch { expected: String, found: String }` | MAJOR | CONFIG_ONLY | NONE |
| `E-FUZZ-BLD-007` | `TargetBinEntryMissing { name: FuzzTargetName }` | MAJOR | CONFIG_ONLY | NONE |
| `E-FUZZ-BLD-008` | `NameCollision { name: FuzzTargetName, path1: RelativeHarnessPath, path2: RelativeHarnessPath }` | MAJOR | CONFIG_ONLY | NONE |
| `E-FUZZ-BLD-009` | `FeatureNotEnabled { feature: String }` | MAJOR | CONFIG_ONLY | NONE |
| `E-FUZZ-BLD-010` | `ProfileMissing { profile: String }` | MINOR | CONFIG_ONLY | NONE |

### E-FUZZ-INS (Instrumentation Phase)

| Code | Variant | Severity | Recoverability | ProductionImpact |
|------|---------|----------|----------------|-----------------|
| `E-FUZZ-INS-001` | `NoLibfuzzerSymbols { target: FuzzTargetName }` | CRITICAL | FATAL | NONE |
| `E-FUZZ-INS-002` | `BinaryNotFound { target: FuzzTargetName, expected_path: String }` | MAJOR | FATAL | NONE |
| `E-FUZZ-INS-003` | `HelpFlagFailed { target: FuzzTargetName }` | MAJOR | FATAL | NONE |
| `E-FUZZ-INS-004` | `StdinTargetMissingFeatureGate { target: FuzzTargetName }` | MINOR | CONFIG_ONLY | NONE |

### E-FUZZ-SMK (Smoke Phase)

| Code | Variant | Severity | Recoverability | ProductionImpact |
|------|---------|----------|----------------|-----------------|
| `E-FUZZ-SMK-001` | `SmokeCrash { target: FuzzTargetName, artifact_path: String }` | LETHAL | FATAL | CONFIRMED_BUG |
| `E-FUZZ-SMK-002` | `SmokeTimeout { target: FuzzTargetName }` | MAJOR | FATAL | POTENTIAL_BUG |
| `E-FUZZ-SMK-003` | `SmokeOom { target: FuzzTargetName }` | CRITICAL | RETRYABLE | POTENTIAL_BUG |
| `E-FUZZ-SMK-004` | `SmokeZeroExecs { target: FuzzTargetName }` | MAJOR | FATAL | NONE |
| `E-FUZZ-SMK-005` | `SmokeStdioError { target: FuzzTargetName, error: io::Error }` | MINOR | RETRYABLE | NONE |

### E-FUZZ-ASM (ASAN Smoke Phase)

| Code | Variant | Severity | Recoverability | ProductionImpact |
|------|---------|----------|----------------|-----------------|
| `E-FUZZ-ASM-001` | `AsanCrash { target: FuzzTargetName, sanitizer: Sanitizer, artifact_path: String }` | CRITICAL | FATAL | CONFIRMED_BUG |
| `E-FUZZ-ASM-002` | `AsanLeak { target: FuzzTargetName, leak_bytes: usize }` | CRITICAL | FATAL | CONFIRMED_BUG |
| `E-FUZZ-ASM-003` | `UbsanViolation { target: FuzzTargetName, violation: String }` | CRITICAL | FATAL | UB_DETECTED |
| `E-FUZZ-ASM-004` | `AsanBuildFailed { target: FuzzTargetName, error: String }` | MAJOR | CONFIG_ONLY | NONE |

### E-FUZZ-CMP (Campaign Phase)

| Code | Variant | Severity | Recoverability | ProductionImpact |
|------|---------|----------|----------------|-----------------|
| `E-FUZZ-CMP-001` | `CampaignCrash { target: FuzzTargetName, duration: CampaignDurationSecs, crash_hash: CrashHash }` | CRITICAL | FATAL | CONFIRMED_BUG |
| `E-FUZZ-CMP-002` | `CampaignLeak { target: FuzzTargetName, leak_bytes: usize, leak_count: usize }` | CRITICAL | FATAL | CONFIRMED_BUG |
| `E-FUZZ-CMP-003` | `CampaignNoCorpusGrowth { target: FuzzTargetName, execs: Executions }` | MAJOR | FATAL | NONE |
| `E-FUZZ-CMP-004` | `CampaignTimeout { target: FuzzTargetName, time_elapsed: u64 }` | MINOR | RETRYABLE | NONE |
| `E-FUZZ-CMP-005` | `CampaignStalled { target: FuzzTargetName, execs: Executions, no_new_edges_since: u64 }` | MINOR | RETRYABLE | NONE |

### E-FUZZ-HDN (Hardening Phase)

| Code | Variant | Severity | Recoverability | ProductionImpact |
|------|---------|----------|----------------|-----------------|
| `E-FUZZ-HDN-001` | `HarnessStillCoverageOnly { target: FuzzTargetName }` | MINOR | FATAL | NONE |
| `E-FUZZ-HDN-002` | `ErrorMatchNotExhaustive { target: FuzzTargetName, missing_variants: Vec<String> }` | MAJOR | FATAL | NONE |
| `E-FUZZ-HDN-003` | `AssertionTooWeak { target: FuzzTargetName, assertion_kind: String }` | MINOR | FATAL | NONE |
| `E-FUZZ-HDN-004` | `PanicInHarnessBody { target: FuzzTargetName, location: String }` | LETHAL | FATAL | NONE |
| `E-FUZZ-HDN-005` | `UncheckedArithmetic { target: FuzzTargetName, expression: String }` | CRITICAL | FATAL | NONE |
| `E-FUZZ-HDN-006` | `NotSafe (violates forbid(unsafe_code))` | LETHAL | FATAL | NONE |

### E-FUZZ-CRP (Corpus Phase)

| Code | Variant | Severity | Recoverability | ProductionImpact |
|------|---------|----------|----------------|-----------------|
| `E-FUZZ-CRP-001` | `CorpusIsEmpty { target: FuzzTargetName }` | MINOR | DATA_ONLY | NONE |
| `E-FUZZ-CRP-002` | `CorpusInsufficient { target: FuzzTargetName, current: SeedCount, minimum: SeedCount }` | MINOR | DATA_ONLY | NONE |
| `E-FUZZ-CRP-003` | `CorpusMissingCategory { target: FuzzTargetName, missing_categories: Vec<SeedCategory> }` | MINOR | DATA_ONLY | NONE |
| `E-FUZZ-CRP-004` | `SeedFileUnreadable { path: String, error: io::Error }` | MINOR | DATA_ONLY | NONE |
| `E-FUZZ-CRP-005` | `SeedFileCausesPanic { target: FuzzTargetName, seed_path: String }` | CRITICAL | DATA_ONLY | POTENTIAL_BUG |

### E-FUZZ-REF (Refactor Phase)

| Code | Variant | Severity | Recoverability | ProductionImpact |
|------|---------|----------|----------------|-----------------|
| `E-FUZZ-REF-001` | `DuplicateBoilerplateFound { paths: Vec<RelativeHarnessPath> }` | COSMETIC | FATAL | NONE |
| `E-FUZZ-REF-002` | `SharedModuleMissing { expected_path: String }` | MAJOR | FATAL | NONE |
| `E-FUZZ-REF-003` | `RefactorCompilationFailed { target: FuzzTargetName, error: String }` | MAJOR | FATAL | NONE |

## Error Handling Pattern (Railway)

Every fuzz operation follows the railway pattern:

```rust
// In fuzz harness body:
let result = production_crate::decode(data);
match result {
    Ok(value) => {
        // Structural assertions on success
        assert!(/* domain invariant */);
    }
    Err(e) => {
        // Typed error exhaustiveness
        match e {
            CrateError::Variant1 { .. } => {}
            CrateError::Variant2 { .. } => {}
            // ... all known variants ...
            _ => {} // forward-compat only; must not match current variants
        }
    }
}
// NEVER: let _ = result.ok();   ← suppresses all errors
// NEVER: let _ = result;        ← no assertion at all
```

## Crash Triage Procedure

When a fuzz campaign produces a crash artifact:

1. **Minimize**: `cargo fuzz tmin TARGET fuzz/artifacts/TARGET/crash-*`
2. **Deduplicate**: Compare crash hash against known crashes
3. **Classify**: Is this a production-code bug or a harness bug?
   - Production bug → `E-FUZZ-CMP-001`, file bead for crate maintainer
   - Harness bug → `E-FUZZ-HDN-004`, fix assertion or bounds check
4. **Fix**: Either fix production code or fix harness
5. **Regression test**: Add minimized crash as seed corpus entry
6. **Re-fuzz**: Resume campaign from checkpoint
