# Kani Report - vb-qi37.5.3

STATUS: PASS

Command:

```bash
rtk cargo kani -p vb_compile --harness idempotency_gate_parity
```

Evidence:

```text
SUMMARY:
 ** 0 of 145 failed (2 unreachable)

VERIFICATION:- SUCCESSFUL
Manual Harness Summary:
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
```

Scope: all 45 `(side_effect, retry_safety, idempotency)` combinations via current `vb_compile` and `vb_validate` decision helpers.
