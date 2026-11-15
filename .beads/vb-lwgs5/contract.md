# Contract: standalone secret-reference diagnostic parity

## Intent

An undeclared reference rooted at `$secrets` is classified by the existing typed `ValidationError::SecretNotDeclared`, rendered with the already-registered `CODE_SECRET_NOT_DECLARED` / `0x0203` / `SECRET_NOT_DECLARED`, and remains represented as the existing compiler `CompileError::UnknownReferenceName { kind: "secrets", ... }` on canonical compilation.

## Scope

Change only shared standalone reference validation and the existing canonical compiler error adapter as required to preserve their current separate diagnostic contracts. No new error variants, public methods/signatures, IDs, schema/version, dependencies, or parser/source acceptance. Do not expand legacy `parse_ast` behavior.

## Invariants

- Declared secret references continue to validate successfully.
- Missing `$secrets.<name>` returns exactly `SecretNotDeclared { secret: <first path segment> }` from standalone validation.
- Canonical compiler errors retain existing `UnknownReferenceName` kind, original full reference, and missing secret name.
- Missing `$input.*` and `$var(s).*` retain exact `UnknownReference { reference }` behavior.
- Bare `$secrets`, `$secrets.`, non-reference text, runtime references, and step-reference behavior remain unchanged.
- Existing tail/path handling is unchanged; this work does not add accessor validation.

## Verification obligations

Add an exact public-validator diagnostic regression and retain controls for present/missing secrets, missing input/var, malformed/empty roots, and non-reference text. Keep the canonical compiler secret regression and assert its exact name/reference. Record the focused RED before implementation, then run focused `vb_validate` reference/diagnostic and `vb_compile` reference tests, plus repository-prescribed formatting/source gates. Preserve all existing tests and report any unrelated gate failure without weakening it.
