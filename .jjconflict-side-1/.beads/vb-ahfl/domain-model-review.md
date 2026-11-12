# Domain Model Review: vb-ahfl

## Scope Reviewed

- Input artifacts: `codebase-map.md`, `delivery-scope.jsonl`, `baseline-report.md`, and bead JSON from `bd --db /home/lewis/src/velvet-ballistics/.beads/dolt show vb-ahfl --json`.
- State 2 domain: UI artifact schema bounds and CLI/UI parity.
- Conflict: bead JSON title/description points to engine YAML-to-IR semantic evidence. State 3 records this as `UiArtifactError::ScopeConflict` and OQ-001.

## Ubiquitous Language Check

- `schema_version`: version of the serialized UI artifact contract, not crate version.
- `kind`: artifact family discriminator used by CLI and UI model canonicalization.
- `generated_at`: artifact production timestamp; must be represented deterministically in canonical comparison if time differs by source.
- `source`: provenance of the artifact, such as CLI command, runtime inspection source, or verification report source.
- `redaction_status`: explicit public/redacted/mixed/unknown state; absent status is invalid.
- `bounded collection`: data that cannot grow unbounded before render or JSON/JSONL emission.
- `canonical form`: stable semantic comparison object for CLI/UI parity; not raw JSON text.

## Aggregate Boundaries

1. Universal metadata aggregate
   - Owns required fields for every artifact.
   - Prevents missing schema/kind/source/redaction state.
2. Workflow graph aggregate
   - Owns nodes, edges, graph identity, and packet/taint path flags.
   - Prevents dangling step references and graph/event identity drift.
3. Run events aggregate
   - Owns event rows, sequence ordering, run id, status, evidence digest, and attempts.
   - Prevents unordered or untraceable event rows.
4. Redacted value aggregate
   - Owns taint, digest, summary, and redaction status only.
   - Prevents raw secret serialization.
5. CLI/UI parity aggregate
   - Owns canonicalization relation and equality result.
   - Prevents format-only or string-only comparison from masking schema drift.

## Illegal States To Make Unrepresentable

- Artifact without universal metadata.
- Artifact with `kind` that does not match its typed family.
- UI-facing collection with neither maximum bound nor cursor/truncation metadata.
- Redacted/secret-sensitive artifact carrying raw secret text.
- Workflow edge referencing a node not present in the graph.
- Event row with `step_idx` not mapped to a known step identity when a graph context exists.
- CLI artifact and UI artifact claimed equal without canonicalization evidence.
- `vb_ui_model` depending on Makepad, hot runtime internals, async runtime, HTTP, YAML parsing, or execution mutation.

## Type Model Recommendations For Later States

- Introduce a single universal metadata type or trait-equivalent adapter before adding fields ad hoc.
- Prefer bounded newtypes for artifact collections rather than raw `Vec` in render-facing structs.
- Use explicit truncation metadata when clipping is acceptable; otherwise return typed errors.
- Model redaction as a sum type where raw secret content is not a variant available to serialized UI views.
- Model canonicalization as a pure function from CLI/UI artifacts to a shared canonical data shape.
- Keep Makepad consumption one-way: Makepad reads typed UI models; UI model does not import Makepad.

## Verification Fit

- Verus: pure metadata, bounds, redaction projection, graph reference, event ordering, and canonicalization invariants.
- Kani/proptest: bounded state exploration for malformed metadata, oversized collections, graph references, and parity differences.
- Static scan/clippy: dependency boundary and zero-panic/no-unsafe constraints.
- API compatibility: public `vb_ui_model` type changes and CLI JSON/JSONL compatibility.
- TLA+: not primary unless implementation turns parity into an asynchronous lifecycle.
- Lean: not justified; Verus is enough for this domain model.

## Review Verdict

STATUS: REPAIRED_SCOPE_EXPLICIT

The State 2 UI model domain is coherent and contractable. `BLOCKER-SCOPE-001` is resolved for this repaired State 3 stack by explicitly accepting `.beads/vb-ahfl/delivery-scope.jsonl` as the contract scope. Engine YAML-to-IR semantic evidence is outside this domain model and requires regenerated State 2/3/4/5 artifacts if selected by the owner/orchestrator.
