# Boundary Map — Digest Coverage of `for_each` Semantics

**Bead:** vb-xi2f.28  
**State:** 3 (rust-contract)  
**Date:** 2026-05-25  
**Status:** DRAFT

---

## 1. Pure Core / Imperative Shell Boundary

```
┌─────────────────────────────────────────────────────────────────┐
│                        PURE CORE                                 │
│                                                                  │
│  ┌──────────────────────────────────────────────────────┐       │
│  │  canonical_digest(&WorkflowSource) → WorkflowDigest   │       │
│  │  digest_step_primitive(&mut Hasher, &StepPrimitive)  │       │
│  │  canonical_primitive_name(&StepPrimitive) → &str     │       │
│  │                                                       │       │
│  │  PROPERTIES:                                          │       │
│  │  - No I/O, no network, no storage, no time            │       │
│  │  - Pure function: same input → same output always     │       │
│  │  - No randomness, no global state, no env vars        │       │
│  │  - No unsafe code                                     │       │
│  │  - All inputs are immutable references                │       │
│  └──────────────────────────────────────────────────────┘       │
│                                                                  │
│  ┌──────────────────────────────────────────────────────┐       │
│  │  canonical_digest (duplicate)                         │       │
│  │  digest_step_primitive (duplicate)                   │       │
│  │  canonical_primitive_name (duplicate)                │       │
│  │                                                       │       │
│  │  SAME PROPERTIES AS ABOVE                            │       │
│  └──────────────────────────────────────────────────────┘       │
│                                                                  │
├─────────────────────────────────────────────────────────────────┤
│                      IMPERATIVE SHELL                            │
│                                                                  │
│  ┌──────────────────────────────────────────────────────┐       │
│  │  vb_yaml::ast::parse(source_bytes: &[u8])             │       │
│  │  → Result<WorkflowSource, YamlError>                  │       │
│  │                                                        │       │
│  │  PROPERTIES:                                           │       │
│  │  - Parser boundary: bytes → AST                        │       │
│  │  - Can fail (malformed YAML, invalid fields)           │       │
│  │  - Produces WorkflowSource consumed by pure core       │       │
│  └──────────────────────────────────────────────────────┘       │
│                                                                  │
│  ┌──────────────────────────────────────────────────────┐       │
│  │  lower_steps_to_ir(nodes, ..., digest)                │       │
│  │  → Result<CompiledWorkflow, CompileErrors>            │       │
│  │                                                        │       │
│  │  PROPERTIES:                                           │       │
│  │  - Orchestration: builds IR from nodes + digest       │       │
│  │  - Validates IR via vb_validate::shared::validate     │       │
│  │  - Wraps validated IR in CompiledWorkflow             │       │
│  └──────────────────────────────────────────────────────┘       │
│                                                                  │
│  ┌──────────────────────────────────────────────────────┐       │
│  │  compute_compiled_digest(source: &[u8])               │       │
│  │  → WorkflowDigest                                     │       │
│  │                                                        │       │
│  │  PROPERTIES:                                           │       │
│  │  - Pure function (byte → digest)                      │       │
│  │  - Different layer: hashes serialized IR bytes        │       │
│  │  - NOT the target of this fix                         │       │
│  └──────────────────────────────────────────────────────┘       │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. Crate Boundary Map

```
                vb_yaml
          ┌──────────────┐
          │ ast/types.rs  │  WorkflowSource, StepAst, StepPrimitive, ScalarValue
          │ ast/parse.rs  │  YAML parser (shell boundary)
          │ ast/mod.rs    │
          └──────┬───────┘
                 │ (dependency)
                 ▼
            vb_compile
   ┌─────────────────────────────────────────────┐
   │                                              │
   │  compile/mod.rs                              │
   │    ├── canonical_digest()          ← FIX A  │
   │    ├── digest_step_primitive()     ← FIX A  │
   │    ├── canonical_primitive_name()            │
   │    ├── lower_for_each()                      │
   │    └── lower_steps_to_ir()                   │
   │                                              │
   │  mod_compile_lowering/part_05.rs             │
   │    ├── canonical_digest()          ← FIX B  │
   │    ├── digest_step_primitive()     ← FIX B  │
   │    ├── canonical_primitive_name()            │
   │    └── lower_steps_to_ir()                   │
   │                                              │
   │  mod_compile_lowering/part_01.rs             │
   │    └── compile_source(), canonical_layout()  │
   │                                              │
   │  mod_compile_lowering/part_02.rs             │
   │    └── lower_canonical_for_each()            │
   │                                              │
   │  mod_compile_core.rs                         │
   │    └── compute_compiled_digest()  (OK)      │
   │                                              │
   └──────────────────┬──────────────────────────┘
                      │ (dependency)
                      ▼
                   vb_core
          ┌────────────────────┐
          │ ids/mod.rs          │  WorkflowDigest
          │ compiled_workflow.rs│  WorkflowParts, CompiledWorkflow
          │ nodes.rs            │  CompiledNodeKind::ForEachStart/Next/Join
          │ validation/         │  IR validation
          └────────────────────┘
                      │
          ┌───────────┴───────────┐
          ▼                       ▼
     vb_storage              vb_runtime
  ┌──────────────┐      ┌──────────────┐
  │ admission.rs  │      │ recovery.rs  │
  │ uses digest   │      │ uses digest  │
  │ for identity  │      │ for matching │
  └──────────────┘      └──────────────┘
