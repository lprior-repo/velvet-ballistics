# Test Plan: Atomize xtask Section 77 Command-Center Gates

**Bead**: vb-2i0t
**Phase**: State 1.5 (Test Plan)
**System under test**: `xtask/src/` — atomized xtask command wrappers for Section 77 quality gates
**Key modules**: `xtask/src/main.rs`, `xtask/src/evidence.rs`, `xtask/src/gates.rs`

---

## 1. Behavior Inventory

Every guaranteed behavior stated as: **[Subject] [action] [outcome] when [condition]**

### Evidence Bundle Structure (POST-004, POST-005)

1. **GateEvidence serializes** all required fields (kind, gate_name, command, exit_code, log, status, why_failed) to YAML when input is valid
2. **GateEvidence round-trips** through serde_yaml deserialization with all fields preserved
3. **WhyFailed populates** hint and repair_command fields when explain_failure is called on failed evidence
4. **YamlSerializationFailed** error is returned when serde_yaml cannot serialize evidence

### Gate Profiles (POST-001, POST-002, POST-003, POST-007)

5. **ai-fast profile runs** 6 gates (fmt, check, clippy, nextest, forbidden-scan, hotpath-scan) in sequence and aggregates evidence
6. **ai-deep profile runs** 4 gates (miri, mutants, llvm-cov, fuzz-build) in sequence and aggregates evidence
7. **ai-release profile runs** 11 gates (check, test, supply-chain, miri, fuzz-smoke, coverage, mutants-smoke, bench-build, feature-powerset, source-length, maxperf) and aggregates evidence
8. **Profile emits YAML** evidence bundle to stdout when --bead is absent
9. **Profile emits YAML** evidence bundle to `.evidence/<bead-id>/<profile>.yaml` when --bead flag is provided

### Exit Code Semantics (POST-008)

10. **Exit code 0** when all gates in profile pass and all evidence is present
11. **Exit code 1** when any gate in profile fails
12. **Exit code 1** when evidence is missing for a required gate (INV-001 fail-closed)

### Individual Gate Wrappers (POST-001/002/003)

13. **fmt gate** executes `cargo +nightly fmt --all` and emits evidence with exit_code
14. **check gate** executes `moon run :check` and emits evidence with exit_code
15. **clippy gate** executes `cargo +nightly clippy --workspace` and emits evidence with exit_code
16. **nextest gate** executes `cargo nextest run --workspace` and emits evidence with exit_code
17. **forbidden-scan gate** executes forbidden-pattern scan and emits evidence
18. **hotpath-scan gate** executes hotpath scan and emits evidence
19. **miri gate** executes `cargo +nightly miri test --workspace` and emits evidence
20. **mutants gate** executes `cargo mutants --package velvet_ballastics` and emits evidence
21. **llvm-cov gate** executes `cargo llvm-cov` and emits evidence
22. **fuzz-build gate** executes `cargo fuzz build` and emits evidence
23. **supply-chain gate** delegates to moon `:supply-chain` and emits evidence
24. **fuzz-smoke gate** delegates to moon `:fuzz-smoke` and emits evidence
25. **coverage gate** delegates to moon `:coverage` and emits evidence
26. **mutants-smoke gate** delegates to moon `:mutants-smoke` and emits evidence
27. **bench-build gate** delegates to moon `:bench-build` and emits evidence
28. **feature-powerset gate** delegates to moon `:feature-powerset` and emits evidence
29. **source-length gate** executes `bash scripts/check-source-length.sh` and emits evidence
30. **maxperf gate** executes maxperf build and emits evidence
31. **why-failed subcommand** reads evidence YAML and emits diagnostic with hint + repair_command

### Error Taxonomy (all Error variants)

32. **GateTimeout error** is constructed when gate duration exceeds configured timeout
33. **GateFailed error** propagates exit_code and log path from failed subprocess
34. **MissingEvidence error** is returned when validate_evidence_dir finds absent evidence file
35. **EvidenceWriteFailed error** is returned when YAML file cannot be written to disk
36. **SubcommandNotFound error** is returned for unknown xtask subcommand names
37. **BeadDirectoryCreationFailed error** is returned when `.evidence/<bead>/` cannot be created
38. **YamlSerializationFailed error** is returned when serde_yaml fails on evidence serialization
39. **UpstreamMoonFailed error** is returned when moon task returns non-zero
40. **UpstreamJustFailed error** is returned when just recipe returns non-zero

### Invariants (INV-001 through INV-006)

