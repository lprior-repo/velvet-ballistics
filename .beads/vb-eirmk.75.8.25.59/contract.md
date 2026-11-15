# Contract: bind canonical repeat attempt references

## Scope

Resolve exactly `$attempt.number` in a canonical `repeat` body to the current loop's one-based numeric attempt. Reuse the existing packed attempt slot and runtime node/bytecode representation. Do not expose packed state. Nested repeats shadow the binding; non-repeat nested bodies inherit it; references outside the repeat body and unsupported accessors remain errors. Add no public API, persisted format, IR variant, dependency, or gate change.

## Domain facts

- `RepeatStart` initializes the existing attempt slot with packed I64 `(max_attempts << 32) | 0`; `RepeatCheck` increments its low 32-bit attempt count. The public numeric attempt is therefore `low_count + 1`, not the packed slot and not the zero-based low count.
- The compiler's canonical scope is a borrowed parent chain. Any binding must live only in the repeat body's lexical scope, with a nested repeat's local binding taking precedence.
- Expression and direct input lowering resolve references through `StepReferenceResolver`; paths can be accepted as accessor suffixes, so `$attempt.number.bad` must be rejected explicitly. Bare `$attempt`, `$attempt.bad`, and arbitrary `attempt.*` tails are not aliases.
- Existing expression bytecode/IR and runtime evaluation must carry the visible numeric slot while the engine retains and updates the packed slot.

## Observable behavior

- Canonical compilation of a repeat-body `set`, `do`, or `wait` reference to `$attempt.number` succeeds with I64 type.
- Runtime execution reports one-based values matching the actual repeat engine: first iteration `1`, second `2`, continuing through the configured maximum; a nested repeat sees its own count while a nested non-repeat construct inherits its enclosing repeat count. Outside references do not resolve.
- `$attempt`, `$attempt.bad`, `$attempt.number.bad`, unsupported accessors, and invalid type contexts are rejected. Existing unrelated references and repeat layout/runtime behavior remain unchanged.

## Boundaries and hazards

- Preserve packed slot encoding, RepeatStart/RepeatCheck behavior, public compiler signatures, IR variants, byte encoding, and all current limits.
- Keep generated node/layout width accounting coherent in every canonical repeat width owner and verify nested node/slot layout assertions.
- Tests must distinguish packed state from visible number and zero-based from one-based values; compiler success alone is insufficient.
