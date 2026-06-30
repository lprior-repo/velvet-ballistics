---
section: 8
title: "Gleam-Like Explicitness Rules"
parent: velvet-ballistics-MASTER.md
---

## 8. Gleam-Like Explicitness Rules

Every workflow must make operational behavior explicit at compile time.

No hidden behavior:

```text
No hidden side effects.
No hidden retries.
No hidden timeouts.
No hidden capabilities.
No hidden secrets.
No hidden unbounded loops.
No hidden failure policy.
No hidden declassification.
No hidden runtime lookup.
No hidden dynamic dependencies.
```

The compiler must force every workflow to answer:

```text
What actions can run?
What side effects can occur?
What capabilities are required?
What secrets are referenced?
What can retry?
How many times?
With what backoff?
With what idempotency key?
What failures are handled?
What failures fail the run?
What is the maximum fanout?
What is the maximum runtime?
What is the maximum output size?
What durable events prove completion?
What data can enter the public result?
```

If a workflow cannot answer these questions, it cannot produce an accepted artifact.

---

