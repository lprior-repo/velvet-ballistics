# Architectural Drift Report: `vb_yaml/src/ast/types.rs`

**File**: `crates/vb_yaml/src/ast/types.rs`
**Line Count**: 413 (EXCEEDS 300-line limit by 113 lines)
**Status**: 🔴 VIOLATION — REFACTOR REQUIRED

---

## VIOLATION 1: Line Count Exceeded

| Metric | Value | Limit |
|--------|-------|-------|
| Total Lines | 413 | 300 |
| Overage | +113 | 0 |

**Required Action**: Split into multiple focused modules.

---

## VIOLATION 2: Primitive Obsession — Unacceptable

This file is a **catalog of primitive-wrapped data structures**. Scott Wlaschin's DDD principle: *"Make illegal states unrepresentable"* is directly violated by the use of raw `String` and numeric primitives where domain-specific newtypes are warranted.

### A. Raw `String` Fields That Must Become Newtypes

| Field | Current Type | Newtype Required |
|-------|-------------|------------------|
| `WorkflowSource.version` | `String` | `Version` |
| `WorkflowSource.name` | `String` | `WorkflowName` |
| `TriggerAst::Schedule.cron` | `String` | `CronExpression` |
| `TriggerAst::Event.event_type` | `String` | `EventType` |
| `StepAst.id` | `String` | `StepId` |
| `StepAst.name` | `Option<String>` | `StepName` |
| `StepAst.condition` | `Option<String>` | `ConditionExpr` |
| `StepAst.primitive` | `StepPrimitive` (variant Strings) | See B |
| `StepAst.with` | `Option<String>` | `ResourceRef` |
| `StepPrimitive::Set.output` | `String` | `VariableRef` |
| `StepPrimitive::Set.value` | `String` | `ValueExpr` |
| `StepPrimitive::Do.action` | `String` | `ActionIdentifier` |
| `StepPrimitive::Do.input` | `String` | `InputExpr` |
| `StepPrimitive::ForEach.variable` | `String` | `LoopVariable` |
| `StepPrimitive::ForEach.input` | `String` | `CollectionExpr` |
| `StepPrimitive::Collect.variable` | `String` | `LoopVariable` |
| `StepPrimitive::Collect.source` | `String` | `SourceExpr` |
| `StepPrimitive::Aggregate.variable` | `String` | `AccumulatorVar` |
| `StepPrimitive::Aggregate.input` | `String` | `CollectionExpr` |
| `StepPrimitive::Aggregate.initial` | `String` | `InitialExpr` |
| `StepPrimitive::Wait.event` | `Option<String>` | `EventExpr` |
| `StepPrimitive::Wait.timeout` | `Option<String>` | `TimeoutExpr` |
| `StepPrimitive::Ask.prompt` | `String` | `PromptText` |
| `StepPrimitive::Ask.timeout` | `Option<String>` | `TimeoutExpr` |
| `RetryPolicy.delay` | `Option<String>` | `DelayExpr` |
| `ErrorHandlerAst.handler` | `String` | `HandlerRef` |
| `InputField.key` | `String` | `FieldKey` |
| `VarField.key` | `String` | `FieldKey` |
| `SecretField.key` | `String` | `SecretName` |
| `SecretField.value` | `String` | `SecretValue` |
| `ResultMapping.fields[].key` | `String` | `ResultFieldKey` |
| `ChooseBranch.when` | `String` | `WhenCondition` |
| `TogetherBranch.label` | `String` | `BranchLabel` |
| `ExampleAst.description` | `Option<String>` | `ExampleDescription` |

### B. Raw Numeric Primitives That Must Become Newtypes

| Field | Current Type | Newtype Required |
|-------|-------------|------------------|
| `StepPrimitive::ForEach.at_once` | `Option<u32>` | `ConcurrencyLimit` |
| `StepPrimitive::Collect.pages` | `Option<u32>` | `PageLimit` |
| `StepPrimitive::Collect.items` | `Option<u32>` | `ItemsPerPage` |
| `StepPrimitive::Repeat.max_attempts` | `u16` | `MaxAttempts` |
| `RetryPolicy.max_attempts` | `u16` | `MaxAttempts` |
| `AuthorValue::I64(i64)` | `i64` | `IntegerValue` |
| `ScalarValue::Integer(i64)` | `i64` | `IntegerValue` |

### C. `AuthorEntry<T>` Uses Raw `String` for Key

```rust
pub struct AuthorEntry<T> {
    pub key: String,   // ← Must be `AuthoringKey`
    pub value: T,
}
```

