---
section: 25
title: "Mandatory Function Surface: `vb_yaml`"
parent: velvet-ballistics-MASTER.md
---

## 25. Mandatory Function Surface: `vb_yaml`


**Source of truth:** `crates/vb_yaml/src/`.

Required coverage areas:

| Area | Required public surface |
|------|------------------------|
| Parsing | `parse_yaml_events`, `parse_workflow_source`. |
| Profile validation | `validate_yaml_profile` (rejects anchors, aliases, merge keys, duplicate keys, ambiguous scalars, custom tags, binary scalars, multiple documents). |
| Source maps | `build_source_map`, `span_for_node`. |
| Fixtures | `load_fixture_source`. |

---
