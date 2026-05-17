bead_id: vb-0253.7
bead_title: cli: Make lifecycle tracker event-applied
phase: 1
updated_at: 2026-05-17T20:32:59.341998+00:00
attempt: 1-of-7

STATUS: BASELINE_CAPTURED
source_checkout: /home/lewis/src/velvet-ballistics
isolated_workspace: /home/lewis/src/femdation-vb-0253-7

Path isolation:
- pwd -P: /home/lewis/src/femdation-vb-0253-7
- equals source: False
- nested under source: False

bd show exit: 1
```json
{
  "error": "no issues found matching the provided IDs"
}

```

jj status exit: 0
```text
The working copy has no changes.
Working copy  (@) : nyputrkz e0e18d7b (empty) (no description set)
Parent commit (@-): xxsyqsus 97be914f main | test: kill mutation survivors in canonical/validate functions


```

Baseline note: full moon ci intentionally not run in State 1 to avoid pre-edit fleet cost; State 11 must run scoped bead gates and compare against this clean isolated jj baseline.