41. **Fail-closed on missing evidence** — validate_evidence_dir returns MissingEvidence for absent files; never silent-pass
42. **Bounded ai-fast** — no unbounded loops; per-gate timeout enforced
43. **Deterministic evidence** — identical inputs produce bit-identical YAML across runs
44. **No panic in wrappers** — no panic!/unwrap!/expect! in xtask evidence or gate modules; all fallible ops return Result
45. **Structured output only** — stdout is valid YAML; raw tool output redirected to log files
46. **Agent-executable profiles** — ai-fast/ai-deep/ai-release require no interactive input; all flags have defaults

### Preconditions (PRE-001 through PRE-004)

47. **Gate inventory documented** — all 28 Section 77 gates mapped to xtask variants or explicit blockers
48. **xtask builds** — `cargo build -p xtask` succeeds without error
49. **Toolchain available** — fmt, clippy, nextest, miri, cargo, moon, just all present in PATH

---

## 2. Testing Trophy Allocation

| Layer | Target % | Scope | Tool |
|-------|----------|-------|------|
| **Static analysis** | 5% | `panic!`/`unwrap!`/`expect!` scan, gate-coverage ripgrep, `forbid(unsafe_code)` | `ripgrep`, `clippy`, `cargo-deny` |
| **Unit tests** | 30% | Per-module pure functions; error variant construction; evidence serialization; gate list exhaustive enumeration | `cargo nextest` with `#[cfg(test)]` modules |
| **Integration tests** | 60% | Full `cargo xtask <subcommand>` invocations; YAML bundle verification; exit code propagation; bead directory creation | `cargo nextest` + bash scripts + ` XBEAK_ID=` environment |
| **E2E / Acceptance** | 5% | Full `ai-fast`, `ai-deep`, `ai-release` profiles on clean dirty trees | `moon ci` + manual QA |
| **Formal verification** | — | Timeout arithmetic (Kani); Result-path completeness (Kani); evidence round-trip (proptest) | `cargo kani`, `proptest` |

**Rationale**: The verified kernel is CLI orchestration + YAML serialization — behaviors are best exercised by actually invoking the xtask binary. Pure functions (evidence serialization, gate list checking) get unit + proptest coverage. Critical safety properties (no panic, timeout bounds, fail-closed) get Kani + cargo-mutants.

---

## 3. BDD Scenarios (per behavior)

### Evidence Structure

#### Behavior: GateEvidence serializes with all required fields when input is valid
```
Given: a valid GateEvidence struct with all fields populated (kind="fmt", gate_name="fmt",
        command="cargo +nightly fmt --all", exit_code=0, log="target/evidence/fmt.log",
        status=Pass, why_failed=None)
When:  the evidence is serialized to YAML via serde_yaml
Then:  the output contains "kind: fmt", "gate_name: fmt", "command: cargo +nightly fmt --all",
        "exit_code: 0", "log: target/evidence/fmt.log", "status: Pass"
        and no field is omitted
```

#### Behavior: GateEvidence round-trips through serde_yaml deserialization
```
Given: a GateEvidence with kind="clippy", gate_name="clippy", command="cargo clippy ...",
        exit_code=1, log="target/evidence/clippy.log", status=Fail,
        why_failed=Some(WhyFailed { gate_name: "clippy", hint: "...", repair_command: "..." })
When:  the evidence is serialized to YAML bytes and deserialized back
Then:  the deserialized struct equals the original (all fields match exactly)
```

#### Behavior: explain_failure populates hint and repair_command for failed gates
```
Given: a GateEvidence with status=Fail, gate_name="clippy", command="cargo clippy ..."
When:  explain_failure(evidence) is called
Then:  the returned WhyFailed has gate_name="clippy",
        hint contains "run 'cargo clippy' to see diagnostics",
        repair_command="cargo +nightly clippy --fix --allow-dirty"
```

#### Behavior: YamlSerializationFailed is returned when serde_yaml cannot serialize
```
Given: a GateEvidence containing a field value that serde_yaml cannot serialize
       (e.g., a type with no Serialize impl)
When:  serialization is attempted
Then:  Err(Error::YamlSerializationFailed { gate: "clippy", cause: "..." }) is returned
```

### Gate Profiles

#### Behavior: ai-fast profile runs all 6 gates and aggregates evidence when workspace is clean
```
Given: a clean workspace with all 6 ai-fast tools available (fmt, check, clippy,
        nextest, forbidden-scan, hotpath-scan)
When:  cargo xtask ai-fast --bead vb-test is executed
Then:  exit code is 0
        file .evidence/vb-test/ai-fast.yaml exists
        YAML document contains 6 gate entries
        each entry has fields: kind, gate_name, command, exit_code, log, status
```

