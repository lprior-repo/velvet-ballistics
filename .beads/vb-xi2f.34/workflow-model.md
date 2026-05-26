# Workflow Model — vb-xi2f.34: Finish Digest Semantics

**Bead**: vb-xi2f.34
**Date**: 2026-05-24
**Rust Contract Agent**: rust-contract

---

## States

The digest computation is a pure, stateless function. The relevant state machine is the **digest lifecycle** from YAML source to persisted artifact.

### State 1: Source Parsed (AST)
- **Entry**: YAML bytes → `WorkflowSource` (via `vb_yaml::parse_workflow_source`)
- **Data**: `WorkflowSource { version, name, trigger, steps }`
- **Invariant**: All steps are structurally valid per YAML schema. Step order is as-authored.

### State 2: Digest Computed
- **Entry**: `WorkflowSource` → `canonical_digest()` → `WorkflowDigest`
- **Transition**: Pure function; no side effects. Deterministic.
- **Data**: `WorkflowDigest([u8; 32])`
- **Invariants**: 
  - Deterministic (same source → same digest).
  - Covers version, name, trigger, step IDs, step primitives (including Finish result).
  - Does NOT depend on slot layout, constant pool, or bytecode.

### State 3: Lowered to IR (WorkflowParts)
- **Entry**: `WorkflowSource` + `WorkflowDigest` → lowered nodes → `WorkflowParts`
- **Transition**: `compile_source()` calls `canonical_digest()` at line 46 (canonical) or line 97 (legacy), then lowers each step.
- **Data**: `WorkflowParts { digest, nodes, expressions, ... }`
- **Guard**: `validate_parts()` checks slot bounds, reachability, forward edges.
- **Invariant**: `WorkflowParts.digest` equals the value computed in State 2.

### State 4: Validated (CompiledWorkflow)
- **Entry**: `WorkflowParts` → `validate_parts()` + `validate_budget()` → `CompiledWorkflow::try_from_parts()`
- **Transition**: `try_from_parts()` validates all numeric references (slots, steps, expressions, accessors). Budget is checked against `ResourceContract`.
- **Data**: `CompiledWorkflow { digest, nodes, ... }`
- **Invariant**: `digest` field is preserved from `WorkflowParts.digest`.
- **Terminal**: This is a terminal state for compilation. The `CompiledWorkflow` is immutable.

### State 5: Persisted / Recovered
- **Entry**: `CompiledWorkflow` → Postcard serialization → Fjall persistence
- **Transition**: The workflow artifact is stored with its digest as a key or metadata.
- **Invariant**: Deserialization produces identical `CompiledWorkflow` with identical `digest`.

---

## Transitions

### T1: Source → Digest (pure)

```
canonical_digest(source: &WorkflowSource) -> WorkflowDigest
```

**Guard**: `source` is a valid parsed AST (caller responsibility).
**Effect**: Hashes all digest-covered fields into a `blake3::Hasher` and finalizes.
**Failure modes**: None. The function is infallible.
**Determinism guarantee**: Same input → same output. blake3 is deterministic per specification.

### T2: Digest → WorkflowParts (compile)

```
compile_source(source: &WorkflowSource) -> Result<CompiledWorkflow, CompileErrors>
```

**Guard**: `validate_canonical_compile_scope(source)` passes.
**Steps**:
1. Compute `digest = canonical_digest(source)`
2. Lower each step to `CompiledNode`
3. Pack into `WorkflowParts { digest, ... }`
4. Validate via `vb_validate::shared::validate(&parts)`
5. Construct `CompiledWorkflow::try_from_parts(parts)`

**Failure modes**:
- `UnsupportedStepPrimitive` — step uses a primitive not handled by the compiler path
- `StepFieldShape` — invalid field types
- `SlotIndexOutOfRange` — slot index exceeds u16 range
- `UnknownOutputName` — finish result name not in output map
- Validation failures from `validate_parts()` — slot bounds, reachability, forward edges
- Budget failures from `validate_budget()`

