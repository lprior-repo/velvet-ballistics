# Theorem Kernel Projection

## Boundary

- TLA+-owned temporal model: accepted-run ordering, failure-before-ack, and no strict raw bypass.
- Verus-owned Rust core: digest-binding predicates, strict admission witness typing, total pure validation decisions.
- Theorem-owned kernel: none at this time.
- Rust/runtime shell: CLI I/O, Fjall storage calls, runtime construction, shard scheduling, and operator diagnostics.
- External systems excluded from theorem proof: filesystem, Fjall internals, wall-clock time, CLI process behavior, and storage crash behavior beyond the abstract atomic/failing write relation.

## Theorem-Owned Clauses

- None.

## Rationale

The critical properties are either temporal workflow properties better covered by TLA+ or Rust-local predicate/type invariants expressible in Verus. A Lean/Aeneas/Hax theorem kernel would be premature unless `vb-core-accepted-artifact-format` introduces a compact algebraic proof object whose semantics exceed Verus expressiveness.

## Conditional Theorem Trigger

If the accepted artifact format introduces a nontrivial proof lattice for gate counts/capabilities, downstream proof planning may add a tiny theorem kernel with:

- Contract clause: `INV-002` / `ERR-004`.
- Model: abstract artifact proof flags, required gates, granted capabilities, required capabilities.
- Claim: proof acceptance is monotone in valid gates and rejects missing required capabilities.
- Shell exclusions: storage, YAML, runtime scheduling, and CLI I/O.

## Waivers

- THM-WAIVE-001: Lean/Aeneas/Hax not required for this bead at contract time. Owner: State 3 contract. Reason: Verus and TLA+ cover the known critical properties. Expiry: revisit after `vb-core-accepted-artifact-format` closes. Compensating evidence: Verus obligations `VERUS-DIGEST-001`, `VERUS-POLICY-001`, `VERUS-ADMISSION-001`; TLA obligation `TLA-ACCEPT-001`.
