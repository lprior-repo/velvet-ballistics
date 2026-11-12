---
section: 54
title: "Single-Server Ownership and Database Locking"
parent: velvet-ballistics-MASTER.md
---

## 54. Single-Server Ownership and Database Locking


- One active runtime process may own a database path at a time.
- Startup must acquire an exclusive process lock (e.g., `flock`).
- If the lock is already held, startup fails with a typed error.
- No distributed coordination, leader election, replication, or multi-writer mode in v1.
- Many IPC clients may connect to one server. One server owns the runtime and Fjall database.

---
