---
section: 13
title: "Action Manifest and Executor Split"
parent: velvet-ballistics-MASTER.md
---

## 13. Action Manifest and Executor Split

Action manifests are compiler-visible. Action executors are runtime-visible.

### Action manifest

```rust
pub struct ActionManifest {
    pub name: ActionName,
    pub action_id: ActionId,
    pub action_contract_digest: ActionContractDigest,
    pub input_schema_digest: SchemaDigest,
    pub output_schema_digest: SchemaDigest,
    pub side_effect: SideEffect,
    pub retry_safety: RetrySafety,
    pub idempotency_scope: IdempotencyScope,
    pub required_capabilities: RequiredCapabilities,
    pub required_secrets: SecretRequirements,
    pub timeout_policy: TimeoutPolicy,
    pub max_input_bytes: u32,
    pub max_output_bytes: u32,
    pub failure_codes: Box<[ActionFailureSpec]>,
}
```

### Action executor

```rust
pub trait ActionExecutor {
    fn action_id(&self) -> ActionId;

    fn execute(
        &self,
        ctx: ActionExecutionContext<'_>,
        ticket: ActionTicket,
        input: ActionInput,
    ) -> ActionResult<ActionCompletion>;
}
```

Runtime dispatch uses numeric `ActionId` only. Action names are cold metadata.

---

