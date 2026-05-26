# Trusted Base Plan: Wait Digest Coverage

**Bead:** vb-xi2f.32
**Date:** 2026-05-24
**State:** proof-planner (State 4)
**Schema:** trusted-base-plan/v1

## 1. Trusted Components

The following components are assumed correct and are NOT re-verified by this bead's proof plan. They form the trusted computing base (TCB) for the proof obligations.

### TB-001: blake3 Cryptographic Hash Library

| Property | Details |
|----------|---------|
| **Component** | `blake3` crate (v1.x) — Rust implementation of BLAKE3 |
| **Trusted property** | Collision resistance, determinism, panic-freedom of `Hasher::update()` and `Hasher::finalize()` |
| **Why trusted** | Industry-standard cryptographic hash function. Audited reference implementation. Used by many Rust projects. |
| **Impact if broken** | All digest-based identity/integrity checks invalidated. This is a catastrophic system-wide failure, not specific to this bead. |
| **Boundary** | `blake3::Hasher::new()`, `hasher.update()`, `hasher.finalize()` |
| **Proof artifacts** | None planned. Out of scope for P1 bead. |

### TB-002: YAML Validation Gate

| Property | Details |
|----------|---------|
| **Component** | `validate_wait_shape()` in `crates/vb_compile/src/mod_compile_validation/part_03.rs:186` |
| **Trusted property** | Rejects illegal `Wait { event: None, timeout: None }` shape before compilation |
| **Why trusted** | `compile_source()` (part_01.rs) calls validation BEFORE `canonical_digest()` is computed. The digest function never encounters the illegal (None, None) state. |
| **Impact if broken** | `digest_step_primitive` would encounter (None, None) Wait — the panic would occur in lowering (lower_canonical_wait) before it reaches the digest. Defensive handling in digest would be good but is not required. |
| **Boundary** | `validate_canonical_compile_scope` → `validate_wait_shape` |
| **Proof artifacts** | None planned. Trusted as existing validated gate. Hazard RH-2 rates this LOW. |

### TB-003: Rust Standard Library String/&str

| Property | Details |
|----------|---------|
| **Component** | `std::string::String`, `str::as_bytes()`, `Option::as_deref()`, `Option::is_some()` |
| **Trusted property** | `as_bytes()` returns valid UTF-8 bytes; `as_deref()` correctly converts `Option<String>` → `Option<&str>`; `is_some()`/`is_none()` are correct |
| **Why trusted** | Rust standard library fundamentals. Panic-free for non-null pointers. |
| **Impact if broken** | Entire Rust ecosystem broken. Not specific to this bead. |
| **Boundary** | All standard library calls in the new Wait match arm |

### TB-004: WorkflowDigest Type

| Property | Details |
|----------|---------|
| **Component** | `WorkflowDigest([u8; 32])` in `crates/vb_core/src/ids/mod.rs:342` |
| **Trusted property** | `from_bytes()` constructor is correct; `Eq`/`Hash`/`Copy` derive macros are correct |
| **Why trusted** | Simple newtype wrapper. No logic beyond standard derive macros. |
| **Impact if broken** | Digest comparison/identity would fail. |
| **Boundary** | `WorkflowDigest::from_bytes(hasher.finalize().into())` |

### TB-005: YAML Parser (vb_yaml)

| Property | Details |
|----------|---------|
| **Component** | `vb_yaml::ast::StepPrimitive::Wait { event: Option<String>, timeout: Option<String> }` |
| **Trusted property** | Correctly parses YAML into the AST type. `event` and `timeout` fields contain the correct slot expression text. |
| **Why trusted** | Existing, tested parser. Not modified by this bead. |
| **Impact if broken** | Wrong field values in AST → wrong digests. Affects all compilation, not just this bead. |
| **Boundary** | YAML bytes → `StepPrimitive::Wait` via `parse_workflow_source` |

## 2. Assumed Bounds

| Bound | Value | Rationale |
|-------|-------|-----------|
| Slot expression text length | ≤ 16 chars (Kani), ≤ 255 chars (proptest/fuzz) | Real slot text is typically 1-7 chars (`"0"`, `"255"`, `"slot_0"`). 16 chars is an honest Kani bound. |
| Step count | ≤ 256 (existing validation bound) | Workflow steps are bounded by validation. |
| Event string alphabet | a-zA-Z0-9_ (proptest), a-z (Kani) | Real slot expressions use these characters. |
| Sentinel value | Fixed constant `b"none"` | Must not change without updating proof harnesses. |

## 3. Trusted Harness Artifacts (Proof-Writer Responsibility)

The proof-writer must create or reuse these harnesses. The planner assumes they are constructed correctly:

### TH-001: Kani Arbitrary for Wait field values
- **What:** `kani::Arbitrary` impl for `StepPrimitive::Wait` (or a test-only WaitFields type) using `kani::any::<u8>()` to generate bounded-length strings.
- **Constraint:** Never generate `(event=None, timeout=None)`. Use `kani::assume()` to filter.
- **Violation of GOD RULE 1 if:** harness uses hardcoded dummy Wait structs instead of `kani::Arbitrary` or generator harnesses.

### TH-002: Cross-path test fixture
- **What:** A test that calls both `compile_source()` (cold-path) and `compile_workflow()` (warm-path) with the same workflow source YAML and asserts digest equality.
- **Constraint:** Must test WaitYAML, not just non-Wait primitives.

### TH-003: Fuzz target Arbitrary
- **What:** `libfuzzer_sys::arbitrary::Arbitrary` impl for workflow source containing Wait steps.
- **Constraint:** Must generate valid YAML (valid wait shapes, non-empty steps). Use `Arbitrary` trait, not manual byte manipulation.

## 4. Untrusted / Guarded Elements

| Element | Risk | Guard Strategy |
|---------|------|---------------|
| New Wait match arm in `digest_step_primitive` (both copies) | Panic, collision, divergence | Kani (PO-001, PO-005, PO-010, PO-013, PO-015), proptest (PO-002, PO-004, PO-006, PO-008, PO-009, PO-011, PO-016), fuzz (PO-003, PO-007, PO-012) |
| Discriminator constants `b"wait_until"` / `b"wait_event"` | Typo, mismatch with WaitKind | proptest PO-004, Kani PO-005 |
| Sentinel `b"none"` | Ambiguity with real slot text `"none"` | proptest PO-006, fuzz PO-007 |
| Hash update ordering | Order-dependent collision | proptest PO-002, PO-011 — implicitly tested by sensitivity tests |

## 5. Extensions Required (Not Owned by Planner)

| Extension | Owner | Description |
|-----------|-------|-------------|
| `kani::Arbitrary` for `StepPrimitive::Wait` | proof-writer | Generate bounded, non-empty Wait configurations |
| `Arbitrary` for Wait YAML workflows | proof-writer | Generate valid YAML byte sequences with Wait steps for fuzzing |
| Cross-path test scaffolding | proof-writer or test-writer | Set up both compiler paths for comparison |
| proptest strategy for Wait field variants | proof-writer or test-writer | Extend `primitive_case_strategy()` or add new strategy for Wait-only tests |
