---
section: 24
title: "Mandatory Function Surface: `vb_core`"
parent: velvet-ballistics-MASTER.md
---

## 24. Mandatory Function Surface: `vb_core`


**Source of truth:** `crates/vb_core/src/`. This section states required behavioral coverage. Exact function names and signatures are defined by the code.

Required coverage areas:

| Area | Required public surface |
|------|------------------------|
| ID accessors | Every numeric ID type must provide checked raw access (e.g., `get()`, `as_usize()`). |
| Value operations | `FiniteF64::new`, `FiniteF64::get`, `SlotValue::type_name`, `SlotValue::is_true`, `ConstValue::to_slot_value`. |
| Frame operations | `RunFrame::new`, `run_id`, `pc`, `executed`, `set_pc`, `read_slot`, `write_slot`, `read_taint`, `write_taint`, `write_slot_with_taint`, `mark_*` for all 7 step states, `step_state`, `reinitialize`, `increment_executed`. |
| Budget | `StepBudget::new`, `try_take`, `remaining`. |
| Execution | `step_once`, `drive_deterministic` (core), expression evaluation with `ValueStore`, accessor evaluation. |
| IR validation | `CompiledWorkflow::try_from_parts` validates node bounds, resource contracts, transition targets, expression stack bounds. |
| Value store | `ValueStore::new`, `insert_symbol`, `insert_list`, `insert_object`, `insert_blob`, lookup methods for each handle type, `object_field`, `list_item`. |

---
