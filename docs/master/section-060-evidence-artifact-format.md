---
section: 60
title: "Evidence Artifact Format"
parent: velvet-ballistics-MASTER.md
---

## 60. Evidence Artifact Format


A bead is not closable without an evidence artifact:

```toml
# .evidence/<bead-id>.toml
bead = "runtime-engine-setconst"
phase = 13
git_commit = "abc123..."
rustc = "nightly-2026-04-28"

[[commands]]
command = "cargo +nightly fmt --all -- --check"
exit = 0
log = "logs/fmt.txt"

[[commands]]
command = "cargo +nightly nextest run -p vb_core"
exit = 0
log = "logs/nextest-vb-core.txt"

[[benchmarks]]
name = "transition_set"
before = "1234ns"
after = "987ns"
file = "bench/transition_set.json"
```

---
