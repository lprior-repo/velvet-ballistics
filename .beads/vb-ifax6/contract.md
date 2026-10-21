# vb-ifax6: Rust Lint Cleanup — Compiler Borrow + CLI Dead Declarations

**Status:** Contract stage (source/test to follow)
**Packages:** `vb_compile`, `velvet-ballistics`
**Files:** 3 (1 compiler, 2 CLI)

## Scope

Fix canonical source lints blocking advancement. No behavioral change, no new validation, no wire format change.

## Changes

### 1. Compiler: needless borrow (vb_compile)

**File:** `crates/vb_compile/src/mod_compile_lowering/part_01.rs:46`

```diff
- allocate_input_slots(&source.inputs(), &mut builder)
+ allocate_input_slots(source.inputs(), &mut builder)
```

`source.inputs()` already returns a borrowed slice; the explicit `&` is redundant.

### 2. CLI: prune dead private declarations (velvet_ballistics)

**File:** `crates/velvet_ballistics/src/run_compiled.rs`

Remove never-constructed enum variant and its display arm:

```diff
 pub(crate) enum InputMappingError {
     DecodeFailed,
     SlotCountExceeded,
-    SlotIndexOutOfRange,
     KindMismatch { slot, expected_kind },
 }
```

```diff
- Self::SlotIndexOutOfRange => formatter.write_str(INPUT_MAPPING_SLOT_INDEX_OUT_OF_RANGE_MESSAGE),
```

**File:** `crates/velvet_ballistics/src/app_impl.rs`

Remove unused constant and its re-export:

```diff
- pub(crate) const INPUT_MAPPING_SLOT_INDEX_OUT_OF_RANGE_MESSAGE: &str =
-     "INPUT_MAPPING_FAILED: input slot index out of range";
```

```diff
- INPUT_MAPPING_SLOT_INDEX_OUT_OF_RANGE_MESSAGE,
```

## Preserved (explicitly not changed)

- Active `KindMismatch` Display literal with slot and expected kind is unchanged. The generic `INPUT_MAPPING_KIND_MISMATCH_MESSAGE` constant and import are unused and are removed.
- `InputMappingError::KindMismatch` variant with typed `slot`/`expected_kind` fields
- All active CLI diagnostic strings and exit codes
- `CompiledWorkflow::try_from_parts` out-of-range IR metadata rejection
- Positional `Box<[SlotValue]>` wire format — no indexed wire invented

## Rationale

- `SlotIndexOutOfRange` is never constructed: the mapper zips positional values against validated slots; `try_from_parts` already rejects malformed IR metadata before mapping.
- Removing a private, unreachable variant carries no public API compatibility obligation.
- The variant currently exists but is dead code; this is cleanup, not a claim it never existed.

## Verification

- `moon run :lint-src` — no remaining needless-borrow or unused-declaration lints
- Compiler unit tests pass
- mapper16 + public CLI 10 subcases pass
- No proof planning required; changes are mechanical lint fixes
## Source-grounded correction

Both local draft versions mislabeled the unused generic kind-mismatch constant as active, contrary to the actual Display literal; coordinator corrected that claim above before implementation. No source depended on the mistaken assertion. Frozen current source and rg callsite search show only definition/import for the generic constant. Existing executable detailed KindMismatch text remains unchanged.
