# Error Taxonomy: Digest Covers Collect Semantics (vb-xi2f.38)

## Error Hierarchy

```
WorkflowError (vb_core/src/workflow/mod.rs)
├── EmptyNodes
├── EntryOutOfBounds { entry }
├── StepOutOfBounds { step }
├── SlotOutOfBounds { slot }
├── ConstOutOfBounds { constant }
├── NodeIdMismatch { expected, actual }
├── Expression(CoreError)
├── ResourceContractExceeded { resource }
└── ResourceContractTooLarge { resource }

CompileErrors (vb_compile/src/compile/mod.rs)
└── Vec<CompileError>
    └── CompileError (from ValidationError)

ValidationError (vb_validate/src/lib.rs)
├── DUPLICATE_KEY
├── FORBIDDEN_YAML_FEATURE
├── UNKNOWN_TOP_LEVEL_FIELD
├── UNKNOWN_STEP_FIELD
├── MISSING_REQUIRED_FIELD { field }
├── INVALID_VERSION { version }
├── INVALID_ID { id }
├── RESERVED_ID { id }
├── DUPLICATE_ID { id }
├── MULTIPLE_STEP_PRIMITIVES
├── MISSING_STEP_PRIMITIVE
├── UNKNOWN_REFERENCE { reference }
├── FUTURE_REFERENCE { reference }
├── SECRET_NOT_DECLARED { secret }
├── DIRECT_RUNTIME_REFERENCE
├── INVALID_THEN_TARGET
├── CONTROL_FLOW_CYCLE
├── UNREACHABLE_STEP { step }
├── INVALID_CHOOSE
├── INVALID_FOR_EACH
├── INVALID_TOGETHER
├── INVALID_COLLECT          ◄── Collect-specific validation
├── INVALID_REDUCE
├── INVALID_REPEAT
├── INVALID_WAIT
├── INVALID_ASK
├── INVALID_FINISH
├── INVALID_RETRY
├── INVALID_ON_ERROR
├── SECRET_RESULT_LEAK
├── TYPE_MISMATCH { expected, found }
├── PAYLOAD_TOO_LARGE
├── LIMIT_REQUIRED { resource }
├── LIMIT_EXCEEDED { resource }
├── UNSUPPORTED_TRIGGER { trigger }
├── HTTP_TRIGGER_OUT_OF_CORE
├── EXPRESSION_STACK_EXCEEDED { declared, limit }
└── EXPRESSION_STACK_MISMATCH { expr_index, declared, computed }

CoreError (vb_core/src/errors.rs)
├── ExpressionStackExceeded { declared, limit }
├── SlotOutOfBounds { slot }
├── ExprOutOfBounds { index }
├── ValueMismatch { expected, found }
└── ... (other expression/slot errors)
```

---

## Digest-Specific Error Variants

### Domain Error: DigestCoverageDefect (Conceptual)
**Severity**: CRITICAL — this is the bug being fixed

**Description**: `digest_step_primitive` for `StepPrimitive::Collect` only hashes the primitive name `"collect"` but omits:
- `variable: String`
- `source: String`
- `pages: Option<u32>`
- `items: Option<u32>`
- `body: Vec<StepAst>`

**Impact**: Two workflows with different `Collect` parameters but identical step IDs produce identical source digests.

**Detection**: Property test with two workflows differing only in `Collect.pages`; assert digests differ.

**Remediation**: Add explicit `Collect` match arm to `digest_step_primitive` that hashes all fields recursively.

---

### Domain Error: IncompletePrimitiveHash (Conceptual)
**Severity**: HIGH — risk of incorrect content-addressing

**Description**: All `StepPrimitive` variants other than `Set` and `Finish` use the catch-all `canonical_primitive_name` path, which only hashes the variant name string.

**Affected Variants**:
- `Collect` (CURRENT BUG — confirmed)
- `ForEach` (same risk pattern)
- `Aggregate` (same risk pattern)
- `Together` (branches not hashed)
- `Choose` (branches/otherwise not hashed)
- `Repeat` (max_attempts not hashed)
- `Do` (action/input not hashed)
- `Save` (value not hashed)
- `Wait` (event/timeout not hashed)
- `Ask` (prompt/timeout not hashed)

**Impact**: Any two workflows differing only in these primitive fields produce identical digests.

---

## ValidationError::InvalidCollect

**YAML Shape**: `collect` primitive with invalid structure

**Validation Rules**:
1. `variable` field MUST be present and non-empty string
2. `source` field MUST be present and non-empty string
3. `pages` if present MUST be `>= 1`
4. `items` if present MUST be `>= 1`
5. `body` MUST contain at least one step

**Error Display**: `INVALID_COLLECT`

**Occurrences**:
- `vb_validate/src/gates.rs` — `CollectStart` node validation
- `vb_validate/src/shared.rs` — shared validation pipeline
- `vb_validate/src/type_taint_tests.rs` — type/limit checking

---

## Artifact Digest Errors

### ArtifactDigestMismatch (Storage Test Concept)
**Description**: Submitted artifact bytes produce a different digest than claimed.

**Detection**: `vb_storage` admission computes `compute_compiled_digest(artifact_bytes)` and compares to claimed `WorkflowDigest`.

**Failure Mode**: Fail-closed — artifact rejected if digest doesn't match.

---

## Railway Error Map: Digest Computation

```
canonical_digest(source)
    │
    ├─[OK]─► WorkflowDigest (content-addressed identity)
    │
    └─[ERR]─► No errors possible (pure function)
                Note: panics if source is malformed (should not happen
                as source comes from successful YAML parse)

digest_step_primitive(hasher, primitive)
    │
    ├─[OK]─► (hasher mutated)
    │
    └─[ERR]─► No errors possible (pure function)

compute_compiled_digest(artifact_bytes)
    │
    ├─[OK]─► WorkflowDigest
    │
    └─[ERR]─► No errors possible (BLAKE3 is total)
```

---

## Semantic Error Lattice

```
FATAL (blocks compilation)
├── YAML parse failure
├── Duplicate step IDs
└── Control flow cycle

HARSH (compilation succeeds, validation fails)
├── INVALID_COLLECT
├── INVALID_REDUCE
├── INVALID_FOR_EACH
└── (other INVALID_* variants)

GRACEFUL (compilation/validation succeed, runtime may fail)
├── CollectPaginationState: source list empty
├── CollectPaginationState: cursor past limit
└── CollectPaginationState: page_size mismatch

BENIGN (execution continues)
├── CollectPage: body step succeeded
└── CollectFinish: collector_slot populated
```

---

## Error Tags for Digest Coverage

| Error Code | Category | Digest Impact |
|------------|----------|---------------|
| `INVALID_COLLECT` | Semantic validation | Pre-digest: source rejected before digest |
| `DIGEST_COLLISION` | Digest coverage | Post-fix: two different sources produce same digest |
| `INCOMPLETE_FIELD_HASH` | Digest coverage | BUG: Collect fields not in hasher state |
| `ARTIFACT_DIGEST_MISMATCH` | Storage admission | Artifact bytes don't match claimed digest |
