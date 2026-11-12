# State 8 Release-Block Repair — vb-qi37.16.4

**bead_id:** vb-qi37.16.4
**phase:** state-8 release-block repair
**date:** 2026-05-11
**release_critical:** true
**STATUS:** REPAIRED

---

## Minimal Fix Applied

### `vb_ipc/src/server/handlers.rs:243` — `cannot find value encoded`

**Root cause:** The `handle_answer_ask` function referenced an undefined variable `encoded` on line 243. The variable was intended to hold the byte length of the encoded answer payload but was never introduced into scope.

**Fix:** Replaced `encoded.len() as u32` with `answer.len() as u32`.

- `answer` is the `Vec<u8>` field from the decoded `IpcPayload::AnswerAsk` struct (line 217–218)
- `answer.len()` is the correct encoded length of the answer payload in bytes
- This preserves the `AskAnswer::encoded_len` contract ("Encoded length of the answer payload in bytes")
- The secret-redaction contract is preserved: only the byte length is recorded, not the content

**Diff:**
```diff
-        encoded_len: encoded.len() as u32,
+        encoded_len: answer.len() as u32,
```

---

## Command Evidence

### 1. `rtk cargo check -p vb_ipc --all-targets`

```
cargo build: 0 errors, 1 warnings (13 crates)
```

**STATUS: PASS**

### 2. `rtk cargo test -p vb_runtime --lib -- "shard::lifecycle::tests::red_"`

```
cargo test: 12 passed, 1337 filtered out (1 suite, 0.00s)
```

**STATUS: PASS**

### 3. `rtk cargo test -p vb_runtime --lib`

```
cargo test: 1349 passed (1 suite, 0.30s)
```

**STATUS: PASS**

### 4. `rtk cargo fmt -- --check`

```
(no output — clean)
```

**STATUS: PASS**

---

## Gate Summary

| Gate | Result |
|------|--------|
| `cargo check -p vb_ipc --all-targets` | PASS (0 errors) |
| `cargo test -p vb_runtime --lib -- "shard::lifecycle::tests::red_"` | PASS (12 passed) |
| `cargo test -p vb_runtime --lib` | PASS (1349 passed) |
| `cargo fmt -- --check` | PASS (clean) |

**All four required gates now pass.**

---

## Classification

The `encoded` reference was a pre-existing bug in `vb_ipc` (outside vb-qi37.16.4's `touched_crates`). With `release_critical=true`, the minimal safe fix was applied: `answer.len()` which correctly represents the encoded answer byte length per the `AskAnswer::encoded_len` contract and preserves the answer secret-redaction invariant.

No state advancement requested per instruction.
