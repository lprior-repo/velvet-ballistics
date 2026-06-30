---
section: 34
title: "Repository Model and Agent Skills"
parent: velvet-ballistics-MASTER.md
---

## 34. Repository Model and Agent Skills

A Velvet workflow repository may contain:

```text
.velvet/
  artifacts/
  runs/
  reports/
  skills/
  policies/
  action-manifests/
  simulations/
  evidence/

workflows/
  issue_triage.rs
  credential_rotation.rs

actions/
  github.rs
  slack.rs
  filesystem.rs

policies/
  strict_ai.toml
  prod_default.toml

scenarios/
  issue_triage_success.toml
  issue_triage_timeout.toml
```

Agent-discoverable skills are allowed and encouraged, but skills are instructions only. They are not trusted runtime code.

`cargo velvet agent-context` emits bounded, versioned JSON describing:

```text
available workflows
action manifests
policy profiles
compiler commands
diagnostic codes
repair schema
simulation scenarios
runtime commands
forbidden patterns
```

---

