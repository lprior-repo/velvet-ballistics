# Boundary Map: Wait Digest Coverage

**Bead:** vb-xi2f.32
**Date:** 2026-05-24
**State:** rust-contract (State 3)
**Schema:** boundary-map/v1

## 1. Architecture Boundary Model

```
┌──────────────────────────────────────────────────────────────┐
│              PURE FUNCTIONAL CORE                             │
│                                                              │
│  ┌────────────────────┐    ┌───────────────────────────┐     │
│  │  canonical_digest  │    │  compute_compiled_digest  │     │
│  │  (both copies)     │    │  (mod_compile_core.rs)    │     │
│  │                    │    │  pub fn(source: &[u8])    │     │
│  │  pure: &Workflow   │    │  → WorkflowDigest        │     │
│  │  Source → Digest   │    │  pure: bytes → blake3    │     │
│  └────────┬───────────┘    └───────────────────────────┘     │
│           │                                                   │
│  ┌────────▼───────────┐                                      │
│  │ digest_step_       │                                      │
│  │ primitive (both)   │                                      │
│  │ pure: StepPrimitive│                                      │
│  │ → hasher.update()  │                                      │
│  │                     │                                      │
│  │ [BUG] Wait fields  │    ←── FIX APPLIED HERE             │
│  │ not hashed         │                                      │
│  └────────────────────┘                                      │
│                                                              │
│  ┌──────────────────────────────────────────────────┐       │
│  │ Type definitions (vb_core)                       │       │
│  │  - WorkflowDigest([u8; 32])  — no validation    │       │
│  │  - WorkflowParts { digest }  — no validation    │       │
│  │  - CompiledNodeKind::WaitUntil, WaitEvent       │       │
│  │  - SlotIdx, StepIdx          — newtypes         │       │
│  └──────────────────────────────────────────────────┘       │
│                                                              │
│  ┌──────────────────────────────────────────────────┐       │
│  │ WaitKind (part_07.rs)                            │       │
│  │  enum { Until{deadline}, Event{event,timeout} }  │       │
│  │  Type-safe discriminator — illegal states        │       │
│  │  unrepresentable                                 │       │
│  └──────────────────────────────────────────────────┘       │
└──────────────────────────────────────────────────────────────┘
                            │
                ┌───────────┴───────────┐
                │                       │
┌───────────────▼───────────┐  ┌────────▼──────────────────┐
│  IMPERATIVE SHELL         │  │  PARSER BOUNDARY          │
│                           │  │                           │
│  ┌───────────────────┐    │  │  YAML AST (vb_yaml)       │
│  │ lower_canonical_  │    │  │  StepPrimitive::Wait {    │
│  │ wait (part_04.rs) │    │  │   event: Option<String>,  │
│  │                   │    │  │   timeout: Option<String> │
│  │ Matches on        │    │  │  }                        │
│  │ (event, timeout)  │    │  │                           │
│  │ → WaitKind        │    │  │  Parse once at boundary.  │
│  │ → lower_wait()    │    │  │  Strings enter core as    │
│  └───────────────────┘    │  │  &str or String.          │
│                           │  └───────────────────────────┘
│  ┌───────────────────┐    │
│  │ lower_wait        │    │  ┌───────────────────────────┐
│  │ (part_07.rs)      │    │  │ VALIDATION BOUNDARY       │
│  │ WaitKind →        │    │  │                           │
│  │ CompiledNode      │    │  │ validate_wait_shape       │
│  └───────────────────┘    │  │ (part_03.rs)              │
│                           │  │ Rejects:                  │
│  ┌───────────────────┐    │  │  - unknown fields          │
│  │ compile_source    │    │  │  - (None, None) shape     │
│  │ (part_01.rs)      │    │  │  - (Some, Some) with      │
│  │                   │    │  │    until+event ambiguity  │
│  │ Orchestrates:     │    │  └───────────────────────────┘
│  │  1. validate      │    │
│  │  2. digest        │    │
│  │  3. lower steps   │    │
│  │  4. assemble parts│    │
│  └───────────────────┘    │
└───────────────────────────┘
```

## 2. Boundary Classification

| Boundary | Type | Crate(s) | What crosses |
|----------|------|----------|--------------|
| YAML → AST | Parser | vb_yaml | Raw bytes → `WorkflowSource` with `StepPrimitive::Wait { event: Option<String>, timeout: Option<String> }` |
| AST → Compiler | Type boundary | vb_yaml → vb_compile | `&WorkflowSource`, `&StepPrimitive` |
| Compiler → Core | Type boundary | vb_compile → vb_core | `CompiledWorkflow`, `WorkflowParts { digest: WorkflowDigest }` |
| Digest → Storage | I/O boundary | vb_compile → vb_storage | `WorkflowDigest` embedded in serialized artifact |
| Core → Runtime | Type boundary | vb_core (internal) | `WorkflowDigest` used for identity/integrity checks |

## 3. Functional Core vs Shell

### Pure Core (no I/O, no time, no randomness)
- `canonical_digest(source: &WorkflowSource) → WorkflowDigest` — **FIX TARGET**
- `digest_step_primitive(hasher: &mut Hasher, primitive: &StepPrimitive) → ()` — **FIX TARGET**
- `canonical_primitive_name(primitive: &StepPrimitive) → &'static str`
- `compute_compiled_digest(source: &[u8]) → WorkflowDigest`
- `WorkflowDigest` — data type, no logic
- `WaitKind` — data type, no logic

### Imperative Shell (orchestration, mutation)
- `compile_source(source: &WorkflowSource) → Result<CompiledWorkflow, CompileErrors>` — orchestrates validation + digest + lowering
- `lower_canonical_wait(index, id, event, timeout, next, builder) → Result<(), CompileErrors>` — resolves slots, builds nodes
- `lower_wait(id, kind, builder) → CompiledNode` — records slots, creates node

### Parser Boundary
- `validate_wait_shape(body, index, last_step) → Result<(), CompileError>` — validates YAML shape
- `slot_from_text(text, index, field) → Result<SlotIdx, CompileError>` — resolves slot expression
- YAML deserialization (vb_yaml) — parses raw bytes into AST

### Storage / I/O Boundary (out of scope for this bead)
- Persisting `CompiledWorkflow` with digest
- Loading compiled artifacts and verifying digest

## 4. Unsafe / FFI Boundaries

**None.** No `unsafe` code in the digest computation path. `blake3::Hasher::update` is a safe Rust API. `WorkflowDigest([u8; 32])` is `repr(transparent)` but does not use `unsafe` in its safe constructors.

## 5. Time Boundary

**None.** `canonical_digest` is time-independent (pure function). No clock access. `Wait { timeout: Option<String> }` is a string field in the AST, not a real-time value.

## 6. Network / HTTP Boundary

**None.** Per the master architecture contract: "Runtime-core crates must remain YAML/JSON/HTTP-free." The digest is computed at compile time, not at runtime.

## 7. Async Shell

**None.** `canonical_digest` and `digest_step_primitive` are synchronous `fn` calls. The compiler is not async.
