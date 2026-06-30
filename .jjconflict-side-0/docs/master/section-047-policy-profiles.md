---
section: 47
title: "Policy Profiles"
parent: velvet-ballistics-MASTER.md
---

## 47. Policy Profiles

Policies are data. Profile names are convenience labels only.

Required profiles:

```text
dev
test
strict_ai
prod_default
financial_strict
local_only
no_external_writes
human_approval_required
```

Every policy compiles to a canonical policy digest.

Policy controls:

```text
max_parallel
max_action_tickets
max_run_time
max_result_bytes
max_arena_bytes
require_timeouts
require_idempotency_for_external_writes
reject_explicit_secret_conditions
warn_on_possible_implicit_secret_flow
allow_process_actions
allow_unsafe_shell
durability_minimum
capability_check_mode
warning_promotions
```

Accepted artifacts bind to `policy_digest`, not profile name.

---

