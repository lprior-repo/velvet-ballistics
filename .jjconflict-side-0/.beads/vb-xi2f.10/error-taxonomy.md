# Error Taxonomy — Section 16 Symbolic Diagnostic Codes

**Bead**: vb-xi2f.10  
**Phase**: State 3 — Rust Contract

---

## 1. Top-Level Error Classification

```
Diagnostic Errors
├── Validation Errors (Section 16)
│   ├── Schema Validation Errors (E01xx)
│   ├── Reference Validation Errors (E02xx)
│   ├── Control-Flow Validation Errors (E03xx)
│   └── Type/Taint/Resource Validation Errors (E04xx)
├── Gate Verifier Errors (E05xx)
├── Contract Discovery Errors (E06xx)
├── Compilation Errors (E10xx–E14xx)
│   ├── Internal Compilation Errors (E10xx)
│   ├── Workflow IR Errors (E11xx)
│   ├── Expression Errors (E12xx)
│   ├── Accessor/Path Errors (E13xx)
│   └── Lowering Errors (E14xx)
├── Storage Errors (E20xx)
├── Runtime Errors (E30xx)
│   └── Runtime Core Errors
├── Runtime Boundary Errors (E40xx)
│   └── Input/Output/Network Errors
└── Parse/Codec Errors
    ├── DiagnosticCodeParseError
    └── SymbolicCodeParseError
```

---

## 2. Complete Error Variant → Symbolic Code Mapping

### 2.1 ValidationError (vb_validate) — 58 variants

#### Schema Validation (E01xx): 11 codes

| # | Variant | Symbolic Code | Numeric |
|---|---------|--------------|---------|
| 1 | `DuplicateKey` | `DUPLICATE_KEY` | E0101 |
| 2 | `ForbiddenYamlFeature` | `FORBIDDEN_YAML_FEATURE` | E0102 |
| 3 | `UnknownTopLevelField` | `UNKNOWN_TOP_LEVEL_FIELD` | E0103 |
| 4 | `UnknownStepField` | `UNKNOWN_STEP_FIELD` | E0104 |
| 5 | `MissingRequiredField { field }` | `MISSING_REQUIRED_FIELD` | E0105 |
| 6 | `InvalidVersion { version }` | `INVALID_VERSION` | E0106 |
| 7 | `InvalidId { id }` | `INVALID_ID` | E0107 |
| 8 | `ReservedId { id }` | `RESERVED_ID` | E0108 |
| 9 | `DuplicateId { id }` | `DUPLICATE_ID` | E0109 |
| 10 | `MultipleStepPrimitives` | `MULTIPLE_STEP_PRIMITIVES` | E010A |
| 11 | `MissingStepPrimitive` | `MISSING_STEP_PRIMITIVE` | E010B |

#### Reference Validation (E02xx): 4 codes

| # | Variant | Symbolic Code | Numeric |
|---|---------|--------------|---------|
| 12 | `UnknownReference { reference }` | `UNKNOWN_REFERENCE` | E0201 |
| 13 | `FutureReference { reference }` | `FUTURE_REFERENCE` | E0202 |
| 14 | `SecretNotDeclared { secret }` | `SECRET_NOT_DECLARED` | E0203 |
| 15 | `DirectRuntimeReference` | `DIRECT_RUNTIME_REFERENCE` | E0204 |

#### Control-Flow Validation (E03xx): 9 codes

| # | Variant | Symbolic Code | Numeric |
|---|---------|--------------|---------|
| 16 | `InvalidThenTarget` | `INVALID_THEN_TARGET` | E0301 |
| 17 | `ControlFlowCycle` | `CONTROL_FLOW_CYCLE` | E0302 |
| 18 | `UnreachableStep { step }` | `UNREACHABLE_STEP` | E0303 |
| 19 | `InvalidChoose` | `INVALID_CHOOSE` | E0304 |
| 20 | `InvalidForEach` | `INVALID_FOR_EACH` | E0305 |
| 21 | `InvalidTogether` | `INVALID_TOGETHER` | E0306 |
| 22 | `InvalidCollect` | `INVALID_COLLECT` | E0307 |
| 23 | `InvalidReduce` | `INVALID_REDUCE` | E0308 |
| 24 | `InvalidRepeat` | `INVALID_REPEAT` | E0309 |

#### Type/Taint/Resource Validation (E04xx): 12 codes

| # | Variant | Symbolic Code | Numeric |
|---|---------|--------------|---------|
| 25 | `InvalidWait` | `INVALID_WAIT` | E0401 |
| 26 | `InvalidAsk` | `INVALID_ASK` | E0402 |
| 27 | `InvalidFinish` | `INVALID_FINISH` | E0403 |
| 28 | `InvalidRetry` | `INVALID_RETRY` | E0404 |
| 29 | `InvalidOnError` | `INVALID_ON_ERROR` | E0405 |
| 30 | `SecretResultLeak` | `SECRET_RESULT_LEAK` | E0406 |
| 31 | `TypeMismatch { expected, found }` | `TYPE_MISMATCH` | E0407 |
| 32 | `PayloadTooLarge` | `PAYLOAD_TOO_LARGE` | E0408 |
| 33 | `LimitRequired { resource }` | `LIMIT_REQUIRED` | E0409 |
| 34 | `LimitExceeded { resource }` | `LIMIT_EXCEEDED` | E040A |
| 35 | `UnsupportedTrigger { trigger }` | `UNSUPPORTED_TRIGGER` | E040B |
| 36 | `HttpTriggerOutOfCore` | `HTTP_TRIGGER_OUT_OF_CORE` | E040C |

