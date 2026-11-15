# Capability query result-use contract

Scope: add `#[must_use]` to exactly these eight existing public functions in `crates/vb_core/src/capability.rs`: `Capability::new`, `Capability::name`, `Capability::action_id`, `CapabilitySet::empty`, `CapabilitySet::from_grants`, `CapabilitySet::grants`, `CapabilitySet::len`, and `CapabilitySet::is_empty`.

The annotations make ignored returned capabilities, views, authorization decisions, and collection queries produce compiler warnings. Preserve all signatures and behavior, including exact name/action matching and empty-name refusal. Do not add type-level attributes, tests that only mirror annotations, APIs, dependencies, schemas, formats, or gates.

Acceptance: exact eight method attributes, source clean under the repository strict source lint, existing `vb_core` capability tests pass, and the canonical repository source task passes.
