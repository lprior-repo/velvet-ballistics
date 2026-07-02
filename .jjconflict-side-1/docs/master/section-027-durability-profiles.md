---
section: 27
title: "Durability Profiles"
parent: velvet-ballistics-MASTER.md
---

## 27. Durability Profiles

Only three durability profiles exist.

| Profile | Behavior | Use |
|---|---|---|
| `volatile` | no crash guarantee | unit tests, benchmarks only |
| `journaled` | bounded group-commit loss window | default production when some loss is acceptable |
| `strict` | persist before required acknowledgements and dispatches | compliance, financial, critical automation |

Every status report must identify the profile. Never emit `durable: true` without profile and boundary evidence.

Example:

```json
{
  "durability": {
    "profile": "strict",
    "run_accepted_persisted": true,
    "outbox_persisted_before_dispatch": true,
    "completion_persisted_before_frame_mutation": true
  }
}
```

---

