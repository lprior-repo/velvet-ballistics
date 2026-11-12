# Hazard Analysis — Digest Coverage of `for_each` Semantics

**Bead:** vb-xi2f.28  
**State:** 3 (rust-contract)  
**Date:** 2026-05-25  
**Status:** DRAFT

---

## 1. Temporal Hazards

| ID | Hazard | Severity | Probability | Impact |
|---|---|---|---|---|
| **HZ-T01** | Concurrent compilation of same source across two processes produces different digests | N/A — not a concern | `canonical_digest` is a pure function; no shared state exists | Zero |
| **HZ-T02** | Digest computed before and after a source modification produces same digest (coverage gap) | **CRITICAL** | **CERTAIN** (current state) | Storage accepts semantically different workflow as "same" |
| **HZ-T03** | Digest staleness: fix is applied to one copy but not the other; one path produces new correct digest while the other produces old incomplete digest | **HIGH** | **MEDIUM** (risk during fix) | Cross-path digest divergence; admission may reject valid workflow |

---

## 2. Rust-Core Invariant Hazards

| ID | Hazard | Severity | Description |
|---|---|---|---|
| **HZ-I01** | Non-exhaustive match: `digest_step_primitive` uses `other => { hasher.update(name) }` catch-all pattern, silently skipping field hashing for all non-Set/non-Finish primitives | **CRITICAL** | The `other` arm is a **generalized digest coverage gap**. Adding new primitives in the future will NOT trigger a compiler error to remind developers to add field hashing. |
| **HZ-I02** | Silent field addition: adding a new field to `StepPrimitive::ForEach` variant (e.g., `done_target: Option<String>`) will NOT be caught by the compiler — the new field will be silently ignored by `other =>` arm if it falls through | **HIGH** | The compiler does not warn about unused struct fields in enum variants when destructured via catch-all. |
| **HZ-I03** | Body recursion depth: deeply nested ForEach bodies (ForEach inside ForEach body) could cause stack overflow during recursive `digest_step_primitive` calls | **LOW** | Recursion depth is bounded by YAML nesting, which is human-authored. Still, a hostile YAML with 1000+ nested ForEach bodies could overflow. |
| **HZ-I04** | Body step ID collision: two body steps with the same `id` produce identical hash input sequences, potentially masking body content differences | **LOW** | Step IDs are YAML-specified and uniqueness is enforced elsewhere (validation). Redundant but not critical. |

---

## 3. Bounded State Hazards

| ID | Hazard | Severity | Description |
|---|---|---|---|
| **HZ-B01** | Integer overflow in `at_once` canonical representation: `at_once == Some(u32::MAX)` is hashed as `[255,255,255,255]` (LE). No arithmetic overflow possible since hashing feeds raw bytes. | N/A | Not a hazard — no arithmetic on at_once during hashing. |
| **HZ-B02** | Very large body: ForEach.body with thousands of steps increases digest computation time linearly. Bounded by YAML source size (practically <100 steps). | **LOW** | Digest computation is O(steps) and infrequent (compile-time only). |
| **HZ-B03** | Empty body normalization: `ForEach { body: [] }` produces hash `b"body:"` with no following content. Is this distinguishable from a ForEach variant that has no body field? | **LOW** | ForEach always has a body field in the AST; an empty body is a legal-but-unusual case. |

---

## 4. Refinement Hazards

| ID | Hazard | Severity | Description |
|---|---|---|---|
| **HZ-R01** | `at_once::None` and `at_once::Some(0)` produce identical canonical hash input (`[0,0,0,0]`). Is `Some(0)` a valid value? | **MEDIUM** | If `Some(0)` is rejected by validation before hashing, this refinement is irrelevant. If `Some(0)` reaches `canonical_digest`, it creates an alias with `None`. Needs validation-path analysis. |
| **HZ-R02** | Variable name normalization: should `"  item  "` (with whitespace) be trimmed/normalized before hashing? Current approach hashes raw bytes — leading/trailing whitespace is significant. | **LOW** | YAML parser likely normalizes field names; raw bytes reflect parsed value. |

---

## 5. Concurrency Hazards

