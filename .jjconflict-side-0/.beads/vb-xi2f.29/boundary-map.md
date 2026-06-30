# Boundary Map: Digest Coverage for Together

bead_id: vb-xi2f.29
bead_title: P1: digest covers together semantics
phase: 3 (rust-contract)
created_at: 2026-05-24

## Architecture Layers

```
┌───────────────────────────────────────────────────────────────────┐
│                         IMPERATIVE SHELL                           │
│  (CLI, IPC server, runtime — not in digest scope)                 │
└───────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌───────────────────────────────────────────────────────────────────┐
│                          COMPILE BOUNDARY                          │
│  vb_compile crate — compilation pipeline                          │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │ MOD_COMPILE_LOWERING (active code)                          │  │
│  │  part_01.rs: compile_source()  ← entry point                │  │
│  │  part_05.rs: canonical_digest() ← DIGEST HERE               │  │
│  │  part_05.rs: canonical_primitive_name() ← NAME MAP          │  │
│  │  part_05.rs: digest_step_primitive() ← PRIMITIVE HASHER     │  │
│  │  part_02.rs: lower_canonical_step()                         │  │
│  │  part_03.rs: lower_canonical_parallel() ← LOWERING          │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │ compile/mod.rs (DEAD CODE — not declared in lib.rs)         │  │
│  │  canonical_digest(), digest_step_primitive(),               │  │
│  │  canonical_primitive_name(), lower_together()               │  │
│  │  ⚠ DO NOT FIX/MODIFY — delete or mark as dead              │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │ kani_canonical_name.rs (Kani harnesses — proof only)        │  │
│  │  canonical_name_together_harness                            │  │
│  │  canonical_name_aggregate_harness                           │  │
│  │  canonical_name_all_harness                                 │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │ mod_compile_core.rs: compute_compiled_digest()              │  │
│  │  Byte-level blake3 hash — different purpose, NOT IN SCOPE   │  │
│  └─────────────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────────────┘
           │                                    │
           ▼                                    ▼
┌──────────────────────┐          ┌──────────────────────────┐
│  PARSER BOUNDARY     │          │  PURE CORE               │
│  vb_yaml crate       │          │  vb_core crate            │
│                      │          │                           │
│  ast/types.rs:       │          │  ids/mod.rs:              │
│   WorkflowSource     │          │   WorkflowDigest          │
│   StepPrimitive      │          │   (32-byte blake3 hash)   │
│   StepAst            │          │                           │
│   TogetherBranch     │          │  workflow/mod.rs:          │
│                      │          │   CompiledWorkflow        │
│  ast/parse_steps.rs: │          │   CompiledNodeKind::      │
│   parse_parallel()   │          │    TogetherStart,         │
│                      │          │    TogetherBranch,         │
│                      │          │    TogetherJoin            │
│                      │          │   WorkflowParts           │
│                      │          │                           │
└──────────────────────┘          └──────────────────────────┘
           │
           ▼
┌───────────────────────────────────────────────────────────────────┐
│                         EXTERNAL DEPENDENCY                        │
│  blake3 crate — cryptographic hash function                       │
│  Used by: canonical_digest, compute_compiled_digest               │
│  Boundary note: blake3::Hasher is pure — no I/O, deterministic    │
└───────────────────────────────────────────────────────────────────┘
```

## Boundary Rules

### B-001: Digest computation is pure
- `canonical_digest()` has no side effects. It only reads from a `&WorkflowSource` reference and calls `blake3::Hasher` methods.
- `blake3::Hasher` is deterministic and pure.
- No I/O, no async, no network, no time, no randomness.

### B-002: Parser boundary produces validated AST
- `vb_yaml` parses YAML text → `WorkflowSource`. All parsing/validation happens once at this boundary.
- The digest operates on the already-parsed `WorkflowSource` tree.
- Invalid YAML is rejected before digest computation.

### B-003: vb_core is the consumer of digests
- `WorkflowDigest` is stored in `WorkflowParts.digest` and `CompiledWorkflow.digest`.
- `vb_core` does not compute digests; it only stores and compares them.
- `CompiledWorkflow` implements `PartialEq` via digest comparison.

### B-004: Lowering happens AFTER digest
- `lower_canonical_step()`, `lower_canonical_parallel()`, `emit_together_branches()` execute after `canonical_digest()` returns.
- Digest computation does not depend on IR node layout, only on AST structure.
- This is the correct ordering: hash source semantics, then produce IR.

### B-005: Dead code quarantine
- `compile/mod.rs` is NOT declared in `lib.rs`. It is not compiled into the binary.
- Any fixes in `mod_compile_lowering/part_05.rs` MUST NOT be replicated in `compile/mod.rs`.
- Preferred approach: delete `compile/mod.rs` entirely or add `#[cfg(any())]` + `compile_error!` guard.

### B-006: Test boundary
- Digest tests live in `crates/vb_compile/tests/` or inline `#[cfg(test)] mod tests`.
- Tests may construct `WorkflowSource` values via the `vb_yaml::ast` public API or by parsing YAML strings.
- Proptests generate `WorkflowSource` values programmatically using `proptest::arbitrary::Arbitrary` or manual strategies.

### B-007: Kani harness boundary
- Kani harnesses in `kani_canonical_name.rs` import from `crate::mod_compile_lowering::part_05` — the active code.
- Kani harnesses do NOT import from `crate::compile::mod` (dead code).
- Future Kani harnesses for digest should similarly bind to active code only.

## Crate Dependency Graph (Relevant Subset)

```
vb_compile (compilation pipeline)
  ├── vb_yaml: AST types (WorkflowSource, StepPrimitive, TogetherBranch)
  ├── vb_core: IR types (WorkflowDigest, CompiledWorkflow, WorkflowParts)
  ├── vb_validate: IR validation (shared validation)
  ├── blake3: hashing
  └── postcard: serialization

vb_yaml (parser)
  └── (standalone, no vb_ dependencies)

vb_core (IR types)
  └── (standalone for types, depends on serde for WorkflowDigest)
```

## Security / Trust Boundaries

| Boundary | Trust Level | Notes |
|----------|-------------|-------|
| YAML input → AST | Untrusted | Parser validates. Malformed YAML rejected. |
| AST → Digest | Trusted | Pure computation on validated AST. |
| Digest → IR | Trusted | Digest committed to WorkflowParts. Immutable after compilation. |
| Digest comparison | Trusted | Eq trait on [u8; 32]. No timing side-channel concern (digests are public). |
| blake3 | Trusted | Well-known cryptographic library. Deterministic. |
