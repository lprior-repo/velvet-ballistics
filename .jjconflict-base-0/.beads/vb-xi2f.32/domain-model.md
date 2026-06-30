# Domain Model: Wait Digest Coverage

**Bead:** vb-xi2f.32
**Date:** 2026-05-24
**State:** rust-contract (State 3)
**Schema:** domain-model/v1

## 1. Ubiquitous Language

| Term | Definition |
|------|-----------|
| **Workflow Digest** | A 32-byte blake3 cryptographic hash that uniquely identifies a compiled workflow. Used by the engine for identity, idempotency, and integrity verification. |
| **Wait Primitive** | A YAML step that suspends workflow execution until either a deadline is reached (`wait.until`) or an external event arrives (`wait.event`) with an optional timeout. |
| **WaitUntil** | A wait shape with only a `timeout` (deadline) field. The step suspends until the deadline slot is satisfied. No event is expected. |
| **WaitEvent** | A wait shape with an `event` field (and optional `timeout`). The step suspends until the named event slot is satisfied, or the timeout expires if provided. |
| **canonical_digest** | The internal function that hashes a `WorkflowSource` AST into a `WorkflowDigest`. Used by the cold-path compiler (`compile_source` in `part_01.rs`) and the warm-path compiler (`compile_workflow` in `compile/mod.rs`). |
| **compute_compiled_digest** | The public API function that hashes raw source bytes (`&[u8]`) into a `WorkflowDigest`. Always sensitive to source changes but produces a different hash than `canonical_digest` for the same workflow. |
| **digest_step_primitive** | The per-step hashing function dispatched by `canonical_digest`. Currently hashes only the step ID and primitive type name; ignores fields for most primitives including Wait. |
| **WaitKind** | A type-safe discriminator enum in `part_07.rs`: `Until { deadline }` vs `Event { event, timeout }`. Replaces a historical `is_event: bool` flag. |
| **SlotIdx** | A newtype index into the slot array. Both wait event references and timeout references resolve to `SlotIdx` values during compilation. |

## 2. Core Entities

### WorkflowDigest (Identity)
- **Type:** `#[repr(transparent)] struct WorkflowDigest([u8; 32])`
- **Traits:** `Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize`
- **Invariants:**
  - Always 32 bytes of blake3 output
  - Must be deterministic for the same compiled IR
  - Must differ when semantically distinct workflows produce semantically distinct IR
- **Lifecycle:** Created at compile time via `canonical_digest()` or `compute_compiled_digest()`. Consumed at runtime for identity checks.

### Wait (AST StepPrimitive variant)
- **Type:** `StepPrimitive::Wait { event: Option<String>, timeout: Option<String> }`
- **Legal shapes** (enforced by `validate_wait_shape`):
  - `(event=None, timeout=Some)` → WaitUntil (deadline wait)
  - `(event=Some, timeout=None)` → WaitEvent (unbounded event wait)
  - `(event=Some, timeout=Some)` → WaitEvent with timeout (bounded event wait)
- **Illegal shape:** `(event=None, timeout=None)` — rejected by validation

### WaitKind (Compile-Time Discriminator)
- **Type:** `enum WaitKind { Until { deadline: SlotIdx }, Event { event: SlotIdx, timeout: Option<SlotIdx> } }`
- **Purpose:** Replaces `is_event: bool` to make illegal states (e.g., `is_event=false` with a timeout slot) unrepresentable.

### CompiledNodeKind::WaitUntil / WaitEvent (Runtime IR)
- **WaitUntil:** `{ deadline_slot: SlotIdx }`
- **WaitEvent:** `{ event: SlotIdx, timeout_slot: Option<SlotIdx> }`
- These are the runtime representations after compilation.

## 3. Value Objects

| Value Object | Type | Constraints |
|-------------|------|------------|
| `DigestHash` | `[u8; 32]` wrapped in `WorkflowDigest` | 32 bytes of blake3 output |
| `WaitEventField` | `Option<String>` from YAML | Valid slot expression or None |
| `WaitTimeoutField` | `Option<String>` from YAML | Valid slot expression or None |
| `SlotIdx` | newtype `usize` | Bounded to `slot_count` |
| `StepIdx` | newtype `usize` | Bounded to step count |

## 4. Aggregates

### CompiledWorkflow (Root Aggregate)
- **Fields:** name, digest (`WorkflowDigest`), slot_count, nodes, expressions, accessors, constants, entry, resource_contract, step_names
- **Invariant:** `digest` must be cryptographically bound to the semantic content of the workflow. Changing a step's wait fields (event, timeout) must change the digest.
- **Created by:** `compile_source()` (cold-path) or `compile_workflow()` (warm-path)
- **Identified by:** `WorkflowDigest`

## 5. Commands

