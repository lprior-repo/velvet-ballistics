# Pagination Contract (MASTER2236)

Recorded after raw draft, before acceptance.

## Start

Validates: positive page size ≤ limit; source count ≤ limit.

- Empty source → write an empty page to the collector, clear state, jump to done.
- Nonempty first page → retain state, write the first page to the collector, jump to body (even if exhausted).

## Next

Validates: state exists, correct order, cursor valid, within time window.

- Next page available → write the page to the collector, advance cursor, jump to body.
- Exhausted → write an empty page to the collector, remove state, jump to done.

## Overflow

Checked product before builder mutation. On overflow:

```
CompileError::StepFieldShape {
    step: index,
    field: "collect",
    expected: "pages * items must fit u32"
}
```

## Aliasing

`output` None primitive intentionally aliases source. Preserve source list/taint only when distinct collector slot.

## Defaults

pages/items default to 1, unchanged.

Source limit is checked before pagination: a source above total limit is rejected, not truncated. The compiler emits limit=checked(pages*items) and page_size=items. No proof or performance claim.
