---
section: 62
title: "No-Async Rule"
parent: velvet-ballistics-MASTER.md
---

## 62. No-Async Rule


v1 runtime core must not depend on `tokio`, `async-std`, `smol`, `futures` executors, `async_trait`, or async task scheduling. `mio` is the only approved low-level eventing mechanism for IPC. Actions may block only in bounded action worker contexts or return `Suspended`. No async function may appear in `vb_core`, `vb_runtime`, `vb_storage`, or `vb_ipc`.

---