**Key**: Digest is computed BEFORE lowering and BEFORE validation. This means digest computation sees the raw AST only, not the validated IR.

### T3: Finish Result Resolution (within T2)

```
canonical_finish_slot(result: &ScalarValue, outputs: &HashMap<String, SlotIdx>) -> Result<SlotIdx, CompileErrors>
```

**Guard**: Finish step is last step (checked by caller).
**Effect**: 
- `String(name)` → lookup in `outputs` map → `SlotIdx`. Fails with `UnknownOutputName` if not found.
- `Integer(value)` → `u16::try_from(*value)` → `SlotIdx::new(raw)`. Fails with `SlotIndexOutOfRange` if value doesn't fit in u16.
- Other variants → fails with `UnsupportedConstantValue`.

**This resolution does NOT affect the digest.** The digest hashes the `ScalarValue` directly (String value or Integer LE bytes), not the resolved `SlotIdx`.

---

## Guards

| Guard | Where | What it prevents |
|---|---|---|
| `validate_canonical_compile_scope()` | `compile_source()` entry | Rejects unsupported top-level declarations (inputs, outputs, etc.) |
| `index != last` check | `lower_canonical_finish()` | Rejects Finish at non-terminal position |
| `canonical_finish_slot()` | During finish lowering | Rejects unknown output names, out-of-range slot indices |
| `validate_parts()` | Before `try_from_parts()` | Rejects invalid slot references, unreachable nodes, backward edges |
| `validate_budget()` | After `validate_parts()` | Rejects workflows exceeding resource bounds |

---

## Outcomes

### Success
`CompiledWorkflow` with a valid `digest: WorkflowDigest` that correctly fingerprints the source semantics.

### Semantic Errors (no IR produced)
- `CompileErrors` — compilation fails before `WorkflowParts` is constructed.
- `WorkflowError` — validation fails; no `CompiledWorkflow` produced.

### Terminal States
- `CompiledWorkflow` — immutable, ready for admission/runtime.
- `Err(CompileErrors)` — compilation rejected; source must be fixed.
- `Err(WorkflowError)` — IR rejected; impossible state reached.

---

## Temporal Hazards

### HZ-1: Digest computed before IR lowering
The digest is computed from the AST in State 2, but the `CompiledWorkflow` is constructed in State 4 with `try_from_parts()`. If lowering changes the semantics of any step in a way not captured by the digest, the digest becomes stale. **Specifically**: Finish result resolution (`canonical_finish_slot`) happens AFTER digest computation, but the digest hashes `ScalarValue` (not `SlotIdx`). This is correct by design — the digest captures source intent, not IR layout.

### HZ-2: Duplicate code temporal drift
If the canonical path (`part_05.rs::canonical_digest()`) is updated and the legacy path (`mod.rs::canonical_digest()`) is not, any code using the legacy path will produce different digests. Since the legacy path is used only by proptest helpers, the blast radius is limited to test assertions, not production.

### HZ-3: Version field divergence
`canonical_digest()` hashes `source.version().as_bytes()`. If the YAML parser normalizes or transforms the version string (e.g., trimming, case-folding), two syntactically different but semantically equivalent version strings would produce different digests. Current implementation hashes the raw string from the AST.

### HZ-4: Step ordering
Steps are hashed in source order. Reordering steps (even if semantically equivalent) produces a different digest. This is intentional — step order is significant in the workflow model.

---

## Retry / Idempotence

- `canonical_digest()` is idempotent: calling it multiple times with the same `WorkflowSource` always returns the same digest.
- There are no retry paths in the digest computation — it is a pure, infallible function.
- The compile pipeline (`compile_source()`) is not idempotent in the presence of errors — failed validation does not produce a partial artifact. Retry requires a fixed source.

---

## Cancellation

- Digest computation is synchronous and cannot be cancelled mid-computation (no async, no IO).
- Compilation can be cancelled by the caller (drop the future/thread), but no partial state is persisted.