#### Behavior: ai-fast exits with code 1 when any gate fails
```
Given: a workspace where the fmt gate fails (exit_code != 0)
When:  cargo xtask ai-fast --bead vb-test is executed
Then:  exit code is 1
        evidence bundle contains status=Fail for the fmt gate
        why_failed block is populated for the fmt gate
```

#### Behavior: ai-fast exits with code 1 when evidence is missing (INV-001)
```
Given: .evidence/vb-test/ contains fmt.yaml but is missing clippy.yaml
When:  cargo xtask ai-fast --bead vb-test is executed
Then:  exit code is 1
        output contains "MissingEvidence" error for "clippy" gate
        no silent pass occurs
```

#### Behavior: ai-deep profile runs all 4 deep gates in sequence
```
Given: a clean workspace with miri, mutants, llvm-cov, fuzz-build available
When:  cargo xtask ai-deep --bead vb-test is executed
Then:  exit code is 0
        .evidence/vb-test/ai-deep.yaml contains 4 gate entries
```

#### Behavior: ai-release profile aggregates evidence from all 11 release gates
```
Given: a clean workspace with moon and just available
When:  cargo xtask ai-release --bead vb-test is executed
Then:  exit code is 0
        .evidence/vb-test/ai-release.yaml contains 11 gate entries
        entries include: check, test, supply-chain, miri, fuzz-smoke, coverage,
        mutants-smoke, bench-build, feature-powerset, source-length, maxperf
```

#### Behavior: profile emits YAML to stdout when --bead is absent
```
Given: cargo xtask ai-fast is run without --bead flag
When:  the command completes
Then:  stdout is valid YAML (parseable by serde_yaml)
        no .evidence/ directory is created
```

#### Behavior: profile emits YAML to bead directory when --bead is provided
```
Given: .evidence/ does not exist prior to running
When:  cargo xtask ai-fast --bead vb-test is executed
Then:  .evidence/vb-test/ directory is created
        .evidence/vb-test/ai-fast.yaml exists and contains valid YAML
```

### Error Variants

#### Behavior: GateTimeout is constructed when gate exceeds its timeout duration
```
Given: a gate with configured timeout of 5 seconds
When:  the gate command blocks for > 5 seconds
Then:  Err(Error::GateTimeout { gate: "miri", duration_secs: 5 }) is returned
```

#### Behavior: GateFailed propagates subprocess exit_code and log path
```
Given: a gate subprocess returns exit code 101
When:  run_gate is called with that gate's command
Then:  Err(Error::GateFailed { gate: "clippy", exit_code: 101,
        log: PathBuf::from("target/evidence/clippy.log") }) is returned
```

#### Behavior: MissingEvidence is returned when evidence file does not exist
```
Given: validate_evidence_dir is called with a directory missing clippy.yaml
When:  the function checks for required gates
Then:  Err(Error::MissingEvidence { gate: "clippy",
        path: PathBuf::from(".evidence/vb-test/clippy.yaml") }) is returned
```

#### Behavior: EvidenceWriteFailed is returned when YAML file cannot be written
```
Given: the evidence directory is not writable (permission denied)
When:  run_gate attempts to write the evidence bundle
Then:  Err(Error::EvidenceWriteFailed { gate: "fmt", path: "...", cause: "..." }) is returned
```

#### Behavior: SubcommandNotFound is returned for unknown subcommand names
```
Given: cargo xtask unknown-gate-name is executed
When:  the CLI parses the subcommand
Then:  exit code 1
        output contains "SubcommandNotFound" and "unknown-gate-name"
```

#### Behavior: BeadDirectoryCreationFailed is returned when .evidence/<bead>/ cannot be created
```
Given: the parent of .evidence/ has no write permissions
When:  cargo xtask ai-fast --bead vb-test is executed
Then:  Err(Error::BeadDirectoryCreationFailed { bead: "vb-test", cause: "..." }) is returned
```

#### Behavior: UpstreamMoonFailed is returned when moon task returns non-zero
```
Given: moon run :nonexistent-task returns non-zero
When:  cargo xtask ai-release --profile moon-fail is executed
Then:  Err(Error::UpstreamMoonFailed { task: "nonexistent-task", cause: "..." }) is returned
```

#### Behavior: UpstreamJustFailed is returned when just recipe returns non-zero
```
Given: just nonexistent-recipe returns non-zero
When:  cargo xtask ai-release --profile just-fail is executed
Then:  Err(Error::UpstreamJustFailed { recipe: "nonexistent-recipe", cause: "..." }) is returned
```

### Invariants

#### Behavior: Missing evidence causes profile runner to fail (fail-closed)
```
Given: .evidence/vb-test/ exists but is missing evidence for "hotpath-scan"
When:  validate_evidence_dir is called with required_gates=["fmt","clippy","hotpath-scan"]
Then:  MissingEvidence error is returned for "hotpath-scan"
        no evidence file is silently created
        profile exits with failure
```

