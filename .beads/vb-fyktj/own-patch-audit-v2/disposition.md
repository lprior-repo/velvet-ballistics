# vb-fyktj Historical Own-Patch Audit v2 — Disposition

**Scope:** 8 historical own-commit patches from vb-fyktj branch, each compared against current production (b752db4c1a) in this isolated workspace. No implementation changes; classification only.

## Methodology

Each patch's own source change was compared to the actual current production code at the corresponding files via `read`/`grep`. The old `APPROVED` label on review beads is not evidence of acceptance. File existence of proof/harness artifacts is not acceptance; production binding is required per GOD RULE 2.

## Disposition Table

| # | Commit | Bead | Subject | Classification | Rationale |
|---|--------|------|---------|----------------|-----------|
| 1 | dec0e41e2690 | vb-0qwfq | stamp accepted artifact sequence | **already-present** | `submit_artifact()` at crates/vb_storage/src/admission.rs:341 implements the delegated admission flow. CLI calls it with `RuntimePolicy::Strict`. GAP-007 doc comment on `accepted_at_seq` is still old version ("always set to EventSeq::new(0)") but functional change is delivered. |
| 2 | bd2920512bde | vb-3wn7x | black-hat review - APPROVED | **already-present** | `PendingActionIndexOp` enum and `stage_pending_action_index_op` function exist at crates/vb_storage/src/batch/action_index.rs:39,124. Insert/remove semantics for action lifecycle match own change. |
| 3 | 0f71f398cf66 | vb-3y9q5 | black-hat review - APPROVED | **already-present** | Pre-allocated frame pool at crates/vb_runtime/src/frame_pool.rs:26-47. `take()` only uses pool frames (no fresh allocation fallback). Doc comments match own change exactly. |
| 4 | dd50c691bb67 | vb-4pg6g | StepBudget::from_env PO-VERUS-032 | **useful-missing-needs-delivery** | `contracts/proof_obligations.yaml` only contains PO-VERUS-001..PO-VERUS-031. PO-VERUS-032 entry for StepBudget env-parsing boundedness is absent. Proof harness (if any) at verification/verus/signals_from_env.rs cannot bind to a non-registered obligation. |
| 5 | 56d14ca7878d | vb-6r7f5 | route flux through canonical scripts | **useful-missing-needs-delivery** | `.moon/tasks/flux.yml` does not exist. `scripts/flux-check-package.sh` is the old 22-line simple wrapper (not the canonical multi-mode script with --all/--crates/--dry-run). xtask flux lane (xtask/src/lanes.rs:83) calls `cargo flux -p` directly, not through the script. |
| 6 | 6351cc8cd904 | vb-b3kxw | consolidate stale cargo-vet | **useful-missing-needs-delivery** | `Cargo.toml` has no `[workspace.metadata.cargo-vet]` section. The 453-byte own change adding this section was not delivered. |
| 7 | a8451f741f71 | vb-bywha | resolve clippy all-targets vs source-only | **useful-missing-needs-delivery** | `.cargo/config.toml:15` still has `ci = "clippy --workspace --all-targets --all-features -- -D warnings"`. `contracts/invariants.yaml:405` still has `--all-targets` for unsafe_code check. The compile source change (`allocate_input_slots(source.inputs(), ...)`) IS present at crates/vb_compile/src/mod_compile_lowering/part_01.rs:33, but the clippy policy changes are not. |
| 8 | 220e4d21d719 | vb-ck4a1 | relocate JSON diagnostic tests | **useful-missing-needs-delivery** | Inline `#[cfg(test)]` test module still declared at crates/vb_core/src/diagnostic.rs:2091-2092. `tests_and_verification.rs` file still exists (380 lines). The workspace test `vb_test_validate_diagnostic_behavior.rs` is a DIFFERENT suite (validation error codes E01xx-E06xx, not diagnostic code parsing). Tests were not relocated; own change not delivered. |

## Summary

- **Already present:** 3/8 (dec0e41e2690, bd2920512bde, 0f71f398cf66)
- **Useful-missing-needs-delivery:** 5/8 (dd50c691bb67, 56d14ca7878d, 6351cc8cd904, a8451f741f71, 220e4d21d719)
- **Superseded/rejected:** 0/8

**Inherited branch deltas remain separate:** These 8 own patches are only the TIP commits of the vb-fyktj branch. Non-ancestor delta content between vb-fyktj and main@b752db4c1a is not covered by this audit.

## Suggested Next Priority (useful-missing)

1. **a8451f741f71 (clippy policy):** Low-risk config-only change. Resolves source-lint vs test-lint policy gap. Contract: clippy runs on `--lib --bins --examples --all-features` not `--all-targets`.
2. **6351cc8cd904 (cargo-vet metadata):** Low-risk, 2-line Cargo.toml addition. Consolidates vet config location.
3. **56d14ca7878d (flux routing):** Medium risk, requires script and xtask changes. Enables consistent flux invocation via canonical script.
4. **220e4d21d719 (diagnostic test relocation):** Medium risk, code change in core. Requires moving tests to workspace_tests.
5. **dd50c691bb67 (PO-VERUS-032):** Highest verification impact. Requires proof obligation registry update + verification harness delivery.
