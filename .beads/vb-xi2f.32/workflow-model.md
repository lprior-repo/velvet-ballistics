# Workflow Model: Wait Digest Coverage

**Bead:** vb-xi2f.32
**Date:** 2026-05-24
**State:** rust-contract (State 3)
**Schema:** workflow-model/v1

## 1. Digest Computation Workflow

### State Machine: `canonical_digest`

```
                             ┌─────────────┐
                             │   Idle      │
                             └──────┬──────┘
                                    │ source: &WorkflowSource
                                    ▼
                         ┌──────────────────┐
                         │  VersionHashed   │
                         │  hasher.update(  │
                         │  source.version) │
                         └────────┬─────────┘
                                  │
                                  ▼
                         ┌──────────────────┐
                         │   NameHashed     │
                         │  hasher.update(  │
                         │  source.name)    │
                         └────────┬─────────┘
                                  │
                                  ▼
                         ┌──────────────────┐
                         │ TriggerHashed    │
                         │  hasher.update(  │
                         │  trigger bytes)  │
                         └────────┬─────────┘
                                  │ for each step in source.steps()
                                  ▼
                    ┌─────────────────────────┐
                    │     StepLoop            │◄── more steps
                    │  hasher.update(step.id) │
                    │  digest_step_primitive  │
                    └────────────┬────────────┘
                                 │ last step
                                 ▼
                    ┌─────────────────────────┐
                    │     DigestFinalized     │
                    │  hasher.finalize()      │
                    │  → WorkflowDigest       │
                    └─────────────────────────┘
```

**Guards:**
- `source.steps()` must be non-empty (empty falls through to no iterations — hashes only version+name+trigger, which is degenerate)
- Each `step.id` must be unique within the workflow (validation guard, not digest guard)

**Terminal state:** `DigestFinalized` — produces a `WorkflowDigest`.

### State Machine: `digest_step_primitive` — Wait arm (PROPOSED)

```
                             ┌──────────┐
                             │  Entry   │
                             │ Wait{    │
                             │  event,  │
                             │  timeout │
                             │ }        │
                             └────┬─────┘
                                  │
                    ┌─────────────▼──────────────┐
                    │  Discriminate              │
                    │  event.is_some() ?         │
                    └───┬───────────────┬────────┘
                        │               │
              event=None│               │event=Some(e)
                        ▼               ▼
          ┌─────────────────┐   ┌──────────────────┐
          │  HashWaitUntil  │   │  HashWaitEvent   │
          │  hasher.update( │   │  hasher.update(  │
          │  b"wait_until") │   │  b"wait_event") │
          └────────┬────────┘   └────────┬─────────┘
                   │                     │
                   │                     ▼
                   │          ┌──────────────────────┐
                   │          │  HashEventField      │
                   │          │  hasher.update(      │
                   │          │  e.as_bytes())       │
                   │          └──────────┬───────────┘
                   │                     │
                   │                     ▼
                   │          ┌──────────────────────┐
                   │          │  DiscriminateTimeout │
                   │          │  timeout.is_some()?  │
                   │          └───┬──────────────┬───┘
                   │              │timeout=None  │timeout=Some(t)
                   │              ▼              ▼
                   │    ┌──────────────┐  ┌──────────────┐
                   │    │HashNullMarker│  │HashTimeout   │
                   │    │hasher.update(│  │hasher.update(│
                   │    │b"none")      │  │t.as_bytes()) │
                   │    └──────┬───────┘  └──────┬───────┘
                   │           │                 │
                   ▼           ▼                 ▼
          ┌──────────────────────────────────────────┐
          │  HashTimeoutField                        │
          │  hasher.update(t.as_bytes())             │
          │  (WaitUntil always has timeout=Some(t))  │
          └────────────────────┬─────────────────────┘
                               │
                               ▼
                         ┌──────────┐
                         │   Done   │
                         └──────────┘
```

**Guard:** The `Wait { event: None, timeout: None }` case is impossible because validation rejects it. The state machine does not handle it.

**Terminal state:** `Done` — control returns to `canonical_digest`'s StepLoop.

## 2. Legal Wait Transitions (Compilation)

```
Wait AST (StepPrimitive::Wait)
         │
         │ lower_canonical_wait(event, timeout)
         ▼
    ┌──────────────┐
    │  MatchShape  │
    │ (event,      │
    │  timeout)    │
    └──┬───┬───┬──┘
       │   │   │
       │   │   └── (None, None)  → ERROR: StepFieldShape
       │   │
       │   └──── (None, Some(t)) → WaitKind::Until { deadline: slot_from_text(t) }
       │                                   ↓
       │                          CompiledNodeKind::WaitUntil { deadline_slot }
       │
       └────── (Some(e), t) ────→ WaitKind::Event { event: slot_from_text(e),
       │                                            timeout: optional_slot_from_text(t) }
       │                                   ↓
       │                          CompiledNodeKind::WaitEvent { event, timeout_slot }
       ▼
   Wait IR Node (CompiledNode)
```

**Guards:**
- `slot_from_text` succeeds (valid slot expression)
- `optional_slot_from_text` succeeds or returns None
- Both resolve to valid `SlotIdx` values within `slot_count`

**Terminal outcome:** Wait IR node appended to `SlotCompiler` node list.

## 3. Cancellation/Error Paths

| Path | Trigger | Outcome |
|------|---------|---------|
| Invalid wait shape | `validate_wait_shape` rejects | `CompileError::StepFieldShape` → compilation fails |
| Invalid slot reference | `slot_from_text` fails | `CompileError` → compilation fails |
| Digest mismatch (runtime) | Integrity check fails | Execution rejected (runtime, not compile-time) |
| Duplicate fix divergence | One copy of `digest_step_primitive` fixed, other not | Different digests from cold-path vs warm-path for same source |

## 4. Idempotence Requirements

- **IR-1:** `canonical_digest(source)` is idempotent — same source always produces same digest.
- **IR-2:** `canonical_digest` is deterministic — no external state, time, or randomness affects the hash.
- **IR-3:** After fix, same source with same wait fields always produces same digest across both compiler paths.

## 5. Concurrency Model

- `canonical_digest` is a pure function. It borrows `source: &WorkflowSource` immutably and owns the `blake3::Hasher`.
- No shared mutable state. No synchronization required.
- `blake3::Hasher` is `Send + Sync` but not used concurrently in the compiler.
- The two copies of `canonical_digest` run in different call stacks (cold-path vs warm-path). They do not race.

## 6. Retry / Recovery

- Compilation failures are terminal for that invocation. No retry loop.
- If `canonical_digest` produces a hash that differs from a previously persisted digest, the runtime detects this at integrity check time (out of scope for this bead).
- Remediation: recompile from YAML source. The fixed `canonical_digest` will produce the correct hash.

## 7. Workflow Diagram: Fix Impact

```
Before fix:
  canonical_digest(wf_A) == canonical_digest(wf_B)
    where wf_A has wait.event="0", wait.timeout="30"
      and wf_B has wait.timeout="5" (WaitUntil)
  → Same digest! Integrity bypass!

After fix:
  canonical_digest(wf_A) ≠ canonical_digest(wf_B)
  → Different digests. Integrity preserved.
  → All existing tests for other digest-sensitivity properties remain passing.
  → Existing persisted digests become invalid (need recompilation).
```
