# Proof Strategy: contracts-as-data suite (vb-6f02)

## Strategy Overview

The contract-discovery module is a cold-path xtask command that walks `contracts/`, validates CUE schema metadata, runs `cue vet`, and produces `GateEvidence` for the evidence pipeline. Because it is **tooling (not runtime core)**, the proof strategy prioritizes:

1. **Safety** — no panics, no unwrap, no undefined behavior
2. **Correctness** — invariant enforcement (schema_version, kind, monotonicity, determinism)
3. **Integration** — GateEvidence parity with existing pipeline

**Principle**: Use the cheapest verifier that proves the obligation. Kani for exhaustiveness, proptest for randomized coverage, Verus for mathematical invariants, forbidden-scan for code properties, CI for integration.

## Obligation-by-Obligation Strategy

### OBL-001: schema_version required

**Verifier**: proptest
**Why proptest**: The obligation is a simple parse correctness check — does the parser return the right error when `schema_version` is missing? No mathematical depth needed.
**Proof plan**:
- Generate random `.cue` content (1000 properties via `Arbitrary`)
- Remove `schema_version` field from a copy
- Assert `parse_schema_version` returns `Err(ValidationError::MissingSchemaVersion)`
- Property: for all strings `s` not matching `^\d+\.\d+\.\d+$`, parse returns `Err`
- Property: for all strings `s` matching `^\d+\.\d+\.\d+$`, parse returns `Ok(s)`
**Command**: `cargo test -p xtask --test contract_test -- schema_version --exact`
**Phase**: Phase 1 (foundation)

### OBL-002: kind closed set

**Verifier**: proptest + Kani
**Why proptest + Kani**: Need both randomized coverage (proptest) and exhaustive enum verification (Kani). Proptest generates random strings not in the enum; Kani exhaustively verifies all 6 variants are covered.
**Proof plan**:
- proptest: generate 1000 random strings, assert all non-enum strings return `Err(InvalidKind)`
- Kani: `kani::any::<ContractKind>()` — exhaustively iterate all variants
- Property: `ContractKind::all_values().len() == 6`
- Property: exhaustive match on `ContractKind` covers all 6 variants (compile-time check via `#[deny(non_exhaustive_omitted_patterns)]`)
**Command (proptest)**: `cargo test -p xtask --test contract_test -- kind_closed --exact`
**Command (Kani)**: `cargo kani -p xtask --harness kani_kind_exhaustive`
**Phase**: Phase 1 (foundation)

### OBL-003: cue vet exit code parsing

**Verifier**: Kani
**Why Kani**: Must handle all i32 exit codes without panic. Proptest can't exhaustively cover 2^32 possibilities. Kani symbolically explores all values.
**Proof plan**:
- `kani::any::<i32>()` for exit code
- Assert `parse_vet_exit_code(exit_code)` never panics
- Assert `exit_code == 0` iff `vet_ok == true`
- Assert `exit_code < 0` maps to system error (not generic failure)
**Command**: `cargo kani -p xtask --harness kani_vet_exit_code`
**Phase**: Phase 1 (foundation)

### OBL-004: semver monotonicity

**Verifier**: Verus
**Why Verus**: This is a mathematical property — semver comparison is a well-ordered relation. Proptest can't prove "for all versions, monotonicity holds." Verus proves it by specification.
**Proof plan**:
- Define `compare_semver(old: &str, new: &str) -> i32` as a `spec fn` with `requires` and `ensures` clauses
- `ensures result > 0 == (new > old in semver ordering)`
- `ensures result < 0 == (new < old in semver ordering)`
- `ensures result == 0 == (new == old)`
- Split into sub-proof: `parse_semver` extracts (major, minor, patch) as u32 tuple
- Split into sub-proof: `u32_lex_compare` is a strict weak order
- Proof binds to `VersionViolation.expected` and `VersionViolation.actual` via `#[verifier(external)]`
**Command**: `cargo verus -p xtask -- contracts/semver.rs`
**Phase**: Phase 2 (formal) — depends on OBL-001 implementation

### OBL-005: deterministic output

**Verifier**: proptest
**Why proptest**: Determinism is a property that can be verified by randomized shuffling. Generate random file paths, shuffle N times, assert sorted output is identical.
**Proof plan**:
- Generate random set of file paths (up to 100, matching hardware limit)
- Shuffle 100 times
- Assert `sort(paths) == sort(shuffled_paths)` for all shuffles
- Property: `sort` is stable (same input always gives same output)
- Property: sorted output is lexicographic on `PathBuf`
**Command**: `cargo test -p xtask --test contract_test -- deterministic_output --exact`
**Phase**: Phase 1 (foundation)

### OBL-006: GateEvidence parity

