# Boundary Map — vb-xi2f.34: Finish Digest Semantics

**Bead**: vb-xi2f.34
**Date**: 2026-05-24
**Rust Contract Agent**: rust-contract

---

## Architecture Layers

```
┌──────────────────────────────────────────────┐
│                 IMPERATIVE SHELL              │
│  (vb_yaml: parse YAML, produce AST)          │
│  (vb_storage: persist/recover CompiledWF)    │
│  (vb_runtime: execute CompiledWorkflow)      │
│  (vb_ipc: transport frames)                  │
└──────────────────┬───────────────────────────┘
                   │ WorkflowSource
                   ▼
┌──────────────────────────────────────────────┐
│               FUNCTIONAL CORE                 │
│                                               │
│  ┌─────────────────────────────────────┐     │
│  │ PURE DIGEST SUBSYSTEM               │     │
│  │                                     │     │
│  │ canonical_digest(source) → Digest   │     │
│  │ digest_step_primitive(hasher, prim) │     │
│  │ canonical_primitive_name(prim)      │     │
│  │                                     │     │
│  │ Dependencies: blake3 crate only     │     │
│  │ Side effects: none                  │     │
│  │ IO: none                            │     │
│  │ Async: none                         │     │
│  │ Unsafe: none                        │     │
│  └─────────────────────────────────────┘     │
│                                               │
│  ┌─────────────────────────────────────┐     │
│  │ COMPILE SUBSYSTEM (lowering)        │     │
│  │                                     │     │
│  │ compile_source(source) → Workflow   │     │
│  │ lower_canonical_finish(...)         │     │
│  │ canonical_finish_slot(result, outs) │     │
│  │ lower_finish(id, slot, builder)     │     │
│  │                                     │     │
│  │ Dependencies: AST types, IDs        │     │
│  │ Side effects: mutates SlotCompiler  │     │
│  │ IO: none                            │     │
│  └─────────────────────────────────────┘     │
│                                               │
│  ┌─────────────────────────────────────┐     │
│  │ VALIDATION SUBSYSTEM                │     │
│  │                                     │     │
│  │ validate_parts(parts) → Result      │     │
│  │ validate_budget(parts) → Result     │     │
│  │                                     │     │
│  │ Dependencies: IDs, limits, budget   │     │
│  └─────────────────────────────────────┘     │
│                                               │
└──────────────────────────────────────────────┘
```

---

## Pure Core (Digest Subsystem)

| Boundary | In | Out | Side Effects |
|---|---|---|---|
| `canonical_digest` | `&WorkflowSource` | `WorkflowDigest` | None |
| `digest_step_primitive` | `&mut Hasher, &StepPrimitive` | `()` (mutates hasher) | Mutates hasher state (pure in the blake3 sense) |
| `canonical_primitive_name` | `&StepPrimitive` | `&'static str` | None |

**Rationale**: These functions are pure. They take immutable or exclusive references to non-IO types and produce deterministic output. They are trivially testable, fuzzable, and amenable to Kani verification.

---

## Pure Core (Compile/Lowering Subsystem)

| Boundary | In | Out | Side Effects |
|---|---|---|---|
| `compile_source` | `&WorkflowSource` | `Result<CompiledWorkflow, CompileErrors>` | Mutates `SlotCompiler` (accumulator pattern) |
| `canonical_finish_slot` | `&ScalarValue, &HashMap<String, SlotIdx>` | `Result<SlotIdx, CompileErrors>` | None |
| `lower_finish` | `StepIdx, SlotIdx, &mut SlotCompiler` | `CompiledNode` | Mutates `SlotCompiler` |
| `lower_canonical_finish` | `usize, usize, StepIdx, &ScalarValue, &HashMap, &mut SlotCompiler` | `Result<(), CompileErrors>` | Mutates `SlotCompiler` |

**Rationale**: These functions are pure in domain terms (no IO, no time, no randomness) but mutate a `SlotCompiler` builder. The builder is a local accumulator — not shared state, not persisted.

---

## Imperative Shell

| Boundary | Responsibility |
|---|---|
| `vb_yaml::parse_workflow_source` | Parse YAML text → `WorkflowSource` AST. This is the **parser boundary**. All text/format concerns live here. |
| `vb_storage::artifacts` | Persist `CompiledWorkflow` via Postcard + Fjall. The digest is serialized as part of `WorkflowParts`. |
| `vb_runtime::admission` | Admit `CompiledWorkflow` for execution. The runtime trusts the digest as an identity token. |
| `vb_ipc::frame` | Transport compiled workflow frames. Digest may be used for matching/verification. |

---

## Unsafe Boundary

**None.** The digest subsystem and compile/lowering subsystem are in `#![forbid(unsafe_code)]` crates:
- `vb_core/src/workflow/mod.rs` — `#![forbid(unsafe_code)]` (line 1)
- `vb_compile/src/compile/mod.rs` — `#![forbid(unsafe_code)]` (line 1)
- `vb_yaml` — must check

No `unsafe` blocks exist in any digest-relevant code path.

---

## Async Boundary

**None.** Digest computation is synchronous. Compilation is synchronous. All relevant functions return `Result<T, E>`, not `Future<T>`.

---

## Time Boundary

**None.** `canonical_digest()` does not use `Instant`, `SystemTime`, or any time source. The digest is fully deterministic and reproducible given the same AST.

---

## Storage Boundary

**Indirect.** The digest is stored as part of `CompiledWorkflow` (via `WorkflowParts` serialization). The storage layer (`vb_storage`) uses Postcard for serialization. The digest type (`WorkflowDigest`) derives `Serialize, Deserialize`.

---

## Network Boundary

**None.** No network IO in digest computation or finish lowering.

---

## Randomness Boundary

**None.** `canonical_digest()` uses `blake3::Hasher::new()` which is deterministic (no random seed, no nonce).

---

## FFI Boundary

**None.** `blake3` is a pure-Rust crate. No C FFI or native library calls.

---

## Parser Boundary

The critical boundary is between the YAML parser and the digest computation:

```
YAML bytes (untrusted, text)
  └─ vb_yaml::parse_workflow_source() → WorkflowSource (structured, typed)
       └─ canonical_digest(&WorkflowSource) → WorkflowDigest (opaque bytes)
```

The digest function accepts a **typed AST**, never raw bytes. This means:
- The parser validates structure.
- The digest operates on semantic types, not text.
- Two textually different but AST-equivalent sources produce the same digest.

---

## Duplicate Code Boundary

```
                    ┌─ mod_compile_lowering::compile_source() → canonical_digest() [CANONICAL]
                    │
WorkflowSource ─────┤
                    │
                    └─ compile::mod::compile_source() → canonical_digest() [LEGACY]
                           ↑
                           └── used by proptest helpers only
```

Both paths call their respective `canonical_digest()`. There is no shared abstraction — each module has its own copy. This is a **boundary violation**: the pure digest function should have exactly one definition shared by all callers.

---

## Cross-Cutting Concerns

| Concern | Handled By |
|---|---|
| Determinism | Pure function design; blake3 library |
| Reproducibility | No time/random/IO inputs |
| Performance | blake3 is SIMD-accelerated; digest computation is O(steps) |
| Forward compatibility | `#[non_exhaustive]` on ScalarValue; `_` arms in match |
| Security | Hash is not cryptographic for security — it's an identity fingerprint. blake3 is collision-resistant. |
