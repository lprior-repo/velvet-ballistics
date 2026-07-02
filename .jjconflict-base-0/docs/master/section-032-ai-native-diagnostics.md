---
section: 32
title: "AI-Native Diagnostics"
parent: velvet-ballistics-MASTER.md
---

## 32. AI-Native Diagnostics

Every compiler, runtime, storage, IPC, action, and replay failure must have:

```text
stable code
severity
span or artifact location
machine path
human message
reason
policy gate
repair hints when safe
whether repair requires human review
```

Diagnostic example:

```json
{
  "schema_version": "velvet.diagnostic/v1",
  "kind": "DiagnosticReport",
  "status": "fail",
  "diagnostics": [
    {
      "code": "ACTION_REQUIRES_IDEMPOTENCY",
      "severity": "error",
      "span": {
        "file": "workflows/issue.rs",
        "line_start": 18,
        "column_start": 9,
        "line_end": 28,
        "column_end": 10
      },
      "reason": "external write action is reachable from retry max_attempts = 3",
      "required": [
        "deterministic idempotency key",
        "no secret key ingredients",
        "key scope PerBusinessObject"
      ],
      "repair": {
        "kind": "add_field",
        "field": "idempotency_key",
        "value": "key!(\"github.issue_create\", input.repo, input.ticket_id)",
        "confidence": "high",
        "requires_human_review": false
      }
    }
  ]
}
```

Not every repair is auto-applicable. Semantic repairs default to `requires_human_review: true`.

---

