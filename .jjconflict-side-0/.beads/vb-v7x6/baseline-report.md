bead_id: vb-v7x6
phase: 1
attempt: 1-of-7

# Baseline

- Source checkout: `/home/lewis/src/velvet-ballistics`.
- Isolated workspace: `/tmp/opencode/go-skill-vb-v7x6`.
- Bead claim: `bd update vb-v7x6 --claim` succeeded in source checkout.
- Parent evidence showed `moon run :doc` failing in `xtask::ui_release_gates ai_release_includes_ui_release_gates` with `Err(ENOENT)`.
- Local baseline before repair also exposed an unrelated all-target check warning in `crates/vb_storage/src/recovery/recovery_unit_tests.rs`; repaired because `moon run :doc` depends on `test` and `check`.
