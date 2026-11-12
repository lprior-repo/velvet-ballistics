---
section: 11
title: "Hot/Cold Data Layout"
parent: velvet-ballistics-MASTER.md
---

## 11. Hot/Cold Data Layout


Hot runtime structs carry no diagnostic fields. They do not store source spans, YAML paths, human names, formatted messages, or string references. Cold side tables carry spans, names, YAML paths, source snippets, diagnostic messages, and trace rendering metadata.

No allocation after run admission in turbo mode: all hot slots, step states, taint arrays, expression stacks, trace events, queue commands, action tickets, and journal buffers are preallocated or reservation-checked before a run is accepted. If capacity cannot be reserved, admission fails with a typed error.

Cold path components may use maps when they improve clarity and diagnostics:

- `vb_yaml`
- `vb_validate`
- `vb_compile`
- diagnostics
- tests
- fixtures
- benchmark harness setup

`HashMap` and `BTreeMap` are allowed in parser, validator, compiler, diagnostics, and tests.

Hot runtime state must not use `HashMap<String, Value>`, runtime state maps, dynamic object maps, or string-keyed lookup. Hot state uses numeric indices, handle tables, boxed slices, fixed-capacity stacks, bounded queues, and typed handles.

---
