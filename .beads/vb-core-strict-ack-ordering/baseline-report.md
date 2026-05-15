# Baseline Report — vb-core-strict-ack-ordering

## Git Log (baseline)

```
131d1788 (HEAD -> main) bd init: initialize beads issue tracking
973e47b2 (origin/main, origin/HEAD) fix(vb-qi37.1.4): decouple verus proof from cargo
4630df73 style: format rebased recovery proof
fc4f7d3a style: format integrated postcard tests
db5f12bf feat(vb-qi37.13): structure CLI diagnostics
```

## Bead Summary

- **id**: vb-core-strict-ack-ordering
- **title**: runtime/storage: Prove strict persistence before acknowledgement ordering
- **status**: in_progress
- **priority**: 0
- **labels**: core-priority, durability, engine, runtime, storage, strict
- **owner**: priorlewis43@gmail.com

## Acceptance Criteria

Every strict submit/action/wait/ask/retry/cancel/terminal mutation persists required journal/storage evidence before acknowledgement or externally visible state; injected persistence failure returns typed fail-closed error without in-memory-only acknowledged mutation; restart evidence matches acknowledged state.

## Known Dependents (blocks)

1. **vb-core-atomic-admission** — Persist accepted run as atomic Fjall batch
2. **vb-core-yaml-e2e-chain** — Prove YAML-origin Fjall runtime inspect events recovery chain
3. **vb-engine-yaml** — Durable YAML runtime acceptance without UI or generated Rust
4. **vb-qi37.12** — Eliminate silent discard paths

## Known Constraints

- Requires strict persistence-before-ack ordering for all mutation types
- Must prove fail-closed behavior on persistence injection
- Dependent beads must close before this bead can close
- Core engine/runtime/storage scope only