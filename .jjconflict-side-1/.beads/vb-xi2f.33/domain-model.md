# Domain Model — vb-xi2f.33: Digest Covers Ask Semantics

## Ubiquitous Language

| Term | Definition |
|------|-----------|
| Canonical digest | A semantic `WorkflowDigest` produced by `canonical_digest()` that MUST be a function of all semantically meaningful workflow fields. Embedded in `WorkflowParts` and recovered at runtime via `CompiledWorkflow::digest()`. |
| Compiled digest | A raw blake3 hash of serialized source bytes, produced by `compute_compiled_digest()`. This is NOT the canonical digest; it is an artifact-level integrity check. |
| Ask primitive | A `StepPrimitive::Ask { prompt: String, timeout: Option<String> }` in the YAML AST. Represents a request for human input at a workflow step. |
| Semantic field | A primitive field whose value changes the meaning or behavior of the workflow. For `Ask`: prompt text and timeout expression. |
| Prompt | Required `String` field of `Ask`: the text displayed to the human user. |
| Timeout | Optional `Option<String>` field of `Ask`: an expression (duration string) limiting how long the engine waits for input. `None` means no timeout. |
| Digest sensitivity | The property that changing a semantic field changes the canonical digest. The current implementation violates this for Ask. |
| Digest determinism | The property that two compilations of the same source produce identical `canonical_digest` results. Relies on blake3 determinism and consistent field ordering. |
| Digest collision | Two semantically different workflows producing the same canonical digest — a violation of the semantic-integrity contract. |
| Workflow substitution | A security hazard where an attacker changes `prompt` or `timeout` without detection because the digest does not change. |
| Duplicate digest path | The `canonical_digest` + `digest_step_primitive` pair exists in two files: `mod_compile_lowering/part_05.rs` (active path) and `compile/mod.rs` (legacy path). Both have the same bug. |

## Entities and Value Objects

### WorkflowSource (Aggregate Root)
- **Type**: `vb_yaml::ast::WorkflowSource`
- **Role**: The parsed YAML source that is the input to `canonical_digest()`.
- **Identity**: Implicit; not stored. The digest serves as the identity surrogate.
- **Invariants**:
  - Contains zero or more steps, each with a unique step ID.
  - Each step has a `StepPrimitive`, which may be `Ask`.

### WorkflowDigest (Value Object)
- **Type**: `vb_core::ids::WorkflowDigest`
- **Role**: A 32-byte blake3 hash identifying a workflow's semantics.
- **Construction**: `WorkflowDigest::from_bytes([u8; 32])` — raw bytes, no validation at this type level.
- **Traits**: `Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize`
- **Invariants**:
  - Produced ONLY by `canonical_digest()` (semantic) or `compute_compiled_digest()` (raw-bytes).
  - A valid canonical digest MUST change when any semantic field of the source changes.

### Ask Fields (Value Objects)
- **Prompt**: `String` — arbitrary human-facing text. Empty string `""` is legal (no text prompt).
- **Timeout**: `Option<String>` — an optional duration expression string. `None` (no timeout) and `Some("")` (empty expression) are semantically distinct from `Some("30s")`.

## Commands

- **ComputeCanonicalDigest(source: WorkflowSource) → WorkflowDigest**: Produces the semantic digest by hashing version, name, trigger fields, step IDs, and each primitive's semantic fields.
- **DigestStepPrimitive(hasher: &mut blake3::Hasher, primitive: StepPrimitive)**: Dispatches per-primitive field hashing into the running hasher.

## Domain Events (Conceptual)

- `DigestComputed { source_name, digest }`: Emitted when `canonical_digest` completes.
- `DigestMismatch { expected, actual, source_name }`: Detected at admission/idempotency time when a runtime digest does not match the stored compiled digest.

## Invariants

- **INV-ASK-001 (Semantic sensitivity)**: For any two `WorkflowSource` values `A` and `B` that differ ONLY in an `Ask { prompt }` or `Ask { timeout }` field, `canonical_digest(A) != canonical_digest(B)`.
- **INV-ASK-002 (Determinism)**: For any `WorkflowSource` `S`, `canonical_digest(S)` produces the same `WorkflowDigest` every time, regardless of compiler instance, process lifetime, or platform.
- **INV-ASK-003 (Empty prompt edge case)**: An ask with `prompt = ""` produces a well-defined digest distinct from any non-empty prompt.
- **INV-ASK-004 (None vs Some("") timeout distinction)**: `timeout: None` and `timeout: Some("")` produce different digest contributions (they are semantically different).
- **INV-ASK-005 (Duplicate implementation parity)**: The active path (`part_05.rs`) and legacy path (`compile/mod.rs`) `canonical_digest` implementations produce identical digests for identical sources (both after the fix).

## Forbidden States

- A `WorkflowDigest` that embeds only the string `"ask"` with no prompt/timeout content (current bug).
- A `canonical_digest` implementation that uses a catch-all arm for `Ask` in `digest_step_primitive`.
- Two active divergent `canonical_digest` implementations that produce different results for the same source (post-fix).

## Open Domain Questions

1. Should the legacy `compile/mod.rs` path be unified (single `canonical_digest`) or kept with parity tests?
2. Should the empty-string sentinel for `None` timeout use `b""` or a distinct sentinel like `b"\0"` to avoid ambiguity?
3. Should `digest_step_primitive` also be fixed for other primitives (`Do`, `Wait`, `Choose`, etc.) in this or a separate bead? (Scope says Ask only.)
4. Should `canonical_digest` hash slot indices or their resolved values? (Resolved values — slot indices are compilation artifacts, not source semantics. However, the Ask primitive at the YAML AST level has pre-resolution string values, not slot indices. Slot indices appear only in the compiler AST.)
