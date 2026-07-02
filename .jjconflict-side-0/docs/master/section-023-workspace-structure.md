---
section: 23
title: "Workspace Structure"
parent: velvet-ballistics-MASTER.md
---

## 23. Workspace Structure


Target structure:

```text
velvet-ballistics/
  Cargo.toml
  rust-toolchain.toml
  clippy.toml
  deny.toml
  moon.yml
  supply-chain/
    config.toml
  crates/
    vb_core/
    vb_yaml/
    vb_validate/
    vb_expr/
    vb_compile/
    vb_storage/
    vb_runtime/
    vb_ipc/
    velvet_ballistics/
  benches/
  fuzz/
  crates/workspace_tests/
```

Current state: the active backend workspace target is the underscore crate contract above (`vb_core`, `vb_yaml`, `vb_validate`, `vb_expr`, `vb_compile`, `vb_storage`, `vb_runtime`, `vb_ipc`, and `velvet_ballistics`). Any future hyphenated internal crate name is a regression unless it is explicitly labeled as a migration artifact.

Removed crates: `vb_codegen`, `vb_ui_model`, and `vb_ui_makepad` are not active current-scope workspace requirements. They must not appear as active workspace members or current release gates.

---
