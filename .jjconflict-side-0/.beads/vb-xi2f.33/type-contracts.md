# Type Contracts — vb-xi2f.33: Digest Covers Ask Semantics

## Existing Type Audit

### StepPrimitive::Ask (vb_yaml::ast::types, lines 244-250)

```rust
// Current definition:
Ask {
    prompt: String,           // Required, parsed by parse_ask() as required string
    timeout: Option<String>,  // Optional, parsed from YAML string or absent
}
```

**Status**: The types themselves are adequate. `prompt` is a required `String`, `timeout` is an optional `String`. Both are parsed at the YAML boundary and cannot be absent/malformed when this variant is constructed.

**No type-level change required**: The bug is NOT in the type definition but in the `digest_step_primitive` function that fails to hash these fields.

### WorkflowDigest (vb_core::ids, lines 339-356)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct WorkflowDigest([u8; 32]);
```

**Status**: Correct value-object design. A transparent wrapper with `from_bytes` constructor and `as_bytes` accessor. No validation — raw bytes are accepted.

**No type-level change required**. The fix is in how bytes enter this type, not in the type itself.

### canonical_digest signature (part_05.rs, lines 116-138 and compile/mod.rs, lines 220-241)

```rust
pub(super) fn canonical_digest(source: &vb_yaml::ast::WorkflowSource) -> WorkflowDigest
```

**Status**: Signature is correct — takes `&WorkflowSource`, returns `WorkflowDigest`. Pure function at the type level. The bug is in the body.

**Fix needed**: The body must be updated to pass ask fields to the hasher.

### digest_step_primitive signature (part_05.rs, lines 140-162 and compile/mod.rs, lines 243-261)

```rust
pub(super) fn digest_step_primitive(
    hasher: &mut blake3::Hasher,
    primitive: &vb_yaml::ast::StepPrimitive,
)
```

**Status**: Signature is correct — takes mutable reference to hasher and reference to primitive. The match arms are inadequate for Ask.

**Fix needed**: Add an explicit `Ask { prompt, timeout }` arm that hashes the fields.

## Type-Level Contract for digest_step_primitive

The function MUST transition from the current catch-all arm:

```rust
// CURRENT (BUGGY) — in both part_05.rs and compile/mod.rs
other => {
    hasher.update(canonical_primitive_name(other).as_bytes());
}
```

To an explicit arm for Ask:

```rust
// REQUIRED CONTRACT
vb_yaml::ast::StepPrimitive::Ask { prompt, timeout } => {
    hasher.update(b"ask");
    hasher.update(prompt.as_bytes());
    match timeout {
        Some(t) => {
            hasher.update(b"timeout");
            hasher.update(t.as_bytes());
        }
        None => {
            hasher.update(b"no_timeout");
        }
    }
}
```

## Contract Clauses

### TC-001: Explicit Ask Arm
**Rule**: `digest_step_primitive` MUST have an explicit `Ask { prompt, timeout }` match arm that hashes both `prompt` and `timeout` fields. The catch-all arm is insufficient.

### TC-002: Deterministic Hashing
**Rule**: The hash input bytes for a given `Ask { prompt, timeout }` value MUST be identical across all compilations. This requires:
- Fixed order: `"ask"` tag first, then `prompt`, then `timeout` (or sentinel).
- `prompt.as_bytes()` is deterministic (String bytes are deterministic).
- Timeout sentinel for `None` MUST be a fixed byte sequence (e.g., `b"no_timeout"`).
- No platform-dependent or locale-dependent operations.

### TC-003: Empty Prompt
**Rule**: An ask with `prompt = ""` MUST produce a digest distinct from all non-empty prompts. Achieved by: `hasher.update(b"ask"); hasher.update(b"")` vs `hasher.update(b"ask"); hasher.update(b"hello")`.

### TC-004: Timeout Sentinel Distinction
**Rule**: The sentinel for `None` timeout MUST NOT collide with any valid timeout value bytes. Using `b"no_timeout"` (a structured sentinel with a `b"timeout"` prefix) avoids collision with actual timeout expression strings.

### TC-005: No Regressions for Set/Finish
**Rule**: The existing explicit arms for `Set` and `Finish` MUST continue to hash their fields identically. The fix for Ask must not alter the hash path for any other primitive.

### TC-006: Duplicate Implementation Parity
**Rule**: The fix MUST be applied identically to BOTH copies:
- `crates/vb_compile/src/mod_compile_lowering/part_05.rs` (active canonical compilation path)
- `crates/vb_compile/src/compile/mod.rs` (legacy path)

Or a single shared implementation must be used by both paths to eliminate the duplication.

### TC-007: No panic / no unwrap
**Rule**: `digest_step_primitive` MUST NOT panic, unwrap, or expect on any valid `StepPrimitive` variant. Hash field extraction is infallible for string and option fields.

## Type Boundaries

### Input Boundary (YAML → Compiler)
- `parse_ask()` validates `prompt` is present and is a string; `timeout` is optional and must be a string if present.
- The `StepPrimitive::Ask { prompt: String, timeout: Option<String> }` type ensures these constraints are satisfied before digest computation.
- **No new validation needed at the digest layer.** Type safety already guarantees `prompt` is a `String` and `timeout` is `Option<String>`.

### Output Boundary (Compiler → WorkflowParts)
- `canonical_digest` returns `WorkflowDigest` which is embedded in `WorkflowParts.digest`.
- `WorkflowParts` is validated by `vb_validate::shared::validate(&parts)` before becoming a `CompiledWorkflow`.
- **No type change needed at this boundary.** The digest field already accepts any `WorkflowDigest`.

## Illegal States Made Unrepresentable

These states are ALREADY unrepresentable at the type level:

| Illegal State | How Prevented |
|---------------|---------------|
| Ask without a prompt | `parse_ask` requires `prompt` field; `StepPrimitive::Ask.prompt` is `String`, not `Option<String>` |
| Ask with non-string timeout | `parse_ask` validates timeout is string if present |
| Corrupted digest bytes | `WorkflowDigest` is `[u8; 32]` — always 32 bytes |

These states REMAIN representable (the bug):

| Illegal State | Why Still Representable |
|---------------|------------------------|
| Canonical digest ignoring ask semantics | `digest_step_primitive` catch-all arm does not hash ask fields |
| Two workflows with different asks having same digest | Consequence of above |

## Open Type Questions

1. Should `WorkflowDigest` gain a `from_canonical(source: &WorkflowSource) -> Self` constructor that guarantees correct hashing, eliminating the possibility of bypassing `canonical_digest`?
2. Should `canonical_digest` return `Result<WorkflowDigest, CompileErrors>` instead of infallibly, to allow future validation of the digest production?
3. Should `digest_step_primitive` be made exhaustive (remove catch-all) to force compiler errors when new primitives are added? This would eliminate entire class of "forgot to hash new field" bugs.
