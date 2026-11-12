---
section: 4
title: "Swamp-Inspired Flow, Velvet-Hardened Contract"
parent: velvet-ballistics-MASTER.md
---

## 4. Swamp-Inspired Flow, Velvet-Hardened Contract

The useful external pattern is agent-centered deterministic automation:

```text
models       typed interfaces to things
workflows    composed multi-step operations
vaults       referenced secrets
artifacts    immutable versioned outputs
extensions   new capabilities agent can create or install
reports      structured post-run analysis
repo state    local repository-visible automation state
skills       agent instructions discoverable in the repo
```

`velvet-ballistics` adopts the spirit, not the weaker runtime boundary.

Velvet keeps:

```text
agent-discoverable skills
repo-local workflow packages
typed models/actions
versioned immutable artifacts
vault/secret references
structured reports after every run
simulation before production execution
human-reviewable generated source
```

Velvet rejects:

```text
YAML workflow definitions
runtime string lookup
runtime TypeScript/JavaScript workflow execution
unverified extension execution
unbounded workflow data
secret-bearing telemetry by default
model methods whose effects are opaque to the compiler
```

The product is not “agents write scripts.” The product is “agents write constrained workflow declarations; the compiler decides whether they are admitted.”

---

