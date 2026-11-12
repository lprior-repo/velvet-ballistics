---
section: 8
title: "Language Specification"
parent: velvet-ballistics-MASTER.md
---

## 8. Language Specification


**Title:** Velvet Ballastics Workflow Language v1
**Canonical version string:** `velvet-ballistics/v1`

Required top-level fields:

```yaml
version: velvet-ballistics/v1
name: <workflow_name>
when: <trigger>
steps: <step_list>
```

Optional top-level fields:

```yaml
inputs:   # input schema declarations
vars:     # static non-secret constants
secrets:  # named secret requirements; literal secret values forbidden
result:   # final output mapping
examples: # executable test fixtures
```

Strict YAML profile:

- Allowed: strings, finite numbers, booleans, null, lists, objects, comments.
- Rejected: duplicate keys, anchors, aliases, merge keys, custom tags, binary scalars, multiple documents, unknown top-level fields, unknown step fields, YAML 1.1 ambiguous booleans (`yes`, `no`, `on`, `off`).

IDs:

- Pattern: `^[a-z][a-z0-9_]{0,63}$`
- Reserved roots: `input`, `inputs`, `vars`, `secrets`, `steps`, `result`, `when`, `item`, `error`, `attempt`, `total`.
- Reserved literals/primitives: `true`, `false`, `null`, `do`, `set`, `choose`, `for_each`, `together`, `collect`, `reduce`, `repeat`, `wait`, `ask`, `try_again`, `on_error`, `then`, `finish`.

References:

- Allowed roots: `$input.x`, `$vars.x`, `$secrets.x`, `$step_id.x`, `$loop_name.x`, `$error.x`, `$attempt.x`, `$total.x`.
- Compiler rule: all references are parsed, validated, type-checked, and compiled to `SlotIdx` or `AccessorIdx` before execution.
- Runtime rule: the runtime never resolves reference strings.

Expressions:

- Operators: `==`, `!=`, `>`, `>=`, `<`, `<=`, `and`, `or`, `not`.
- Bounded arithmetic: `+`, `-`, `*`, `/`.
- Helpers: `contains`, `starts_with`, `ends_with`, `has`, `exists`, `length`, `empty`, `append`, `append_if`, `merge`, `sum`, `count`, `unique`.
- Forbidden: JavaScript, Python, jq, regex in v1, network calls, time/random functions, user-defined functions, loops inside expressions.

---
