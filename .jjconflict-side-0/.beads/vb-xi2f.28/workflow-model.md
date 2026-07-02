# Workflow Model — Digest Computation

**Bead:** vb-xi2f.28  
**State:** 3 (rust-contract)  
**Date:** 2026-05-25  
**Status:** DRAFT

---

## 1. Digest Computation Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                   DIGEST COMPUTATION WORKFLOW                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  [1] YAML Source                                                 │
│       │                                                          │
│       ▼                                                          │
│  [2] Parse → WorkflowSource (vb_yaml::ast)                      │
│       │                                                          │
│       ▼                                                          │
│  [3] canonical_digest(&WorkflowSource) → WorkflowDigest          │
│       │                                                          │
│       ├── [3a] hasher.update(version)                            │
│       ├── [3b] hasher.update(name)                               │
│       ├── [3c] hasher.update(trigger fields)                     │
│       └── [3d] for step in steps:                                │
│                  hasher.update(step.id)                          │
│                  digest_step_primitive(hasher, step.primitive)   │
│                      │                                           │
│                      ├── Set    → hash "set" + output + value    │
│                      ├── Finish → hash "finish" + result         │
│                      ├── ForEach→ hash "for_each" ONLY (GAP!)    │
│                      └── other  → hash primitive name ONLY       │
│       │                                                          │
│       ▼                                                          │
│  [4] hasher.finalize() → [u8; 32]                                │
│       │                                                          │
│       ▼                                                          │
│  [5] WorkflowDigest::from_bytes(...)                             │
│       │                                                          │
│       ▼                                                          │
│  [6] Stored in WorkflowParts.digest                              │
│       │                                                          │
│       ▼                                                          │
│  [7] Wrapped in CompiledWorkflow.digest()                        │
│       │                                                          │
│       ▼                                                          │
│  [8] Used by: vb_storage::admission, vb_runtime::recovery        │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. State Transitions (Digest Computation)

The digest computation is a **pure, stateless pipeline**. There are no temporal state transitions — it is a single-pass computation over an immutable input.

### 2.1 States

| State | Description | Legal transitions |
|---|---|---|
| **Initial** | WorkflowSource exists in memory | → Hashing (step 3) |
| **Hashing** | blake3::Hasher accumulator receives field updates | → Finalized |
| **Finalized** | hasher.finalize() produces [u8; 32] | → Stored |
| **Stored** | WorkflowDigest wrapped in WorkflowParts.digest | (terminal) |

### 2.2 Error States

The canonical_digest computation has **no error states**. It is an infallible pure function:
- `canonical_digest` returns `WorkflowDigest` (not `Result<>`)
- `digest_step_primitive` has no error returns
- `blake3::Hasher::update` is infallible
- `blake3::Hasher::finalize` is infallible
- `WorkflowDigest::from_bytes` is infallible

**This infallibility is deliberate:** A digest computation that can fail would introduce a new error path in compilation that currently doesn't exist. The digest must always succeed.

---

## 3. ForEach Hashing Sub-Workflow (Post-Fix)

With the fix applied, the ForEach arm of `digest_step_primitive` follows this sub-workflow:

```
┌──────────────────────────────────────────────┐
│        ForEach DIGEST SUB-WORKFLOW            │
├──────────────────────────────────────────────┤
│                                               │
│  Input: &StepPrimitive::ForEach {             │
│      variable, input, at_once, body }         │
│                                               │
│  [A] hasher.update(b"for_each")               │
│       │                                       │
│  [B] hasher.update(b"variable:")              │
│       hasher.update(variable.as_bytes())      │
│       │                                       │
│  [C] hasher.update(b"input:")                 │
│       hasher.update(input.as_bytes())         │
│       │                                       │
│  [D] hasher.update(b"at_once:")               │
│       at_once.match {                         │
│           None    => update(0u32.to_le_bytes) │
│           Some(v) => update(v.to_le_bytes)    │
│       }                                       │
│       │                                       │
│  [E] hasher.update(b"body:")                  │
│       for body_step in body:                  │
│           hasher.update(body_step.id)         │
│           digest_step_primitive(body_step)    │
│       │                                       │
│  Output: ()  (hasher mutated in-place)        │
│                                               │
└──────────────────────────────────────────────┘
```

### 3.1 Sub-Workflow Guards

| Guard | Description |
|---|---|
| **G-FE-01** | All four fields (variable, input, at_once, body) MUST be hashed in a fixed canonical order |
| **G-FE-02** | Field delimiters prevent boundary ambiguity |
| **G-FE-03** | `at_once == None` is hashed as `0u32` (canonical: absent = default = 1 = hashed as 0) |
| **G-FE-04** | Body steps are hashed in iteration order (body[0], body[1], ...) |
| **G-FE-05** | Body step IDs are hashed alongside body step primitives |
| **G-FE-06** | Empty body is legal: hashes `b"body:"` with no following step content |

---

## 4. Compilation Workflow (Context)

The digest computation is embedded in the broader compilation workflow:

```
[YAML file]
    │
    ├── compilation path 1 (programmatic API):
    │       compile_source() in compile/mod.rs
    │          → canonical_digest()  ← FIX LOCATION A
    │          → lower_for_each()
    │          → lower_steps_to_ir()
    │          → CompiledWorkflow
    │
    └── compilation path 2 (lowering pipeline):
            compile_source() in mod_compile_lowering/part_01.rs
               → canonical_layout()
               → canonical_digest()  ← FIX LOCATION B
               → lower_canonical_step()
                  → lower_canonical_for_each()
               → lower_steps_to_ir()
               → CompiledWorkflow

Both paths MUST produce identical CompiledWorkflow.digest() for the same YAML source.
```

### 4.1 Post-Compilation Digest Uses

```
CompiledWorkflow.digest()
    │
    ├── vb_storage::admission:
    │       Checks submitted digest against stored digest.
    │       Digest mismatch → rejection.
    │
    ├── vb_runtime::recovery:
    │       Compares recovery artifact digest against compiled digest.
    │       Digest mismatch → recovery failure.
    │
    └── vb_storage::tests / workspace_tests:
            Tests of idempotency and admission use digest for identity.
```

---

## 5. Terminal Outcomes (Digest Computation)

| Outcome | Description | Is error? |
|---|---|---|
| **Digest computed** | A WorkflowDigest is produced for a WorkflowSource | No — normal |
| **Digest alias** | Two semantically different for_each sources produce identical digests | **YES** — this is the current bug |
| **Digest divergence** | Two compilation paths produce different digests for the same source | **YES** — duplicate code risk |
| **Non-deterministic digest** | Same source produces different digests across runs | **YES** — catastrophic |
