# Theorem Kernel Projection: vb-scxh

## Boundary

- TLA+-owned model: temporal recovery workflow and non-laundering evidence transitions.
- Verus-owned Rust core: none in current bead scope; no production classifier implementation exists.
- Theorem-owned kernel: optional future finite evidence lattice only.
- Runtime shell excluded: BD CLI, git bundle/bookmark, filesystem, CI, report parsing, subagent orchestration.

## Optional theorem-owned clauses

- THM-SCXH-001: evidence lattice theorem that `Subagent` is never sufficient for a required evidence clause.
- THM-SCXH-002: mutation algebra theorem that `FailUnviable` never implies adequacy pass.

## Current decision

Lean/Aeneas/Hax are not required for this State 3 repair because there is no existing Lean project/module and the primary risk is temporal evidence workflow. This is not a proof of theorem coverage; it is an explicit waiver/defer recorded in `proof-obligations.jsonl`.

## Waiver

- WAIVE-LEAN-SCXH-001: Owner State 4/6. Reason: no theorem-kernel target exists; TLA+ plus raw evidence audit better covers this bead. Expiry: before any claim that evidence classification is theorem-proven or before a pure classifier theorem target is introduced. Compensating evidence: TLA+ safety model, primary raw-evidence audits, Truth Serum final decision.
