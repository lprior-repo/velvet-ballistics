# Contract Specification — vb-xi2f.33: Digest Covers Ask Semantics

## Context

- **Bead**: `vb-xi2f.33` / P1: digest covers ask semantics
- **Phase**: 3 (rust-contract)
- **Source**: codebase-map.md, delivery-scope.jsonl from State 2 exploration
- **Scope**: `canonical_digest()` and `digest_step_primitive()` in `vb_compile` crate — ensure Ask primitive semantic fields (prompt, timeout) are hashed
- **Bug**: `digest_step_primitive` catch-all arm hashes only `b"ask"` for Ask; prompt and timeout are invisible to the digest
- **Duplicate sites**: `crates/vb_compile/src/mod_compile_lowering/part_05.rs` (active canonical path) and `crates/vb_compile/src/compile/mod.rs` (legacy path)

## Domain Terms

| Term | Definition |
|------|-----------|
| Canonical digest | Semantic hash of `WorkflowSource` produced by `canonical_digest()`, embedded in `WorkflowParts.digest`. Must capture all semantically meaningful fields. |
| Ask primitive | `StepPrimitive::Ask { prompt: String, timeout: Option<String> }` — human-input request step in YAML AST. |
| Digest sensitivity | Property: changing any Ask semantic field (prompt or timeout) changes `canonical_digest` output. |
| Digest determinism | Property: same `WorkflowSource` always produces same `WorkflowDigest`. |
| Digest collision | Two semantically different sources producing identical canonical digests — a violation of semantic-integrity. |

## Preconditions

- **PRE-001**: `canonical_digest` receives a fully parsed and validated `WorkflowSource`. All fields have been validated by the YAML parser.
- **PRE-002**: `digest_step_primitive` receives a `&mut blake3::Hasher` in valid state and a `&StepPrimitive` with all fields present per the type definition.
- **PRE-003**: `blake3::Hasher::update()` and `blake3::Hasher::finalize()` are deterministic — assumed property of the blake3 crate.
- **PRE-004**: The Ask primitive's `prompt` is a present `String` (not optional). `timeout` is `Option<String>` — `None` means no timeout, `Some(s)` means timeout expression `s`.

## Postconditions

- **POST-001**: For two `WorkflowSource` values differing only in an `Ask { prompt }` field, `canonical_digest` produces different `WorkflowDigest` values.
- **POST-002**: For two `WorkflowSource` values differing only in an `Ask { timeout }` field, `canonical_digest` produces different `WorkflowDigest` values.
- **POST-003**: `canonical_digest` is deterministic: calling it twice on the same source produces identical `WorkflowDigest` values.
- **POST-004**: An Ask with `prompt = ""` produces a well-defined, non-degenerate digest distinct from any non-empty prompt.
- **POST-005**: `timeout: None` and `timeout: Some("")` produce distinct digest contributions.
- **POST-006**: The active canonical compilation path (`part_05.rs`) and legacy path (`compile/mod.rs`) produce identical digests for identical sources after the fix.
- **POST-007**: Existing digest behavior for `Set` and `Finish` primitives is not changed by the fix.

## Invariants

### INV-ASK-001 (Semantic Sensitivity — Prompt)
```
For all WorkflowSource values A, B:
  A == B except one Ask step has prompt p_a and the corresponding step in B has prompt p_b
  where p_a != p_b
⇒ canonical_digest(A) != canonical_digest(B)
```

### INV-ASK-002 (Semantic Sensitivity — Timeout)
```
For all WorkflowSource values A, B:
  A == B except one Ask step has timeout t_a and the corresponding step in B has timeout t_b
  where t_a != t_b
⇒ canonical_digest(A) != canonical_digest(B)
```

### INV-ASK-003 (Determinism)
```
For all WorkflowSource S:
  canonical_digest(S) == canonical_digest(S)
  (always true; verified by test: compile twice, compare digests)
```

### INV-ASK-004 (Empty Prompt)
```
Let S1 = source with Ask { prompt: "", timeout: None }
Let S2 = source with Ask { prompt: "hello", timeout: None }
⇒ canonical_digest(S1) != canonical_digest(S2)
```

