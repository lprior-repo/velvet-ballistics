---
section: 35
title: "LSP and Editor Contract"
parent: velvet-ballistics-MASTER.md
---

## 35. LSP and Editor Contract

The language server is a compiler frontend, not a second language implementation.

Required LSP features:

```text
parse workflow macro DSL
resolve actions
show capability requirements
show side-effect class
show retry/idempotency status
show resource budget estimates
show secret/taint facts
show compile diagnostics with repairs
run verify on save when configured
format DSL blocks
```

LSP must call the same parser, resolver, verifier, and diagnostic machinery as `cargo velvet verify`.

---

