# RP-018: ActionRegistry::len Reports Sparse Slot Capacity Instead Of Registered Actions

- **Severity**: Low
- **Category**: correctness
- **Location**: `crates/vb_runtime/src/action.rs:139`
- **Confidence**: confirmed

## Description
`ActionRegistry::len` claims to return the number of registered actions, but returns the backing slot vector length. A registry with one high-id action reports thousands of actions because empty sparse slots are counted.

## Evidence
The implementation returns `self.slots.len()` directly:

```rust
139:     /// Returns the number of registered actions.
140:     #[must_use]
141:     pub fn len(&self) -> usize {
142:         self.slots.len()
143:     }
```

The rest of the type confirms slots can be sparse and that empty slots are not registered actions:

```rust
25: #[derive(Debug, Clone)]
26: enum ActionSlot {
27:     Empty,
28:     Registered(ActionContract),
29: }
```

`is_empty` and `registered_contracts` filter `ActionSlot::Empty`, but `len` does not.

## Adversarial Check
This is not a naming preference. The doc comment says registered action count, and callers using `len` for diagnostics, capacity checks, or metrics will get a false value after sparse registration. The type already distinguishes empty from registered slots, so counting backing storage contradicts its own model.

## Suggested Fix
Maintain a `registered_count` field updated during registration, or compute `self.slots.iter().filter(|slot| matches!(slot, ActionSlot::Registered(_))).count()` if this is not hot-path code.