| Command | Description | Entry Point |
|---------|------------|-------------|
| `canonical_digest(source)` | Compute semantic digest of parsed WorkflowSource | `part_05.rs:116`, `compile/mod.rs:220` |
| `digest_step_primitive(hasher, primitive)` | Hash individual step's semantic content into digest | `part_05.rs:140`, `compile/mod.rs:243` |
| `compute_compiled_digest(source_bytes)` | Hash raw source bytes | `mod_compile_core.rs:114` |

## 6. Events (Compilation Lifecycle)

| Event | When | Consequence |
|-------|------|------------|
| `SourceParsed` | YAML → WorkflowSource | AST in memory |
| `ValidationCompleted` | validation passes | Source is well-formed |
| `DigestComputed` | canonical_digest called | WorkflowDigest created |
| `WorkflowCompiled` | compile_source produces CompiledWorkflow | Parts assembled with digest |
| `DigestMismatchDetected` | Runtime integrity check fails | Execution rejected |

## 7. Domain Invariants

### DI-1: Semantic Sensitivity
**The `canonical_digest` of a `WorkflowSource` MUST differ when any wait field (`event` or `timeout`) differs between two sources that are otherwise identical.** Two workflows with different wait conditions (different event slots, different timeout slots, WaitUntil vs WaitEvent) must produce different digests.

### DI-2: Digest Determinism
**The same workflow source MUST always produce the same `canonical_digest`.** No time, randomness, or external state may affect the digest.

### DI-3: Digest Integrity
**Once assigned, the `WorkflowDigest` must never change for the lifetime of a `WorkflowParts` or `CompiledWorkflow`.** The digest is copied, not recomputed.

### DI-4: Empty Wait Invalid
**`Wait { event: None, timeout: None }` is an illegal state.** It is rejected by validation before compilation, so `digest_step_primitive` never encounters it.

### DI-5: WaitUntil vs WaitEvent Discrimination
**The digest must distinguish `WaitUntil` (no event, only timeout/deadline) from `WaitEvent` (event with optional timeout).** These produce different IR nodes (`WaitUntil` vs `WaitEvent`) and different runtime behavior.

## 8. Forbidden States

| State | Why Illegal | Guard |
|-------|------------|-------|
| `Wait { event: None, timeout: None }` | No wait condition specified | `validate_wait_shape` rejects at validation boundary |
| `canonical_digest(A) == canonical_digest(B)` when A and B have different wait fields | Digest collision — violates DI-1 | **CURRENTLY BROKEN** — no guard exists |
| `digest_step_primitive` hashing only `"wait"` string for Wait | Fields lost — violates DI-1 | **CURRENTLY BROKEN** — catch-all arm in both copies |
| One copy of `canonical_digest` fixed but the other not | Digest divergence between cold-path and warm-path compilers | **CURRENTLY BROKEN** — duplicate code |
| `canonical_digest` result equals `compute_compiled_digest` result | These are different algorithms and must not collide | Design choice — no equality required |

## 9. Domain Decisions (Open Questions from codebase-map.md §10)

### DD-1: Digest Unification
**Should `canonical_digest` and `compute_compiled_digest` produce the same result?**
- **Current:** They differ. `compute_compiled_digest` hashes raw bytes; `canonical_digest` hashes semantic AST content.
- **Impact:** If unified, existing persisted artifacts with old digests would be orphaned.
- **Recommendation for this bead:** Do not unify. The bead scope is wait digest coverage, not digest unification.

### DD-2: Broader Digest Gap
**Should other primitives (Ask, Do, Save, etc.) be fixed in this bead?**
- **Current:** All primitives except Set and Finish fall through to the name-only catch-all.
- **Impact:** The bead scope is "digest covers wait semantics" — Wait only.
- **Decision:** Fix Wait only in this bead. File a follow-up bead for broader coverage.

### DD-3: Duplicate Code Convergence
**Should the two copies of `canonical_digest`/`digest_step_primitive` be unified?**
- **Current:** Identical logic in `mod_compile_lowering/part_05.rs` (cold-path) and `compile/mod.rs` (warm-path).
- **Impact:** Any fix must be applied to BOTH copies or the two compiler paths will diverge.
- **Decision:** Fix both copies identically in this bead. Refactoring into a shared module is a separate concern.

### DD-4: Null Marker Semantics
**How should absent `event` (i.e., WaitUntil) be represented in the digest?**
- **Options:** (a) hash a "none" marker, (b) hash an "until" marker, (c) hash only "wait" + timeout.
- **Decision:** Hash `"wait"` + discriminator + field value(s). For WaitUntil (event=None), hash an explicit `"wait_until"` discriminator. For WaitEvent (event=Some), hash `"wait_event"` + event value. This ensures WaitUntil ≠ WaitEvent with identical timeout strings.

### DD-5: Absent Timeout Representation
**How should absent `timeout` (i.e., unbounded WaitEvent) be represented in the digest?**
- **Decision:** Hash a sentinel marker (e.g., `"none"`) for absent timeout. This ensures a WaitEvent with timeout=5s differs from a WaitEvent with no timeout.