#### Behavior: ai-fast gates have bounded timeout (no unbounded loops)
```
Given: a gate with 10-second timeout configured
When:  the gate command executes
Then:  it is killed after exactly 10 seconds
        GateTimeout error is returned
        no loop iterates without bound
```

#### Behavior: Identical evidence serializes to bit-identical YAML across runs
```
Given: GateEvidence with kind="fmt", gate_name="fmt", exit_code=0, status=Pass
When:  the evidence is serialized twice with identical input
Then:  both YAML strings are byte-for-byte identical
        (deterministic: no timestamps, no random UUIDs in output)
```

#### Behavior: No panic in xtask wrappers under any condition
```
Given: the xtask binary is running any gate command
When:  any error condition occurs (failed subprocess, missing file, serialization error)
Then:  no panic!, unwrap!, or expect! is reached
        all errors are returned as Result<T, Error>
```

#### Behavior: stdout contains only YAML; raw tool output goes to log files
```
Given: cargo xtask ai-fast is executed
When:  the command completes
Then:  stdout output is valid YAML (parseable)
        raw tool output (fmt diffs, clippy warnings) appears only in log files
        no raw output is mixed into stdout YAML
```

#### Behavior: ai-fast/ai-deep/ai-release require no interactive input
```
Given: ai-fast, ai-deep, ai-release commands are invoked in a non-interactive shell
When:  the commands run with no stdin and no -- flags beyond --bead
Then:  all complete without requiring user input
        all flags have default values
```

### Preconditions

#### Behavior: All 28 Section 77 gates are mapped to xtask variants or explicit blockers
```
Given: Section 77.1 + 77.2 + 77.3 gate list
When:  xtask/src/main.rs Commands enum is inspected
Then:  each of the 28 gates has either a match arm in the Commands enum
        or an explicit "// BLOCKED: <reason>" comment
```

#### Behavior: xtask binary builds without error
```
Given: cargo build -p xtask is executed
When:  the build completes
Then:  exit code is 0
        xtask binary exists at target/debug/xtask or target/release/xtask
```

---

## 4. Proptest Invariants

### Evidence Serialization Round-Trip (POST-004, INV-003, ERR-007)

**Property**: `GateEvidence` arbitrary instance round-trips through `serde_yaml` with all fields preserved.

**Strategy**:
```rust
proptest! {
    fn proptest_evidence_round_trip(evidence: GateEvidence) {
        let yaml = serde_yaml::to_string(&evidence).unwrap();
        let parsed: GateEvidence = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(evidence.kind, parsed.kind);
        assert_eq!(evidence.gate_name, parsed.gate_name);
        assert_eq!(evidence.command, parsed.command);
        assert_eq!(evidence.exit_code, parsed.exit_code);
        assert_eq!(evidence.log, parsed.log);
        assert_eq!(evidence.status, parsed.status);
        assert_eq!(evidence.why_failed, parsed.why_failed);
    }
}
```

**Input generation**: `proptest::arbitrary::any::<GateEvidence>()` with custom
  `Arbitrary` impl covering:
  - `kind`: non-empty ASCII string, max 64 bytes
  - `gate_name`: non-empty ASCII string matching `[a-z][a-z0-9-]*`, max 32 bytes
  - `command`: valid shell command string, max 256 bytes
  - `exit_code`: i32 in range `-2147483648..=2147483647`
  - `log`: valid PathBuf string (UTF-8, no null bytes)
  - `status`: one of `Pass`, `Fail`, `Skipped { reason: String }`
  - `why_failed`: `Option<WhyFailed>` with bounded strings

**Always-failing input class**: GateEvidence with a field containing a Rust type
  that has no `Serialize` impl (covered by compile-time bound, not runtime failure).

### Deterministic Evidence Digests (INV-003)

**Property**: Two `GateEvidence` instances with identical field values produce
  byte-identical YAML strings.

**Strategy**:
```rust
proptest! {
    fn proptest_deterministic_evidence(evidence: GateEvidence) {
        let yaml_a = serde_yaml::to_string(&evidence).unwrap();
        let yaml_b = serde_yaml::to_string(&evidence).unwrap();
        assert_eq!(yaml_a.as_bytes(), yaml_b.as_bytes());
    }
}
```

**Constraint**: Evidence must not contain types that embed timestamps, UUIDs, or
  random data. All timestamps use `SystemTime::UNIX_EPOCH` serialization; no
  `Utc::now()` in the evidence schema.

### YAML Serialization Failure (ERR-007)

