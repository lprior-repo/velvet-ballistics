# `contains(List, T)` parity proposal V5

Status: bounded contract successor for independent CODE and TEST review; this document is not implementation, native evidence, acceptance, or feature closure.

## Authority and provenance

Authoritative master: `velvet-ballistics-MASTER.md`, SHA-256 `fc0282bb901030dd1b195ebbce964d560fbf409c9f5e49c8c6e5203da3a46536`; §46 line 2448 specifies `contains(List, T) -> Bool`, and §46 lines 2426–2433 separately specifies derived `SlotValue::PartialEq` for `==`/`!=` without defining a Contains-specific comparison rule.

Inspected source baseline: `6d7789cd3fc8997e228b568da7270738ba8e2080`; this proposal is scoped to that baseline and must be refreshed if implementation starts from different source.

V5 preserves V4 at `/home/lewis/src/isoloated/velvet-ballistics-has-hot-fixtures-luna/.beads/vb-rihha.1/contains-parity-contract-proposal-v4.md`, SHA-256 `c65bd161f0c8bbf468ae92d5c27ac38404b693df3a3cea372c133ccd7d4276ad`, and addresses independent TEST rejection `/home/lewis/src/isoloated/velvet-ballistics-four-repairs-test-sol/target/evidence/contains-parity-contract-test-review-v1.json`.

Read-only live ownership check: `vb-rihha.1.3` is in progress for Contains parity; sibling `vb-rihha.1.1` is in progress for Length/Count/Empty and explicitly excludes Contains; no Beads state was changed.

The parent-selected interpretation is direct tagged `SlotValue::PartialEq`, matching existing raw-item comparison in `unique`; the master does not explicitly prescribe Contains equality, so this remains the selected interpretation rather than a master quotation.

The parent-selected resolution policy validates/resolves only the receiver ListId; it does not validate or resolve stored item or needle handles, recursively compare payloads, or coerce values; this is consistent with the existing `unique` comparison path and does not change Has semantics.

## Required behavior

Canonical Contains has exactly two operands: receiver `List`, unrestricted needle `T`/`Any`, and Boolean result; membership is true iff one stored raw `SlotValue` is directly equal to the needle, and an empty list returns false.

Variant tags and payloads both participate in equality: same-tag equal scalar payloads match; unequal payloads do not; `I64(7)` differs from `F64(7.0)`; Symbol, List, Object, and Blob handles compare by their typed IDs without resolving referenced payloads; equal symbol text at different SymbolIds is unequal.

No substring overload, stringification, numeric coercion, deep equality, needle-type restriction, or eager inner/needle handle lookup is allowed; preserve Has and every unrelated helper behavior.

For a valid receiver list, raw invalid IDs are ordinary compared values: a list containing invalid handle H contains the identical raw H needle, does not contain a distinct invalid handle H2, and an empty valid list does not contain invalid H; an invalid stored handle compared with a different scalar is false.

## Canonical and cold static checking

Canonical typechecking must newly validate Contains argument position zero as `List | Unknown`; known non-List receiver categories, including Null, Text, Bool, I64, F64, and Object, return the existing `ExprError::TypeMismatch { expected: "list", found: type_name(receiver) }`; the needle position remains unconstrained, and canonical arity remains exactly two with the existing arity error.

Cold `parse_ast` checking must newly validate a present Contains receiver at position zero as List-compatible under that path’s existing ValueType/Unknown rules; a known wrong present receiver returns the existing `choose.condition` type mismatch, while valid cold prefix arities zero, one, and three retain current behavior because cold checking does not own arity validation.

Do not describe either receiver guard as existing behavior: at the inspected baseline canonical `check_helper_positions` maps Contains to `Ok(())`, and cold `helper_expected_type` has no Contains case.

Do not introduce cold-path arity checking or assume canonical `ExprType::Any`; the current static type is `Unknown`, while the master’s `Any` denotes the unconstrained needle domain.

## Runtime boundaries and errors

