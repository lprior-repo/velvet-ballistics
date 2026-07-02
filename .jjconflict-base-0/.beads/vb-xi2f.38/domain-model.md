# Domain Model: Digest Covers Collect Semantics (vb-xi2f.38)

## Ubiquitous Language

| Term | Type | Definition |
|------|------|------------|
| `WorkflowDigest` | Value Object | 32-byte BLAKE3 hash of workflow source content; serves as content-addressed identity for admission, persistence, replay, and recovery |
| `Source Digest` | Concept | Stage-1 digest computed from YAML AST (`canonical_digest`); must capture all semantically significant fields including `Collect` primitive parameters |
| `Artifact Digest` | Concept | Stage-2 digest computed from serialized `WorkflowParts`; depends on source digest being correct |
| `StepPrimitive::Collect` | Entity Variant | Paginated collection loop with: `variable` (loop var name), `source` (input expression), `pages` (max pages), `items` (page size), `body` (step list) |
| `CollectStart` | IR Node Kind | First node emitted for collect; carries `source` slot, `limit` (=pages), `page_size` (=items), `body` step, `done` step |
| `CollectPage` | IR Node Kind | Re-entry node for body processing; carries `collector_slot`, `body` step, `done` step |
| `CollectFinish` | IR Node Kind | Terminal node; carries `collector_slot` |
| `CollectPaginationState` | Runtime State | Carries `source` list ID, `page` cursor, `limit`, `page_size` for iteration |
| `digest_step_primitive` | Function | Hashes a `StepPrimitive` into the running BLAKE3 hasher |
| `canonical_primitive_name` | Function | Maps `StepPrimitive` variant to its canonical string name (e.g., `"collect"`) |

## Core Entities and Value Objects

### WorkflowDigest
- **Type**: Newtype wrapper around `[u8; 32]`
- **Construction**: `WorkflowDigest::from_bytes(blake3::hash(data).into())`
- **Invariant**: Two `WorkflowDigest` values that are equal MUST represent byte-for-byte identical source content
- **Forbidden states**: A digest value that was computed from content A but equals a digest computed from content B (digest collision)

### StepPrimitive
- **Type**: Non-exhaustive enum with 12 variants
- **Digest-relevant fields by variant**:
  - `Set { output, value }` — both fields hashed
  - `Finish { result }` — result value hashed
  - `Collect { variable, source, pages, items, body }` — ALL fields MUST be hashed (CURRENT BUG: only name `"collect"` is hashed)
  - `ForEach { variable, input, at_once, body }` — ALL fields SHOULD be hashed (same catch-all bug risk)
  - `Aggregate { variable, input, initial, body }` — ALL fields SHOULD be hashed
  - `Together { branches }` — branch content SHOULD be hashed
  - `Choose { branches, otherwise }` — branch content SHOULD be hashed
  - `Repeat { max_attempts, body }` — both fields SHOULD be hashed
  - `Do { action, input }` — both fields SHOULD be hashed
  - `Save { value }` — value SHOULD be hashed
  - `Wait { event, timeout }` — both fields SHOULD be hashed
  - `Ask { prompt, timeout }` — both fields SHOULD be hashed

### WorkflowSource
- **Type**: Top-level AST produced by YAML parser
- **Fields**: `version`, `name`, `trigger`, `inputs`, `vars`, `secrets`, `steps`, `result`, `examples`
- **Digest input**: version + name + trigger + each step's `id` + each step's `primitive` (via `digest_step_primitive`)

### StepAst
- **Type**: Single workflow step in YAML AST
- **Fields**: `id` (unique identifier), `name`, `condition`, `primitive`, `with`, `retry`, `on_error`, `then`
- **Digest input**: `id` is hashed per step; `primitive` is hashed via `digest_step_primitive`

## Key Invariants

1. **Digest Determinism**: `canonical_digest(source_a) == canonical_digest(source_b)` iff `source_a` and `source_b` are semantically identical for all workflow execution behavior
2. **Collect Field Coverage**: If two workflows differ only in `Collect.variable`, `Collect.source`, `Collect.pages`, `Collect.items`, or `Collect.body`, their digests MUST differ
3. **Step ID Uniqueness**: A `WorkflowSource` with duplicate step IDs is a validation error, not a digest concern
4. **Digest Collision Resistance**: BLAKE3-256 provides collision resistance; workflow digests are content-addressed keys

## Forbidden States (Made Unrepresentable)

1. A `WorkflowDigest` with no backing content (all zeros is allowed as sentinel only)
2. A `Collect` primitive with `pages == 0` or `items == 0` (validation rejects these)
3. An empty `body` in `Collect` (validation requires at least one body step)
4. A `Collect` with `source` that evaluates to a non-list type at runtime (typed as error in `InvalidCollect`)

## Commands / Operations

| Operation | Input | Output | Errors |
|-----------|-------|--------|--------|
| `canonical_digest(source)` | `WorkflowSource` | `WorkflowDigest` | None (pure) |
| `digest_step_primitive(hasher, primitive)` | BLAKE3 hasher + `StepPrimitive` | (mutates hasher) | None (pure) |
| `compute_compiled_digest(source_bytes)` | `&[u8]` | `WorkflowDigest` | None (pure) |
| `compile_workflow(source)` | `&[u8]` YAML | `CompiledWorkflow` | `CompileErrors`, `ValidationError` |

## Events / Emissions

The digest computation does not emit events. It is a pure hashing function.

## Policies

1. **Digest Policy**: Source digest is computed before any compilation/transformation; it is the content-addressed identity of the workflow definition
2. **Admission Policy**: Storage admission checks that submitted artifact bytes produce the claimed `WorkflowDigest`
3. **Replay Policy**: Replay uses `WorkflowDigest` to look up the original artifact; if artifact bytes don't match digest, replay fails-closed

## Aggregate Boundaries

- **WorkflowSource** (YAML AST) → **CompiledWorkflow** (IR) → **WorkflowParts** (serialized artifact)
- Digest flows forward: source digest is embedded in `CompiledWorkflow.digest` and `WorkflowParts.digest`
- The artifact digest (BLAKE3 of serialized `WorkflowParts`) is a function of the source digest and all IR

## Related Domains

- **Compilation**: `vb_compile` — `canonical_digest` computes source digest; `compute_compiled_digest` computes artifact digest
- **Validation**: `vb_validate` — validates `Collect` shape; emits `ValidationError::InvalidCollect`
- **Runtime**: `vb_runtime` — executes `CollectStart/Page/Finish` nodes; carries `CollectPaginationState`
- **Storage**: `vb_storage` — uses `WorkflowDigest` for admission; artifact digest must match claimed digest
- **Core**: `vb_core` — defines `WorkflowDigest`, `CompiledWorkflow`, `WorkflowParts`, `CompiledNodeKind::Collect*`
