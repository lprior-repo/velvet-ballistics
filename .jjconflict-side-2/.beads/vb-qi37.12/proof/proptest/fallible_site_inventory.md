# Proptest Target Plan: Fallible Site Inventory

- Obligation: `PO-013`.
- Status: `BLOCKED_TOOLING` for execution in this state because the requested boundary forbids production source, dependency, CI, and test edits, and no exact existing proptest named `vb_qi37_12_fallible_site_inventory_proptest` is wired.
- Intended command after wiring: `cargo test -p workspace_tests vb_qi37_12_fallible_site_inventory_proptest`.
- Oracle: generated inventory records must reject unclassified release-critical fallible sites and keep `typed_optional` distinct from `typed_best_effort_discard`.
- Model link: `.beads/vb-qi37.12/proof/verus/discard_classification.rs` proves the abstract acceptance boundary used by this property.
