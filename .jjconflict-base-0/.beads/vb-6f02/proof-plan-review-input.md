# Proof Plan Review Input: contracts-as-data (vb-6f02)

## Module Under Verification

```
xtask/src/contracts.rs
  ├── pub fn discover(dir: &Path) -> Result<DiscoveryReport, XtaskCommandError>
  ├── pub fn parse_schema_version(raw: &str) -> ValidationResult<String>
  ├── pub fn parse_contract_kind(raw: &str) -> ValidationResult<ContractKind>
  ├── pub fn parse_vet_output(exit_code: i32) -> VetResult
  ├── pub fn compare_semver(old: &str, new: &str) -> i32
  └── pub fn gate_evidence_from_report(total: u32, valid: u32, invalid: u32) -> Result<GateEvidence, Error>
```

## Bind Points

| Function | File | Line (est.) | Binds To |
|----------|------|-------------|----------|
| `discover` | xtask/src/contracts.rs | 1-100 | INV-005 (deterministic) |
| `parse_schema_version` | xtask/src/contracts.rs | 101-180 | INV-001 (schema_version required) |
| `parse_contract_kind` | xtask/src/contracts.rs | 181-250 | INV-002 (kind closed set) |
| `parse_vet_output` | xtask/src/contracts.rs | 251-280 | INV-003 (cue vet passes) |
| `compare_semver` | xtask/src/contracts.rs | 281-350 | INV-004 (monotonicity) |
| `gate_evidence_from_report` | xtask/src/contracts.rs | 351-420 | INV-006 (GateEvidence parity) |
| `ValidationError` variants | crates/vb_validate/src/lib.rs | +4 new variants | OBL-001 through OBL-004 error mapping |

## Pre-conditions for Proof Execution

1. `contracts/` directory exists at workspace root
2. `cue` CLI is installed and on PATH
3. `cargo kani` is available (Kani toolchain installed)
4. `cargo verus` is available (Verus toolchain installed)
5. `vb_validate` crate compiles with 4 new `ValidationError` variants

## Proof Goals

### Goal G1: INV-001 — schema_version required
```
forall cue_content: bytes,
  let meta = parse_top_level_meta(cue_content) in
  if meta.schema_version == absent
  then parse_schema_version(raw) == Err(MissingSchemaVersion)
  else parse_schema_version(raw) == Ok(normalized_version)
```

### Goal G2: INV-002 — kind closed set
```
forall raw: str,
  let k = parse_contract_kind(raw) in
  k in {CliEnvelope, UiTokens, AcceptedArtifacts, EvidenceBundle, Diagnostics, GateOutput}
  <=> raw in {"cli_envelope", "ui_tokens", "accepted_artifacts", "evidence_bundle", "diagnostics", "gate_output"}
```

### Goal G3: INV-003 — vet exit code
```
forall exit_code: i32,
  let vet = parse_vet_output(exit_code) in
  vet == VetResult::Pass <=> exit_code == 0
  vet == VetResult::Fail <=> exit_code != 0
  // No panic for any i32 value
```

### Goal G4: INV-004 — semver monotonicity
```
forall old, new: str where is_semver(old) and is_semver(new),
  let cmp = compare_semver(old, new) in
  (cmp > 0) <=> new >_semver old
  (cmp < 0) <=> new <_semver old
  (cmp == 0) <=> new ==_semver old
```

### Goal G5: INV-005 — deterministic output
```
forall paths: [PathBuf],
  let sorted1 = sort(paths) in
  let shuffled = shuffle(paths) in
  let sorted2 = sort(shuffled) in
  sorted1 == sorted2
```

### Goal G6: INV-006 — GateEvidence parity
```
forall total, valid, invalid: u32 where valid + invalid == total,
  let evidence = gate_evidence_from_report(total, valid, invalid) in
  evidence.status == Pass <=> invalid == 0
  evidence.status == Fail <=> invalid > 0
  evidence.exit_code == 0 <=> invalid == 0
  evidence.why_failed.is_some() <=> invalid > 0
  // No u32 overflow on valid + invalid (bounded by precondition)
```

## Verification Layer Assignments

| Goal | Primary Verifier | Secondary Verifier | Phase |
|------|-----------------|-------------------|-------|
| G1 | proptest | — | 1 |
| G2 | proptest | Kani | 1 |
| G3 | Kani | — | 1 |
| G4 | Verus | — | 2 |
| G5 | proptest | — | 1 |
| G6 | Kani | — | 2 |

## Dependency Graph

```
G1 (OBL-001) ──────────────────────────┐
                                       ├──> G4 (OBL-004) [Verus]
G2 (OBL-002) ──────────────────────────┤
                                       │
G3 (OBL-003) ──────────────────────────┤
                                       ├──> G6 (OBL-006) [Kani]
G5 (OBL-005) ──────────────────────────┤
                                       │
G8 (OBL-008) ──────────────────────────┤
                                       │
G10 (OBL-010) ─────────────────────────┤

All Phase 1 goals ────────────────────┤
                                       ├──> G9 (OBL-009) [CI]
G4 (OBL-004) ──────────────────────────┤
G6 (OBL-006) ──────────────────────────┘

G7 (OBL-007) [independent, existing]
```

## Review Checklist

- [ ] Each verifier is the cheapest tool that proves the obligation
- [ ] No Kani harness hardcodes structural inputs (uses `kani::any()` or `Arbitrary`)
- [ ] Verus specs bind to actual Rust `exec fn` in production code
- [ ] TLA+ models bounded hardware limits (100 file max in contracts/)
- [ ] Proptest property counts are sufficient (≥100 iterations per property)
- [ ] No proof alters the mathematical contract to make tests green
- [ ] OBL-007 (forbidden-scan) does not need new proof — existing tool suffices
- [ ] OBL-009 (moon task) is an integration gate, not a formal proof — appropriate
- [ ] OBL-010 (cue vet) uses the CUE tool itself as verifier — appropriate

## Post-conditions

When all proofs pass:
1. Every `.cue` file in `contracts/` has `schema_version` and `kind`
2. `kind` values are restricted to the 6 valid enum members
3. `cue vet` exit codes are correctly mapped to pass/fail
4. Semver comparison is monotonically increasing
5. Discovery output is deterministic (independent of filesystem order)
6. GateEvidence is always produced and correctly represents report state
7. No YAML/JSON/HTTP in runtime core (existing invariant)
8. Empty contracts/ directory produces valid Pass report
9. Moon task integration runs correctly in CI
10. CUE schemas enforce metadata at the schema level