### INV-ASK-005 (None vs Some("") Timeout)
```
Let S1 = source with Ask { prompt: "p", timeout: None }
Let S2 = source with Ask { prompt: "p", timeout: Some("") }
⇒ canonical_digest(S1) != canonical_digest(S2)
```

### INV-ASK-006 (Duplicate Parity)
```
For all WorkflowSource S:
  canonical_digest_in_part05(S) == canonical_digest_in_compile_mod(S)
```

### INV-ASK-007 (No Set/Finish Regression)
```
For all WorkflowSource S containing only Set and Finish steps:
  canonical_digest_before_fix(S) == canonical_digest_after_fix(S)
```
(Actually: no change, since the Ask arm is only added, not changing Set/Finish arms.)

## Contract Signatures

### `canonical_digest(source: &WorkflowSource) -> WorkflowDigest`
- **Pure function**: No side effects beyond hasher computation.
- **Deterministic**: Same input → same output.
- **Semantically complete**: All fields of all primitives that carry semantic meaning are hashed.
- **Must hash for Ask**: `b"ask"` tag, `prompt.as_bytes()`, and `timeout` (sentinel or value).

### `digest_step_primitive(hasher: &mut blake3::Hasher, primitive: &StepPrimitive)`
- **Must have explicit Ask arm**: Not relying on catch-all.
- **Hashing order for Ask**: tag → prompt → timeout sentinel/value.
- **Timeout sentinel**: `b"no_timeout"` for `None`; `b"timeout"` + value bytes for `Some`.
- **No panic, unwrap, expect**: All operations on `String` and `Option<String>` are infallible.

### `compute_compiled_digest(source: &[u8]) -> WorkflowDigest`
- **Not in scope for this bead**: This function is already correct (raw blake3 over source bytes).
- **Documented for clarity**: NOT the semantic digest; used for artifact integrity, not semantic identity.

## Requirements Traceability

| R-ID | Requirement | Source | Contract Clause |
|------|------------|--------|-----------------|
| R1 | `digest_step_primitive` must hash Ask prompt | codebase-map.md | INV-ASK-001 |
| R2 | `digest_step_primitive` must hash Ask timeout | codebase-map.md | INV-ASK-002 |
| R3 | Digest must be deterministic | codebase-map.md | INV-ASK-003 |
| R4 | Empty prompt must produce distinct digest | edge-case analysis | INV-ASK-004 |
| R5 | None vs Some("") timeout distinction | edge-case analysis | INV-ASK-005 |
| R6 | Fix must be applied to both duplicate sites | codebase-map.md | INV-ASK-006 |
| R7 | No regression for Set/Finish | defensive analysis | INV-ASK-007 |

## Assumptions

- **A1**: `blake3` crate is deterministic and cryptographically sound.
- **A2**: YAML parser guarantees `prompt` is always a `String` (not absent) and `timeout` is always an `Option<String>` (not malformed).
- **A3**: The active canonical compilation path (`part_05.rs`) is the primary path; the legacy path (`compile/mod.rs`) may be deprecated but must be kept in parity for now.
- **A4**: `String::as_bytes()` for the same `String` value always produces the same bytes (deterministic).
- **A5**: `b"no_timeout"` does not collide with any valid timeout expression as a hash prefix. The `b"timeout"` prefix is added before the expression value for `Some` cases, disambiguating.

## Non-Goals

- Fixing digest coverage for non-Ask primitives (`Do`, `Wait`, `Choose`, etc.) — separate bead.
- Removing or deprecating the legacy compilation path — separate bead.
- Changing `WorkflowDigest` type or adding validation to its constructor — not in scope.
- Modifying `compute_compiled_digest` (already correct) — not in scope.
- Adding runtime digests or changing admission/idempotency logic — not in scope.
- TLA+, Verus, or Kani proofs — those belong to later states (proof-planner, proof-writer).

## Open Contract Questions

1. Should the canonical digest also hash the step's `id` (already done in `canonical_digest` loop) alongside the primitive fields? (Yes, step ID is already hashed in the enclosing loop.)
2. Should the field-order for Ask be formally specified (tag → prompt → timeout) or is it sufficient that it is deterministic? (Formally specified — the order is part of the contract for determinism.)
3. Should empty timeout string `Some("")` be rejected at parse time rather than disambiguated at hash time? (This is a parser-level decision, outside the digest contract scope.)