**Property**: `YamlSerializationFailed` is returned for evidence with
  non-serializable field values (using a marker type to simulate).

**Strategy**: Custom `DoSerialize` wrapper that returns error on `to_yaml()`.

### Evidence Path Determinism (INV-003)

**Property**: Evidence path construction is deterministic given identical
  bead ID and gate name.

```rust
proptest! {
    fn proptest_evidence_path_determinism(bead_id: String, gate_name: String) {
        let path1 = evidence_path(&bead_id, &gate_name);
        let path2 = evidence_path(&bead_id, &gate_name);
        assert_eq!(path1, path2);
    }
}
```

**Valid input**: `bead_id` matching `[a-z0-9-]+`, `gate_name` matching
  `[a-z0-9-]+`. **Always-failing input**: bead_id containing `..` or `/`
  — path traversal attempts must be rejected.

---

## 5. Fuzz Targets

### FUZZ-001: Evidence Bundle YAML Parsing

**Target**: `parse_evidence_yaml` (bolero harness)

**Risk**: Medium — untrusted YAML from disk used in evidence bundles.
  Panic would violate INV-004 (no panic in wrappers).

**Corpus seeds**:
- `fuzz/corpus/valid_evidence_pass.yaml` — minimal Pass evidence
- `fuzz/corpus/valid_evidence_fail.yaml` — Fail evidence with why_failed
- `fuzz/corpus/valid_evidence_skipped.yaml` — Skipped evidence
- `fuzz/corpus/valid_evidence_all_fields.yaml` — all optional fields populated

**Input type**: `&[u8]` (raw YAML bytes)

**Property under test**: Deserialization either succeeds with valid `GateEvidence`
  or returns `Error::YamlSerializationFailed`. Must **never panic**.

**Fuzzing strategy**:
1. Generate random bytes (max 4 KB)
2. Attempt `serde_yaml::from_slice::<GateEvidence>`
3. If success: assert all required fields are present and valid
4. If error: assert error is `YamlSerializationFailed` (typed), not panic

### FUZZ-002: Gate Name CLI Parsing

**Target**: `fuzz_gate_name` (bolero harness on clap argument parsing)

**Risk**: Low — gate names are internal identifiers; malformed input should
  return a clean error, not crash.

**Input type**: random `&str` passed as xtask subcommand arg

**Property**: Unknown gate name returns `Error::SubcommandNotFound` with clean
  exit code 1. No stack overflow from deeply nested arguments.

### FUZZ-003: Evidence Path Traversal

**Target**: `fuzz_evidence_path` (bolero harness)

**Risk**: Medium — path components from bead IDs could escape `.evidence/`
  directory (security: path traversal).

**Input type**: bead_id string containing path traversal attempts

**Property**: Path must be confined to `.evidence/<bead-id>/`. Any `..`,
  absolute path, or null-byte in bead ID must be rejected, returning
  `Error::BeadDirectoryCreationFailed`.

**Seeds**: `fuzz/corpus/safe_bead_ids.txt` — valid bead IDs like `vb-test`,
  `vb-abc123`

---

## 6. Kani Harnesses

### KANI-001: Gate Timeout Arithmetic (INV-002, ERR-001)

**Property to prove**: When a gate's timeout duration `D` is set and the
  elapsed time exceeds `D`, `GateTimeout { gate, duration_secs: D }` is
  returned — no arithmetic overflow in duration comparison.

**Harness**: `gate_timeout_harness`
```rust
#[kani::proof]
fn gate_timeout_harness() {
    // Formalize: elapsed > timeout => GateTimeout
    let timeout_secs: u64 = kani::any();
    let elapsed_secs: u64 = kani::any();
    // Bound: realistic timeout range (1..=3600 seconds)
    kani::assume(timeout_secs > 0 && timeout_secs <= 3600);
    kani::assume(elapsed_secs <= u64::MAX); // natural bound

    if elapsed_secs > timeout_secs {
        let result = check_timeout("miri", timeout_secs, elapsed_secs);
        assert!(matches!(result, Err(Error::GateTimeout { .. })));
    }
}
```

### KANI-002: Result Path Completeness (INV-004b)

**Property to prove**: Every fallible function in `evidence.rs` and
  `gates.rs` returns `Result<T, Error>` — no unwrap paths reachable.

**Harness**: `evidence_result_paths`
```rust
#[kani::proof]
fn evidence_result_paths() {
    // For each public function in evidence.rs:
    // validate_evidence_dir, run_gate, explain_failure,
    // evidence_path, write_evidence
    // Prove: all code paths return Result, none reach unwrap/panic
}
```

