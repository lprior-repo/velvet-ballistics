# Trusted Base Plan — Digest Coverage of `for_each` Semantics

**Bead:** vb-xi2f.28
**State:** 4 (proof-planner)
**Date:** 2026-05-25
**Status:** PLANNED

---

## 1. Trusted Base Overview

The digest computation in this bead relies on a small, well-defined set of trusted components. Most are standard Rust ecosystem libraries with no unsafe code and well-understood semantics. The narrow scope of the bead (two functions, one primitive variant) limits the trusted base surface.

## 2. Trusted Base Ledger Entries

| ID | Obligation | Artifact | Location | Marker | Trust Kind | Reason | Scope | Impact | Behavior-Affecting | Compensating Evidence | Owner | Expiry |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| TBD-FE-01 | PO-K-FE-01 through PO-K-FE-05, PO-K-FE-07, PO-K-FE-09 | `blake3::Hasher` | `Cargo.lock` blake3 1.x | `external_body` | external_library | blake3 is a widely-audited cryptographic hash library. Its update/finalize are deterministic pure functions. No unsafe code path in the hashing layer. | All obligations that hash through blake3 | If blake3::Hasher were non-deterministic, digest determinism would be impossible. If update() dropped bytes, field hashing would be silently ignored. | true | blake3's Rust implementation uses safe code only. The library has been audited by the Rust Crypto project. BLAKE3 is a NIST-standardized hash function (RFC). Determinism is a documented property. | proof-planner | 2027-05-25 |
| TBD-FE-02 | PO-K-FE-01 through PO-K-FE-05 | `WorkflowDigest::from_bytes` | `crates/vb_core/src/ids/mod.rs` | `trusted` | domain_type | WorkflowDigest is a #[repr(transparent)] newtype over [u8; 32]. from_bytes() is an infallible constructor with no logic beyond wrapping. No byte manipulation, no validation, no filtering. | All obligations that produce or compare WorkflowDigest | If from_bytes() non-deterministically modified bytes, digest equality would be unreliable. Impact: HIGH — but the code is trivially correct (1-line body). | true | Unit tests in vb_core::ids::mod.rs verify from_bytes/as_bytes roundtrip and PartialEq behavior. The type is exercised by dozens of existing compilation and storage tests. | proof-planner | 2027-05-25 |
| TBD-FE-03 | PO-K-FE-02 | `u32::to_le_bytes` | `core::u32` | `trusted` | language_primitive | Rust's u32::to_le_bytes is a language primitive with specified behavior: produces 4 bytes in little-endian order. Deterministic on all platforms regardless of native endianness. | at_once canonical representation (at_once.unwrap_or(1).to_le_bytes()) | If to_le_bytes() were non-deterministic or platform-dependent, the digest would not be reproducible. Impact: MEDIUM — but u32::to_le_bytes is a core primitive with a long track record. | true | Rust standard library specification guarantees endian-independent output. The function has been exhaustively tested across all Rust target platforms. | proof-planner | 2027-05-25 |
| TBD-FE-04 | PO-K-FE-04 | Recursion termination in `digest_step_primitive` | `crates/vb_compile/src/mod_compile_lowering/part_05.rs:140` | `model_bound` | structural_guard | ForEach body steps call digest_step_primitive recursively. Termination is guaranteed by the AST structure: body steps are sub-nodes of the parent ForEach, and YAML nesting depth is finite. No cycles possible (no cyclic references in AST). | Body step recursive hashing | If recursion could diverge (infinite loop), the hasher would never finalize. Impact: HIGH — but AST structure guarantees finiteness. | true | YAML AST is a tree, not a graph. No back-references or cycles exist in StepAst. The recursion depth is bounded by YAML nesting, limited by practical human-authored workflows (typically < 10 levels). | proof-planner | 2027-05-25 |
| TBD-FE-05 | All Kani obligations | Kani harness must use `kani::Arbitrary` not hardcoded shapes | `crates/vb_compile/src/kani_proofs/*.rs` | `assume` | tool_requirement | Per GOD RULE #1: Kani harnesses MUST use kani::Arbitrary for core structures, not hardcoded dummy data. If the proof-writer uses hardcoded inputs, the proof is vacuous. | All Kani obligations | If Kani harnesses use hardcoded ForEach variants instead of arbitrary input generation, the verification proves nothing about the implementation under diverse inputs. | true | Proof-plan-reviewer must verify that proof-writer implements kani::Arbitrary for StepPrimitive::ForEach (or uses kani::any() with assume constraints). Proof-reviewer must reject vacuous harnesses. | proof-planner | 2027-05-25 (must be resolved by State 6 review) |

## 3. Trusted Base Summary

| Trust Level | Count | Items |
|---|---|---|
| Trusted (external) | 2 | blake3::Hasher, u32::to_le_bytes |
| Trusted (domain) | 1 | WorkflowDigest::from_bytes |
| Structural Guard | 1 | Recursion termination |
| Tool Requirement | 1 | Kani Arbitrary mandate |

## 4. Trusted Base Risks

### 4.1 blake3::Hasher as External Trust

**Risk Level:** LOW

blake3 is a mature, audited Rust crate with no unsafe hashing code. The library is widely deployed and its determinism is a core property. If blake3 were non-deterministic, it would be widely known and reported. No compensating action needed beyond noting the dependency.

### 4.2 WorkflowDigest as Domain Trust

**Risk Level:** NEGLIGIBLE

WorkflowDigest is a 1-line newtype constructor. Its correctness is trivially verifiable by inspection. The extensive test suite in vb_core provides behavioral validation.

### 4.3 Recursion Termination

**Risk Level:** LOW

YAML AST trees are finite by construction. A hostile YAML with extreme nesting depth could cause stack overflow, but this is a DDoS-level concern, not a correctness concern. The bounded Kani harnesses verify behavior within practical recursion bounds.

### 4.4 Kani Arbitrary Requirement

**Risk Level:** HIGH if violated

This is the most important trusted base assumption: that the proof-writer will implement kani::Arbitrary for ForEach structures rather than hardcoding. If violated, all Kani obligations are vacuous. The proof-plan-reviewer and proof-reviewer must enforce this requirement.

## 5. Non-Trusted Components (Verified by Obligations)

| Component | Verified By | Reason |
|---|---|---|
| `digest_step_primitive` ForEach arm | PO-K-FE-01 through PO-K-FE-04, PO-K-FE-09, PO-K-FE-10 | Bounded Kani proof of field hashing |
| `canonical_digest` determinism | PO-K-FE-05, PO-P-FE-05 | Kani + proptest |
| Dual-path equivalence | PO-P-FE-06 | Proptest cross-path comparison |
| Non-regression Set/Finish | PO-P-FE-08 | Proptest regression guard |
| at_once canonical representation | PO-K-FE-07 | Kani equivalence proof |
| Field delimiter safety | PO-K-FE-10 | Kani byte-level proof |

All behavior-affecting components are verified by at least one proof obligation. The trusted base is limited to standard library primitives and a widely-audited external crate.
