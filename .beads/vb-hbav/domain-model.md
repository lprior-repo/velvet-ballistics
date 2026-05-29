# Domain Model — Fuzz Hardening (vb-hbav)

## Ubiquitous Language

| Term | Definition |
|------|-----------|
| **Fuzz Target** | A named entry point (libfuzzer `fuzz_target!` macro or stdin `main`) that accepts `&[u8]` and exercises one production-code boundary. |
| **Fuzz Harness** | The body of a fuzz target: the function that receives bytes, calls production APIs, and asserts invariants. |
| **Harness Category** | Classification of a harness by what it verifies: Parser, Roundtrip, Property, Hostile, Differential, StructureAware, CoverageOnly. |
| **Assertion Strength** | A measure of what a harness proves: **CoverageOnly** (panic-freedom only, `let _ =`), **TypedError** (matches error variants exhaustively), **Structural** (asserts field-level shape invariants on success), **Equivalence** (differential: two paths produce identical results), **Roundtrip** (decode→encode→decode preserves bytes). |
| **Weak Harness** | A harness at CoverageOnly strength. Categorized as weak because it finds no semantic bugs, only panics/OOM/UB. |
| **Hardened Harness** | A harness upgraded from CoverageOnly to at least TypedError + Structural strength. |
| **Seed Corpus** | A directory of known-valid input files that jump-start the fuzzer's mutation engine. |
| **Fuzz Campaign** | A timed execution of one or more fuzz targets with specific sanitizer flags (ASAN, UBSAN, LSan). |
| **Stdin Boilerplate** | The duplicated `run_with_stdin`/`write_stderr`/`main` pattern (~25 lines) repeated in 38+ `src/bin/*.rs` files. |
| **Instrumentation Kind** | **Libfuzzer**: `#![no_main]` + `fuzz_target!()` macro, coverage-guided, ASAN-compatible. **Stdin**: `feature="fuzz"` gated `main()` reading from pipe, no coverage feedback, no ASAN integration. |
| **C.21-C.24** | Bead IDs for previously-fixed targets: `generated_compare`, `compiled_ir`, `ipc_frame`, `expression` — claimed FIXED, need ASAN verification. |
| **C.25** | Bead ID for `collect_page` — function was entirely missing in `fuzz/src/lib.rs` at time of discovery (LETHAL). |
| **Orphan Harness** | A `.rs` fuzz target file with no `[[bin]]` entry in `fuzz/Cargo.toml` — invisible to `cargo fuzz`. |
| **Red Queen Campaign** | fuzzing at scale: libfuzzer with ASAN+UBSAN, 1-hour per target, seed corpora, mutation-resistant assertions. |

## Entities and Aggregates

### FuzzTarget (Aggregate Root)
```
FuzzTarget {
    name: FuzzTargetName,          // unique, kebab-case, matches Cargo.toml [[bin]] name
    path: HarnessPath,             // fuzz_targets/*.rs or src/bin/*.rs
    instrumentation: InstrumentationKind,
    harness: FuzzHarness,
    assertion_strength: AssertionStrength,
    category: HarnessCategory,
    production_crate: CrateName,   // the crate whose API is exercised
    corpus: Option<SeedCorpus>,
}
```

### FuzzHarness (Value Object)
```
FuzzHarness {
    body_function: HarnessBodyRef,  // reference to fuzz_lib::fuzz_* function or inline closure
    input_type: &[u8],              // always raw bytes
    assertions: Vec<Assertion>,
    error_exhaustiveness: bool,     // true if every known error variant is matched
}
```

### Assertion (Value Object)
```
Assertion {
    kind: AssertionKind,           // PanicFreedom, TypedError, Structural, Determinism, Equivalence, Roundtrip
    invariant: InvariantExpr,      // human-readable invariant
    source_line: usize,            // line number in lib.rs where assertion lives
}
```

### SeedCorpus (Entity)
```
SeedCorpus {
    target_name: FuzzTargetName,
    path: CorpusPath,               // fuzz/corpus/<target_name>/
    seed_count: usize,
    min_seed_size: usize,           // minimum bytes in any seed
    categories: Vec<SeedCategory>,  // Empty, SingleByte, MagicBytes, ValidHappyPath, EdgeCase, OneBitFlipped
}
```

