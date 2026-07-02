---
section: 3
title: "Source of Truth Map"
parent: velvet-ballistics-MASTER.md
---

## 3. Source of Truth Map

The master contract is the architectural spine. Executable contracts live in versioned data files and generated tests. The code is authoritative for exact Rust field layout, but this document is authoritative for behavior.

```text
/velvet-ballistics-MASTER.md                     normative architecture spine
contracts/sdk_language.ron                       SDK DSL grammar and allowed constructs
contracts/forbidden_syntax.ron                   forbidden imports/macros/tokens by zone
contracts/action_abi.ron                         action manifest wire contract
contracts/idempotency.ron                        idempotency scope/key rules
contracts/policy_profiles.ron                    policy profile definitions
contracts/journal_events.ron                     durable history event registry
contracts/ipc_commands.ron                       binary IPC command registry
contracts/cli_commands.ron                       CLI and cargo subcommand registry
contracts/errors.ron                             stable diagnostic/runtime/storage/IPC codes
contracts/resource_profiles.ron                  boundedness and runtime resource defaults
contracts/invariants.ron                         machine-checkable invariants
contracts/compile_fail_tests.ron                 required SDK compile-fail fixtures
```

Generated Rust enums, artifact schemas, CLI help, agent context, diagnostic code tables, fuzz seed corpora, and conformance tests must derive from these contracts where practical.

---

