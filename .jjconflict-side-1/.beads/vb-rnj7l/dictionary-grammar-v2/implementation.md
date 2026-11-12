# vb-rnj7l Dictionary Grammar Fix — Implementation

## Files Edited (6)

All edits are in `fuzz/fuzz_targets/`. Each fix is a single mechanical transformation: remove the leading `"` from each line and move the closing `"` from after `=` to after the final escape sequence. No byte values, names, or entry counts changed.

### 1. `decode_record.dict` — 17 entries fixed

Before: `"MAGIC_JOURNAL_EVENT_LE="\x45\x4a\x42\x56` (and 16 similar lines)
After: `MAGIC_JOURNAL_EVENT_LE="\x45\x4a\x42\x56"` (and 16 similar lines)

Entries: MAGIC_JOURNAL_EVENT_LE, MAGIC_BLOB_LE, MAGIC_SNAPSHOT_LE, MAGIC_COMPILED_ARTIFACT_LE, MAGIC_WORKFLOW_SOURCE_LE, MAGIC_INDEX_RECORD_LE, MAGIC_IPC_FRAME_LE, BOUNDARY_ZERO_MAGIC, BOUNDARY_MAX_MAGIC, KIND_RunAccepted_LE, KIND_RunCancelled_LE, KIND_RunFailed_LE, KIND_RunKilled_LE, KIND_Snapshot_LE, KIND_Blob_LE, HEADER_LEN_60_LE, MAX_PAYLOAD_1MB_LE

### 2. `ipc_frame.dict` — 16 entries fixed

Before: `"MAGIC_VBLT_LE="\x54\x4c\x42\x56` (and 15 similar lines)
After: `MAGIC_VBLT_LE="\x54\x4c\x42\x56"` (and 15 similar lines)

Entries: MAGIC_VBLT_LE, VERSION_LE_1, CMD_SubmitRun_LE, CMD_SubmitRunInline_LE, CMD_CancelRun_LE, CMD_InspectRun_LE, CMD_ListEvents_LE, CMD_AnswerAsk_LE, CMD_CompleteAction_LE, CMD_FailAction_LE, CMD_DrainTrace_LE, CMD_Health_LE, CMD_Shutdown_LE, FLAG_PAYLOAD_PRESENT_LE, LEN_24_LE, ZERO_8B

### 3. `journal_decode.dict` — 10 entries fixed

Before: `"MAGIC_JOURNAL_EVENT_LE="\x45\x4a\x42\x56` (and 9 similar lines)
After: `MAGIC_JOURNAL_EVENT_LE="\x45\x4a\x42\x56"` (and 9 similar lines)

Entries: MAGIC_JOURNAL_EVENT_LE, MAGIC_BLOB_LE, MAGIC_SNAPSHOT_LE, MAGIC_COMPILED_ARTIFACT_LE, MAGIC_WORKFLOW_SOURCE_LE, MAGIC_INDEX_RECORD_LE, KIND_RunKilled_LE, KIND_AskTimedOut_LE, KIND_WaitResolved_LE, KIND_ActionAbandoned_LE

### 4. `journal_event.dict` — 3 entries fixed

Before: `"MAGIC_JOURNAL_EVENT_LE="\x45\x4a\x42\x56` (and 2 similar lines)
After: `MAGIC_JOURNAL_EVENT_LE="\x45\x4a\x42\x56"` (and 2 similar lines)

Entries: MAGIC_JOURNAL_EVENT_LE, KIND_RunAccepted_LE, KIND_StepStarted_LE

### 5. `kind_validation.dict` — 10 entries fixed

Before: `"MAGIC_JOURNAL_EVENT_LE="\x45\x4a\x42\x56` (and 9 similar lines)
After: `MAGIC_JOURNAL_EVENT_LE="\x45\x4a\x42\x56"` (and 9 similar lines)

Entries: MAGIC_JOURNAL_EVENT_LE, MAGIC_BLOB_LE, MAGIC_SNAPSHOT_LE, MAGIC_COMPILED_ARTIFACT_LE, MAGIC_WORKFLOW_SOURCE_LE, MAGIC_INDEX_RECORD_LE, BOUNDARY_KIND_28_LE, BOUNDARY_KIND_30_LE, BOUNDARY_KIND_40_LE, BOUNDARY_KIND_UNKNOWN_999_LE

### 6. `vb_storage_codec.dict` — 9 entries fixed

Before: `"MAGIC_JOURNAL_EVENT_LE="\x45\x4a\x42\x56` (and 8 similar lines)
After: `MAGIC_JOURNAL_EVENT_LE="\x45\x4a\x42\x56"` (and 8 similar lines)

Entries: MAGIC_JOURNAL_EVENT_LE, MAGIC_BLOB_LE, MAGIC_SNAPSHOT_LE, MAGIC_COMPILED_ARTIFACT_LE, MAGIC_WORKFLOW_SOURCE_LE, MAGIC_INDEX_RECORD_LE, BOUNDARY_ZERO_MAGIC, BOUNDARY_MAX_MAGIC, DEADBEEF_LE

## Total

65 dictionary entries fixed across 6 files. All edits are identical in nature: the malformed `"NAME="\xHH...` syntax was corrected to the libFuzzer-standard `NAME="\xHH..."` syntax.

## Files Untouched (2)

- `expression.dict` — already correct syntax (quoted string literals, no names)
- `vb_f04l_yaml_compiler_compile.dict` — already correct syntax (quoted string literals, no names)

## Verification Notes

The fix is purely mechanical quote-boundary relocation. No byte values were changed, no entries added or removed. The root will verify:
1. Each fixed dictionary parses without `ParseDictionaryFile` errors when loaded by an actual compiled libFuzzer binary
2. Entry counts are preserved (17, 16, 10, 3, 10, 9)
3. Each token's name and value bytes are identical to the original intent