### FuzzCampaign (Entity)
```
FuzzCampaign {
    target_names: Vec<FuzzTargetName>,
    engine: FuzzEngine,             // libfuzzer (primary)
    sanitizers: Vec<Sanitizer>,    // ASAN, UBSAN, LSan
    duration: Duration,             // 10s smoke, 1hr deep
    rss_limit_mb: usize,
    exit_gate: FuzzCampaignGate,   // ZeroCrashes, ZeroLeaks, CorpusGrowth > 0
}
```

## Commands

| Command | Description | Produces |
|---------|-------------|----------|
| `DeclareTarget(name, path)` | Add `[[bin]]` entry to Cargo.toml | TargetDeclared event |
| `BuildTargets` | `cargo fuzz build` all declared targets | TargetsBuilt event |
| `SmokeTest(target, duration)` | Run target for duration with no sanitizers | SmokeResult event |
| `AsanSmokeTest(target, duration)` | Run target for duration with ASAN | AsanSmokeResult event |
| `HardenHarness(target, assertions)` | Add behavioral assertions to a weak harness | HarnessHardened event |
| `CreateSeedCorpus(target, seeds)` | Create seed files for a target | CorpusSeeded event |
| `RefactorStdinBoilerplate` | Extract `run_with_stdin` to shared module | BoilerplateRefactored event |
| `RunDeepCampaign(targets, duration)` | Run 1-hour ASAN campaign | CampaignResult event |
| `FixCrash(target, crash_artifact)` | Minimize, fix, and re-fuzz a crash | CrashFixed event |

## Events

| Event | Payload |
|-------|---------|
| `TargetDeclared` | `{ name, path, instrumentation }` |
| `TargetsBuilt` | `{ total: usize, failed: Vec<FuzzTargetName> }` |
| `SmokeResult` | `{ target, duration, execs: u64, crashes: usize, corpus_growth: bool }` |
| `AsanSmokeResult` | `{ target, duration, execs: u64, crashes: usize, leaks: usize }` |
| `HarnessHardened` | `{ target, old_strength: AssertionStrength, new_strength: AssertionStrength, assertion_count: usize }` |
| `CorpusSeeded` | `{ target, seed_count: usize, categories: Vec<SeedCategory> }` |
| `BoilerplateRefactored` | `{ files_removed: usize, lines_saved: usize }` |
| `CampaignResult` | `{ target, duration, sanitizer, execs: u64, crashes: usize, corpus_size: usize }` |
| `CrashFixed` | `{ target, crash_hash: Sha256, bead_id: BeadId }` |

## Policies / Invariants

- **PI-01**: Every fuzz target MUST have at least TypedError assertion strength. CoverageOnly is forbidden after Phase 1 hardening.
- **PI-02**: Every fuzz target MUST match all known error variants explicitly; wildcard `_ => {}` is allowed only for forward-compat of unknown future variants.
- **PI-03**: Every `fuzz_targets/` .rs file MUST have a corresponding `[[bin]]` entry in `fuzz/Cargo.toml`.
- **PI-04**: Every target MUST have at least 1 seed corpus file; structure-aware targets MUST have at least 5 seeds.
- **PI-05**: Stdin boilerplate MUST NOT be duplicated; `run_with_stdin` MUST live in exactly one shared module.
- **PI-06**: C.21-C.24 targets MUST pass 1-hour ASAN campaign with zero crashes before bead closure.
- **PI-07**: C.25 `fuzz_collect_page_pagination` MUST be implemented in `fuzz/src/lib.rs` with behavioral assertions.
- **PI-08**: Zero crashes, zero leaks in any ASAN campaign. Any crash is a BLOCKER.
- **PI-09**: No `unwrap`, `expect`, `panic`, `todo`, `unimplemented` in any fuzz harness body.
- **PI-10**: Panic-freedom is baseline, not a deliverable; every target must prove at least one domain invariant.

## Forbidden States

- Target declared in fuzz_targets/ but missing from Cargo.toml (orphan)
- Harness at CoverageOnly strength after Phase 1 exit gate
- Seed corpus directory missing when corresponding target exists
- C.25 collect_page entry in Cargo.toml without corresponding function in lib.rs
- Multiple copies of `run_with_stdin`/`write_stderr` across src/bin/ files after refactor
- ASAN leak or crash in any target that was previously claimed FIXED