| ID | Hazard | Severity | Description |
|---|---|---|---|
| **HZ-C01** | Shared `blake3::Hasher` across threads: `canonical_digest` takes `&WorkflowSource` (immutable) and creates a local `Hasher`. No shared state. | N/A | Not a hazard. |
| **HZ-C02** | Parallel compilation: two threads compiling different workflows independently. No shared state, no locks, no contention. | N/A | Not a hazard. |

---

## 6. Unsafe / Provenance Hazards

| ID | Hazard | Severity | Description |
|---|---|---|---|
| **HZ-U01** | Raw pointer provenance in `blake3::Hasher::update`: feeds `&[u8]` slices. Safe Rust; no provenance issues. | N/A | blake3 crate is safe Rust. |
| **HZ-U02** | `WorkflowDigest` transmutation: `#[repr(transparent)]` over `[u8; 32]`. No unsafe transmutation needed. | N/A | Safe by construction. |

---

## 7. Hostile Input Hazards

| ID | Hazard | Severity | Description |
|---|---|---|---|
| **HZ-H01** | Maliciously crafted ForEach.body with self-referential step IDs: if body steps reference each other by ID in a way that could cause infinite recursion during hashing | N/A — not applicable | `digest_step_primitive` only hashes field *content* (strings, numbers); it does not follow step references or resolve identifiers. No recursion through references possible. |
| **HZ-H02** | Extremely long field values (megabyte-length variable names or input expressions) increase hash computation time. | **LOW** | BLAKE3 is fast; YAML sources are bounded in practice. Hash computation is linear in input size. |
| **HZ-H03** | Unicode normalization: variable names with composed/decomposed Unicode (e.g., `"café"` vs `"cafe\u{0301}"`) produce different hash input bytes. | **LOW** | Standards-compliant. NFD/NFC normalization is YAML-level concern, not digest-level. |

---

## 8. Performance Hazards

| ID | Hazard | Severity | Description |
|---|---|---|---|
| **HZ-P01** | Recursive body hashing: `digest_step_primitive` recursively calls itself for ForEach.body steps. Each body step triggers its own `digest_step_primitive` dispatch. | **LOW** | O(total steps) amortized cost. Compile-time only. |
| **HZ-P02** | Duplicate hashing: since both `compile/mod.rs` and `part_05.rs` have identical logic, the same source is hashed twice if both paths are exercised. | **LOW** | Only one path is used per compilation; they are alternatives, not sequential. |

---

## 9. Release / API Hazards

| ID | Hazard | Severity | Description |
|---|---|---|---|
| **HZ-A01** | Digest format change: after the fix, previously-compiled workflows will have different digests from newly-compiled versions of the same source. | **HIGH** | This is a **breaking change** for any system that compares digests across compilation boundaries. Stored digests from pre-fix compilations will not match post-fix digests. |
| **HZ-A02** | Silent acceptance regression: storage admission tests that compare digests across compilation runs may fail if test fixtures were compiled pre-fix. | **MEDIUM** | Test fixtures may need regeneration. |
| **HZ-A03** | Migration path: no version field in `WorkflowParts` distinguishes pre-fix from post-fix digests. Both are opaque 32-byte arrays. | **LOW** | The digest is a content hash; changing its computation is semantically equivalent to "changing the hash function." Consumers treat it as opaque. |

---

## 10. Hazard Severity Summary

| Severity | Count | Hazards |
|---|---|---|
| **CRITICAL** | 2 | HZ-T02 (certain digest aliasing), HZ-I01 (non-exhaustive catch-all) |
| **HIGH** | 3 | HZ-T03 (fix asymmetry), HZ-I02 (silent field addition), HZ-A01 (breaking digest change) |
| **MEDIUM** | 2 | HZ-R01 (None vs Some(0) alias), HZ-A02 (test fixture breakage) |
| **LOW** | 5 | HZ-I03, HZ-I04, HZ-B02, HZ-B03, HZ-H02, HZ-H03, HZ-P01, HZ-P02 |
| **N/A** | 5 | HZ-T01, HZ-B01, HZ-C01, HZ-C02, HZ-U01, HZ-U02, HZ-H01 |
