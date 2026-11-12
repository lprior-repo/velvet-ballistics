---
section: 26
title: "Mandatory Function Surface: `vb_validate`"
parent: velvet-ballistics-MASTER.md
---

## 26. Mandatory Function Surface: `vb_validate`


**Source of truth:** `crates/vb_validate/src/`.

Required coverage areas:

| Area | Required public surface |
|------|------------------------|
| Schema validation | `validate_workflow_schema` (required fields, ID rules, primitive count, trigger types including HTTP rejection). |
| References | `validate_references` (forward refs, runtime refs, undeclared secrets). |
| Control flow | `validate_control_flow`, `validate_forward_only_then`, `validate_reachability`. |
| Type/taint | `validate_types`, `validate_taint` (taint propagation, secret leak detection). |
| Resources | `validate_resource_limits`. |
| Diagnostics | `diagnostic_from_error`, `error_code`. |

---
