---
section: 9
title: "Trigger Contract"
parent: velvet-ballistics-MASTER.md
---

## 9. Trigger Contract


v1 supports exactly these triggers in YAML authoring:

```yaml
when:
  manual: {}
```

```yaml
when:
  schedule:
    cron: "0 * * * *"
```

```yaml
when:
  event:
    type: github.pull_request
```

```yaml
when:
  webhook: {}
```

`manual` means direct Rust API submission (via `Runtime::submit`). `schedule`, `event`, and `webhook` are cold-path triggers handled by external adapters before submitting compiled artifacts to the runtime. HTTP/webhook adapters live outside `vb_core`, `vb_runtime`, `vb_storage`, and `vb_ipc`.

The binary IPC protocol (`vb_ipc`) is a separate runtime ingress mechanism, not a YAML trigger. `ipc` in the IR refers to the `ShardCommand::Submit` protocol, not a YAML-authored trigger.

---
