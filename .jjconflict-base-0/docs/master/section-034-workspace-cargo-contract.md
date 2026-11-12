---
section: 34
title: "Workspace Cargo Contract"
parent: velvet-ballistics-MASTER.md
---

## 34. Workspace Cargo Contract


```toml
[workspace]
members = [
  "crates/vb_core",
  "crates/vb_yaml",
  "crates/vb_validate",
  "crates/vb_expr",
  "crates/vb_compile",
  "crates/vb_storage",
  "crates/vb_runtime",
  "crates/vb_ipc",
  "crates/velvet_ballistics",
  "crates/workspace_tests",
  "fuzz",
]
resolver = "2"

[workspace.package]
edition = "2024"
license = "MIT OR Apache-2.0"
version = "0.1.0"

[workspace.dependencies]
thiserror = "2"
serde = { version = "1", default-features = false, features = ["derive", "alloc"] }
postcard = { version = "1", default-features = false, features = ["alloc"] }
byteorder = "1.5"
bytes = "1"
arrayvec = "0.7"
indexmap = "2"
logos = "0.15"
saphyr-parser = "0.0.6"
fjall = "3.1"
crossbeam-queue = "0.3"
rtrb = "0.3"
mio = "1"
criterion = "0.8"
iai-callgrind = "0.16"
proptest = "1"

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

[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
strip = "symbols"

[profile.bench]
inherits = "release"
debug = true
lto = "thin"
codegen-units = 1
```

Removed workspace members and dependencies (`vb_codegen`, `vb_ui_model`, `vb_ui_makepad`, Makepad, generated workflow dependencies, and maxperf-only profile policy) must not be treated as current workspace acceptance requirements.

---
