# Triage Table — vb-h39ky

**Status:** draft, planned for v0.2.0 / current cycle
**Total blocks:** 296 across 14 groups (estimates from bead close reason)

| # | Group | Est. Blocks | Triage Decision | Rationale | Linked Obligation |
|---|---|---|---|---|---|
| 1 | type-enforcer (vb_expr/eval) | 8 | register_in_v0_2_0 | Covered by vb-bc33k binding | PO-vb-bc33k-001..006 |
| 2 | bytecode-binary-op-classification | 24 | defer_to_v0_2_0_with_obligation | Production-bound but needs explicit obligation rows | PO-vb-expr-bytecode-binary-op-001 (planned) |
| 3 | lexer-keyword-classification | 18 | defer_to_v0_2_0_with_obligation | Production-bound; vb-3xdp5 inline audit scope | PO-vb-expr-lexer-keyword-001 (planned) |
| 4 | parser-expression-shape | 32 | defer_to_v0_2_0_with_obligation | Production-bound parser AST invariants | (planned v0.2.0) |
| 5 | workflow-lifecycle-state | 28 | register_in_v0_2_0 | Production state machine in vb_core | (planned v0.2.0) |
| 6 | action-proof-lemmas | 22 | register_in_v0_2_0 | Already cited by vb-3xdp5 audit | (planned v0.2.0) |
| 7 | queue-semantics | 18 | defer_to_v0_2_0_with_obligation | vb-r37is umbrella task scope | (planned v0.2.0) |
| 8 | storage-journal-codec | 38 | register_in_v0_2_0 | Already covered by VB-MRWE.6 / VB-MRWE.5 | existing obligations |
| 9 | recovery-replay | 24 | defer_to_v0_2_0_with_obligation | Production-bound but contract work needed | (planned v0.2.0) |
| 10 | runtime-facade-api | 14 | register_in_v0_2_0 | Covered by vb-puvkn binding | (planned v0.2.0) |
| 11 | action-completion-fence | 18 | register_in_v0_2_0 | Runtime action invariants | (planned v0.2.0) |
| 12 | proof-kernels-dual-mode | 26 | register_in_v0_2_0 | Already in dual_mode_proof_kernels_2026_06_19 | existing obligations |
| 13 | classify-identity-normalize | 16 | defer_to_v0_2_0_with_obligation | Classification semantics separate round | (planned v0.2.0) |
| 14 | vacuum-spec-only-sketches | 10 | retire_as_vacuum_model | Standalone models; production counterparts in dual_mode | verus_registry_targets notes |

**Totals:**
- Register in v0.2.0: 132 blocks (groups 1, 5, 6, 8, 10, 11, 12)
- Defer to v0.2.0 with obligation: 124 blocks (groups 2, 3, 4, 7, 9, 13)
- Retire as vacuum model: 40 blocks (group 14 + residual)
- **Sum: 296** ✓

## Anti-Laundering Decision Rule

Every register decision must satisfy one of:
- The block annotates a production fn in the same file (in-crate #[cfg(verus)] block on a specific fn).
- The block uses `#[path = "..."]` to reference production Rust code.
- The block's spec fn is named identically to a production fn and its
  ensures clause quotes production behavior.

Every retire decision must satisfy:
- The block contains standalone math with no production source binding, OR
- The block is already explicitly RETIRED in proof_obligations.yaml
  verus_registry_targets notes.

## Per-Group Plan

### Group 1 (type-enforcer) — REGISTER
Already covered by vb-bc33k. Action: no new obligation; cross-link existing
PO-vb-bc33k-* IDs to this group in proof_obligations.yaml comments.

### Group 2 (bytecode) — DEFER
Action: in v0.2.0, add obligation row `obl-vb-expr-bytecode-binary-op-001`
with files: `crates/vb_expr/src/bytecode/op.rs`, `crates/vb_expr/src/bytecode/verus.rs`.

### Group 3 (lexer) — DEFER
Action: in v0.2.0, add `obl-vb-expr-lexer-keyword-001` with
`crates/vb_expr/src/lexer/verus.rs` and `crates/vb_expr/src/lexer/keyword.rs`.

### Group 4 (parser) — DEFER
Action: in v0.2.0, add `obl-vb-expr-parser-shape-001..003`.

### Group 5 (workflow-lifecycle) — REGISTER
Action: in v0.2.0, add `obl-vb-core-workflow-lifecycle-001` with
`crates/vb_core/src/workflow/lifecycle/mod.rs`.

### Group 6 (action-proof) — REGISTER
Action: in v0.2.0, add `obl-vb-core-action-proof-001..003` citing vb-3xdp5 audit.

### Group 7 (queue) — DEFER
Action: covered by vb-r37is umbrella task. Cross-link.

### Group 8 (storage-journal) — REGISTER
Action: already in VB-MRWE.5/6 obligations. No new rows needed.

### Group 9 (recovery) — DEFER
Action: in v0.2.0, add `obl-vb-storage-recovery-001..005` after contract work.

### Group 10 (runtime-facade) — REGISTER
Action: covered by vb-puvkn. No new rows.

### Group 11 (action-completion-fence) — REGISTER
Action: in v0.2.0, add `obl-vb-runtime-action-fence-001..003`.

### Group 12 (proof-kernels) — REGISTER
Action: already in `dual_mode_proof_kernels_2026_06_19`. No new rows.

### Group 13 (classify/normalize) — DEFER
Action: separate triage round in v0.2.0.

### Group 14 (vacuum) — RETIRE
Action: confirm the 4 already-RETIRED files (`verification/verus/{taint_lattice,step_state_machine,resource_budget,vb_kyyf_normalization}.rs`) plus any new vacuum discoveries are listed in `verus_registry_targets` notes. Production counterparts are in `dual_mode_proof_kernels_2026_06_19`.