**Strategy**: Use Kani's `--restrict-vtable` and mark all `unwrap()` /
  `expect()` calls as unreachable via harness instrumentation. Model
  filesystem operations as nondeterministic `Result`.

### KANI-003: Evidence Path Bounds (INV-001)

**Property to prove**: `evidence_path(bead_id, gate_name)` never returns
  a path outside `.evidence/<bead-id>/`.

**Harness**:
```rust
#[kani::proof]
fn evidence_path_no_traversal() {
    let bead_id: String = kani::any();
    let gate_name: String = kani::any();
    let path = evidence_path(&bead_id, &gate_name);
    assert!(path.starts_with(".evidence/"));
    assert!(!path.contains(".."));
    assert!(path.components().count() <= 3); // .evidence / bead_id / gate.yaml
}
```

---

## 7. Mutation Testing Checkpoints

**Framework**: `cargo-mutants`
**Package**: `xtask`
**Target functions**: evidence serialization, gate orchestration, validate_evidence_dir

### Required Kill Points (≥90% mutation coverage target)

| Mutant | Location | Mutation | Kill Test |
|--------|----------|----------|-----------|
| M1: evidence_path returns wrong bead dir | `evidence.rs:evidence_path` | Swap bead_id component | `test_evidence_goes_to_bead_dir` |
| M2: validate_evidence_dir returns Ok on missing file | `evidence.rs:validate_evidence_dir` | Invert path existence check | `test_missing_evidence_is_failure` |
| M3: run_gate ignores exit_code | `gates.rs:run_gate` | Set exit_code = 0 always | `test_gate_failed_propagates` |
| M4: explain_failure returns None | `evidence.rs:explain_failure` | Return None instead of WhyFailed | `test_why_failed_fields_present` |
| M5: write_evidence swallows write error | `evidence.rs:write_evidence` | Return Ok(()) on write failure | `test_evidence_write_fails` |
| M6: gate runner skips timeout check | `gates.rs:run_gate` | Remove timeout comparison | `test_gate_timeout_enforced` |
| M7: Commands enum missing fmt arm | `main.rs:Commands` | Remove fmt match arm | `test_ai_fast_gates_all_wrapped` |
| M8: exit code always 0 | `main.rs` | Replace FAILURE with SUCCESS | `test_exit_code_1_on_fail` |
| M9: MissingEvidence uses wrong gate name | `evidence.rs:validate_evidence_dir` | Use wrong key from missing files map | `test_missing_evidence_is_failure` |
| M10: serde_yaml serializes with non-deterministic order | `evidence.rs` | Change BTreeMap iteration order | `proptest_deterministic_evidence` |

**Command**: `cargo mutants --package xtask --timeout 60 -- evidence::validate_evidence_dir`

---

## 8. Combinatorial Coverage Matrix

