# Rust Domain/Type Contract — vb-zpaad (CV-106)

**Bead:** vb-zpaad (bug-hunt CV-106 follow-up; sub-bead of `vb-8muyy`).
**Finding:** `bug-hunt-2026-06-21/findings/core-value/CV-106-span-new-no-validation.md`.
**Source:** `crates/vb_core/src/span.rs:20-24` (`Span::new`),
`crates/vb_core/src/lib.rs:119` (re-export of `Span`).
**Pipeline caveat:** the original bead plan assumes an agent runtime
with a `task` / subagent tool. This environment does not expose one,
so every artifact in this directory (including this contract) is
**self-authored by the orchestrator**, not by a subagent. Each
artifact carries an explicit "self-authored" marker.

## Ubiquitous Language

- **Span** — a half-open byte-offset range `[start, end)` into a source
  document, where `start` is inclusive and `end` is exclusive.
- **Empty span** — a `Span` whose `start == end`; carries zero bytes.
- **Inverted span** — a `Span` whose `start > end`; carries a negative
  byte count and is the bug-hunt CV-106 finding.
- **Unchecked constructor** — `Span::new(start, end)`, an existing
  `const fn` that accepts any `u32` pair and performs no validation.
- **Checked constructor** — `Span::try_new(start, end)`, a new
  `const fn` that returns `Result<Span, SpanError>` and rejects
  `start > end`.

## Value Object: `Span` (unchanged public shape)

```rust
pub struct Span {
    pub start: u32,  // inclusive
    pub end: u32,    // exclusive
}
```

Fields stay `pub` for source-compatibility. Direct construction
`Span { start, end }` is intentionally not validated; the checked
constructor `try_new` is the safe path.

## New Constructor: `Span::try_new`

```rust
impl Span {
    #[must_use]
    pub const fn try_new(start: u32, end: u32) -> Result<Self, SpanError> {
        if start > end {
            return Err(SpanError::StartGreaterThanEnd { start, end });
        }
        Ok(Self { start, end })
    }
}
```

`try_new` is `const` so it is usable in `const` contexts, mirroring
`Span::new`. It performs exactly one comparison and a single struct
construction; no allocations, no panics, no unwraps.

### Acceptance table

| Pre-state              | Call                       | Post-state                                        |
|------------------------|----------------------------|---------------------------------------------------|
| `start <= end`         | `Span::try_new(start,end)` | `Ok(Span { start, end })`                         |
| `start == end`         | `Span::try_new(start,end)` | `Ok(Span { start, end })`, and `is_empty()` true  |
| `start > end`          | `Span::try_new(start,end)` | `Err(SpanError::StartGreaterThanEnd { start, end })` |
| any                    | `Span::new(start,end)`     | `Span { start, end }` (unchecked, **unchanged**)  |

## Error Type: `SpanError`

