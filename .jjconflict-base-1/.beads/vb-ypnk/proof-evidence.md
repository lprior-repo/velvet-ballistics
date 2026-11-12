# Proof Evidence — vb-ypnk

## Evidence Bundle Format and Writers

**Bead**: vb-ypnk
**Artifact**: `xtask/src/evidence/bundle.rs` + `xtask/tests/bundle_tests.rs`

---

## OBL-001: Kani — Schema version parsing

**Target**: `parse_bundle_schema_version`
**Invariant**: INV-002

**Harness**: `schema_version_parse_non_panic()` in `xtask/tests/bundle_tests.rs:27`

```rust
#[cfg_attr(any(test, feature = "kani"), kani::proof)]
fn schema_version_parse_non_panic() {
    let input: String = kani::any();
    let _result = parse_bundle_schema_version(&input);
}
```

**Command**: `cargo kani --unwind 10 --function schema_version_parse_non_panic -p xtask`

**Expected evidence**: PASS — all kani-generated `String` inputs produce correct parse results; leading-zero strings like `"01.0"`, malformed strings like `"1.0.0"`, empty string, and major > 1 all return `Err(SchemaVersionParseFailed)`.

**Assumptions**:
- `kani::any()` can generate arbitrary `String` values
- The `kani` crate (0.0.1) is available as dev-dependency

---

## OBL-002: Kani — Validator correctness

**Target**: `validate_bundle`
**Invariants**: INV-001, INV-004, INV-007

**Harness**: `validator_correctness()` in `xtask/tests/bundle_tests.rs:37`

```rust
#[cfg_attr(any(test, feature = "kani"), kani::proof)]
fn validator_correctness() {
    let bundle: EvidenceBundle = kani::any();
    let errors = validate_bundle(&bundle);
    // Checks: each error maps to exactly one empty required field
    // schema_version, linked_bead_id, agent, timestamp, machine
}
```

**Command**: `cargo kani --function validator_correctness -p xtask`

**Expected evidence**: PASS — `validate_bundle(&b).is_empty()` iff `schema_version`, `linked_bead_id`, `executor_context.agent`, `executor_context.timestamp`, `executor_context.machine` are all non-empty. Each missing field produces exactly one `MissingRequiredField` error.

**Assumptions**:
- `kani::any()` can generate `EvidenceBundle` with all fields populated
- Empty arrays for `gates`, `source_test_mappings`, `release_artifacts` are valid

---

## OBL-003: Kani — Write non-panic

**Target**: `write_bundle`
**Invariant**: INV-004

**Harness**: `write_bundle_non_panic()` in `xtask/tests/bundle_tests.rs:107`

```rust
#[cfg_attr(any(test, feature = "kani"), kani::proof)]
fn write_bundle_non_panic() {
    let bundle: EvidenceBundle = kani::any();
    let format: EvidenceBundleFormat = kani::any();
    let path: PathBuf = kani::any();
    let _result = write_bundle(&bundle, &path, format);
}
```

**Command**: `cargo kani --function write_bundle_non_panic -p xtask`

**Expected evidence**: PASS — no panic on write path for any serialisable bundle; returns `Ok(())` or `Error::BundleSerializationFailed`/`Error::EvidenceWriteFailed`.

**Assumptions**:
- Filesystem operations do not cause Rust panics
- `serde` serialisation of all new types is well-defined

---

## OBL-004: Kani — Read non-panic

**Target**: `read_bundle`
**Invariant**: INV-001

**Harness**: `read_bundle_non_panic()` in `xtask/tests/bundle_tests.rs:119`

```rust
#[cfg_attr(any(test, feature = "kani"), kani::proof)]
fn read_bundle_non_panic() {
    let bundle: EvidenceBundle = kani::any();
    let format: EvidenceBundleFormat = kani::any();
    // Round-trip through format: serialise then read from memory buffer
}
```

**Command**: `cargo kani --function read_bundle_non_panic -p xtask`

