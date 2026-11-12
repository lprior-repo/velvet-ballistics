---
section: 37
title: "Fuzz Targets"
parent: velvet-ballistics-MASTER.md
---

## 37. Fuzz Targets


Required fuzz harnesses (actual paths: `fuzz/src/bin/*.rs`):

| Target | Coverage requirement |
|--------|---------------------|
| `yaml_events` | Arbitrary UTF-8 bytes → parser never panics |
| `expression` | Arbitrary UTF-8 bytes → lexer/parser/compiler never panics |
| `ipc_frame` | Arbitrary bytes → decoder never panics, length checks hold |
| `journal_event` | Arbitrary bytes → Postcard decode failure is typed |
| `compiled_ir` | Arbitrary bytes → decode/validate never panics |

---
