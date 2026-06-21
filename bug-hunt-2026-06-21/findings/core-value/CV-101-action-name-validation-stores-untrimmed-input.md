# CV-101: ActionName validates a trimmed string but stores the untrimmed input

- **Severity**: Medium
- **Category**: correctness
- **Location**: `crates/vb_core/src/action/classification.rs:39`
- **Confidence**: confirmed

## Description

`ActionName::new` validates `s.trim()` but stores the original `String`. Leading or trailing whitespace is accepted even though the type contract says action names contain no whitespace, and equality/hash/serialization operate on the untrimmed string while `as_str` displays the trimmed value.

## Evidence

Construction stores `s` after validating a borrowed trimmed view:

```rust
pub fn new(s: impl Into<String>) -> Result<Self, ActionNameError> {
    let s = s.into();
    Self::validate(&s)?;
    Ok(Self(s))
}
```

Validation trims before checking length and whitespace:

```rust
let trimmed = s.trim();
if trimmed.is_empty() { ... }
if trimmed.len() > MAX_ACTION_NAME_LENGTH { ... }
if trimmed.chars().any(|c| c.is_whitespace()) { ... }
```

The public accessor trims again:

```rust
pub fn as_str(&self) -> &str {
    self.0.trim()
}
```

## Adversarial Check

This is not harmless normalization. `ActionName("foo")` and `ActionName(" foo ")` display as the same name but are different for derived `Eq`, `Hash`, `Serialize`, and `Deserialize` because the stored string differs. That breaks name-based lookup and persistence invariants.

## Suggested Fix

Either reject any whitespace in the original string, including leading/trailing whitespace, or store `trimmed.to_owned()` after validation. Prefer rejecting original whitespace to preserve the documented no-whitespace invariant.
