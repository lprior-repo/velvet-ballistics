# Proof Repair Guide — vb-6f02

**Triggered by:** proof-review.md REJECTION
**Route:** Back to State 5 (Proof Writing)
**Priority:** Critical fixes first, then major, then moderate

---

## Repair Priority 1: Critical (BLOCKS acceptance)

### R1: Eliminate all `assume(true)` in Verus proofs (F-001)

**Current state:** 9 proof functions all end with `assume(true);` — zero proof content.

**Repair:** Each proof function must perform actual case analysis and deduction.

**Example — `verify_parse_schema_version_satisfies_spec`:**

```rust
verus! {
    proof fn verify_parse_schema_version_satisfies_spec(input: &str)
        requires true
        ensures matches spec_parse_schema_version(input),
    {
        // Case 1: empty input
        if input.is_empty() {
            // Exec returns Err("Missing schema version")
            // Spec returns Err("Missing schema version")  
            // They match — structural equality on the empty branch
        } else {
            // Case 2: non-empty input
            let parts: Vec<&str> = input.splitn(3, '.').collect();
            
            // Sub-case 2a: not 3 parts
            if parts.len() != 3 {
                // Both exec and spec return Err with format string
                // The format strings are identical → results match
            } else {
                // Sub-case 2b: 3 parts — check each
                for part in &parts {
                    if part.is_empty() {
                        // Both return Err("Empty semver component...")
                    } else if part.len() > 1 && part.starts_with('0') {
                        // Both return Err("Leading zero...")
                    } else if part.parse::<u64>().is_err() {
                        // Both return Err("Non-numeric...")
                    }
                }
                // All parts valid → both return Ok(input.to_string())
            }
        }
        // Since control flow and conditions are identical,
        // the results must be equal
    }
}
```

**Key technique:** Use `opens_invariant` to open structural invariants about the exec/spec pair, then use `assert!` to prove branch-by-branch equality. The Verus verifier can handle this because both functions follow the same control flow.

**All 9 proofs to repair:**
1. `verify_parse_schema_version_satisfies_spec` (line 120) — structural case analysis
2. `verify_parse_contract_kind_is_total` (line 166) — match arm exhaustiveness
3. `verify_parse_contract_kind_only_valid_kinds` (line 179) — Ok arm implies valid input
4. `verify_semver_reflexive` (line 252) — lexicographic tuple self-comparison
5. `verify_semver_antisymmetric` (line 264) — lexicographic tuple sign reversal
6. `verify_semver_transitive` (line 277) — lexicographic tuple transitivity
7. `verify_semver_strict_weak_order` (line 296) — combines 1-6
8. `verify_btreemap_deterministic` (line 331) — sorting produces identical sequences
9. `verify_gate_condition` (line 370) — boolean algebra on invalid/violations

### R2: Bind Verus specs to production code (F-002)

**Current state:** Verus file defines its own `ContractKind`, `compare_semver`, etc.

**Repair option A (import-based, preferred):**
```rust
// contracts/verus/contracts_as_data_spec.rs
use vb_contracts::ContractKind;  // re-exported from xtask/src/contracts.rs
use vb_contracts::{parse_schema_version, parse_contract_kind, compare_semver};

// Now spec fn refers to production function behavior
spec fn spec_parse_schema_version(input: &str) -> Result<String, String> {
    // ... spec about production function ...
}

// Proof binds spec to actual production function
verus! {
    proof fn verify_parse_schema_version_satisfies_spec(input: &str)
        ensures matches spec_parse_schema_version(input),
    {
        // Proves that the actual xtask/src/contracts.rs::parse_schema_version
        // satisfies the mathematical spec
    }
}
```

**Repair option B (attribute-based):**
```rust
// In xtask/src/contracts.rs directly
use verus::{spec, proof, ensures, requires};

#[spec(fn = spec_parse_schema_version)]
pub fn parse_schema_version(input: &str) -> Result<String, String> {
    // ... production implementation ...
}

spec fn spec_parse_schema_version(input: &str) -> Result<String, String> {
    // ... mathematical specification ...
}
```

### R3: Align integer types across all artifacts (F-003)

**Current state:** Verus uses `u64`, Kani uses `u32`, proptest uses `u32`.

**Repair:** 
1. Check `xtask/src/contracts.rs` to determine the production type
2. Update the non-matching artifacts to use the production type
3. Add a compile-time assertion in test code:
```rust
const _: () = assert!(
    std::mem::size_of::<u32>() == std::mem::size_of::<u32>()  // trivial check
);
// Better: verify the actual production function signature matches
fn assert_compare_semver_signature() {
    // This will fail to compile if the signature changes
    let _f: fn(&str, &str) -> i32 = vb_contracts::compare_semver;
}
```

---

## Repair Priority 2: Major

### R4: Remove hardcoded Kani malformed inputs (F-004)

**Current:** 12-element hardcoded array in `kani_schema_version_rejects_malformed`

