# Type Contracts: Wait Digest Coverage

**Bead:** vb-xi2f.32
**Date:** 2026-05-24
**State:** rust-contract (State 3)
**Schema:** type-contracts/v1

## 1. Newtype Definitions

### TC-1: WaitHashMarker
**Type:** `WaitHashMarker` — a marker type used in digest hashing to discriminate WaitUntil from WaitEvent.

```rust
/// Discriminator written to the digest to separate WaitUntil from WaitEvent.
enum WaitHashMarker {
    /// write(bytes: b"wait_until") — deadline wait only
    Until,
    /// write(bytes: b"wait_event") — event wait (with optional timeout)
    Event,
}
```
- **Canonical bytes:** `WaitHashMarker::Until` → `b"wait_until"`, `WaitHashMarker::Event` → `b"wait_event"`
- **Invariant:** Exactly ONE marker is written per Wait step. No fallback path.
- **Source of truth:** Derived from `event: Option<String>` at digest time. `event=None` → Until; `event=Some(...)` → Event.

### TC-2: NullFieldMarker
**Type:** Sentinel marker for absent optional fields in digest hashing.

```rust
/// Sentinel value written to the digest when an optional wait field is absent.
const NULL_FIELD_MARKER: &[u8] = b"none";
```
- **Use:** Written when `timeout` is `None` or (conceptually) when any optional field is absent.
- **Invariant:** Always the same bytes `"none"`. Never ambiguous with a real field value (real values are slot expression text like `"5"` or `"0"`).

## 2. Smart Constructor Contracts

### TC-3: `canonical_digest` (both copies)
**Precondition:** `source: &WorkflowSource` — validated, non-empty steps.
**Postcondition:** Returns `WorkflowDigest` that is:
- Deterministic for same AST content
- Sensitive to all Wait fields (`event`, `timeout`)
- Sensitive to WaitUntil vs WaitEvent discrimination
**Invariant:** Same hash as the other copy of `canonical_digest` for the same source.

### TC-4: `digest_step_primitive` — Wait arm (both copies)
**Precondition:** `primitive: &StepPrimitive::Wait { event, timeout }` — validated shape (not both None).
**Postcondition:** The hasher state includes:
- `b"wait_until"` or `b"wait_event"` (discriminator)
- When `event=Some(value)`: `value.as_bytes()`
- When `timeout=Some(value)`: `value.as_bytes()`
- When `timeout=None`: `b"none"`
**Invariant:** The hasher state after processing Wait MUST differ between any two different Wait configurations.

### TC-5: `compute_compiled_digest`
**Precondition:** `source: &[u8]` — raw bytes (including whitespace, comments, etc.)
**Postcondition:** Returns `WorkflowDigest` that always differs from `canonical_digest` for the same logical workflow.
**Invariant:** NOT required to equal `canonical_digest`.

## 3. Type-State Contracts

### TS-1: WorkflowCompilation Lifecycle
```
Unparsed(source: &[u8])
    → Parsed(WorkflowSource)        // after YAML parsing
    → Validated(WorkflowSource)     // after validate_canonical_compile_scope
    → Digesting(WorkflowSource)     // during canonical_digest call
    → Compiled(CompiledWorkflow)    // after compile_source returns Ok
```

### TS-2: Wait Field States
The `Wait { event: Option<String>, timeout: Option<String> }` type allows 4 states, but only 3 are legal:

| State | event | timeout | Legal? | Digest marker |
|-------|-------|---------|--------|---------------|
| EmptyWait | None | None | **NO** — rejected by `validate_wait_shape` | Never reached |
| WaitUntil | None | Some(t) | Yes | `b"wait_until"` + t.as_bytes() |
| WaitEvent (unbounded) | Some(e) | None | Yes | `b"wait_event"` + e.as_bytes() + b"none" |
| WaitEvent (bounded) | Some(e) | Some(t) | Yes | `b"wait_event"` + e.as_bytes() + t.as_bytes() |

## 4. Parser/Deserialization Boundaries

### PB-1: Wait field extraction
**Location:** `part_02.rs:67` — `lower_canonical_step` match arm for `StepPrimitive::Wait`
**Contract:** `event.as_deref()` and `timeout.as_deref()` extract `Option<&str>` from `Option<String>`.
**Boundary:** This is where `Option<String>` from the YAML AST crosses into the compilation pipeline. The strings are slot expression text (not yet resolved to `SlotIdx`).

### PB-2: Wait shape validation
**Location:** `part_03.rs:186` — `validate_wait_shape`
**Contract:** Rejects `(event=None, timeout=None)`. Accepts the 3 legal states.
**Boundary:** Validation happens at YAML parse time, before compilation.

## 5. Railway Error Contracts

### RE-1: Wait Validation Failure
**Error:** `CompileError::StepFieldShape { step, field: "wait", expected: "..." }`
**When:** `validate_wait_shape` rejects invalid wait shape.
**Recovery:** Compilation halts. No `WorkflowDigest` is produced.

### RE-2: Wait Slot Resolution Failure
**Error:** `CompileErrors(vec![...])` from `slot_from_text` or `optional_slot_from_text`
**When:** `lower_canonical_wait` cannot resolve a slot expression text to a valid `SlotIdx`.
**Recovery:** Compilation halts. No `WorkflowDigest` is produced.

### RE-3: Digest Inconsistency (future)
**Error:** (not yet implemented) `CompileError::DigestMismatch` or runtime integrity failure
**When:** `canonical_digest` result differs between cold-path and warm-path compilers for the same source.
**Recovery:** This is the bug being fixed — currently no detection exists.

## 6. Hash Ordering Contract

### HC-1: Wait Digest Order
The hash update order for a Wait step MUST be:
1. Discriminator: `"wait_until"` or `"wait_event"`
2. If `event` is Some: event field bytes
3. If `timeout` is Some: timeout field bytes; if None: `"none"`

**Rationale:** Consistent ordering prevents hash collisions due to field reordering. The discriminator always comes first to distinguish WaitUntil (no event field at all) from WaitEvent.

## 7. Missing Type Safety (Gap Analysis)

| Gap | Current State | Desired State |
|-----|--------------|---------------|
| Wait fields not hashed | `other =>` catch-all hashes only `"wait"` | Exhaustive match arm hashing all fields |
| WaitUntil vs WaitEvent not distinguished | Hashed identically (both hash `"wait"`) | Different discriminators in hash |
| Duplicate code divergence risk | Two copies, no shared definition | Both copies match; refactoring is separate bead |
| Missing proptest coverage | No test verifies different wait conditions → different digests | Proptest: forall a ≠ b ⇒ digest(a) ≠ digest(b) for wait fields |
