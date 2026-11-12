# Error Taxonomy — vb-xi2f.33: Digest Covers Ask Semantics

## Error Domain Context

The `canonical_digest` and `digest_step_primitive` functions are currently infallible (return `WorkflowDigest`, not `Result`). The bugs in this bead are NOT runtime errors but **design errors**: the production of incorrect digests that violate the semantic-integrity contract.

This taxonomy covers:
1. Current incorrect behavior (design errors / missing logic)
2. Potential future error conditions if the digest computation is hardened
3. Downstream errors caused by incorrect digests

## Semantic Error Categories

### E-SEM-001: Insufficient Semantic Coverage (CURRENT BUG)
- **Severity**: HIGH
- **Classification**: Design error / missing logic
- **Location**: `digest_step_primitive` in `part_05.rs` and `compile/mod.rs`
- **Description**: The Ask primitive's catch-all arm contributes only `canonical_primitive_name(Ask) = b"ask"` to the digest. The `prompt` and `timeout` fields are silently ignored.
- **Consequence**: Two workflows differing only in ask prompt or timeout produce identical canonical digests.
- **Detection**: NOT detected at compile time. NOT detected at runtime. Requires test/proof to surface.
- **Recovery**: Fix the `digest_step_primitive` match arm. No runtime recovery possible for already-compiled workflows with incorrect digests.

### E-SEM-002: Duplicate Implementation Drift
- **Severity**: MEDIUM
- **Classification**: Design error / maintenance hazard
- **Location**: `canonical_digest` + `digest_step_primitive` duplicated in `part_05.rs` (active path) and `compile/mod.rs` (legacy path)
- **Description**: Two copies of the same digest logic exist. Both have the same bug. Any future fix to one copy risks the other diverging.
- **Consequence**: If only one copy is fixed, active-path and legacy-path compilations produce different digests for the same source.
- **Detection**: Test coverage comparing digests from both paths on the same source.
- **Recovery**: Apply fix to both copies, or refactor to a single shared implementation.

### E-SEM-003: Workflow Substitution (Downstream Impact)
- **Severity**: HIGH (security)
- **Classification**: Runtime hazard caused by E-SEM-001
- **Description**: A compiled workflow's digest does not change when ask prompt/timeout changes. An attacker or misconfiguration can change a user-facing prompt with no digest mismatch detection.
- **Consequence**: At admission time, the digest check passes despite the ask content being different. At idempotency check time, an ask with a different prompt is treated as "same workflow."
- **Detection**: Compile-time: digest comparison tests. Runtime: admission/idempotency digest check (but currently passes incorrectly).
- **Recovery**: Recompile affected workflows after fixing `canonical_digest`.

### E-SEM-004: Empty Prompt Ambiguity
- **Severity**: LOW (when E-SEM-001 is also present; MEDIUM once E-SEM-001 is fixed)
- **Classification**: Edge-case handling
- **Description**: An ask with `prompt = ""` (empty string) must produce a distinct digest from all non-empty prompts. The fix must ensure `"".as_bytes()` (zero-length) contributes to the hash correctly.
- **Consequence**: If empty prompt is hashed as the same as a missing prompt, an empty-prompt workflow and a workflow where prompt was somehow absent could collide.
- **Detection**: Unit test and Kani harness covering `prompt = ""`.
- **Recovery**: Ensure `hasher.update(b"")` (zero bytes) is a valid and distinct contribution to the hash.

### E-SEM-005: None vs Some("") Timeout Collision
- **Severity**: MEDIUM (once E-SEM-001 is fixed)
- **Classification**: Ambiguity hazard
- **Description**: `timeout: None` (no timeout) and `timeout: Some("")` (empty expression string) are semantically different but could hash to the same bytes if the sentinel is not chosen carefully. Using a sentinel like `b""` for `None` would collide with `Some("")`.
- **Consequence**: A workflow with no timeout and a workflow with an empty timeout expression produce identical digests.
- **Detection**: Unit test and Kani harness covering both `None` and `Some("")`.
- **Recovery**: Use distinct sentinels: `b"no_timeout"` for `None`, `b"timeout"` followed by the expression bytes for `Some`.

## Compiler Error Categories (Pre-existing, not introduced by this bead)

These errors are already part of the compiler pipeline and are not affected by the digest fix:

| Error | Location | Digest Impact |
|-------|----------|---------------|
| `CompileError::UnsupportedStepPrimitive` | `compile/mod.rs` line 85-92 | Legacy path rejects Ask entirely before digest is computed |
| `CompileError::StepFieldShape` | `parse_ask` validation | Ask parsing fails before digest sees the primitive |
| `CompileErrors` (aggregate) | Various | Compilation fails before digest is embedded |

## Error Recovery Strategy

| Error | Prevention | Detection | Mitigation |
|-------|-----------|-----------|------------|
| E-SEM-001 | Code fix: add Ask arm | Kani harness + unit/property tests | Recompile affected workflows |
| E-SEM-002 | Unify to single impl OR parity tests | Tests comparing both paths | Apply fix to both paths |
| E-SEM-003 | Fix E-SEM-001 | Admission check (post-fix) | Recompile + redeploy |
| E-SEM-004 | Kani: empty prompt edge case | Kani harness | Code fix ensures `b""` is valid hash input |
| E-SEM-005 | Kani: None vs Some("") | Kani harness | Code fix uses distinct sentinels |

## Railway-Oriented Error Flow

The current digest computation is not railway-oriented (returns value, not Result). If hardened:

```
YAML Source
    │
    ▼
Parse YAML ──── Fail ──▶ CompileError (already in place)
    │
    ▼
canonical_digest() ──▶ WorkflowDigest (infallible currently)
    │
    ▼
Embed in WorkflowParts
    │
    ▼
Validate parts ──── Fail ──▶ CompileError (already in place)
    │
    ▼
CompiledWorkflow
```
