# Domain Model Review — vb-core-lower-coverage-matrix

## Review Status
Domain model is embedded in `contract.md`. This document provides a quick reference.

## Domain Model Summary

### v1 YAML Construct Taxonomy

**Top-Level Fields**:
- `version`: Required, must be "velvet-ballistics/v1"
- `name`: Required, workflow identifier
- `when`: Trigger (manual, schedule, event, webhook)
- `inputs`: Unsupported top-level (compile rejection)
- `vars`: Parsed but validation coverage unknown
- `secrets`: Parsed but validation coverage unknown
- `steps`: Core workflow body
- `result`: Unsupported top-level (compile rejection)
- `examples`: Parsed but handling unknown

**Step Primitives**:
- **Supported**: set, for_each, together, collect, reduce, repeat, wait, ask, finish
- **Unsupported**: save, do, choose (compile rejection)

**Triggers**:
- manual, schedule, event, webhook

## Review Notes
- Domain model is well-defined in AST types (`vb_yaml/src/ast/types.rs`)
- Error taxonomy covers all known failure modes
- Gap analysis identifies 3 areas requiring follow-up beads

## Verdict
Domain model is sufficient for coverage matrix work.
