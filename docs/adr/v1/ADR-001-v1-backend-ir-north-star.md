# ADR 001 (v1): Backend / IR Interpreter North Star

## Status

Accepted as architecture baseline. Implementation completion requires evidence.

## Context

The active product is a Rust-nightly, no-unsafe, no-panic, single-server durable execution engine. The current milestone is Backend / IR Interpreter Complete.

## Decision

The active runtime mode is compiled numeric IR interpreted by shard-owned runtime state. YAML is authoring input only. The production trust unit is the accepted artifact, not the source YAML.

The product is not a generic DAG runner, low-code graph editor, YAML programming framework, Airflow replacement, Temporal clone, distributed workflow cluster, or current generated-code engine.

## Invariants

- Runtime never interprets YAML.
- Runtime core crates do not parse JSON, serve HTTP, or route text commands.
- Runtime references are numeric IDs and slots.
- Fjall persistence and Postcard records are current-scope requirements.
- Generated Rust, maxperf, PGO, and native UI are not current acceptance paths.

## Consequences

- Implementation agents must optimize and test the IR interpreter first.
- Any adapter using HTTP or JSON must live outside runtime core.
- Public claims must state the single-server boundary plainly.

## Master Anchors

- Section 0: Prime Directive
- Section 22: Removed Rust Codegen and Maxperf
- Section 44: Backend / IR Interpreter Definition of Done
- Section 68: Durable Execution Architecture Contract
