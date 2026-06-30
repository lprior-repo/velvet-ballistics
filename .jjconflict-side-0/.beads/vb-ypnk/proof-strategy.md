# Proof Strategy — vb-ypnk: Evidence Bundle Format and Writers

## Scope Summary

This bead adds serialisable bundle types and writer/reader/validator functions to `xtask::evidence::bundle`.
It is a **data-format artifact**: no unsafe, no concurrency, no state machines, no ghost state.

## Verifier Selection

| Verifier | Lanes | Obligations | Justification |
|----------|-------|-------------|---------------|
| **Kani** | L1–L4 | OBL-001, OBL-002, OBL-003, OBL-004 | The contract demands non-panic guarantees on all public API entry points (parse, validate, read, write). Kani's bounded model checking is the cheapest way to prove absence of panics across all reachable code paths (arbitrary inputs, serde deserialisation edge cases, filesystem errors). No temporal reasoning or heap predicates needed. |
| **proptest** | L5–L7 | OBL-005, OBL-006, OBL-007 | Property-based testing is the right tool for structural invariants that are inherently universal (round-trip equality, fail-closed validation, path determinism). These are not panics but semantic contracts: the property must hold for *any* valid bundle, not just a few hand-crafted cases. proptest's shrinking makes failures actionable. |
| **Miri** | L8 | OBL-008 | Postcard serialisation uses `unsafe` internally (byte-level repr transmutes). Miri catches UB in Postcard's serialization/deserialization paths that Kani's safe-only model checker cannot see. This is the only lane where Miri adds value over Kani for this bead. |

## Verifiers Explicitly Not Used

| Verifier | Reason |
|----------|--------|
| **TLA+** | No concurrency, no state-machine behavior, no distributed protocol. The bundle is a single-threaded serialisable record. |
| **Verus** | No ghost state, no heap predicates, no mathematical invariants over data structures beyond what Kani/proptest cover. The types are plain structs with string/vec fields. |
| **Flux RS** | No refinement types, no indexed types, no constraint refinement needed. The format is `String`, `Vec<T>`, and a simple enum. |
| **Loom** | No concurrency, no lock-free code, no atomics. |
| **Kani fuzzing** | The input domain (strings, paths, file content) is better covered by proptest's arbitrary generators than by Kani's randomised exploration. |

## Proof Strategy Detail

### Kani — Non-Panic + Correctness Proofs

**OBL-001: `parse_bundle_schema_version` correctness**
- Harness: `kani::any::<String>` fed to `parse_bundle_schema_version`
- Prove: If `Ok`, the string matches `^(0|[1-9][0-9])\.(0|[1-9][0-9])$`
- Prove: If string contains leading zeros (e.g. "01.0"), returns `Err`
- Prove: If string has no dot, extra dots, empty parts, or negative signs, returns `Err`
- Unwind bound: 10 (sufficient for string pattern matching)

**OBL-002: `validate_bundle` correctness**
- Harness: `kani::any::<EvidenceBundle>`
- Prove: `validate_bundle(&b).is_empty()` iff all required fields are non-empty
- Prove: Each missing required field produces exactly one `MissingRequiredField` error
- Prove: Empty `gates` and `source_test_mappings` and `release_artifacts` arrays are valid (zero-length arrays are allowed by INV-004)

**OBL-003: `write_bundle` non-panic**
- Harness: `kani::any::<EvidenceBundle>` + any valid `PathBuf`
- Prove: No `panic!`, no `unwrap()`, no `expect()` in the write path
- Prove: Returns `Ok(())` for serialisable bundles or `Err(BundleSerializationFailed|EvidenceWriteFailed)`
- Assumption: Filesystem is available (Kani cannot prove actual file creation, only non-panic)

**OBL-004: `read_bundle` non-panic**
- Harness: Well-formed serialised content in memory (simulated file read)
- Prove: No panic on deserialisation of arbitrary valid bundle data
- Prove: No panic on deserialisation of data with extra fields (serde's `#[serde(deny_unknown_fields)]` is NOT used, so unknown fields are ignored gracefully)

### proptest — Property Coverage

**OBL-005: Round-trip identity (all formats)**
- Generator: Arbitrary `EvidenceBundle` with non-empty strings for required fields
- Property: `read_bundle(write_bundle(&b, p, fmt), fmt)` produces bundle equal to `b` for Yaml, Json, and Postcard
- Note: Postcard round-trip must be byte-identical in-memory (not bit-identical serialised bytes, but structurally equal deserialised values)

**OBL-006: Fail-closed validation**
- Generator: Bundles with `linked_bead_id = ""`, empty executor context fields
- Property: `validate_bundle` returns non-empty error vec for each missing field
- Shrinking: proptest should shrink to minimal failing bundle

**OBL-007: Path determinism**
- Generator: Arbitrary bead ID strings (non-empty), arbitrary format
- Property: `bundle_path(bead_id, fmt)` is deterministic: same inputs → same output
- Property: Extension matches `EvidenceBundleFormat::extension()`
- Property: Path starts with `.evidence/`

### Miri — UB Detection on Postcard Path

**OBL-008: No UB in serialization/deserialization**
- Run: `cargo +nightly miri test` on `xtask` crate with bundle module included
- Scope: All `write_bundle` and `read_bundle` calls with `EvidenceBundleFormat::Postcard`
- Justification: Postcard internally uses `unsafe` for repr transmutes. Miri is the only mainstream verifier that catches UB in unsafe code. Kani cannot check unsafe blocks.

## Assumptions

| # | Assumption | Impact if False |
|---|-----------|-----------------|
| A-001 | `GateEvidence`, `GateStatus`, `WhyFailed` derive `Serialize`/`Deserialize` | Kani proptest generators for `EvidenceBundle` cannot be constructed |
| A-002 | `serde_saphyr` is YAML-1.2 compatible with `serde` semantics | Round-trip test OBL-005 for YAML may fail on edge cases |
| A-003 | Postcard serialisation is deterministic for the same input | OBL-005 Postcard round-trip fails (not a correctness issue, just test reliability) |
| A-004 | Filesystem operations do not cause panics in the test environment | Kani OBL-003 non-panic proof only covers code logic, not actual I/O |

## Risk Assessment

| Risk | Severity | Mitigation |
|------|----------|-----------|
| `serde_saphyr` YAML parser panics on edge-case input | High | Kani OBL-004 covers `read_bundle` non-panic; Miri catches any UB in the parser |
| Postcard repr transmutes cause UB on non-platform data | High | Miri OBL-008 specifically targets this |
| `kani::any` cannot generate `EvidenceBundle` due to complex serde derives | Medium | Fallback: manual `Arbitrary` impl for proptest generators |
| Pre-existing compile errors in xtask block proof artifact compilation | High | Must resolve compile errors before running Kani/Miri |