| Scenario | Input Class | Expected Output | Layer | Test |
|----------|-------------|-----------------|-------|------|
| evidence: pass | GateEvidence with status=Pass | Ok(bundle) with status=Pass | unit | `proptest_evidence_round_trip` |
| evidence: fail | GateEvidence with exit_code!=0 | Ok(bundle) with status=Fail + why_failed | unit | `test_why_failed_fields_present` |
| evidence: skipped | GateEvidence with Skipped | Ok(bundle) with status=Skipped { reason } | unit | `test_skipped_gate_has_skipped_status` |
| evidence: serialization error | Invalid UTF-8 or unserializable field | Err(YamlSerializationFailed) | proptest | `proptest_yaml_serialization` |
| evidence: write error | Permission-denied directory | Err(EvidenceWriteFailed) | unit | `test_evidence_write_fails` |
| profile: ai-fast all pass | Clean workspace, 6 gates | Exit 0, 6 entries all Pass | integration | `test_ai_fast_profile_aggregates_evidence` |
| profile: ai-fast one fail | fmt fails | Exit 1, fmt=Fail, others=Pass | integration | `test_exit_code_1_on_fail` |
| profile: ai-fast missing evidence | One evidence file absent | Exit 1, MissingEvidence error | integration | `test_missing_evidence_is_failure` |
| profile: ai-deep all pass | Clean workspace, 4 gates | Exit 0, 4 entries all Pass | integration | `test_ai_deep_profile_aggregates_evidence` |
| profile: ai-release all pass | Clean workspace, 11 gates | Exit 0, 11 entries all Pass | integration | `test_ai_release_profile_aggregates_evidence` |
| gate: fmt pass | `cargo +nightly fmt --all` exit 0 | Ok(evidence with exit_code=0) | unit | `test_fmt_gate_passes` |
| gate: fmt fail | `cargo +nightly fmt --all` exit 1 | Err(GateFailed) + evidence | unit | `test_fmt_gate_fails` |
| gate: clippy fail | clippy returns non-zero | Err(GateFailed) + exit_code | unit | `test_gate_failed_propagates` |
| gate: timeout | Gate command blocks > timeout | Err(GateTimeout) | unit + kani | `test_gate_timeout_error` |
| gate: unknown subcommand | `cargo xtask unknown` | Err(SubcommandNotFound), exit 1 | unit | `test_unknown_subcommand` |
| path: bead dir creation fail | Unwritable parent of .evidence/ | Err(BeadDirectoryCreationFailed) | unit | `test_bead_dir_creation_fails` |
| path: no --bead flag | Profile run without --bead | YAML to stdout, no .evidence/ dir | integration | `test_bead_flag_propagates` |
| path: with --bead flag | Profile run with --bead=vb-test | .evidence/vb-test/*.yaml created | integration | `test_evidence_goes_to_bead_dir` |
| upstream: moon fails | moon :nonexistent returns non-zero | Err(UpstreamMoonFailed) | integration | `test_upstream_moon_failed` |
| upstream: just fails | just nonexistent returns non-zero | Err(UpstreamJustFailed) | integration | `test_upstream_just_failed` |
| invariant: deterministic | Run ai-fast twice | Bit-identical YAML both times | proptest | `proptest_deterministic_evidence` |
| invariant: fail-closed | Missing evidence file | Exit 1 + MissingEvidence | unit + mutants | `test_missing_evidence_is_failure` |
| invariant: no panic | Any error condition | No panic reached | kani | `evidence_result_paths` |
| invariant: structured output | ai-fast stdout | Valid YAML only | integration | `test_no_raw_tool_output_on_stdout` |
| invariant: bounded timeout | Gate timeout=5s, blocks 10s | GateTimeout after 5s | kani | `gate_timeout_harness` |
| pre: xtask builds | `cargo build -p xtask` | Exit 0, binary exists | moon ci | `test_xtask_builds` |
| pre: toolchain available | PATH has cargo, rustup | Commands succeed | static | `test_toolchain_available` |
| pre: gate list exhaustive | 28 Section 77 gates | All mapped or blocked | unit | `test_gate_list_exhaustive` |
| err: GateTimeout | elapsed > timeout | GateTimeout { gate, duration_secs } | kani | `gate_timeout_harness` |
| err: GateFailed | subprocess exit 101 | GateFailed { exit_code: 101 } | unit | `test_gate_failed_propagates` |
| err: MissingEvidence | file absent | MissingEvidence { path } | unit + mutants | `test_missing_evidence_is_failure` |
| err: EvidenceWriteFailed | write permission denied | EvidenceWriteFailed { path } | unit | `test_evidence_write_fails` |
| err: SubcommandNotFound | unknown subcommand | SubcommandNotFound { name } | unit | `test_unknown_subcommand` |
| err: BeadDirectoryCreationFailed | cannot create .evidence/ | BeadDirectoryCreationFailed | unit | `test_bead_dir_creation_fails` |
| err: YamlSerializationFailed | serde_yaml fails | YamlSerializationFailed { gate } | proptest | `proptest_yaml_serialization` |
| err: UpstreamMoonFailed | moon returns non-zero | UpstreamMoonFailed { task } | integration | `test_upstream_moon_failed` |
| err: UpstreamJustFailed | just returns non-zero | UpstreamJustFailed { recipe } | integration | `test_upstream_just_failed` |
| fuzz: valid yaml parse | Valid YAML bytes | GateEvidence deserialized | bolero | `parse_evidence_yaml` |
| fuzz: invalid yaml parse | Random bytes | Err(YamlSerializationFailed), no panic | bolero | `parse_evidence_yaml` |
| fuzz: path traversal | bead_id="../../../etc" | Err/rejected, no path escape | bolero | `fuzz_evidence_path` |
| cov: >80% coverage | All xtask/src/ modules | Line + branch > 80% | cargo-llvm-cov | `COV-001` |

---

## 9. Proof Obligation Address Table

All 30 proof obligations from `proof-obligations.jsonl` are covered:

| ID | Clause | Layer | Test(s) | Status |
|----|--------|-------|---------|--------|
| PRE-001 | Gate inventory documented | static-scan | `test_gate_list_exhaustive` (ripgrep) | Covered |
| PRE-002 | 28 gates enumerated | unit-test | `test_gate_list_exhaustive` | Covered |
| PRE-003 | xtask builds | moon ci | `moon run :check` | Covered |
| PRE-004 | Toolchain available | static | `test_toolchain_available` | Covered |
| POST-001 | 6 ai-fast gates wrapped | static-scan + unit | `test_ai_fast_gates_all_wrapped` | Covered |
| POST-002 | 4 ai-deep gates wrapped | static-scan + unit | `test_ai_deep_gates_all_wrapped` | Covered |
| POST-003 | 11 ai-release gates wrapped | static-scan + unit | `test_ai_release_gates_all_wrapped` | Covered |
| POST-004 | Evidence fields + round-trip | proptest | `proptest_evidence_round_trip` | Covered |
| POST-005 | why-failed populated | unit-test | `test_why_failed_fields_present` | Covered |
| POST-006 | Bead-dir path scoping | unit + integration | `test_evidence_goes_to_bead_dir` | Covered |
| POST-007 | Aggregate profile YAML | integration | `test_ai_fast_profile_aggregates_evidence` | Covered |
| POST-008 | Exit code semantics | integration | `test_exit_code_0_on_pass`, `test_exit_code_1_on_fail` | Covered |
| POST-009 | --bead flag propagates | unit + integration | `test_bead_flag_propagates` | Covered |
| INV-001 | Fail-closed on missing evidence | cargo-mutants + unit | `test_missing_evidence_is_failure`, `test_silent_pass_prevented` | Covered |
| INV-001b | validate_evidence_dir MissingEvidence | unit-test | `test_missing_evidence_is_failure` | Covered |
| INV-002 | Timeout bounded | kani | `gate_timeout_harness` | Covered |
| INV-003 | Deterministic evidence | proptest | `proptest_deterministic_evidence` | Covered |
| INV-004 | No panic in wrappers | static-scan + kani | `ripgrep`, `evidence_result_paths` | Covered |
| INV-004b | All fallible functions return Result | kani | `evidence_result_paths` | Covered |
| INV-005 | Structured output only | integration | `test_no_raw_tool_output_on_stdout` | Covered |
| INV-006 | Agent-executable profiles | manual-qa | `test_no_interactive_prompt_in_profiles` | Covered |
| ERR-001 | GateTimeout error | kani + unit | `gate_timeout_harness`, `test_gate_timeout_error` | Covered |
| ERR-002 | GateFailed propagates | unit + integration | `test_gate_failed_propagates` | Covered |
| ERR-003 | MissingEvidence on absent file | cargo-mutants + unit | `test_missing_evidence_is_failure` | Covered |
| ERR-004 | EvidenceWriteFailed | unit | `test_evidence_write_fails` | Covered |
| ERR-005 | SubcommandNotFound | unit | `test_unknown_subcommand` | Covered |
| ERR-006 | BeadDirectoryCreationFailed | unit | `test_bead_dir_creation_fails` | Covered |
| ERR-007 | YamlSerializationFailed | proptest | `proptest_yaml_serialization` | Covered |
| ERR-008 | UpstreamMoonFailed | integration | `test_upstream_moon_failed` | Covered |
| ERR-009 | UpstreamJustFailed | integration | `test_upstream_just_failed` | Covered |
| FUZZ-001 | YAML parsing no panic | bolero | `parse_evidence_yaml` | Covered |
| COV-001 | >80% coverage | cargo-llvm-cov | `llvm-cov-report.txt` | Covered |

---

## 10. Test Execution Order

```
Phase 1 (unit + integration compile-fail first):
  cargo nextest run -p xtask --test-type unit 2>&1  # all should compile-fail before impl

Phase 2 (implementation):
  Implement xtask/src/evidence.rs, xtask/src/gates.rs
  Add new Commands variants to xtask/src/main.rs

Phase 3 (unit tests green):
  cargo nextest run -p xtask
  All 30+ unit/integration tests must pass

Phase 4 (static + formal):
  ripgrep -n 'panic!|unwrap!|expect!' xtask/src/  # zero matches
  cargo kani --harness gate_timeout_harness        # proven
  cargo kani --harness evidence_result_paths       # proven
  cargo mutants --package xtask --timeout 60 -- evidence::validate_evidence_dir

Phase 5 (fuzz):
  cargo fuzz run parse_evidence_yaml               # 10 min minimum
  cargo fuzz run fuzz_evidence_path                # 10 min minimum

Phase 6 (coverage):
  cargo llvm-cov nextest -p xtask --html --output-dir target/llvm-cov/xtask
  # Open target/llvm-cov/xtask/index.html and verify >80% line/branch

Phase 7 (full profile integration):
  cargo xtask ai-fast --bead vb-test && echo PASS || echo FAIL
  cargo xtask ai-deep --bead vb-test && echo PASS || echo FAIL
  cargo xtask ai-release --bead vb-test && echo PASS || echo FAIL

Phase 8 (moon ci gate):
  moon ci  # canonical CI gate — all gates must green
```
