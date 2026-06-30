# Domain Model Review: vb-f04l

## Review Boundary

- Reviewed domain model only; no production code, tests, or proofs were written.
- Inputs: bead JSON, State2 artifacts, and isolated workspace source reads.
- Primary domain seam: `vb_yaml::ast::WorkflowSource` to `vb_core::CompiledWorkflow` through `vb_compile::compile_source`.

## Domain Alignment

- The canonical AST model is source-shaped and human-authored: nested bodies, branch labels, loop variables, event/prompt strings, and scalar values.
- The runtime IR model is numeric and machine-shaped: dense `StepIdx`, `SlotIdx`, `ConstIdx`, `ActionId`, and `CompiledNodeKind` variants.
- The missing domain service is not parsing or runtime execution. It is a lowering planner that maps source-shaped nested primitive structure into validated numeric graph structure.

## Correct Aggregate Boundary

- Aggregate root: `WorkflowSource` during input; `WorkflowParts` during construction; `CompiledWorkflow` after validation.
- Value objects: `StepIdx`, `SlotIdx`, `ConstIdx`, `WorkflowDigest`, source step IDs, output names, branch labels.
- Domain service: deterministic primitive lowering and nested body expansion.
- Repository/external concerns excluded: file system, YAML byte parsing after `WorkflowSource`, runtime journal/storage, generated Rust, HTTP, JSON.

## Illegal States To Make Unrepresentable Or Reject

- Empty workflow accepted by lowerer.
- Duplicate source step IDs in top-level or nested scopes.
- Body/done/join/resume target outside emitted nodes.
- Node IDs that differ from array position.
- Slot references not covered by `slot_count`.
- Valid v1 control primitive reported as unsupported.
- Together branch count that overflows `u16`.
- Repeat attempt count or generated resume/slot index that overflows target widths.
- Wait with neither event nor timeout/deadline semantics clearly selected.
- Ask without answer slot/resume route.

## Type-Driven Design Recommendations For Implementation State

- Introduce a lowering-plan domain type before emitting `CompiledNode` values. The plan should carry next node index, slot allocation, named outputs, and emitted node intents.
- Distinguish source labels from numeric targets with separate newtypes or private constructors.
- Distinguish `WaitUntilSpec` from `WaitEventSpec`; do not encode wait shape with booleans.
- Use fallible checked allocation functions for `StepIdx`, `SlotIdx`, `ConstIdx`, and branch counts.
- Represent nested body expansion with an explicit result containing `entry`, `exit`, emitted nodes, and written outputs.
- Preserve `WorkflowParts` validation as the final aggregate invariant gate.

## Accepted Existing Model Pieces

- `vb_yaml::ast::StepPrimitive` already has canonical variants for every target primitive.
- `vb_core::CompiledNodeKind` already has runtime variants for loop/fanout/collect/reduce/repeat/wait/ask families.
- `vb_validate::shared::validate` and `CompiledWorkflow::try_from_parts` already form the right validation closure.
- `WaitKind` in `vb_compile` is a good domain split for wait shape.

## Model Risks

- `lower_for_each`, `lower_together`, and `lower_repeat` comments mention node families not fully emitted by current helpers; implementation must either justify omitted variants with tests/proofs or emit them.
- Source expressions are currently strings, while IR requires slots/constants/expressions; expression-to-slot policy is a separate high-risk seam.
- Recursive body lowering can easily introduce off-by-one targets, duplicate IDs, hidden non-dense nodes, or unreachable done/join nodes.
- Existing low-level legacy AST lowering can mask canonical-source gaps; tests must enter through canonical `WorkflowSource`/`compile_workflow` paths.

## Review Verdict

STATUS: READY_FOR_CONTRACT_REVIEW

The domain model is coherent if implementation treats lowering as a deterministic graph-planning domain service and rejects every shape it cannot map safely. The current model is not yet safe to implement by direct ad-hoc node pushes without a typed lowering plan, because nested bodies and synthetic targets need explicit invariants.
