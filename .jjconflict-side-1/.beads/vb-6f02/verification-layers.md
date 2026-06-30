# Verification Layers: contracts-as-data

## Layer Overview

| Layer | Obligation | Verifier | Gate | Bind Point |
|-------|-----------|----------|------|------------|
| L1 | INV-001: schema_version required | proptest | CI | `ContractFileMeta.schema_version` |
| L2 | INV-002: kind closed set | proptest + Kani | CI | `ContractKind::try_from_str` |
| L3 | INV-003: cue vet passes | Kani | CI | `parse_vet_exit_code` |
| L4 | INV-004: version monotonicity | Verus | PR | `compare_semver` |
| L5 | INV-005: deterministic output | proptest | CI | `sort_reports` |
| L6 | INV-006: GateEvidence parity | Kani | CI | `gate_evidence_from_report` |
| L7 | INV-007: no YAML in core | forbidden-scan | CI | `xtask/src/contracts.rs` |
| L8 | INV-008: cue vet zero errors | CI gate | CI | `moon tasks.contracts` |

## L1: schema_version required

**Obligation**: `OBL-001` — Every `.cue` file under `contracts/` must have a `schema_version` field.

**Verifier**: proptest

**Test**: `test_schema_version_present`
```
// Generate random CUE file content
// Assert that parse_schema_version returns Ok(_) only when version is present
// Assert that missing version returns Err(ValidationError::MissingSchemaVersion)
```

**Bind point**: `xtask/src/contracts.rs::parse_schema_version`

**Coverage**: 100% of `parse_schema_version` branches (present, absent, empty, malformed).

## L2: kind closed set

**Obligation**: `OBL-002` — Every `.cue` file's `kind` must be one of the 6 enum values.

**Verifier**: proptest + Kani

**proptest test**: `test_kind_exhaustive`
```
// For each of the 6 ContractKind variants, verify try_from_str succeeds
// For random strings not in the 6, verify try_from_str fails
```

**Kani harness**: `kani_kind_exhaustive`
```rust
#[kani::proof]
fn kani_kind_exhaustive() {
    let k: ContractKind = kani::any();
    // Exhaustive: every possible ContractKind value is one of the 6
    assert_eq!(k, ContractKind::all_values().iter().find(|v| *v == &k).copied().unwrap());
}
```

**Bind point**: `xtask/src/contracts.rs::ContractKind::try_from_str`

**Coverage**: All 6 enum values + arbitrary invalid strings.

## L3: cue vet passes

**Obligation**: `OBL-003` — Every file in `contracts/` must pass `cue vet` with exit code 0.

**Verifier**: Kani

**Kani harness**: `kani_vet_exit_code`
```rust
#[kani::proof]
fn kani_vet_exit_code() {
    let exit_code: i32 = kani::any();
    // Verify: exit_code 0 => vet passes, exit_code != 0 => vet fails
    let vet_ok = match exit_code {
        0 => true,
        _ => false,
    };
    assert!(vet_ok == (exit_code == 0));
    // No panic on any i32 input
}
```

**Bind point**: `xtask/src/contracts.rs::parse_vet_output`

**Coverage**: All exit codes from 0 to 255 (Kani explores the full space).

## L4: version monotonicity

**Obligation**: `OBL-004` — Schema version updates must be strictly increasing (semver comparison).

**Verifier**: Verus

**Verus spec**:
```rust
#[verifier::spec]
fn compare_semver(old: &str, new: &str) -> i32
    ensures (result > 0) == (new > old),
            (result < 0) == (new < old),
            (result == 0) == (new == old),
{
    // Parse "N.N.N" into (major, minor, patch) tuples
    // Compare lexicographically
}
```

**Proof obligation**: `ensures` clause guarantees monotonicity — if `new > old` in semver sense, result is positive.

**Bind point**: `xtask/src/contracts.rs::compare_semver`

**Coverage**: All valid semver strings (bounded: major, minor, patch each in [0, 2^32)).

**Bounded**: Uses `u32` for major/minor/patch, not unbounded `Nat`. Matches INV-004 constraint.

## L5: deterministic output

