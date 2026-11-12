---
section: 12
title: "Forbidden Hot-Path APIs"
parent: velvet-ballistics-MASTER.md
---

## 12. Forbidden Hot-Path APIs


The following are forbidden in hot runtime paths:

```text
serde_json::Value
HashMap<String, _>
BTreeMap<String, _>
format!
println!
eprintln!
dbg!
identifier String::from/to_string
runtime maps
serde_json
String reference lookup
YAML parser calls
JSON parser calls
HTTP server/client calls
filesystem path parsing
environment variable reads
string action lookup
unbounded channel creation
Vec push without prior capacity/resource check
allocations for expression stack
allocations for trace event
allocations for queue command
blocking filesystem calls per deterministic step
blocking Fjall persist per deterministic node unless strict durability requires it
thread spawn per run
async task spawn per step
per-step thread spawn
unchecked indexing or slicing
unchecked arithmetic or casts
```

Nuance: these APIs are allowed in cold parser, validator, compiler, diagnostics, CLI, benchmark harness setup, and tests when covered by tests and kept out of hot runtime execution.

---