#### Gate Verifier (E05xx): 19 codes

| # | Variant | Symbolic Code | Numeric |
|---|---------|--------------|---------|
| 37 | `ExpressionStackExceeded { declared, limit }` | `EXPRESSION_STACK_EXCEEDED` | E0501 |
| 38 | `ExpressionStackMismatch { .. }` | `EXPRESSION_STACK_MISMATCH` | E0502 |
| 39 | `AccessorSlotOutOfRange { .. }` | `ACCESSOR_SLOT_OUT_OF_RANGE` | E0503 |
| 40 | `AccessorPathInvalid { .. }` | `ACCESSOR_PATH_INVALID` | E0504 |
| 41 | `AccessorPathTooDeep { .. }` | `ACCESSOR_PATH_TOO_DEEP` | E0512 |
| 42 | `AccessorSymbolOutOfBounds { .. }` | `ACCESSOR_SYMBOL_OUT_OF_BOUNDS` | E0513 |
| 43 | `SlotReferenceOutOfRange { .. }` | `SLOT_REFERENCE_OUT_OF_RANGE` | E0505 |
| 44 | `LoopBodyStepOutOfRange { .. }` | `LOOP_BODY_STEP_OUT_OF_RANGE` | E0506 |
| 45 | `SlotDependencyCycle { slot, chain }` | `SLOT_DEPENDENCY_CYCLE` | E0507 |
| 46 | `NodeKindConstraintViolation { .. }` | `NODE_KIND_CONSTRAINT_VIOLATION` | E0508 |
| 47 | `ActionContractMissing { .. }` | `ACTION_CONTRACT_MISSING` | E0509 |
| 48 | `ActionContractOrphan { .. }` | `ACTION_CONTRACT_ORPHAN` | E050A |
| 49 | `SlotTypeInconsistency { slot }` | `SLOT_TYPE_INCONSISTENCY` | E050B |
| 50 | `NonDeterministicPath { .. }` | `NON_DETERMINISTIC_PATH` | E050C |
| 51 | `CapabilityNameEmpty { .. }` | `CAPABILITY_NAME_EMPTY` | E050D |
| 52 | `CapabilityNameTooLong { .. }` | `CAPABILITY_NAME_TOO_LONG` | E050E |
| 53 | `CapabilityNameInvalid { .. }` | `CAPABILITY_NAME_INVALID` | E050F |
| 54 | `CapabilityActionMismatch { .. }` | `CAPABILITY_ACTION_MISMATCH` | E0510 |
| 55 | `CapabilityDuplicate { .. }` | `CAPABILITY_DUPLICATE` | E0511 |

#### Contract Discovery (E06xx): 3 codes

| # | Variant | Symbolic Code | Numeric |
|---|---------|--------------|---------|
| 56 | `MissingSchemaVersion` | `MISSING_SCHEMA_VERSION` | E0601 |
| 57 | `CueVetFailed { file }` | `CUE_VET_FAILED` | E0602 |
| 58 | `VersionMonotonicityBreach { .. }` | `VERSION_MONOTONICITY_BREACH` | E0603 |

---

### 2.2 CompileError (vb_compile) — 60+ variants → 30+ symbolic codes

CompileError's `code()` method maps 60+ variants to 30+ symbolic codes. Some symbolic codes are shared with ValidationError; others are compilation-specific.

#### Compilation-specific symbolic codes (not in Section 16)

| Symbolic Code | Used By | Notes |
|--------------|---------|-------|
| `UNKNOWN_INPUT_SCHEMA_FIELD` | `UnknownInputSchemaField { .. }` | Compiler: input schema validation |
| `UNSUPPORTED_TOP_LEVEL_DECLARATION` | `UnsupportedTopLevelDeclaration { .. }` | Compiler: top-level declarations |
| `UNKNOWN_OUTPUT_NAME` | `UnknownOutputName { .. }` | Compiler: output name references |
| `UNSUPPORTED_ACCESSOR_REFERENCE` | `UnsupportedAccessorReference { .. }` | Compiler: accessor patterns |
| `INVALID_EXPRESSION` | 8 expression variants | Compiler: expression lex/parse/lower |
| `IDEMPOTENCY_VIOLATION` | `IdempotencyViolation { .. }` | Compiler: action idempotency |
| `INVALID_COMPILED_WORKFLOW` | 12+ WorkflowError variants | Compiler: IR validation |
| `CONST_OUT_OF_BOUNDS` | `WorkflowError::ConstOutOfBounds` | Compiler: constant indices |

---

