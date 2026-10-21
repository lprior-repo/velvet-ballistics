# Input Schema Kind Resolution Contract

## Bug
Compiler called `kind_from_author_value` on schema declarations. Shorthand strings became `Symbol`; longform mappings became `Object`.

## Fix
Parse declared schema meaning, not author value shape.

## Schema Forms
- **Shorthand:** `text`, `number`, `boolean`, `object`, `any`, `list<any>`, `list<text>`, `list<number>`, `list<boolean>`
- **Longform:** `{is: text|number|boolean|object|list|any, of: ..., fields: ..., default: ...}`

## Kind Mappings
| Schema | InputSlotKind |
|--------|---------------|
| text | Symbol |
| number | I64 |
| boolean | Bool |
| object | Object |
| list | List |
| any | Any (new) |

## Any Variant
- Appended after `Blob` in enum; existing postcard discriminants unchanged
- Accepts every recognized `SlotValue` shape in CLI mapping and runtime seeding
- Typed exact kinds still reject mismatched values
- Unknown future values fail closed
- Schema wildcard only; not a stored `SlotValue` type

## Error Handling
- `kind_from_input_schema` returns `Result`
- No unchecked indexing, casts, `unwrap`, `unsafe`, or `panic`
- Unknown/malformed schema in `compile_source` returns `CompileError::InvalidInputSchema`
- No `Null` fallback

## Invariants
- Numeric source order preserved
- Slots and digest include input schemas
- Actual types: `AuthorValue::Text(String)`, `Mapping(Vec<AuthorEntry<AuthorValue>>)`

## Runtime & CLI
- Runtime and helper kind matching updated for `Any`
- CLI mapper handles `Any` for all primitives
- Kind mismatch errors preserved for typed slots
- IPC receives runtime validation; no new HTTP types

## Behavioral Tests
- Public compiler: shorthand/longform kind resolution, digest binding, source order
- Invalid direct schema returns `CompileError::InvalidInputSchema`
- Runtime: exact kind acceptance vs `Any` wildcard
- CLI: `Any` maps all primitives; kind mismatch preserved for typed slots

## Scope
No full owned handle persistence. No v1 complete claim.