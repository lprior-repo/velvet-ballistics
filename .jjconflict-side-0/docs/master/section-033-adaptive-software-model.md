---
section: 33
title: "Adaptive Software Model"
parent: velvet-ballistics-MASTER.md
---

## 33. Adaptive Software Model

Adaptive does not mean runtime chaos. It means controlled variation under compiler-verified boundaries.

Allowed adaptation mechanisms:

```text
choose branches from typed conditions
retry with explicit policy
fallback action with explicit failure class
policy-selected action implementation with same manifest contract
runtime operator grants narrower than or equal to artifact requirements
parameterized resource profiles bound by policy digest
agent-generated workflow patch followed by compiler verification
simulation scenario selection before production submission
```

Forbidden adaptation mechanisms:

```text
self-modifying workflow at runtime
action changing side-effect class after verification
runtime loading unknown actions by string name
agent patching an installed artifact without new verification
secret-dependent public branch claimed as secret-safe
resource limit expansion without new policy digest
silent fallback to unsafe shell/process execution
```

Adaptive workflow authoring loop:

```text
observe incident/replay report
AI proposes workflow or policy patch
compiler verifies patch
simulation exercises scenario
operator approves artifact/grants
runtime runs new artifact
old artifact remains immutable
```

This is adaptive software through artifact evolution, not mutable production behavior.

---