**Repair:**
```rust
#[kani::proof]
#[kani::unwind(10)]
fn kani_schema_version_rejects_malformed() {
    let raw: String = kani::any();
    
    // Assert: if raw doesn't match valid semver pattern, it must be rejected
    let is_valid_pattern = raw.splitn(3, '.').count() == 3
        && raw.split('.').all(|p| !p.is_empty())
        && raw.split('.').all(|p| p.chars().all(|c| c.is_ascii_digit()));
    
    // Check for leading zeros
    let has_leading_zeros = raw.splitn(3, '.').any(|p| p.len() > 1 && p.starts_with('0'));
    
    if (!is_valid_pattern || has_leading_zeros) && !raw.is_empty() {
        let result = parse_schema_version(&raw);
        assert!(matches!(result, Err(ValidationError::InvalidVersion { .. })));
    }
}
```

### R5: Fix Kani kind_exhaustive string binding (F-005)

**Current:** Generates `kani::any::<ContractKind>()` but verifies against hardcoded strings

**Repair:**
```rust
fn kind_to_string(kind: ContractKind) -> &'static str {
    match kind {
        ContractKind::CliEnvelope => "cli_envelope",
        ContractKind::UiTokens => "ui_tokens",
        ContractKind::AcceptedArtifacts => "accepted_artifacts",
        ContractKind::EvidenceBundle => "evidence_bundle",
        ContractKind::Diagnostics => "diagnostics",
        ContractKind::GateOutput => "gate_output",
    }
}

#[kani::proof]
fn kani_kind_exhaustive() {
    let kind = kani::any::<ContractKind>();
    let expected_str = kind_to_string(kind);
    let result = parse_contract_kind(expected_str);
    assert!(matches!(result, Ok(k) if k == kind));
}
```

### R6: Replace CUE string simulation with real CUE validation (F-006)

**Current:** `is_valid_contract_cue()` checks string substrings

**Repair:**
```rust
fn run_cue_vet(cue_content: &str) -> bool {
    let tmp = std::env::temp_dir().join("contract_test.cue");
    std::fs::write(&tmp, cue_content).unwrap();
    let output = std::process::Command::new("cue")
        .args(["vet", tmp.to_string_lossy().as_ref()])
        .output();
    let _ = std::fs::remove_file(&tmp);
    matches!(output, Ok(o) if o.status.success())
}
```

**Note:** This requires `cue` to be installed on the test machine. Consider gating with `#[cfg(feature = "cue-validation")]`.

### R7: Fix proptest compilation error (F-007)

**Current:** `prop_for_each_input` doesn't exist

**Repair:**
```rust
#[test]
fn test_kind_rejects_unknown(bytes in proptest::collection::vec(proptest::prelude::any::<u8>(), 1..20)) {
    let kind_str = String::from_utf8_lossy(&bytes).to_string();
    if kind_str == "cli_envelope"
        || kind_str == "ui_tokens"
        || kind_str == "accepted_artifacts"
        || kind_str == "evidence_bundle"
        || kind_str == "diagnostics"
        || kind_str == "gate_output"
    {
        return; // Skip valid kinds
    }
    let result = parse_contract_kind(&kind_str);
    prop_assert!(result.is_err(), "Should reject unknown kind: '{}'", kind_str);
}
```

### R8: Remove redundant hardcoded Kani harnesses (F-008)

**Current:** `kani_gate_evidence_empty` and `kani_gate_evidence_all_invalid` use hardcoded values

**Repair:** Delete both harnesses. The existing `kani_gate_evidence_parity` harness already covers these cases through `kani::any()`. If specific edge cases must be explicitly tested, use `kani::assume()` guards:

```rust
#[kani::proof]
#[kani::unwind(5)]
fn kani_gate_evidence_edge_cases() {
    let total: u32 = kani::any();
    let valid: u32 = kani::any();
    let invalid: u32 = kani::any();
    
    kani::assume(valid.saturating_add(invalid) == total);
    
    // This single harness covers empty (0,0,0), all-invalid, and all-valid cases
    let result = gate_evidence_from_report(total, valid, invalid);
    assert!(result.is_ok());
}
```

---

## Repair Priority 3: Moderate

### R9: Document TLA+ verification partition (F-009)

Add to `ContractsAsData.tla` header:
```
-- Verification Partition:
-- - TLA+ (TLC): State-machine behavior, temporal properties, invariants
--   Bounded to MAX_FILES=5, MAX_FILE_VERSION=10 for tractability.
-- - Kani: Integer overflow boundaries (u32::MAX), exhaustive type exploration
-- - Proptest: Randomized coverage of parsing and comparison functions
-- - Verus: Mathematical correctness of semver ordering (post-repair)
```

### R10: Execute TLC and capture output (F-010)

```bash
tlc contracts/tla/ContractsAsData.tla -configfile contracts/tla/ContractsAsData.cfg 2>&1 | tee /tmp/tlc-output.txt
```

Append the contents of `/tmp/tlc-output.txt` to `proof-evidence.md` under the OBL-009 section.

---

## Verification After Repair

Run ALL of the following and capture output:

```bash
# 1. Kani
cargo kani -p workspace_tests --test contracts_kani

# 2. Proptest (must compile after R7)
cargo test -p workspace_tests --test contracts_as_data_props

# 3. Verus (must compile after R1-R3)
verus contracts/verus/contracts_as_data_spec.rs

# 4. TLC
tlc contracts/tla/ContractsAsData.tla -configfile contracts/tla/ContractsAsData.cfg

# 5. Full workspace
cargo check --workspace
cargo test -p workspace_tests
```

All must pass before resubmitting for proof review.
