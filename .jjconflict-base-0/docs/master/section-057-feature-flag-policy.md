---
section: 57
title: "Feature Flag Policy"
parent: velvet-ballistics-MASTER.md
---

## 57. Feature Flag Policy


- Default features: none (all code always compiled).
- `bench` feature: enables benchmark-only harness code.
- `volatile` feature: enables volatile storage mode (test-only).
- Forbidden features: `json`, `http` in v1 runtime crates.
- `generated` and `maxperf` are removed and must not be current default or release features.

---
