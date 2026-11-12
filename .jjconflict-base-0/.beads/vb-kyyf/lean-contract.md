# vb-kyyf Theorem Kernel Projection

## Boundary
- TLA+-owned temporal model: replay/recovery attempts, digest mismatch transitions, side-effect non-reexecution, and generated/IR observation convergence.
- Verus-owned Rust core: pure normalization and normalized observation comparison.
- Theorem-owned kernel: none at contract time.
- Rust/runtime shell: CLI invocation, Fjall I/O, action dispatch adapters, runtime shard execution, generated Rust compilation/execution.
- External systems excluded from theorem proof: filesystem paths, wall-clock time, process ids, Fjall internals, CLI rendering, compiler execution.

## Theorem-Owned Clauses
- None.

## Non-Applicability Rationale
Lean/Aeneas/Hax is not the right first proof layer for this bead because the critical mathematical kernels are small and Rust-local:
- allowed-normalization whitelist,
- normalized observation equality/rejection,
- journal signature equality and monotonic/contiguous checks.

These are expressible in Verus with abstract models and refinement wrappers. TLA+ owns the temporal replay/recovery policy. Introducing Lean would add proof surface without a theorem that cannot be handled by Verus/TLA+.

## Conditional Escalation
Escalate to Lean/Aeneas/Hax only if contract review identifies a tiny algebraic theorem beyond Verus, such as a canonicalization uniqueness proof for normalized observation digests. If escalated, theorem scope must exclude I/O, CLI, async/runtime shell, and Fjall internals.

## Waivers
- THM-WAIVER-001: No theorem-kernel obligation for State 3. Owner: proof-planner/proof-reviewer. Expiry: before implementation consumes contract. Compensating evidence: Verus obligations for normalization/comparison plus TLA+ model for replay temporal behavior.