```

---

## 3. Boundary Ownership

| Boundary | Owner Crate | Responsibility | Pure or Impure? |
|---|---|---|---|
| **YAML Parsing** | `vb_yaml` | Parse bytes → `WorkflowSource`; reject malformed input | Impure (I/O at call site) |
| **canonical_digest** | `vb_compile` | Compute deterministic BLAKE3 hash from `WorkflowSource` fields | **Pure** |
| **digest_step_primitive** | `vb_compile` | Dispatch step-type-specific field hashing | **Pure** |
| **lower_steps_to_ir** | `vb_compile` | Assemble `CompiledWorkflow` from nodes + digest; validate | Impure (validation may allocate) |
| **compile_source** | `vb_compile` | Orchestrate YAML → IR pipeline | Impure (calls parser then pure core) |
| **WorkflowDigest** | `vb_core` | Type definition; byte-level equality | Pure (data type) |
| **CompiledWorkflow** | `vb_core` | Immutable IR wrapper with digest accessor | Pure (data type) |
| **Storage Admission** | `vb_storage` | Compare submitted digest against stored | Impure (I/O) |
| **Runtime Recovery** | `vb_runtime` | Compare recovery artifact digest against compiled | Impure (I/O) |

---

## 4. Unsafe / FFI Boundaries

**None.** The entire digest computation pipeline contains no `unsafe` code:
- `blake3::Hasher` is a pure Rust implementation
- No FFI, no inline assembly, no raw pointer manipulation
- `WorkflowDigest` is `#[repr(transparent)]` over `[u8; 32]` with no unsafe transmutes

---

## 5. Time / Randomness Boundaries

**None.** The digest computation has no time or randomness dependencies:
- No `Instant`, no `SystemTime`, no `Instant::now()`
- No `rand`, no `thread_rng`, no OS entropy
- `blake3::Hasher` is deterministic (same input → same hash always)

---

## 6. Storage / Network Boundaries

These boundaries exist at the **consumers** of the digest, not at the digest computation itself:

| Consumer | Boundary Type | Impact of Digest Gap |
|---|---|---|
| `vb_storage::admission` | Storage write | Wrong digest → admission may accept mismatched workflow |
| `vb_runtime::recovery` | Storage read | Wrong digest → recovery may fail or restore wrong workflow |
| `vb_storage::tests` | Test assertion | Digest-identity tests may silently pass despite semantic differences |
