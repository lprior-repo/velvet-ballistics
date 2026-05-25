State: 3 (rust-contract — COMPLETE)

## State 2: Explore — COMPLETE
- Confirmed digest coverage gap for for_each
- Mapped all relevant source files, tests, risks

## State 3: Rust-Contract — COMPLETE

### Artifacts Written

| Artifact | Path | Status |
|---|---|---|
| domain-model.md | `.beads/vb-xi2f.28/domain-model.md` | Written |
| type-contracts.md | `.beads/vb-xi2f.28/type-contracts.md` | Written |
| workflow-model.md | `.beads/vb-xi2f.28/workflow-model.md` | Written |
| error-taxonomy.md | `.beads/vb-xi2f.28/error-taxonomy.md` | Written |
| boundary-map.md | `.beads/vb-xi2f.28/boundary-map.md` | Written |
| hazard-analysis.md | `.beads/vb-xi2f.28/hazard-analysis.md` | Written |
| contract.md | `.beads/vb-xi2f.28/contract.md` | Written |
| proof-seeds.jsonl | `.beads/vb-xi2f.28/proof-seeds.jsonl` | Written (10 seeds) |
| traceability-matrix.jsonl | `.beads/vb-xi2f.28/traceability-matrix.jsonl` | Written (15 rows) |

### Key Findings

1. **Illegal state confirmed representable:** `canonical_digest` does not include ForEach field values in the hash. Two semantically different for_each sources produce identical digests.
2. **Duplicate code risk:** Two identical copies of `canonical_digest`/`digest_step_primitive` exist; fix must be applied to both.
3. **Catch-all hazard:** The `other => { hasher.update(name) }` pattern silently skips field hashing for ALL non-Set/non-Finish primitives.
4. **Breaking change:** Post-fix digests will differ from pre-fix digests for the same source (digest computation changed).

### Domain Decisions

- **DD-01:** Bead scope: ForEach only; other primitives out of scope
- **DD-02:** Target: `canonical_digest` source-level; `compute_compiled_digest` already correct
- **DD-03:** Both copies must be updated identically
- **DD-04:** Canonical `at_once` representation: hash `at_once.unwrap_or(1)` as u32 le_bytes
- **DD-05:** Body steps recursively hashed via `digest_step_primitive`

### Proof Seeds Emitted (10)

| ID | Domain Claim | Suggested Layers |
|---|---|---|
| PS-FE-01 | ForEach.input change → digest change | kani, proptest, fuzz |
| PS-FE-02 | ForEach.at_once change → digest change | kani, proptest |
| PS-FE-03 | ForEach.variable change → digest change | kani, proptest |
| PS-FE-04 | ForEach.body change → digest change | kani, proptest |
| PS-FE-05 | Determinism preserved | kani, proptest |
| PS-FE-06 | Dual-path equivalence | proptest |
| PS-FE-07 | Semantic equivalence None/Some(1) | kani |
| PS-FE-08 | Non-regression Set/Finish | proptest |
| PS-FE-09 | Exhaustive field coverage | flux-rs, kani |
| PS-FE-10 | Field delimiter collision resistance | kani |

### Open Domain Questions

1. **Q-DD-01:** Should `at_once: None` and `at_once: Some(1)` produce identical digest contributions? Decision: YES — they are semantically equivalent in lowering. Hash the resolved value.
2. **Q-DD-02:** Should body step IDs be included in the hash? Decision: YES — step IDs are source-level identifiers.
3. **Q-DD-03:** Should the two `canonical_digest` copies be consolidated? Decision: OUT OF SCOPE for this bead — separate refactoring bead.

### Illegal States Remaining Representable (Post-Fix)

After the fix, the following illegal states remain representable (out of scope for this bead):

1. **Catch-all gap for other primitives:** Collect, Aggregate, Repeat, Together, Wait, Ask, Choose, Do, Save still only hash primitive names, not fields.
2. **Duplicate code:** The two copies remain a maintenance risk until consolidated.
3. **Silent field addition hazard:** If a new field is added to `StepPrimitive::ForEach` in the future, the explicit match arm will force a compiler error (good!) — but for primitives still in the catch-all `other` arm, new fields are silently ignored.

### Next State: State 4 — proof-planning

The `proof-planner` agent should read `proof-seeds.jsonl` and produce lane decisions via `verifier-lane-decision/v1`.
