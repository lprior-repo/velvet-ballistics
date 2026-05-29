# Proof-to-Implementation Input - vb-dybj

## Source Targets

- `crates/vb_core/src/ids/mod.rs`: `RunId`, `RunId::ZERO`, `RunId::new`, `RunId::get`, `WorkflowDigest::from_bytes`, `WorkflowDigest::as_bytes`.
- `crates/vb_storage/src/records.rs`: `RecordKind`, `RecordKind::id`, selected variants `RunAccepted` and one record-family variant.
- `crates/vb_storage/src/codec/mod.rs`: `encode_record`, `decode_record` if storage surface is selected.
- `crates/vb_storage/src/codec/header.rs` and `payload.rs`: fixed 60-byte envelope and short input ordering.
- `crates/vb_storage/src/error/mod.rs`: `JournalError::UnexpectedEof`, `JournalError::PostcardDecodeFailed`.
- `crates/workspace_tests/Cargo.toml`: add/register `restate_postcard_newtype_compat_tests` target.
- `crates/workspace_tests/tests/restate_postcard_newtype_compat_tests.rs`: downstream behavior test/proptest target.

## Required Bridge Mappings

| Obligation | Proof claim | Rust target | Required behavior evidence |
|---|---|---|---|
| PO-VB-DYBJ-001 | RunId constructor/accessor/ZERO/edge validity | `vb_core::RunId` | Fixed tests for `0`, representative, `u64::MAX`; no arithmetic needed. |
| PO-VB-DYBJ-002 | RunId bounded codec panic/overflow freedom | `vb_core::RunId` + Postcard | Kani harness with `kani::any::<u64>()` or exhaustive safe generator. |
| PO-VB-DYBJ-003 | RunId Postcard roundtrip/golden fixtures | workspace test file | `cargo nextest ... run_id`; exact byte constants frozen. |
| PO-VB-DYBJ-004 | WorkflowDigest exact byte preservation | `WorkflowDigest` | Verus-bound source proof plus executable fixture tests. |
| PO-VB-DYBJ-005 | Digest exact 32-byte shape | `WorkflowDigest([u8; 32])` | Flux shape/refinement or explicit tooling-block evidence. |
| PO-VB-DYBJ-006 | Digest encode/decode property | workspace test file | proptest over `[u8; 32]` plus nontrivial fixed pattern. |
| PO-VB-DYBJ-007 | RecordKind ID mapping/surface distinction | `RecordKind::id` and serde/Postcard | Verus source-bound ID proof; trusted external Postcard algorithm noted. |
| PO-VB-DYBJ-008 | Bounded selected RecordKind surface separation | `RecordKind` selected variants | Kani finite selected variants, no exhaustive non_exhaustive claim. |
| PO-VB-DYBJ-009 | Executable RecordKind named surface fixtures | workspace test file | Test names include `postcard_enum` or `envelope_id_u16_le`. |
| PO-VB-DYBJ-010 | Short storage input ordering | storage decode header/payload | Kani arbitrary short header/declared payload inputs. |
| PO-VB-DYBJ-011 | Missing bytes typed short error | storage decode public API | proptest asserts `JournalError::UnexpectedEof`, not strings. |
| PO-VB-DYBJ-012 | Fuzz short storage decode | storage decode public API | cargo-fuzz target with short/truncated seeds. |
| PO-VB-DYBJ-013 | Trailing suffix exact decode rejection | raw Postcard or storage selected surface | Kani bounded suffix 1..8 and explicit surface. |
| PO-VB-DYBJ-014 | Trailing byte property tests | workspace test file | proptest nonempty suffix, typed error variant. |
| PO-VB-DYBJ-015 | Fuzz trailing decode | raw/storage fuzz target | cargo-fuzz target classifies exact valid vs trailing malformed. |
| PO-VB-DYBJ-016 | Migration lifecycle | TLA+ lifecycle model | TLC bounded fixtures, no silent byte-change acceptance. |
| PO-VB-DYBJ-017 | Executable migration-required assertions | workspace test file | Test name/message documents named migration requirement. |
| PO-VB-DYBJ-018 | No forbidden codecs/wrappers | touched test/manifests | deterministic forbidden-token source scan. |

## Bridge Constraints

- Do not implement behavior by regenerating expected bytes at assertion time; expected fixture bytes must be constants.
- Do not conflate raw `postcard::Error` with `JournalError`; storage typed error claims require storage surface or typed adapter.
- Do not add JSON/YAML/HTTP/Bilrost/Protobuf dependencies or wrappers.
- Do not add unsafe/unwrap/expect/panic/todo/unimplemented/dbg in production code.
- If production code changes become necessary, the proof plan must be reviewed for stale source targets.

## Exact Commands Planned for Downstream Evidence

- `cargo nextest run -p velvet-ballistics-workspace-tests --test restate_postcard_newtype_compat_tests --no-fail-fast`
- `moon ci`
- Per-obligation Verus/Kani/Flux/TLC/cargo-fuzz commands from `proof-obligations.planned.jsonl`.
