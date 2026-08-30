---
section: 44
title: "Evidence and Bead Contract"
parent: velvet-ballistics-MASTER.md
---

## 44. Evidence and Bead Contract

Every implementation change must produce an evidence bundle:

```toml
bead = "sdk-idempotency-gate"
phase = 8
git_commit = "..."
rustc = "nightly-..."

[[commands]]
command = "cargo +nightly fmt --all -- --check"
exit = 0
log = "logs/fmt.txt"

[[commands]]
command = "cargo +nightly clippy --workspace --lib --bins --examples --all-features -- -D warnings"
exit = 0
log = "logs/clippy.txt"

[[tests]]
name = "retry_external_write_without_key_fails"
kind = "trybuild"
status = "pass"

[[fuzz]]
target = "sdk_macro_parser"
status = "pass"
seconds = 60
```

No evidence bundle, no closure.

---

