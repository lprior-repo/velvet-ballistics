# Bug Hunt 2026-06-21 — vb_core / vb_storage / vb_runtime Source Review

## Scope

Source-only review (NO tests / proof artifacts / kani harnesses / verification
modules). Every finding must be validated against actual production code paths
before being surfaced.

## Areas

- `storage-journal/`     — vb_storage/src/journal/** (append, replay, incident)
- `storage-recovery/`    — vb_storage/src/recovery/** (event_replay, hydrate, snapshot)
- `storage-admission/`   — vb_storage/src/admission/** + queue/** + batch/**
- `storage-codec/`       — vb_storage/src/codec/** + keys/** + binary.rs + trimming/**
- `runtime-shard/`       — vb_runtime/src/shard/** (lifecycle, transitions, arena, queue, timer)
- `runtime-primitives/`  — vb_runtime/src/primitives/** + action_queue/**
- `runtime-engine/`      — vb_runtime/src/engine/** + handlers/**
- `runtime-admission/`   — vb_runtime/src/admission/** + runtime/** + journal.rs + recovery.rs
- `core-budget/`         — vb_core/src/budget/**
- `core-frame/`          — vb_core/src/frame/** + value_store/** + ids/**
- `core-workflow/`       — vb_core/src/workflow/** (validation, compiled_slug/query, lifecycle)
- `core-engine/`         — vb_core/src/engine/** + replay/**
- `core-value/`          — vb_core/src/value/** + action/** + policy/** + diagnostic/**
- `cross-cutting/`       — findings that span crates

## Finding File Naming

`NNN-short-slug.md` — zero-padded 3-digit ID, area-specific.

## Finding Template

```markdown
# <ID>: <Title>

- **Severity**: Critical | High | Medium | Low | Info
- **Category**: bug | perf | simplification | correctness | concurrency
- **Location**: `crates/<crate>/src/<path>:<line>`- <optional> **Confidence**: confirmed | likely | speculative

## Description
<What is wrong, in 1-2 sentences>

## Evidence
<Code excerpt, invariants violated, or reproduction reasoning>

## Adversarial Check
<Why this is NOT a false positive. Consider alternative interpretations.>

## Suggested Fix
<Concrete patch or functional-rust / holzman-rust inspired simplification>
```

## Severity Bar

- **Critical**: data corruption, deadlock, panic in prod, security hole, lost writes
- **High**: wrong behavior under realistic inputs, race window, resource leak
- **Medium**: edge-case bug, perf regression in hot path, brittle error handling
- **Low**: minor correctness issue, code smell, small perf nit
- **Info**: simplification opportunity, readability, maintainability

## Validation Rules

1. Cite real file:line with code excerpt.
2. Adversarial paragraph required — explicitly argue why this is real.
3. Do NOT flag test/proof/kani code.
4. Do NOT flag intentional `#[cfg(test)]` blocks.
5. Performance claims need a hot-path justification (loop body, allocator
   pressure, lock contention).
