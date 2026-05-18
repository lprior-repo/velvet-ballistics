# Test Plan: MAJOR-6 — SideEffect/RetrySafety Enum Mismatch (Section 65)

## Summary

- **Bead**: MAJOR-6
- **Problem**: Section 65 `SideEffect` and `RetrySafety` enums in the master plan DO NOT MATCH the actual implementation in `vb_core/src/action.rs`.
- **Behaviors identified**: 2 (SideEffect parity, RetrySafety parity)
- **Trophy allocation**: 2 unit (100% unit — pure enum comparison, no integration needed)
- **Proptest invariants**: 0 (enum comparison, no property space)
- **Fuzz targets**: 0 (no parsing boundary for this issue)
- **Kani harnesses**: 0 (enum exhaustiveness proven by compiler)
- **Mutation checkpoints**: 2

---

## 1. Behavior Inventory

1. **SideEffect enum matches master plan** — The `SideEffect` enum in `vb_core/src/action.rs` has exactly the same variants as Section 65 of the master plan.
2. **RetrySafety enum matches master plan** — The `RetrySafety` enum in `vb_core/src/action.rs` has exactly the same variants as Section 65 of the master plan.

---

## 2. Trophy Allocation

| Behavior | Layer | Rationale |
|----------|-------|-----------|
| SideEffect variants match master plan | Unit | Pure enum comparison; no I/O, no state |
| RetrySafety variants match master plan | Unit | Pure enum comparison; no I/O, no state |

**Rationale for 100% unit**: This is a static contract comparison between documentation (master plan) and code (implementation). No runtime behavior, no integration paths, no E2E surface. Unit tests exercising the enum constructors and pattern matches are sufficient.

---

## 3. BDD Scenarios

### Behavior: SideEffect enum matches master plan

**Scenario: side_effect_variants_match_section_65**
```
Given: Master plan Section 65 defines SideEffect with 7 variants: Pure, LocalRead, LocalWrite, ExternalRead, ExternalWrite, Process, UnsafeShell
When: I enumerate all variants of vb_core::action::SideEffect
Then: The enum has exactly 7 variants
And: Each variant name matches the master plan spelling exactly
And: Each variant has a distinct #[repr(u8)] discriminant
```

**Scenario: side_effect_none_variant_matches**
```
Given: Master plan Section 65 defines SideEffect::None (no observable side effects)
When: I inspect the SideEffect::None variant in vb_core
Then: The variant exists with the exact name "None"
And: The discriminant is a unique u8 value
```

**Scenario: side_effect_writes_variant_matches**
```
Given: Master plan Section 65 defines SideEffect::Writes (writes local state)
When: I inspect the SideEffect::Writes variant in vb_core
Then: The variant exists with the exact name "Writes"
And: The discriminant is a unique u8 value distinct from all other variants
```

**Scenario: side_effect_sends_variant_matches**
```
Given: Master plan Section 65 defines SideEffect::Sends (sends a message/notification)
When: I inspect the SideEffect::Sends variant in vb_core
Then: The variant exists with the exact name "Sends"
And: The discriminant is a unique u8 value distinct from all other variants
```

**Scenario: side_effect_creates_variant_matches**
```
Given: Master plan Section 65 defines SideEffect::Creates (creates/provisions a resource)
When: I inspect the SideEffect::Creates variant in vb_core
Then: The variant exists with the exact name "Creates"
And: The discriminant is a unique u8 value distinct from all other variants
```

**Scenario: side_effect_destroys_variant_matches**
```
Given: Master plan Section 65 defines SideEffect::Destroys (destroys/deprovisions a resource)
When: I inspect the SideEffect::Destroys variant in vb_core
Then: The variant exists with the exact name "Destroys"
And: The discriminant is a unique u8 value distinct from all other variants
```

**Scenario: side_effect_exhaustive_match_against_master_plan**
```
Given: Master plan Section 65 defines SideEffect::Pure, LocalRead, LocalWrite, ExternalRead, ExternalWrite, Process, UnsafeShell
When: I compare the full set of master plan variants against vb_core::action::SideEffect
Then: All 7 master plan variants are present in the implementation
And: No additional variants exist beyond the master plan set
And: The match in vb_core::action::verify_idempotency is exhaustive for all 7 variants
```

---

### Behavior: RetrySafety enum matches master plan

