---
section: 40
title: "Cargo and Lint Contract"
parent: velvet-ballistics-MASTER.md
---

## 40. Cargo and Lint Contract

Workspace lints:

```toml
[workspace.lints.rust]
unsafe_code = "forbid"
unused_must_use = "deny"
unreachable_pub = "deny"
rust_2018_idioms = "deny"

[workspace.lints.clippy]
correctness = "deny"
suspicious = "deny"
perf = "deny"
complexity = "deny"
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
panic_in_result_fn = "deny"
todo = "deny"
unimplemented = "deny"
dbg_macro = "deny"
indexing_slicing = "deny"
string_slice = "deny"
get_unwrap = "deny"
arithmetic_side_effects = "deny"
as_conversions = "deny"
let_underscore_must_use = "deny"
await_holding_lock = "deny"
```

`RUSTC_BOOTSTRAP` is rejected. Nightly features are allowlisted per crate and zone. Runtime crates must not enable broad experimental features without a dedicated evidence bead.

---

