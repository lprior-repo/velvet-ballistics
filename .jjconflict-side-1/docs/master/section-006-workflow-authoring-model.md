---
section: 6
title: "Workflow Authoring Model"
parent: velvet-ballistics-MASTER.md
---

## 6. Workflow Authoring Model

The workflow authoring surface is a constrained Rust SDK DSL.

Allowed authoring forms:

```text
velvet_workflow! { ... }
velvet_expr! { ... }
velvet_action! { ... }
#[derive(VelvetInput)]
#[derive(VelvetOutput)]
#[derive(VelvetData)]
policy! { ... }
```

The macro DSL is source code, but the workflow body is not arbitrary Rust. The macro owns the grammar and emits a `WorkflowDefinition`, not executable workflow behavior.

### Example

```rust
use velvet_sdk::prelude::*;

#[derive(VelvetInput)]
pub struct IssueInput {
    pub repo: Symbol,
    pub ticket_id: Symbol,
    pub message: Symbol,
    pub channel: Symbol,
}

#[derive(VelvetOutput)]
pub struct IssueResult {
    pub issue_number: I64,
    pub issue_url: Symbol,
}

velvet_workflow! {
    workflow issue_triage(input: IssueInput) -> IssueResult
    policy strict_ai

    capabilities {
        network.github.write,
        network.slack.write,
        secrets.github_token,
    }

    steps {
        let classified = action ai::classify_ticket {
            input: {
                message: input.message,
            },
            timeout: ms(5_000),
            retry: none,
        };

        let issue = retry max_attempts = 3,
                          backoff = exponential(ms(250), ms(2_000)),
                          on = [RateLimited, Timeout] {
            action github::issue_create {
                input: {
                    repo: input.repo,
                    title: classified.title,
                    body: classified.body,
                },
                timeout: ms(10_000),
                idempotency_key: key!(
                    "github.issue_create",
                    input.repo,
                    input.ticket_id
                ),
            }
        };

        action slack::send_message {
            input: {
                channel: input.channel,
                text: issue.url,
            },
            timeout: ms(5_000),
            idempotency_key: key!(
                "slack.send_message",
                input.channel,
                issue.number
            ),
        };

        finish IssueResult {
            issue_number: issue.number,
            issue_url: issue.url,
        };
    }
}
```

### Required property

This workflow definition compiles into data. It does not become a Rust function that performs actions.

---