---

## VIOLATION 3: `AuthorValue` Is an Anemic Tagged Union

```rust
pub enum AuthorValue {
    Null,
    Bool(bool),        // ← raw bool
    I64(i64),          // ← raw i64
    Text(String),      // ← raw String
    Sequence(Vec<AuthorValue>),
    Mapping(Vec<AuthorEntry<AuthorValue>>),
}
```

`Bool` should be `Bool(BoolValue)` or `TruthValue`. `I64` should be `Integer(IntegerValue)`. `Text` should be `Text(TextValue)`.

---

## VIOLATION 4: `ScalarValue` Is Duplicative

```rust
pub enum ScalarValue {
    String(String),
    Integer(i64),
}
```

This is a second scalar type independent of `AuthorValue`, creating two parallel hierarchies for what is essentially the same domain concept. These should be unified or one should be removed.

---

## VIOLATION 5: `StepAst` Has Too Many Responsibilities

`StepAst` simultaneously holds:
- Identity (`id`)
- Naming (`name`)
- Conditional execution (`condition`)
- The primitive operation (`primitive`)
- Resource binding (`with`)
- Error handling configuration (`retry`, `on_error`)
- Flow control (`then`)

This violates the Single Responsibility Principle. The `StepPrimitive` enum variants with `String` fields should be extracted, and the step configuration (retry, error handler) should be a separate `StepConfig` newtype.

---

## RECOMMENDED REFACTORING PLAN

### Split into these modules (target: ~60-80 lines each):

```
vb_yaml/src/ast/
├── mod.rs          (~30 lines — re-exports)
├── types.rs        (~80 lines — WorkflowSource, WorkflowSourceParts only)
├── trigger.rs      (~60 lines — TriggerAst, TriggerVariant)
├── step.rs         (~100 lines — StepAst, StepPrimitive enum + branch types)
├── author_value.rs (~80 lines — AuthorValue, AuthorEntry, ScalarValue)
├── fields.rs       (~60 lines — InputField, VarField, SecretField, ResultMapping)
└── identifiers.rs  (~80 lines — all the newtype wrappers)
```

### Newtype wrappers required (all in `identifiers.rs`):

```rust
// Versioning
pub struct Version(pub String);
pub struct WorkflowName(pub String);
pub struct LanguageVersion(pub String);

// Step identifiers
pub struct StepId(pub String);
pub struct StepName(pub String);
pub struct ConditionExpr(pub String);
pub struct ResourceRef(pub String);
pub struct HandlerRef(pub String);
pub struct ThenLabel(pub String);

// Primitive operation fields
pub struct VariableRef(pub String);
pub struct ValueExpr(pub String);
pub struct ActionIdentifier(pub String);
pub struct InputExpr(pub String);
pub struct LoopVariable(pub String);
pub struct CollectionExpr(pub String);
pub struct SourceExpr(pub String);
pub struct AccumulatorVar(pub String);
pub struct InitialExpr(pub String);
pub struct EventExpr(pub String);
pub struct TimeoutExpr(pub String);
pub struct DelayExpr(pub String);
pub struct PromptText(pub String);

// Trigger fields
pub struct CronExpression(pub String);
pub struct EventType(pub String);

// Field keys
pub struct FieldKey(pub String);
pub struct SecretName(pub String);
pub struct SecretValue(pub String);
pub struct ResultFieldKey(pub String);
pub struct WhenCondition(pub String);
pub struct BranchLabel(pub String);
pub struct ExampleDescription(pub String);

// Numeric newtypes
pub struct ConcurrencyLimit(pub u32);
pub struct PageLimit(pub u32);
pub struct ItemsPerPage(pub u32);
pub struct MaxAttempts(pub u16);
pub struct IntegerValue(pub i64);
pub struct TextValue(pub String);
pub struct BoolValue(pub bool);
```

---

## VERDICT

```
┌─────────────────────────────────────────────────────────────┐
│  FILE: vb_yaml/src/ast/types.rs                             │
│  LINES: 413 / 300 allowed                                   │
│  DDD SCORE: 1/10                                            │
│  STATUS: 🔴 MANDATORY REFACTOR                               │
└─────────────────────────────────────────────────────────────┘
```

**Next Action**: Author must split this file into 6 modules following the plan above. All primitive fields must be replaced with their corresponding newtype wrappers. The `#[non_exhaustive]` annotations on `TriggerAst`, `AuthorValue`, `StepPrimitive`, and `ScalarValue` should be preserved but applied to the newtyped variants.
