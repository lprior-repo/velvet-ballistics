---
section: 22
title: "Removed Rust Codegen and Maxperf"
parent: velvet-ballistics-MASTER.md
---

## 22. Removed Rust Codegen and Maxperf


Rust workflow code generation is **out of the current core feature set**. The active product goal is backend execution through compiled IR and the IR interpreter.

Current command surface excludes `compile --emit rust`. Current acceptance excludes generated Rust semantic parity, generated compile-fail fixtures, generated-vs-IR ratio benchmarks, PGO release workflows, and `maxperf` release claims.

Historical notes live in:

```text
docs/generated-workflows.md
docs/deferred-codegen-maxperf.md
```

Codegen is not in current scope. Any future reintroduction requires a dedicated master amendment and cannot inherit acceptance credit from historical notes.

---
