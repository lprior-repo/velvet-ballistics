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
rust_2018_idioms = { level = "deny", priority = -1 }
unexpected_cfgs = { level = "warn", check-cfg = ['cfg(flux)', 'cfg(fuzzing)', 'cfg(kani)', 'cfg(loom)', 'cfg(verus)', 'cfg(verus_keep_ghost)', 'cfg(docsrs,test)'] }

[workspace.lints.clippy]
correctness = { level = "deny", priority = -1 }
suspicious = { level = "deny", priority = -1 }
perf = { level = "deny", priority = -1 }
complexity = { level = "deny", priority = -1 }
unwrap_used = "forbid"
expect_used = "forbid"
panic = "forbid"
panic_in_result_fn = "forbid"
todo = "forbid"
unimplemented = "forbid"
dbg_macro = "forbid"
indexing_slicing = "deny"
string_slice = "forbid"
get_unwrap = "deny"
arithmetic_side_effects = "deny"
as_conversions = "deny"
let_underscore_must_use = "deny"
await_holding_lock = "deny"
large_stack_arrays = "warn"
large_types_passed_by_value = "warn"
result_large_err = "warn"
```

`RUSTC_BOOTSTRAP` is rejected. Nightly features are allowlisted per crate and zone. Runtime crates must not enable broad experimental features without a dedicated evidence bead.

---

