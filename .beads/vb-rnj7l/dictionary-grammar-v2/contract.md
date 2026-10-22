# vb-rnj7l Dictionary Grammar Fix — Contract

## Problem

Six libFuzzer dictionary files use malformed syntax that causes `ParseDictionaryFile: error in line 1` when the compiled fuzzer binary loads the dictionary. The malformed pattern is:

```
"NAME="\xHH\xHH...
```

This is interpreted as a quoted string literal `"NAME="` followed by unquoted escape sequences, which the libFuzzer dictionary parser rejects.

The correct libFuzzer dictionary syntax for named tokens is:

```
NAME="\xHH\xHH..."
```

Where the name is unquoted, the `=` separates name from value, and the value is a double-quoted string (possibly containing escape sequences).

## Files To Fix (6)

All in `fuzz/fuzz_targets/`:

1. `decode_record.dict` — 17 entries
2. `ipc_frame.dict` — 17 entries
3. `journal_decode.dict` — 10 entries
4. `journal_event.dict` — 3 entries
5. `kind_validation.dict` — 10 entries
6. `vb_storage_codec.dict` — 9 entries

## Files To Leave Untouched (2)

These already use correct syntax (quoted string literals without names or `=`):

- `expression.dict` — 29 entries (all `NAME="value"` format with plain ASCII values)
- `vb_f04l_yaml_compiler_compile.dict` — 37 entries (all `NAME="value"` format with plain ASCII values)

## Fix Mechanism

For each of the six invalid files, apply this mechanical transformation to every line:

**From:** `"NAME="\xHH\xHH...`
**To:** `NAME="\xHH\xHH..."`

This means:
- Remove the leading `"` character
- Move the closing `"` from after the `=` to after the final escape sequence

## Invariants

- The hex escape byte values (`\xHH` sequences) are preserved exactly: same case, same order, same values.
- The token names are preserved exactly.
- No tokens are added, removed, or renamed.
- No new escape sequences are invented.
- The equals sign is preserved but repositioned from inside the quotes to between the unquoted name and the quoted value.
- Line count is unchanged for each file.
- Entry count is unchanged for each file.

## Verification

Root will verify using an actual compiled libFuzzer binary and an independent parser that each fixed dictionary:
1. Parses without `ParseDictionaryFile` errors
2. Yields the same number of tokens as before the fix
3. Each token's value bytes are identical to the original (name and value preserved)

No changes to fuzz target binaries, flags, budgets, or target wiring. No `.beads/dict-fix` alias or flag. No changes to any files other than the six `.dict` files and this contract/implementation documentation.