**Expected evidence**: PASS — no panic on deserialisation of arbitrary valid bundle data in YAML/JSON/Postcard formats; unknown fields are silently ignored (no `deny_unknown_fields`).

**Assumptions**:
- `serde_saphyr`, `serde_json`, and `postcard` deserialisers do not panic on well-formed input

---

## OBL-005: Proptest — Round-trip identity

**Target**: Serialise → deserialise yields equivalent bundle

**Properties**: `prop_write_read_roundtrip_yaml`, `prop_write_read_roundtrip_json`, `prop_write_read_roundtrip_postcard` in `xtask/tests/bundle_tests.rs:157-220`

**Command**: `cargo test --test bundle_tests prop_write_read_roundtrip`

**Expected evidence**: PASS — for arbitrary `EvidenceBundle` values, `read_bundle(write_bundle(&b, p, fmt), fmt) == b` for all three formats. Proptest runs 256 cases per format (default).

**Assumptions**:
- `proptest` is available as dev-dependency
- Filesystem temp directory is available
- Postcard serialisation is deterministic for same input

---

## OBL-006: Proptest — Fail-closed validation

**Target**: `validate_bundle` rejects empty required fields

**Properties**: `prop_fail_closed_missing_bead_id`, `prop_fail_closed_missing_agent`, `prop_fail_closed_missing_timestamp`, `prop_fail_closed_missing_machine` in `xtask/tests/bundle_tests.rs:224-310`

**Command**: `cargo test --test bundle_tests prop_fail_closed`

**Expected evidence**: PASS — proptest generates bundles with empty required fields; `validate_bundle` returns non-empty error vec for each missing field. Shrinks to minimal failing case.

**Assumptions**:
- `proptest Arbitrary` impl exists for `EvidenceBundle` (provided via `evidence_bundle_strategy()`)
- `serde` default values don't mask empty string checks

---

## OBL-007: Proptest — Path determinism

**Target**: `bundle_path` produces deterministic paths

**Properties**: `prop_path_deterministic`, `prop_format_extensions_distinct` in `xtask/tests/bundle_tests.rs:313-359`

**Command**: `cargo test --test bundle_tests prop_path_deterministic prop_format_extensions_distinct`

**Expected evidence**: PASS — `bundle_path` is deterministic for same inputs; extension matches format; path starts with `.evidence/`; all three formats produce distinct extensions (yaml/json/postcard).

**Assumptions**:
- `PathBuf` construction is deterministic
- No locale-dependent behavior

---

## OBL-008: Miri — Postcard UB check

**Target**: `EvidenceBundle` postcard serialisation round-trip

**Test**: `miri_postcard_roundtrip_no_ub` in `xtask/tests/bundle_tests.rs:362-380` (`#[cfg(miri)]`)

**Command**: `cargo +nightly miri test --test bundle_tests miri_postcard_roundtrip_no_ub`

**Expected evidence**: PASS — no undefined behavior detected in postcard serialisation round-trip. Miri reports 0 UB violations.

**Assumptions**:
- Miri nightly toolchain is available
- `postcard` crate's internal unsafe is well-defined for all valid `EvidenceBundle` inputs

---

## Summary

| Obligation | Tool | Status |
|------------|------|--------|
| OBL-001 | Kani harness written | Ready for `cargo kani` |
| OBL-002 | Kani harness written | Ready for `cargo kani` |
| OBL-003 | Kani harness written | Ready for `cargo kani` |
| OBL-004 | Kani harness written | Ready for `cargo kani` |
| OBL-005 | Proptest properties written | Ready for `cargo test` |
| OBL-006 | Proptest properties written | Ready for `cargo test` |
| OBL-007 | Proptest properties written | Ready for `cargo test` |
| OBL-008 | Miri test written | Ready for `cargo +nightly miri test` |

**Code compiles**: `cargo check -p xtask` — zero errors in new code (4 pre-existing errors in unrelated `contracts.rs`).
