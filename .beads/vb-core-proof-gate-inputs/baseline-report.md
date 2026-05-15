# Baseline Report — vb-core-proof-gate-inputs

## Baseline Git Log (HEAD~4..HEAD)

```
131d1788 (HEAD -> main) bd init: initialize beads issue tracking
973e47b2 (origin/main, origin/HEAD) fix(vb-qi37.1.4): decouple verus proof from cargo
4630df73 style: format rebased recovery proof
fc4f7d3a style: format integrated postcard tests
db5f12bf feat(vb-qi37.13): structure CLI diagnostics
```

## Touched Crates / Files (last 5 commits)

**Beads:**
- `.beads/metadata.json`
- `.beads/vb-qi37.1.4/STATE.md`
- `.beads/vb-qi37.1.4/black-hat-review.md`
- `.beads/vb-qi37.1.4/proof-evidence.md`
- `.beads/vb-qi37.1.4/proof-obligations.jsonl`
- `.beads/vb-qi37.1.4/proof-plan-review-input.md`
- `.beads/vb-qi37.1.4/proof-repair-guide.md`
- `.beads/vb-qi37.1.4/proof-review.md`
- `.beads/vb-qi37.1.4/verification-layers.md`
- `CLAUDE.md`

**Root:**
- `Cargo.toml`

**Crates:**
- `crates/vb_runtime/Cargo.toml`
- `crates/vb_runtime/src/recovery.rs`
- `crates/vb_storage/Cargo.toml`
- `crates/vb_storage/src/recovery/recover.rs`
- `crates/vb_ui_model/src/emitter/binary/tests.rs`

**Crates list (overall workspace):**
vb_benchmark, vb_codegen, vb_compile, vb_core, vb_doc, vb_expr, vb_ipc, vb_proof_kernels, vb_runtime, vb_storage, vb_ui, vb_ui_makepad, vb_ui_model, vb_ui_snapshot, vb_validate, vb_yaml, velvet_ballastics, workspace_tests

## Known Constraints

- `vb-core-proof-gate-inputs` blocks `vb-core-proof-15-gate` and indirectly `vb-engine-yaml`
- Acceptance requires: all 15 gates with concrete producers, failing tests for each, no default-true gates, taint/action/durability/observability/replay evidence rejects typed diagnostics
- Source checkout: `/home/lewis/src/velvet-ballistics`
- This workspace: `/tmp/vb-ws/vb-core-proof-gate-inputs`
