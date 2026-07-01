# Moon CI Verifier-Gate Wiring (vb-g6xgs)

**STATUS:** `moon ci` does not yet invoke the Verus binding-gate
(`scripts/check-verus-production-binding.sh`) or the production-inner
drift-gate (`scripts/check-production-inner-drift.sh`). This document
records the required wiring so the follow-up implementation lands it
without further audit.

## Required Wiring

Add the following task to `.moon/tasks/all.yml` so it is part of the
canonical CI gate:

```yaml
  verifier-production-binding:
    command: 'bash scripts/check-verus-production-binding.sh && bash scripts/check-production-inner-drift.sh'
    toolchains:
      - rust
    inputs:
      - '@globs(verification)'
      - '@globs(scripts)'
    options:
      runInCI: true
```

Then wire `verifier-production-binding` as a dep of the existing `ci`
task:

```yaml
  ci:
    deps:
      - ':lint-src'
      - ':test'
      - ':verifier-production-binding'   # <-- new dep
      # ... existing deps ...
```

## Acceptance Criteria (this bead group)

- [x] Required wiring documented above.
- [x] `scripts/check-verus-production-binding.sh` is identified as the
      source of truth for "is Verus artifact X bound to production?".
- [x] `scripts/check-production-inner-drift.sh` is identified as the
      source of truth for "has the production mirror drifted from
      production?".
- [ ] The wiring above is applied to `.moon/tasks/all.yml` in a
      follow-up bead.

Until the wiring is applied, `moon ci` may pass without checking Verus
binding or production-mirror drift. This document ensures the gap is
visible and the fix is well-specified.