---
section: 53
title: "Hot/Cold Module Classification"
parent: velvet-ballistics-MASTER.md
---

## 53. Hot/Cold Module Classification


### Hot Path Modules

No allocation after admission, no formatting, no maps, no string operations:

- `vb_core::engine`
- `vb_core::frame`
- `vb_runtime::engine`
- `vb_runtime::shard` (tick loop only)
- `vb_runtime::frame_pool`
- `vb_runtime::primitives::*`
- `vb_ipc` decoder after header validation

Generated workflow code is removed from current scope; any residue must be deleted or quarantined.

### Cold Path Modules

Maps, formatting, and allocation allowed:

- `vb_yaml`
- `vb_validate`
- `vb_compile` (except final IR validation helpers used by runtime)
- `vb_runtime::action` (ActionRegistry, validation)
- `vb_runtime::trace` (event rendering)
- `vb_storage::recovery`
- Diagnostics
- CLI
- Test and bench harnesses

### Scanner Policy

The banned-token scanner (Section 12) must be path-aware. `format!` is forbidden in hot modules but allowed in cold modules. `HashMap` is forbidden in hot modules but allowed in cold modules.

---
