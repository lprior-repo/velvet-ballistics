# Domain Model Review

Bead: `vb-qi37.4.2`

## Decision

STATUS: READY_FOR_CONTRACT_REVIEW

The contract model separates four concepts that must not be conflated:

1. Artifact existence: a digest has bytes in storage.
2. Accepted envelope validity: bytes decode as accepted-artifact v1 with required gates.
3. Admission authorization: envelope profile exactly matches runtime requirements.
4. Run allocation: runtime state may be created only after 1-3 pass.

## Illegal States Made Explicit

- A raw `WorkflowParts` artifact cannot be an accepted artifact.
- A relaxed artifact with `gate_count == 0` cannot enter strict runtime admission.
- A storage-submitted artifact with `gate_count == 2` cannot enter strict runtime admission while runtime requires `15`.
- A digest-mismatched envelope cannot be admitted even if it decodes.
- A denied admission cannot have `RunAccepted`, runnable state, or allocated frame.
- `AlwaysPresentArtifactStore` cannot prove strict production admission.

## Required Type Boundaries

- `AcceptedArtifact` must be a distinct validated domain type, not bytes plus convention.
- `AdmissionRecord` must be created only from validated accepted artifacts.
- `AdmissionError` must preserve semantic rejection causes.
- Runtime constructors must distinguish relaxed/test admission from strict/journaled production admission.

## Review Risks

- Gate-count disagreement between `vb_storage::ADMISSION_GATE_COUNT == 2` and `vb_runtime::REQUIRED_GATE_COUNT == 15` is a blocking domain ambiguity for implementation unless normalized.
- Current diagnostics may be too coarse if `AdmissionArtifactInvalid` cannot report raw/malformed/stale/digest mismatch distinctly.
- Existing IPC resolver behavior that decodes `record.ir` as `WorkflowParts` conflicts with storage persisting `AcceptedArtifact` bytes.

## Hand-off

- Proof planner must either extend the existing capability Verus/TLA assets or create separate accepted-envelope models.
- Implementation must not use existence-only APIs as strict admission proof.
