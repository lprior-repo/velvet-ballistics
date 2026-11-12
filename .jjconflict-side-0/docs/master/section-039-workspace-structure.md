---
section: 39
title: "Workspace Structure"
parent: velvet-ballistics-MASTER.md
---

## 39. Workspace Structure

Target workspace:

```text
velvet-ballistics/
  Cargo.toml
  rust-toolchain.toml
  clippy.toml
  moon.yml
  contracts/
  docs/
  crates/
    vb_core/
    vb_artifact/
    vb_compiler/
    vb_sdk/
    vb_sdk_macros/
    vb_action/
    vb_policy/
    vb_storage/
    vb_runtime/
    vb_ipc/
    vb_testkit/
    velvet_ballistics/
  benches/
  fuzz/
  tests/
    compile_fail/
    replay/
    crash_lab/
  examples/
    workflows/
    actions/
    policies/
```

Removed active crates:

```text
vb_yaml
vb_validate as YAML-specific validator
YAML parser fuzz targets
YAML source-map crates
YAML fixture suites
vb_codegen
vb_ui_model
vb_ui_makepad
```

Validation logic moves into `vb_compiler` over the SDK AST and generated intermediate representation.

---