**Scenario: retry_safety_variants_match_section_65**
```
Given: Master plan Section 65 defines RetrySafety with 4 variants: Idempotent, RequiresIdempotencyKey, NotRetrySafe, Unknown
When: I enumerate all variants of vb_core::action::RetrySafety
Then: The enum has exactly 4 variants
And: Each variant name matches the master plan spelling exactly
And: Each variant has a distinct #[repr(u8)] discriminant
```

**Scenario: retry_safety_idempotent_variant_matches**
```
Given: Master plan Section 65 defines RetrySafety::Idempotent (safe to retry unconditionally)
When: I inspect the RetrySafety::Idempotent variant in vb_core
Then: The variant exists with the exact name "Idempotent"
And: The discriminant is a unique u8 value
```

**Scenario: retry_safety_requires_idempotency_key_variant_matches**
```
Given: Master plan Section 65 defines RetrySafety::RequiresIdempotencyKey (safe with valid key)
When: I inspect the RetrySafety::RequiresIdempotencyKey variant in vb_core
Then: The variant exists with the exact name "RequiresIdempotencyKey"
And: The discriminant is a unique u8 value distinct from all other variants
```

**Scenario: retry_safety_not_retry_safe_variant_matches**
```
Given: Master plan Section 65 defines RetrySafety::NotRetrySafe (retry rejected by default)
When: I inspect the RetrySafety::NotRetrySafe variant in vb_core
Then: The variant exists with the exact name "NotRetrySafe"
And: The discriminant is a unique u8 value distinct from all other variants
```

**Scenario: retry_safety_unknown_variant_matches**
```
Given: Master plan Section 65 defines RetrySafety::Unknown (retry rejected)
When: I inspect the RetrySafety::Unknown variant in vb_core
Then: The variant exists with the exact name "Unknown"
And: The discriminant is a unique u8 value distinct from all other variants
```

**Scenario: retry_safety_exhaustive_match_against_master_plan**
```
Given: Master plan Section 65 defines RetrySafety::Idempotent, RequiresIdempotencyKey, NotRetrySafe, Unknown
When: I compare the full set of master plan variants against vb_core::action::RetrySafety
Then: All 4 master plan variants are present in the implementation
And: No additional variants exist beyond the master plan set
And: The match in vb_core::action::verify_idempotency is exhaustive for all 4 variants
```

---

## 4. Enum Comparison Matrix

### SideEffect

| Master Plan Section 65 Variant | Implementation Variant | Status |
|-------------------------------|------------------------|--------|
| `Pure` | MISSING | ❌ MISMATCH |
| `LocalRead` | MISSING | ❌ MISMATCH |
| `LocalWrite` | MISSING | ❌ MISMATCH |
| `ExternalRead` | MISSING | ❌ MISMATCH |
| `ExternalWrite` | MISSING | ❌ MISMATCH |
| `Process` | MISSING | ❌ MISMATCH |
| `UnsafeShell` | MISSING | ❌ MISMATCH |
| — | `None` | ❌ EXTRA (not in master plan) |
| — | `Writes` | ❌ EXTRA (not in master plan) |
| — | `Sends` | ❌ EXTRA (not in master plan) |
| — | `Creates` | ❌ EXTRA (not in master plan) |
| — | `Destroys` | ❌ EXTRA (not in master plan) |

**Mismatch count**: 12 (7 missing + 5 extra)

### RetrySafety

| Master Plan Section 65 Variant | Implementation Variant | Status |
|-------------------------------|------------------------|--------|
| `Idempotent` | `Safe` | ❌ NAME MISMATCH (semantically similar) |
| `RequiresIdempotencyKey` | `KeyRequired` | ❌ NAME MISMATCH (semantically similar) |
| `NotRetrySafe` | MISSING | ❌ MISMATCH |
| `Unknown` | MISSING | ❌ MISMATCH |
| — | `Unsafe` | ❌ EXTRA (not in master plan, semantically different from NotRetrySafe) |

**Mismatch count**: 5 (2 name mismatches + 2 missing + 1 extra)

---

## 5. Gap Analysis

### SideEffect Gap (Implementation vs Master Plan)

The implementation has **5 variants** but master plan requires **7 variants**.

**Missing from implementation:**
1. `Pure` — pure computation, no side effects (maps to DeterministicPure)
2. `LocalRead` — reads local state only
3. `LocalWrite` — writes local state
4. `ExternalRead` — reads external state
5. `ExternalWrite` — writes external state (maps to AtLeastOnceExternal)
6. `Process` — spawns or manages a process
7. `UnsafeShell` — arbitrary shell execution