A new `#[non_exhaustive]` enum lives in `crates/vb_core/src/span.rs`
and is re-exported from `vb_core::span::SpanError` and via
`vb_core::SpanError` (added to `lib.rs`'s `pub use span::...` line).

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SpanError {
    #[error("span start {start} is greater than end {end}")]
    StartGreaterThanEnd { start: u32, end: u32 },
}
```

`#[non_exhaustive]` matches the surrounding `CoreError` /
`EngineError` discipline so future variants can be added without a
breaking change.

## Error Taxonomy Integration

`SpanError` is intentionally a **focused, module-local** error type.
It plugs into the core taxonomy through a `From<SpanError> for
CoreError` conversion that maps to a new `CoreError::InvalidSpan`
variant.

```rust
// crates/vb_core/src/errors.rs (additions)

#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CoreError {
    // ... existing variants ...
    /// A Span was constructed with start > end via try_new.
    #[error("invalid span: start {start} is greater than end {end}")]
    InvalidSpan {
        start: u32,
        end: u32,
    },
}

impl CoreError {
    pub const INVALID_SPAN_CODE: DiagnosticCode = DiagnosticCode::new(0x1315);
    // ... existing codes ...

    pub const fn diagnostic_code(&self) -> DiagnosticCode {
        match self {
            // ... existing arms ...
            Self::InvalidSpan { .. } => Self::INVALID_SPAN_CODE,
        }
    }
}

impl From<SpanError> for CoreError {
    fn from(err: SpanError) -> Self {
        match err {
            SpanError::StartGreaterThanEnd { start, end } => {
                Self::InvalidSpan { start, end }
            }
        }
    }
}
```

The diagnostic code `0x1315` is assigned to the `0x13xx` "handle /
index out of bounds" bucket already in use by `SYMBOL_OUT_OF_BOUNDS`
(`0x1311`), `LIST_OUT_OF_BOUNDS` (`0x1312`), `OBJECT_OUT_OF_BOUNDS`
(`0x1313`), and `BLOB_OUT_OF_BOUNDS` (`0x1314`). `InvalidSpan` does
not cross a runtime boundary, so no `runtime_code` arm is added.

## Workflow Contract

```text
try_new(start, end) -> Result<Span, SpanError>
```

| Pre-state    | Action                  | Post-state                                                       |
|--------------|-------------------------|------------------------------------------------------------------|
| `s <= e`     | `try_new(s,e)`          | `Ok(Span{s,e})`                                                  |
| `s == e`     | `try_new(s,e)`          | `Ok(Span{s,e})`; `result.is_empty()` is `true`                   |
| `s > e`      | `try_new(s,e)`          | `Err(SpanError::StartGreaterThanEnd { start: s, end: e })`       |

## Invariants

1. `Span::new(s, e)` keeps its existing post-state `Span { start: s, end: e }`
   for any `u32` pair, including inverted spans. The constructor is
   explicitly documented as unchecked.
2. `Span::try_new(s, e)` is total over `u32 × u32`: it returns
   `Ok` iff `s <= e`, otherwise `Err(SpanError::StartGreaterThanEnd)`.
3. The struct field order, sizes, alignment, `#[derive(...)]` list,
   and `pub` visibility are unchanged.
4. `Span::ZERO` (`Span { start: 0, end: 0 }`) is still constructible
   via `Span::default()` and `Span::try_new(0, 0)`.

## Sentinel-Value Discipline

- `Span::try_new(0, 0)` returns `Ok` and yields `Span::ZERO`. The
  empty span is a valid, first-class value, not an error.
- The error variant carries the offending `start` and `end` verbatim
  for diagnostics; no clamping, no normalisation.

## Public API Impact

- **Additive only.** `Span::new` signature is unchanged. `Span` field
  visibility is unchanged. The only new public surface is:
  - `Span::try_new(start, end) -> Result<Span, SpanError>`
    (added to `impl Span`).
  - `SpanError` enum and its `StartGreaterThanEnd` variant.
  - `vb_core::SpanError` re-export.
  - `CoreError::InvalidSpan { start, end }` variant.
  - `From<SpanError> for CoreError` impl.
  - `CoreError::INVALID_SPAN_CODE` constant.
- No call site is forced to migrate. Existing
  `Span::new(start, end)` uses with `start <= end` keep working
  unchanged. Call sites that previously constructed `Span { start, end }`
  via struct literal also keep working.

## Hazards

- **H-1: Direct struct literal bypass.** `Span { start: 5, end: 3 }`
  is still allowed. Mitigation: document the unchecked paths and
  add a doctest to `Span::new` that points callers at `try_new`.
- **H-2: `From<SpanError> for CoreError` may conflict with future
  blanket impls.** Mitigation: `#[non_exhaustive]` on `SpanError` so
  new variants can be added without breaking the impl.
- **H-3: Diagnostic code 0x1315 may already be assigned.** Mitigation:
  verified via `rg "0x1315" crates/` returning no matches in
  `vb_core/src/`. Verified at contract-write time.
- **H-4: `Span::try_new` must be `const`.** Holzman rule (callable in
  const contexts) and parity with `Span::new`. Mitigation: keep the
  function body trivial (`if/return` and struct construction) so the
  compiler accepts it under the `try_blocks`-allowlisted nightly.

## Self-Authoring Marker

This contract was authored by the orchestrator directly, not by a
`rust-contract` subagent, because the runtime does not expose a
subagent tool. The content is the type contract the
`rust-contract` skill would have produced given the same inputs.
