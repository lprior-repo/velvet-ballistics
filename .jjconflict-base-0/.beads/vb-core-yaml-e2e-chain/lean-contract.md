# Theorem Kernel Projection

## Boundary

- TLA+-owned temporal model: lifecycle ordering, strict admission, journal prefix durability, inspect/events projection, crash/restart/recovery, and YAML-free recovery.
- Verus-owned Rust core: digest role predicates, pure mismatch classification, recovery summary determinism, and invariant-preserving pure transitions.
- Theorem-owned kernel: none mandatory at this contract stage.
- Rust/runtime shell: CLI I/O, Fjall I/O, process restart, file paths, wall-clock, storage engine flush semantics, and parser execution.
- External systems excluded from theorem proof: Fjall, OS filesystem, CLI process boundaries, YAML parser implementation, and Postcard runtime decoder internals.

## Theorem-Owned Clauses

- None required now. Verus is the primary Rust-local proof surface for this bead.

## Optional Theorem Obligation

### THM-DIGEST-ROLE-001

- Contract clause: INV-008
- Rust/spec target: digest-role abstraction for source digest vs artifact digest.
- Lean module: BLOCKED until proof-planner/proof-writer decides Verus cannot express the role distinction.
- Theorem shape: source digest role and artifact digest role are not interchangeable in admission/recovery predicates even when represented by the same byte array type.
- Model: abstract `SourceDigest`, `ArtifactDigest`, `SourceBytes`, and `ArtifactBytes` with digest relation predicates.
- Refinement: Rust `WorkflowDigest` values validate into role-tagged abstract digest values at API boundaries.
- Shell exclusions: I/O, YAML parsing, storage, runtime scheduling, Postcard decoding.
- Evidence command: waiver-owned unless a Lean/Aeneas/Hax proof is introduced by proof-planner; current required evidence is `verus verification/verus/yaml_e2e_digest_roles.rs` plus Kani/proptest/E2E shell-linkage obligations.

## Waivers

- Lean/Aeneas/Hax mandatory proof waived for this contract stage.
  - Owner: proof-planner / contract-verification-reviewer.
  - Reason: clauses are expressible as Verus/TLA+ obligations unless downstream proof planning proves otherwise.
  - Expiry: before State 6 retry approval; reviewer must reject if Verus digest-role proof or compensating executable shell evidence is absent.
  - Limitation: waiver does not waive Verus, Kani, proptest, or E2E obligations for executable Rust behavior.
  - Compensating evidence: `verus verification/verus/yaml_e2e_digest_roles.rs`, `cargo kani -p vb_runtime --harness yaml_e2e_admission_matrix` after harness integration, storage/runtime corruption tests, and CLI/recovery E2E evidence.