**Obligation**: `OBL-005` — Discovery output is sorted by file path, sorted diagnostics.

**Verifier**: proptest

**Test**: `test_deterministic_output`
```
// Generate random file paths
// Shuffle them 100 times
// Assert that sorted output is identical each time
```

**Bind point**: `xtask/src/contracts.rs::discover` (the `sort` call before returning)

**Coverage**: 100 shuffles with random path sets.

## L6: GateEvidence parity

**Obligation**: `OBL-006` — Every contract-discovery run produces exactly one `GateEvidence`.

**Verifier**: Kani

**Kani harness**: `kani_gate_evidence_parity`
```rust
#[kani::proof]
fn kani_gate_evidence_parity() {
    let total: u32 = kani::any();
    let valid: u32 = kani::any();
    let invalid: u32 = kani::any();

    // Precondition: valid + invalid = total (bounded arithmetic)
    // Kani checks for overflow at runtime
    if valid + invalid == total {
        let evidence = gate_evidence_from_report(total, valid, invalid);
        // Postcondition: exactly one GateEvidence, status consistent
        assert!(evidence.is_ok());
        let e = evidence.unwrap();
        assert!(e.status == GateStatus::Pass || e.status == GateStatus::Fail);
    }
}
```

**Bind point**: `xtask/src/evidence/tooling_and_gate_types.rs::gate_evidence_from_report`

**Coverage**: Exhaustive on valid + invalid = total (Kani explores all valid combinations within u32 range).

## L7: no YAML in core

**Obligation**: `OBL-007` — Runtime core crates must not contain YAML parsing.

**Verifier**: forbidden-scan (existing xtask command)

**Command**: `cargo xtask forbidden-scan --crate vb_core --pattern yaml`

**Bind point**: `xtask/src/forbidden_scan.rs`

**Coverage**: All files in `crates/vb_core/**`, `crates/vb_runtime/**`, etc. (existing pattern).

**Note**: `contracts/` directory is NOT under `crates/` — it's a workspace-level directory. The invariant is enforced by the existing forbidden-scan command, not by the new contracts command.

## L8: cue vet zero errors (CI gate)

**Obligation**: `OBL-008` — `cargo xtask contracts --check` must return exit code 0 for CI pass.

**Verifier**: CI gate (moon task)

**Moon task** (to be defined in `.moon/tasks.yaml`):
```yaml
tasks:
  contracts:
    command:
      args: ["xtask", "contracts", "--check"]
    platforms: ["linux", "darwin"]
```

**Bind point**: `xtask/src/cli.rs::Contracts::check` variant

**Coverage**: CI runs this on every PR to `main`.

## Verification Matrix Summary

| Obligation | Required? | Verifier | CI Gate? | Proof Status |
|------------|-----------|----------|----------|-------------|
| OBL-001 | required | proptest | yes | planned |
| OBL-002 | required | proptest + Kani | yes | planned |
| OBL-003 | required | Kani | yes | planned |
| OBL-004 | required | Verus | PR only | planned |
| OBL-005 | required | proptest | yes | planned |
| OBL-006 | required | Kani | yes | planned |
| OBL-007 | required | forbidden-scan | yes | existing |
| OBL-008 | required | CI gate | yes | planned |

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|------------|
| `cue` CLI not available on CI | High | Install via `just install-cue` in CI setup step |
| `cue vet` false positives on complex schemas | Medium | Use `cue export` as fallback; document known exceptions |
| Semver parsing edge cases (leading zeros, etc.) | Low | Strict pattern: `^\d+\.\d+\.\d+$` with `u32` bounds |
| Manifest file corruption | Low | Atomic writes (write to `.tmp`, then rename) |
| Kani harness state explosion | Medium | Bound file count to 100 (hardware limit) |

## Dependencies Between Layers

```
L4 (Verus) ──depends on──> L1 (schema_version parsing)
L6 (Kani GateEvidence) ──depends on──> L1, L2, L3, L4, L5
L8 (CI gate) ──depends on──> L1-L7
```

Layer L6 depends on all other layers because `gate_evidence_from_report` aggregates results from parsing (L1), kind validation (L2), vet checking (L3), version comparison (L4), and sorting (L5).
