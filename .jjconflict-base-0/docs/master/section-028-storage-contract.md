---
section: 28
title: "Storage Contract"
parent: velvet-ballistics-MASTER.md
---

## 28. Storage Contract

Fjall remains the required v1 storage engine unless a future amendment replaces it with evidence.

Required keyspaces:

```text
artifact
artifact_source_map
run_header
run_event
run_snapshot
blob
index_status
index_workflow
index_action
index_artifact
index_timer
```

Authoritative source:

```text
run_event + snapshots
```

Indexes are rebuildable caches. If an index is missing or corrupt, `doctor` rebuilds from history.

Multi-keyspace writes should use atomic write batches where available. If not, recovery treats indexes as derived and repairs them.

Recovery never reparses SDK source. It loads accepted artifacts, snapshots, journals, blobs, and action ABI manifests by digest.

---