All three routes must independently produce the same literal Boolean membership results for equivalent inputs: store-aware `vb_expr` via `eval_helper_with_store(ExprHelper::Contains, args, &mut ValueStore)`, hot core via `engine::eval_expr_with_store` on a registered compiled expression, and public replay via `replay::ops::eval_replay_op` with initialized operands and a valid nonzero PC.

With both bytecode operands present, receiver type/ID validation precedes membership comparison; no needle-type or nested-handle validation is performed; retain each route’s existing stack-underflow and allocation behavior.

At the store-aware `vb_expr` public boundary, wrong receiver `receiver` must equal `Err(ExprError::TypeMismatch { expected: "list".into(), found: receiver.type_name().into() })`, and invalid receiver `id` must equal `Err(ExprError::InvalidReference { reference: format!("list:{id:?}") })`, matching the existing Count/Unique mapping; assert the complete typed `Result` and do not infer this boundary from hot-core or replay errors.

At the hot-core public boundary, assert wrong receiver `receiver` as `EngineError::TypeMismatch { expected: "list", found: receiver.type_name() }` and invalid receiver `id` as `EngineError::ListOutOfBounds { list: id }`; assert the complete typed `Result`.

At public replay, assert only `ReplayError::ExpressionEvalFailed { step: StepIdx(1) }` for a Contains evaluation failure at PC one; replay deliberately exposes no typed cause field, so replay tests must not claim or assert an underlying TypeMismatch/ListOutOfBounds cause or distinguish those causes by wrapper payload.

If invalid receiver and invalid needle are supplied together, the store-aware and hot-core boundaries must expose receiver failure before any needle lookup; replay proves only its public step wrapper, not internal cause or precedence.

Keep direct argument-list arity errors distinct from bytecode operand underflow; do not invent a runtime helper-arity error for bytecode.

## Minimum independent acceptance oracles

For each of the three runtime routes, assert literal expected Boolean results independently (never by comparing one evaluator’s result with another): `[I64(7), I64(11)]` contains `I64(7)` => true; the same list contains `I64(13)` => false; empty list contains `I64(7)` => false; `[I64(11)]` contains `I64(7)` => false.

For each route, assert `[F64(7.0)]` contains `F64(7.0)` => true and `[I64(7)]` contains `F64(7.0)` => false, constructing finite floats with the existing checked `FiniteF64` constructor.

For each route, assert same SymbolId membership => true and two separately inserted symbols with equal text but distinct IDs => false; also assert distinct Object IDs => false without resolving object payloads where that route’s fixture surface supports those values.

For each route, assert a valid list containing raw invalid handle H contains the identical raw H needle => true, does not contain distinct invalid H2 => false, and an empty valid list with invalid H needle => false; assert a raw invalid stored item against a nonmatching scalar => false, without looking up those inner or needle handles.

Include Null needle equality and at least one unrestricted nonmatching scalar/tag case with literal results on each route.

For canonical static checking, assert exact accepted List and Unknown receivers, exact TypeMismatch payloads for every listed known wrong receiver category, unconstrained needle categories, and exact arity-two outcomes including wrong arities.

For cold checking, assert valid receiver behavior at prefix arities zero, one, and three, invalid present receiver at arities one and three with the exact existing `choose.condition` TypeMismatch, and the path’s current Unknown/wildcard acceptance; do not assert cold arity errors.

For runtime errors, assert exact store-aware and hot public error variants/payloads for wrong receiver and invalid receiver ListId; for replay assert the exact `ExpressionEvalFailed { step: StepIdx(1) }` wrapper only, plus existing underflow behavior at the relevant route boundary.

All expected values and errors are independent literals constructed from fixture inputs; no expected result may be derived by calling another evaluator, equality helper, or production oracle.

## Review and execution boundary

Implementation may change only the approved Contains behavior and required receiver guards, preserving Has and unrelated semantics; no public API, persisted format, dependency, schema, gate, or parser-wide change is authorized.

The delivery sequence remains contract review, implementation, native behavior tests, independent CODE review, and separate independent TEST review; this proposal ran no implementation, build, or tests and makes no acceptance or closure claim.
