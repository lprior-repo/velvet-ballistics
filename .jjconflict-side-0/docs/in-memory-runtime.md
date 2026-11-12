# In-Memory Runtime

The runtime core is a single-process state-machine engine. It consumes immutable `CompiledWorkflow` values and drives `RunFrame` values through numeric `StepIdx` transitions.

## Current Scope

`vb-core` owns:

```text
RunFrame
StepBudget
step_once
run_until_blocked
CompiledWorkflow
CompiledNodeKind
SlotValue
Taint
```

`RunFrame` stores:

```text
RunId
current StepIdx
boxed numeric slot array
boxed taint array
executed-step counter
```

## Execution Contract

`step_once` executes one compiled node. `run_until_blocked` executes deterministic nodes until finish, error, or budget exhaustion. `StepBudget` must be non-zero.

Current deterministic nodes:

```text
SetConst
Copy
Choose
Finish
```

## Hot Path Prohibitions

The runtime core must not depend on:

```text
YAML parsing
HTTP
JSON values
async runtime
storage APIs
string-keyed runtime lookup
unbounded queues
```

## Future Runtime Work

Phase 2 adds shard ownership and bounded run queues. Later phases add action suspension, replay hydration, frame pools, and turbo-mode preallocation. These must preserve the same no-YAML/no-JSON/no-HTTP hot-path rules.
