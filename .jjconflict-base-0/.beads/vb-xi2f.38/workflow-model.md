# Workflow Model: Digest Covers Collect Semantics (vb-xi2f.38)

## Workflow States and Transitions

### State Machine: Digest Computation Path

```
┌─────────────────────┐
│   WorkflowSource    │
│     (YAML AST)      │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────────────────────────────────────┐
│  canonical_digest(WorkflowSource) -> WorkflowDigest │
│                                                     │
│  1. Hash version bytes                              │
│  2. Hash name bytes                                 │
│  3. Hash trigger (variant + trigger-specific data)   │
│  4. For each step:                                  │
│     a. Hash step.id bytes                           │
│     b. digest_step_primitive(hasher, step.primitive)│
│        [BUG: Collect fields not hashed!]            │
└──────────┬──────────────────────────────────────────┘
           │
           ▼ (resulting digest embedded in WorkflowParts)
┌─────────────────────────────────────────────────────┐
│           compile_workflow(source)                  │
│                                                     │
│  YAML AST -> CompiledWorkflow (IR)                  │
│                                                     │
│  Parsing -> Lowering -> IR Emission                │
│                                                     │
│  Collect primitive --lower_canonical_collect-->      │
│    CollectStart { source, limit, page_size,          │
│                   body, done }                      │
│    SetConst (from body Set step)                    │
│    CollectPage { collector_slot, body, done }       │
│    CollectFinish { collector_slot }                 │
└──────────┬──────────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────────────────┐
│  compute_compiled_digest(artifact_bytes)            │
│                                                     │
│  BLAKE3(artifact_bytes) -> WorkflowDigest          │
│                                                     │
│  Depends on: canonical_digest being correct         │
└─────────────────────────────────────────────────────┘
```

### Legal States

| State | Description |
|-------|-------------|
| `YAML Parsed` | `WorkflowSource` successfully parsed from YAML bytes |
| `Digest Computed` | `canonical_digest` has run; `WorkflowDigest` is available |
| `Compiled` | `CompiledWorkflow` created with validated `WorkflowParts` |
| `Artifact Persisted` | Serialized `WorkflowParts` stored with content-addressed key |

### Terminal Outcomes

| Outcome | Condition |
|---------|-----------|
| `DigestProduced` | `canonical_digest` returns a `WorkflowDigest` |
| `CompilationFailed` | `compile_workflow` returns `Err(CompileErrors)` |
| `ValidationFailed` | `try_from_parts` returns `Err(WorkflowError)` |
| `ArtifactStored` | Artifact bytes stored in Fjall with matching digest key |

### Collect-Specific Workflow

```
Collect primitive in YAML
         │
         ▼
lower_canonical_collect (part_03.rs)
         │
    ┌────┴────┐
    │         │
    ▼         ▼
CollectStart  SetConst ──► CollectPage ──► CollectFinish
                    │
                    ▼
              (body steps)
```

The `Collect` body steps are lowered as a linear sequence inserted between `CollectStart` and `CollectPage`. The `CollectFinish` is the terminal node of the 4-node sequence.

---

## Guard Conditions

| Guard | Input | Behavior |
|-------|-------|----------|
| `Collect::pages.is_some()` | `Collect.pages` | If `Some`, use value; else default to 1 |
| `Collect::items.is_some()` | `Collect.items` | If `Some`, use value; else default to 1 |
| `Collect::body.not_empty()` | `Collect.body` | Must have ≥1 step; else `InvalidCollect` |
| `Collect::pages > 0` | `Collect.pages` | Must be ≥1 if present; else `InvalidCollect` |
| `Collect::items > 0` | `Collect.items` | Must be ≥1 if present; else `InvalidCollect` |

---

## Concurrency Model

- Digest computation is **single-threaded** and **pure**; no concurrency hazards
- The workflow runtime executes `CollectStart/Page/Finish` sequentially; pagination is cooperative (not preemptive)
- Concurrent fan-out is handled by `Together`, not `Collect`

---

## Cancellation and Retry Paths

- `Collect` pages are processed sequentially; cancellation rolls back to last completed page
- `CollectPaginationState` tracks cursor; on resume, iteration resumes from stored cursor
- Retry policy on collect step: retries the entire collect (all pages), not individual pages

---

## Idempotence Requirements

- **Source digest idempotence**: Same `WorkflowSource` bytes → same `WorkflowDigest`
- **Artifact digest idempotence**: Same serialized `WorkflowParts` bytes → same artifact digest
- **Collect replay**: Replaying a `Collect` from a given `WorkflowDigest` with same `CollectPaginationState` → same results

---

## Hazards (Digest-Specific)

1. **Digest collision**: Two different workflow sources produce same `WorkflowDigest` (BLAKE3-256 collision; astronomically unlikely but theoretically possible)
2. **Incomplete field hashing**: `Collect` fields not all hashed → different collect params produce same digest → wrong content-addressed lookup → wrong workflow execution
3. **IR drift**: Source digest correct but lowering produces different IR → same source digest but different behavior (mitigated by artifact digest covering serialized IR)
4. **Serialization non-determinism**: If `WorkflowParts` serialization is non-deterministic, artifact digest would vary for same IR content

---

## Failure Modes

| Mode | Description |
|------|-------------|
| `DigestMismatchOnReplay` | Artifact bytes on replay don't match stored digest; fail-closed |
| `CollectFieldHashMissing` | BUG: only primitive name hashed; two collects with different params have same digest |
| `CompilationError` | YAML parses but lowering fails; no digest produced |
| `ValidationError` | Lowering succeeds but validation fails; digest produced but workflow not admitted |