### 2.3 YamlError (vb_yaml) — 20 variants → 6 symbolic codes

| Symbolic Code | YamlError Variants Mapping To It |
|--------------|--------------------------------|
| `DUPLICATE_KEY` | `DuplicateKey { .. }` |
| `FORBIDDEN_YAML_FEATURE` | `ForbiddenFeature`, `AnchorAliasMerge`, `CustomTag`, `BinaryScalar`, `AmbiguousScalar`, `UnsupportedFeature`, `MultipleDocuments`, `ParseError` |
| `UNSUPPORTED_TRIGGER` | `UnsupportedTrigger { .. }` |
| `PAYLOAD_TOO_LARGE` | `SourceTooLarge { .. }` |
| `LIMIT_EXCEEDED` | `NestingTooDeep`, `NodeLimitExceeded`, `ScalarTooLong`, `SequenceTooLong`, `MappingTooLarge` |
| `UNKNOWN_TOP_LEVEL_FIELD` | `UnknownField { .. }` |
| `MISSING_REQUIRED_FIELD` | `EmptySource`, `MissingField { .. }` |
| `TYPE_MISMATCH` | `FieldShape { .. }` |

---

### 2.4 Codec/Parse Errors — Non-diagnostic errors

These are errors that occur during code parsing, not error diagnosis:

| Error | Variants | Domain |
|-------|----------|--------|
| `DiagnosticCodeParseError` | `InvalidFormat`, `UnsupportedCode` | Parsing `"E0101"` strings |
| `SymbolicCodeParseError` (new) | `UnknownCode` | Parsing symbolic code strings |

These are **not** diagnostic codes themselves — they are errors in the diagnostic code infrastructure.

---

## 3. Error Severity Classification

| Severity | Description | Section 16 Codes |
|----------|-------------|-----------------|
| **Error** | Blocks compilation/validation. Workflow cannot proceed. | All 36 Section 16 codes, all E05xx gate codes, all E06xx contract codes. |
| **Warning** | Diagnostic but non-blocking. | None currently in scope for this bead. Reserved for future use. |
| **Info** | Informational only. | None in scope. |

**Contract**: All `ValidationError`, `GateError`, and `ContractDiscoveryError` variants produce `Severity::Error`.

---

## 4. Error Ownership Boundaries

```
vb_core ──owns──→ SymbolicCode, DiagnosticCode, Diagnostic, CodeRegistry, Severity
vb_validate ──owns──→ ValidationError, ValidationError::code() → SymbolicCode
vb_compile ──owns──→ CompileError, CompileError::code() → SymbolicCode
vb_yaml ──owns──→ YamlError, YamlError::code() → SymbolicCode
vb_runtime ──owns──→ RuntimeError, RuntimeError::symbolic_code() → SymbolicCode
vb_storage ──owns──→ JournalError, JournalError::symbolic_code() → SymbolicCode
```

No crate may define its own `DiagnosticCode` constants or its own code registry. All must import from `vb_core`.

---

## 5. Error Propagation Railway

```
                    ┌─────────────────┐
                    │  External Input  │
                    │  (YAML, HTTP,    │
                    │   CLI args)      │
                    └────────┬────────┘
                             │
                             ▼
               ┌─────────────────────────┐
               │   Parse/Validate        │
               │   Boundary              │
               │   (YamlError,           │
               │    ParseError,          │
               │    CodecError)           │
               └────────────┬────────────┘
                            │
               ┌────────────▼────────────┐
               │  Validation Pipeline    │
               │  (ValidationError)      │
               │  → Schema               │
               │  → References           │
               │  → Control Flow         │
               │  → Type/Taint           │
               │  → Gate Verifier        │
               │  → Contract Discovery   │
               └────────────┬────────────┘
                            │
               ┌────────────▼────────────┐
               │  Compilation Pipeline   │
               │  (CompileError)         │
               │  → AST building          │
               │  → IR lowering           │
               │  → Expression compile    │
               └────────────┬────────────┘
                            │
               ┌────────────▼────────────┐
               │  IR Validation          │
               │  (WorkflowError →       │
               │   CompileError)         │
               └────────────┬────────────┘
                            │
               ┌────────────▼──┬────────┐
               │  Diagnostic   │        │
               │  Emission     │        │
               │  (Diagnostic  │        │
               │   { code:     │        │
               │     Symbolic  │        │
               │     Code })   │        │
               └───────────────┴────────┘
```

At every stage, errors carry a `SymbolicCode`. The railway never loses the code.

---

## 6. Unrepresentable Error States

| Forbidden State | How Enforced |
|----------------|-------------|
| Error without a code | `HasSymbolicCode` trait; exhaustive match in `code()` |
| Code not in registry | `SymbolicCode` smart constructor rejects unregistered strings |
| Symbolic and numeric mismatch | Registry bijection; derived `as_diagnostic_code()` is deterministic |
| Numeric code accepted but not registered | `is_supported_code()` guard in `FromStr` |
| Two variants share same symbolic code (when they should be distinct) | Registry audit test asserts all 58+ codes are unique |