**Verifier**: Kani
**Why Kani**: Must prove `gate_evidence_from_report(total, valid, invalid)` always returns `Ok(_)` and that `valid + invalid = total` doesn't overflow. Kani checks for integer overflow at runtime and exhaustively explores the state space.
**Proof plan**:
- `kani::any::<u32>()` for `total`, `valid`, `invalid`
- Precondition: `valid.saturating_add(invalid) == total`
- Assert `gate_evidence_from_report(total, valid, invalid).is_ok()`
- Assert `status == Pass` iff `invalid == 0`
- Assert `status == Fail` iff `invalid > 0`
- Assert `exit_code == 0` iff `invalid == 0`
- Property: `why_failed.is_some()` iff `invalid > 0`
**Command**: `cargo kani -p xtask --harness kani_gate_evidence_parity`
**Phase**: Phase 2 (formal) — depends on OBL-001, OBL-003 implementation

### OBL-007: no YAML/JSON/HTTP in runtime core

**Verifier**: forbidden-scan (existing CI gate)
**Why forbidden-scan**: This is already an existing xtask command. No new proof needed — just add `contracts/` to the exclusion list (it's tooling, not core).
**Proof plan**:
- Run `cargo xtask forbidden-scan --crate vb_core --pattern yaml`
- Run `cargo xtask forbidden-scan --crate vb_core --pattern serde_json`
- Verify `contracts/` is not under `crates/` (it's workspace-root-relative)
**Command**: `cargo xtask forbidden-scan --crate vb_core --pattern yaml`
**Phase**: Phase 0 (already exists)

### OBL-008: empty contracts directory

**Verifier**: proptest
**Why proptest**: Simple edge case — walk empty directory, assert report has all zeros.
**Proof plan**:
- Create temp directory (empty)
- Call `discover(&empty_dir)`
- Assert `total == 0`, `valid == 0`, `invalid == 0`, `status == Pass`
**Command**: `cargo test -p xtask --test contract_test -- empty_directory --exact`
**Phase**: Phase 1 (foundation)

### OBL-009: moon task integration

**Verifier**: CI gate
**Why CI**: Moon task integration is an integration-level concern. Cannot be proven at unit-test level — must run the actual moon task.
**Proof plan**:
- Define moon task in `.moon/tasks.yaml`
- CI runs `moon run :contracts` on every PR
- Task returns exit 0 if all contracts valid, exit 1 if any invalid
**Command**: `moon run :contracts` (in CI)
**Phase**: Phase 3 (integration) — depends on Phase 1 implementation

### OBL-010: CUE schema #ContractMeta enforcement

**Verifier**: `cue vet` (integration test)
**Why integration test**: CUE schema enforcement is a schema-level property, not a Rust property. The verifier is the CUE tool itself.
**Proof plan**:
- Create a `.cue` file without `schema_version` — assert `cue vet` fails
- Create a `.cue` file with invalid `kind` — assert `cue vet` fails
- Create a valid `.cue` file — assert `cue vet` passes
- Property: `cue vet` exit code == 0 iff file is well-formed
**Command**: `cue vet contracts/cli_envelope.cue` (and all other contract files)
**Phase**: Phase 1 (foundation)

## Execution Order

```
Phase 0 (existing, no dependencies):
  OBL-007 — forbidden-scan

Phase 1 (foundation, parallel):
  OBL-001 — proptest (schema_version parse)
  OBL-002 — proptest + Kani (kind enum)
  OBL-003 — Kani (vet exit code)
  OBL-005 — proptest (deterministic output)
  OBL-008 — proptest (empty directory)
  OBL-010 — cue vet (schema enforcement)

Phase 2 (formal, depends on Phase 1):
  OBL-004 — Verus (semver monotonicity) — needs OBL-001 impl
  OBL-006 — Kani (GateEvidence parity) — needs OBL-001, OBL-003 impl

Phase 3 (integration, depends on Phase 2):
  OBL-009 — CI gate (moon task)
```

## Verifier Distribution Summary

| Verifier | Obligations | Coverage |
|----------|------------|----------|
| proptest | OBL-001, OBL-002, OBL-005, OBL-008 | Randomized correctness |
| Kani | OBL-002, OBL-003, OBL-006 | Exhaustive safety |
| Verus | OBL-004 | Mathematical invariant |
| forbidden-scan | OBL-007 | Code property (existing) |
| CI | OBL-009 | Integration |
| cue vet | OBL-010 | Schema enforcement |

## Risk Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Kani harness explosion on u32 space | OBL-006 may time out | Add precondition `valid.saturating_add(invalid) == total` to bound search space |
| Verus spec requires semver parsing | OBL-004 depends on Rust impl | Prove `parse_semver` correctness separately; compose proofs |
| `cue` CLI unavailable on CI | OBL-010, OBL-003 may fail | Install via `just install-cue` in CI setup; skip if unavailable |
| BTreeMap serialization ordering | OBL-005 determinism may break | Use `BTreeMap` (not `HashMap`) for `errors_by_kind`; assert sorted keys in proptest |
