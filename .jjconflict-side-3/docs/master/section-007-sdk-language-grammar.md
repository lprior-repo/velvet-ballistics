---
section: 7
title: "SDK Language Grammar"
parent: velvet-ballistics-MASTER.md
---

## 7. SDK Language Grammar

The SDK DSL supports exactly these workflow-level constructs:

```text
workflow
policy
capabilities
secrets
inputs
steps
finish
```

The `steps` block supports exactly these primitives:

```text
let
action
choose
retry
for_each
together
collect
reduce
repeat
wait
ask
finish
```

Allowed expression operations:

```text
== != > >= < <=
and or not
+ - * /
contains starts_with ends_with has exists length empty append append_if merge sum count unique
```

Allowed key-expression ingredients:

```text
literal domain separator
workflow_digest()
artifact_digest()
run_id()
step_id()
action_id()
loop_index()
trigger_unique_key()
input.<field>
prior action output fields when declared stable and non-secret
```

Forbidden inside workflow DSL bodies:

```text
while
loop
for
async
await
unsafe
return
break
continue
?
unknown macro calls
arbitrary function calls
thread spawning
time/random/env/fs/net/process access
unbounded Vec/String construction
runtime action lookup by string
runtime reference lookup by string
```

Forbidden imports in workflow source modules:

```text
std::fs
std::net
std::process
std::thread
std::env
tokio
async_std
smol
futures
reqwest
serde_json
rand
chrono
```

The compiler must reject workflow definitions that depend on any forbidden construct. The rejection must be a structured diagnostic with code, span, explanation, and repair guidance where safe.

---