**Extra in implementation (not in master plan):**
- `None` — covered by `Pure` in master plan
- `Writes` — subsumed by `LocalWrite`/`ExternalWrite`
- `Sends` — no equivalent in master plan taxonomy
- `Creates` — subsumed by `Process`
- `Destroys` — subsumed by `Process` or separate destruction category

### RetrySafety Gap (Implementation vs Master Plan)

The implementation has **3 variants** but master plan requires **4 variants**.

**Missing from implementation:**
1. `NotRetrySafe` — retry rejected by default
2. `Unknown` — retry rejected

**Name mismatches (semantically similar but named differently):**
1. `Idempotent` (master plan) vs `Safe` (implementation)
2. `RequiresIdempotencyKey` (master plan) vs `KeyRequired` (implementation)

**Extra in implementation:**
- `Unsafe` — master plan has `NotRetrySafe` which may or may not be semantically equivalent to `Unsafe`

---

## 6. Proptest Invariants

Not applicable. This is a static enum comparison task, not a property-based test scenario.

---

## 7. Fuzz Targets

Not applicable. No parsing/deserialization boundary involved in enum definition comparison.

---

## 8. Kani Harnesses

Not applicable. Compiler-enforced enum exhaustiveness is sufficient for this comparison. Kani would not add value over compiler-level exhaustiveness checking.

---

## 9. Mutation Checkpoints

| Critical mutation | Must be caught by |
|-------------------|-------------------|
| SideEffect variant removed from match | `side_effect_variants_match_section_65` test |
| RetrySafety variant removed from match | `retry_safety_variants_match_section_65` test |
| New SideEffect variant added | `side_effect_exhaustive_match_against_master_plan` test |
| New RetrySafety variant added | `retry_safety_exhaustive_match_against_master_plan` test |

**Threshold**: 100% mutation kill rate for variant-add/remove mutations on these two enums.

---

## 10. Test Function Names

```rust
// SideEffect tests
fn side_effect_variants_match_section_65()
fn side_effect_none_variant_matches()
fn side_effect_writes_variant_matches()
fn side_effect_sends_variant_matches()
fn side_effect_creates_variant_matches()
fn side_effect_destroys_variant_matches()
fn side_effect_exhaustive_match_against_master_plan()

// RetrySafety tests
fn retry_safety_variants_match_section_65()
fn retry_safety_idempotent_variant_matches()
fn retry_safety_requires_idempotency_key_variant_matches()
fn retry_safety_not_retry_safe_variant_matches()
fn retry_safety_unknown_variant_matches()
fn retry_safety_exhaustive_match_against_master_plan()
```

---

## 11. Open Questions

1. **Naming authority**: Should the fix align the implementation to the master plan names (`Pure`, `LocalRead`, `LocalWrite`, `ExternalRead`, `ExternalWrite`, `Process`, `UnsafeShell`, `Idempotent`, `RequiresIdempotencyKey`, `NotRetrySafe`, `Unknown`), or should the master plan be updated to match the existing implementation (`None`, `Writes`, `Sends`, `Creates`, `Destroys`, `Safe`, `KeyRequired`, `Unsafe`)?

2. **Semantic equivalence**: Is `Unsafe` in the implementation semantically equivalent to `NotRetrySafe` in the master plan? The master plan has `Unknown` as a separate variant which does not exist in the implementation.

3. **SideEffect taxonomy**: The master plan's `SideEffect` taxonomy (Pure, LocalRead, LocalWrite, ExternalRead, ExternalWrite, Process, UnsafeShell) is more granular than the implementation's (None, Writes, Sends, Creates, Destroys). Should the implementation be expanded to match the master plan's 7-variant taxonomy, or should the master plan be simplified to match the 5-variant implementation?

4. **Test file location**: Should these tests live in `crates/vb_core/src/action.rs` (adjacent to the enums under test), or in `crates/workspace_tests/` (cross-crate integration tests)?

---

## 12. Acceptance Criteria

1. All 11 test functions compile and pass.
2. `SideEffect` enum in `vb_core/src/action.rs` has exactly 7 variants matching Section 65 master plan names.
3. `RetrySafety` enum in `vb_core/src/action.rs` has exactly 4 variants matching Section 65 master plan names.
4. All variants have distinct `#[repr(u8)]` discriminants.
5. All pattern matches on these enums in `vb_core` are exhaustive.
6. `verify_idempotency` function's match on `RetrySafety` is exhaustive for all 4 master plan variants.
7. No compiler warnings about unused variants.
