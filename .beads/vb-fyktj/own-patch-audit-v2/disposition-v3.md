# Own-Patch Semantic Audit Disposition v3

Corrects v2 draft errors: sequence-stamping claim, frame-pool hunk inspection, cargo-vet semantics, registry-vs-binding.

## 1. 0f71f398cf66 — Frame pool pre-allocation
**Status: PRESENT**

All hunks verified:
- `FramePool::new` pre-allocates capacity frames via loop calling `RunFrame::new` (current `crates/vb_runtime/src/frame_pool.rs:14-45`)
- `take` returns pre-allocated frame with no fresh-allocation fallback (current `:49-56`)
- `release` silently drops mismatched dimensions or over-capacity (current `:59-74`)
- Test expectations updated: `available()` starts at capacity, release returns to capacity (current `crates/vb_runtime/src/frame_pool/tests.rs:43`)

v2 correctly identified this as delivered.

## 2. 220e4d21d719 — Diagnostic test consolidation to workspace_tests
**Status: MISSING**

Patch deletes `crates/vb_core/src/diagnostic/tests_and_verification.rs` and its `#[cfg(test)] #[path]` include in `diagnostic.rs`. Current code still includes the module (current `crates/vb_core/src/diagnostic.rs:2087-2089`) and the file still exists.

```rust
// Current diagnostic.rs still has:
#[cfg(test)]
#[path = "diagnostic/tests_and_verification.rs"]
mod tests_and_verification;
```

The consolidation to `crates/workspace_tests/` was never delivered.

## 3. 56d14ca7878d — Flux Moon task and script consolidation
**Status: MISSING**

Patch adds `.moon/tasks/flux.yml`, expands `scripts/flux-check-package.sh` with `--all`/`--crates`/`--dry-run` options, and adds `flux_command` to `xtask/src/lanes.rs`. Current code has:
- No `.moon/tasks/flux.yml` (glob confirms absence)
- Old single-package-only script (current `scripts/flux-check-package.sh:1-19`)
- No `flux_command` in xtask lanes

```bash
# Current flux script is the old version:
package="$1"
shift
cargo flux -p "$package" --message-format human "$@"
```

## 4. 6351cc8cd904 — Empty [workspace.metadata.cargo-vet]
**Status: NO-OP (semantically useful only with tooling)**

Patch adds empty `[workspace.metadata.cargo-vet]` section. Current `Cargo.toml` lacks it. Empty metadata section has no semantic effect unless a tool reads it. Useful semantics: future cargo-vet tooling integration would expect this section to exist as the config anchor. Without any `.cargo-vet` config files or tooling invocation, it's a no-op addition.

## 5. a8451f741f71 — Clippy target scope narrowing
**Status: MISSING**

Patch narrows `ci` alias from `--all-targets` to `--lib --bins --examples` and removes `&` from `source.inputs()` in compile. Current code has:
- `ci` alias still uses `--all-targets` (current `.cargo/config.toml:14`)
- Compile call still uses `&source.inputs()` (in `crates/vb_compile/src/mod_compile_lowering/part_01.rs`)

## 6. bd2920512bde — Pending action index maintenance (vb-3wn7x)
**Status: PRESENT**

All hunks verified:
- `crates/vb_storage/src/batch/action_index.rs` exists with `stage_pending_action_index_op` (current line 124)
- `stage_recovery_index_ops` wrapper added (current line 103)
- `append_event` calls `stage_recovery_index_ops` (current `crates/vb_storage/src/batch/append_event.rs:105`)
- `delete_action_index` putter added to `putters.rs`

## 7. dd50c691bb67 — Verus PO-VERUS-032 registry entry
**Status: MISSING**

Patch adds PO-VERUS-032 for StepBudget::from_env to `contracts/proof_obligations.yaml`. Current file has no PO-VERUS-032 entry (grep confirms absence). This is a production-bound harness entry — binding exists in `verification/verus/signals_from_env.rs`, but the registry entry is missing.

**Note:** Distinguish binding from registry. A Verus spec can be production-bound via `#[path]` while unregistered. PO-VERUS-032 is unregistered in the current `proof_obligations.yaml`.

## 8. dec0e41e2690 — Artifact acceptance sequence tracking (GAP-007)
**Status: MISSING**

Patch adds `system` keyspace, `accept_seq_allocate`, `submit_artifact_at_seq`, and sequence-aware submission APIs. Current code:
- No `system` keyspace in `constants.rs` (grep confirms)
- No `accept_seq_allocate` method (grep confirms)
- `accepted_at_seq` still uses `EventSeq::new(0)` (current `crates/vb_storage/src/admission.rs:220`)
- Docstring still says "This field is currently always set to `EventSeq::new(0)`"

The CLI delegation change is present (`submit_cli_compiled_artifact` delegates to `vb_storage::submit_artifact`), but the sequence-aware API/body/test changes are not.

## Summary

| Patch | Status |
|-------|--------|
| 0f71f398cf66 (frame pool pre-alloc) | PRESENT |
| 220e4d21d719 (diagnostic tests) | MISSING |
| 56d14ca7878d (flux tasks) | MISSING |
| 6351cc8cd904 (cargo-vet) | NO-OP |
| a8451f741f71 (clippy scope) | MISSING |
| bd2920512bde (action index) | PRESENT |
| dd50c691bb67 (Verus PO-VERUS-032) | MISSING |
| dec0e41e2690 (acceptance seq) | MISSING |

2 delivered, 5 not delivered, 1 no-op.
