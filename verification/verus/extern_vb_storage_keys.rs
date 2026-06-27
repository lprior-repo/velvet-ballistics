// SPDX-License-Identifier: MIT
//
// Extern surface for vb_storage_keys_spec Verus spec.
//
// Models the production storage-key encoding / decoding functions in
// `vb_storage::keys::*` as pure decision fns so Verus can reason about
// their round-trip and length properties.
//
// BINDING LEDGER (GOD RULE 2 compliance):
//   - `try_key_prefix`     mirrors `crates/vb_storage/src/keys.rs:281-295`.
//   - `journal_key`        mirrors `crates/vb_storage/src/keys.rs:436-438`.
//   - `run_event_key`      mirrors `crates/vb_storage/src/keys.rs:81-83`.
//   - `encode_key`         mirrors `crates/vb_storage/src/keys.rs:205-209`
//                          (delegates to `encode_key_into` at keys.rs:162-198).
//   - `decode_storage_key` mirrors `crates/vb_storage/src/keys.rs:346-434`.
//
// This file is plain Rust. It is brought into the spec file via
// `#[path = "extern_vb_storage_keys.rs"]` and is therefore NOT
// inside a `verus! {}` block. The functions are wrapped with
// `#[verifier::external]` to tell Verus to skip body verification —
// the spec file re-checks the contracts through exec wrappers.
//
// The production bodies of `encode_key` / `decode_storage_key` pull in
// `arrayvec::ArrayVec`, `vb_core::*`, `crate::types::*`,
// `crate::constants::*`, and `crate::error::*`. Verus does not model
// `ArrayVec` and these dependency types are not available in this
// single-file verification unit. The projections here capture the
// same decision branches as the production fns and are recorded as
// a trusted base in the binding ledger.
#![forbid(unsafe_code)]
#![allow(dead_code)]

use vstd::prelude::*;

// ---------------------------------------------------------------------------
// Keyspace length constants (mirror crates/vb_storage/src/constants.rs:74-79).
// ---------------------------------------------------------------------------
pub const DIGEST_BYTES: usize = 32;

pub const DIGEST_KEY_BYTES: usize = 33;
pub const RUN_ONLY_KEY_BYTES: usize = 9;
pub const JOURNAL_KEY_BYTES: usize = 17;
pub const INDEX_STATUS_KEY_BYTES: usize = 18;
pub const INDEX_WORKFLOW_KEY_BYTES: usize = 13;
pub const INDEX_ACTION_KEY_BYTES: usize = 13;

// ---------------------------------------------------------------------------
// Prefix byte constants (mirror crates/vb_storage/src/constants.rs:27-43).
// ---------------------------------------------------------------------------
pub const PREFIX_WORKFLOW_SOURCE: u8 = 0x01;
pub const PREFIX_COMPILED_IR: u8 = 0x02;
pub const PREFIX_RUN_HEADER: u8 = 0x10;
pub const PREFIX_RUN_EVENT: u8 = 0x11;
pub const PREFIX_RUN_SNAPSHOT: u8 = 0x12;
pub const PREFIX_BLOB: u8 = 0x20;
pub const PREFIX_INDEX_STATUS: u8 = 0x30;
pub const PREFIX_INDEX_WORKFLOW: u8 = 0x31;
pub const PREFIX_INDEX_ACTION: u8 = 0x32;

/// Mirror of `MIN_OTHER_STATUS_BYTE` at
/// `crates/vb_storage/src/constants.rs:53`. Bytes `0..MIN_OTHER_STATUS_BYTE`
/// collide with the named `IndexStatusState` variants.
pub const MIN_OTHER_STATUS_BYTE: u8 = 3;

// ---------------------------------------------------------------------------
// IndexStatusState mirror (mirror crates/vb_storage/src/types.rs:226-309).
// ---------------------------------------------------------------------------
#[derive(Clone, Copy)]
pub enum SpecIndexStatusState {
    Submitted,
    Active,
    Completed,
    Other(u8),
}

impl SpecIndexStatusState {
    /// Mirror of `IndexStatusState::from_u8`.
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Submitted,
            1 => Self::Active,
            2 => Self::Completed,
            _ => Self::Other(value),
        }
    }

    /// Mirror of `IndexStatusState::to_u8`.
    pub fn to_u8(self) -> u8 {
        match self {
            Self::Submitted => 0,
            Self::Active => 1,
            Self::Completed => 2,
            Self::Other(v) => v,
        }
    }

    /// Mirror of `IndexStatusState::to_u8_checked`. Rejects `Other(v)`
    /// whose byte collides with the named variants (0, 1, 2).
    pub fn to_u8_checked(self) -> Result<u8, SpecKeyEncodeError> {
        let byte = self.to_u8();
        match self {
            Self::Other(_) => {
                if byte < 3 {
                    Err(SpecKeyEncodeError::IndexStatusStateCollision)
                } else {
                    Ok(byte)
                }
            },
            _ => Ok(byte),
        }
    }

    /// Manual `PartialEq` impl (Verus does not yet support
    /// `PartialEq` derive on enums with `Other(u8)` payload).
    pub fn eq(self, other: &Self) -> bool {
        self.to_u8() == other.to_u8()
    }
}

// ---------------------------------------------------------------------------
// StorageKey mirror (mirror crates/vb_storage/src/types.rs:312-348).
// ---------------------------------------------------------------------------
#[derive(Clone, Copy)]
pub enum SpecStorageKey {
    WorkflowSource { digest: [u8; DIGEST_BYTES] },
    CompiledIr { digest: [u8; DIGEST_BYTES] },
    RunHeader { run: u64 },
    RunEvent { run: u64, seq: u64 },
    RunSnapshot { run: u64, seq: u64 },
    Blob { digest: [u8; DIGEST_BYTES] },
    IndexStatus { state: SpecIndexStatusState, timestamp: u64, run: u64 },
    IndexWorkflow { workflow: u32, run: u64 },
    IndexAction { action: u16, run: u64, step: u16 },
}

impl SpecStorageKey {
    /// Manual `PartialEq` impl (Verus derive limitation).
    pub fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::WorkflowSource { digest: a }, Self::WorkflowSource { digest: b }) => a == b,
            (Self::CompiledIr { digest: a }, Self::CompiledIr { digest: b }) => a == b,
            (Self::RunHeader { run: a }, Self::RunHeader { run: b }) => a == b,
            (Self::RunEvent { run: a, seq: b }, Self::RunEvent { run: c, seq: d }) => {
                a == c && b == d
            },
            (Self::RunSnapshot { run: a, seq: b }, Self::RunSnapshot { run: c, seq: d }) => {
                a == c && b == d
            },
            (Self::Blob { digest: a }, Self::Blob { digest: b }) => a == b,
            (
                Self::IndexStatus { state: a, timestamp: b, run: c },
                Self::IndexStatus { state: d, timestamp: e, run: f },
            ) => a.eq(d) && b == e && c == f,
            (
                Self::IndexWorkflow { workflow: a, run: b },
                Self::IndexWorkflow { workflow: c, run: d },
            ) => a == c && b == d,
            (
                Self::IndexAction { action: a, run: b, step: c },
                Self::IndexAction { action: d, run: e, step: f },
            ) => a == d && b == e && c == f,
            _ => false,
        }
    }
}

// ---------------------------------------------------------------------------
// KeyDecodeError mirror (mirror crates/vb_storage/src/error/key_decode.rs).
// ---------------------------------------------------------------------------
#[derive(Clone, Copy)]
pub enum SpecKeyDecodeError {
    EmptyKey,
    UnknownPrefix { prefix: u8 },
    KeyLengthMismatch { prefix: u8, expected: usize, actual: usize },
    InvalidRunId,
    ReservedSeqSentinel,
}

// ---------------------------------------------------------------------------
// KeyEncodeError mirror (sub-set of JournalError needed by encode).
// ---------------------------------------------------------------------------
#[derive(Clone, Copy)]
pub enum SpecKeyEncodeError {
    IndexStatusStateCollision,
    SequenceOverflow,
    KeyCapacity,
}

// ---------------------------------------------------------------------------
// KeyPrefix mirror (mirror crates/vb_storage/src/keys.rs:215-265).
// ---------------------------------------------------------------------------
#[derive(Clone, Copy)]
pub enum SpecKeyPrefix {
    WorkflowSource,
    CompiledIr,
    RunHeader,
    RunEvent,
    RunSnapshot,
    Blob,
    IndexStatus,
    IndexWorkflow,
    IndexAction,
}

impl SpecKeyPrefix {
    /// Mirror of `KeyPrefix::to_u8`.
    pub fn to_u8(self) -> u8 {
        match self {
            Self::WorkflowSource => PREFIX_WORKFLOW_SOURCE,
            Self::CompiledIr => PREFIX_COMPILED_IR,
            Self::RunHeader => PREFIX_RUN_HEADER,
            Self::RunEvent => PREFIX_RUN_EVENT,
            Self::RunSnapshot => PREFIX_RUN_SNAPSHOT,
            Self::Blob => PREFIX_BLOB,
            Self::IndexStatus => PREFIX_INDEX_STATUS,
            Self::IndexWorkflow => PREFIX_INDEX_WORKFLOW,
            Self::IndexAction => PREFIX_INDEX_ACTION,
        }
    }

    /// Mirror of `KeyPrefix::expected_key_len`.
    pub fn expected_key_len(self) -> usize {
        match self {
            Self::WorkflowSource | Self::CompiledIr | Self::Blob => DIGEST_KEY_BYTES,
            Self::RunHeader => RUN_ONLY_KEY_BYTES,
            Self::RunEvent | Self::RunSnapshot => JOURNAL_KEY_BYTES,
            Self::IndexStatus => INDEX_STATUS_KEY_BYTES,
            Self::IndexWorkflow | Self::IndexAction => INDEX_WORKFLOW_KEY_BYTES,
        }
    }

    /// Reverse mapping used by `try_key_prefix`.
    pub fn from_byte(b: u8) -> Option<Self> {
        match b {
            PREFIX_WORKFLOW_SOURCE => Some(Self::WorkflowSource),
            PREFIX_COMPILED_IR => Some(Self::CompiledIr),
            PREFIX_RUN_HEADER => Some(Self::RunHeader),
            PREFIX_RUN_EVENT => Some(Self::RunEvent),
            PREFIX_RUN_SNAPSHOT => Some(Self::RunSnapshot),
            PREFIX_BLOB => Some(Self::Blob),
            PREFIX_INDEX_STATUS => Some(Self::IndexStatus),
            PREFIX_INDEX_WORKFLOW => Some(Self::IndexWorkflow),
            PREFIX_INDEX_ACTION => Some(Self::IndexAction),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Production-mirror exec fns (declared external — bodies are trusted-base).
// ---------------------------------------------------------------------------
/// Mirror of `try_key_prefix` at crates/vb_storage/src/keys.rs:281-295.
#[verifier::external]
pub fn try_key_prefix(bytes: &[u8]) -> Result<SpecKeyPrefix, SpecKeyDecodeError> {
    let prefix = *bytes.first().ok_or(SpecKeyDecodeError::EmptyKey)?;
    SpecKeyPrefix::from_byte(prefix).ok_or(SpecKeyDecodeError::UnknownPrefix { prefix })
}

/// Mirror of `journal_key` at crates/vb_storage/src/keys.rs:436-438.
#[verifier::external]
pub fn journal_key(run: u64, seq: u64) -> Result<[u8; 17], SpecKeyEncodeError> {
    if seq == u64::MAX {
        return Err(SpecKeyEncodeError::SequenceOverflow);
    }
    let mut key = [0u8; 17];
    key[0] = PREFIX_RUN_EVENT;
    key[1] = (run >> 56) as u8;
    key[2] = (run >> 48) as u8;
    key[3] = (run >> 40) as u8;
    key[4] = (run >> 32) as u8;
    key[5] = (run >> 24) as u8;
    key[6] = (run >> 16) as u8;
    key[7] = (run >> 8) as u8;
    key[8] = run as u8;
    key[9] = (seq >> 56) as u8;
    key[10] = (seq >> 48) as u8;
    key[11] = (seq >> 40) as u8;
    key[12] = (seq >> 32) as u8;
    key[13] = (seq >> 24) as u8;
    key[14] = (seq >> 16) as u8;
    key[15] = (seq >> 8) as u8;
    key[16] = seq as u8;
    Ok(key)
}

/// Mirror of `run_event_key` at crates/vb_storage/src/keys.rs:81-83.
#[verifier::external]
pub fn run_event_key(run: u64, seq: u64) -> Result<[u8; 17], SpecKeyEncodeError> {
    journal_key(run, seq)
}

/// Mirror of `encode_key` at `crates/vb_storage/src/keys.rs:205-209`.
/// Delegates to `encode_key_into` semantics: clears the output buffer and
/// writes the typed encoder bytes for the discriminant. Each arm mirrors
/// the typed encoder called by the production `encode_key_into`:
///   - digest variants -> `digest_key`            (keys.rs:462-472)
///   - run-header      -> `run_only_key`           (keys.rs:474-481)
///   - run-event       -> `journal_key`            (keys.rs:436-438)
///   - run-snapshot    -> `sequenced_run_key`      (keys.rs:440-456)
///   - index-status    -> `index_status_key`       (keys.rs:101-122)
///   - index-workflow  -> `index_workflow_key`     (keys.rs:125-137)
///   - index-action    -> `index_action_key`       (keys.rs:140-155)
#[verifier::external]
pub fn encode_key(key: SpecStorageKey) -> Result<Vec<u8>, SpecKeyEncodeError> {
    // The body is opaque to Verus (`#[verifier::external]`); the
    // `assume_specification` contract in `vb_storage_keys_spec.rs` is
    // the verified surface. This body is the trusted-base projection
    // recorded in the binding ledger.
    match key {
        SpecStorageKey::WorkflowSource { digest } => Ok(digest_prefixed(PREFIX_WORKFLOW_SOURCE, &digest).to_vec()),
        SpecStorageKey::CompiledIr { digest } => Ok(digest_prefixed(PREFIX_COMPILED_IR, &digest).to_vec()),
        SpecStorageKey::RunHeader { run } => Ok(run_prefixed(PREFIX_RUN_HEADER, run).to_vec()),
        SpecStorageKey::RunEvent { run, seq } => Ok(journal_key(run, seq)?.to_vec()),
        SpecStorageKey::RunSnapshot { run, seq } => sequenced_run_prefixed(PREFIX_RUN_SNAPSHOT, run, seq),
        SpecStorageKey::Blob { digest } => Ok(digest_prefixed(PREFIX_BLOB, &digest).to_vec()),
        SpecStorageKey::IndexStatus { state, timestamp, run } => {
            let state_byte = state.to_u8_checked()?;
            Ok(index_status_layout(state_byte, timestamp, run).to_vec())
        },
        SpecStorageKey::IndexWorkflow { workflow, run } => {
            Ok(index_workflow_layout(workflow, run).to_vec())
        },
        SpecStorageKey::IndexAction { action, run, step } => {
            Ok(index_action_layout(action, run, step).to_vec())
        },
    }
}

// Local helpers used only by `encode_key` / `decode_storage_key`. These
// are not part of the public surface; they are pure projections that
// build the byte layout per the production typed encoders.
//
// Verus does not support IndexMut on fixed-size arrays with Range /
// RangeFrom operands, nor the `to_be_bytes` intrinsic on u16/u32/u64.
// The helpers below write each byte position explicitly via shift
// arithmetic. The bodies are `#[verifier::external]`-adjacent (still
// opaque to Verus; the assume_specification contracts pin the layout).
//
// `#[verifier::exec_allows_no_decreases_clause]` permits the simple
// counting loops below; the helpers are only reachable from the
// `#[verifier::external]` `encode_key` body, so Verus never has to
// reason about termination of these helpers.

#[verifier::external]
fn write_u64_be(out: &mut [u8; 8], offset: usize, value: u64) {
    out[offset + 0] = (value >> 56) as u8;
    out[offset + 1] = (value >> 48) as u8;
    out[offset + 2] = (value >> 40) as u8;
    out[offset + 3] = (value >> 32) as u8;
    out[offset + 4] = (value >> 24) as u8;
    out[offset + 5] = (value >> 16) as u8;
    out[offset + 6] = (value >> 8) as u8;
    out[offset + 7] = value as u8;
}

#[verifier::external]
fn write_u32_be(out: &mut [u8; 4], offset: usize, value: u32) {
    out[offset + 0] = (value >> 24) as u8;
    out[offset + 1] = (value >> 16) as u8;
    out[offset + 2] = (value >> 8) as u8;
    out[offset + 3] = value as u8;
}

#[verifier::external]
fn write_u16_be(out: &mut [u8; 2], offset: usize, value: u16) {
    out[offset + 0] = (value >> 8) as u8;
    out[offset + 1] = value as u8;
}

#[verifier::external]
fn digest_prefixed(prefix: u8, digest: &[u8; DIGEST_BYTES]) -> [u8; DIGEST_KEY_BYTES] {
    let mut out = [0u8; DIGEST_KEY_BYTES];
    out[0] = prefix;
    let mut i = 0;
    while i < DIGEST_BYTES {
        out[1 + i] = digest[i];
        i += 1;
    }
    out
}

#[verifier::external]
fn run_prefixed(prefix: u8, run: u64) -> [u8; RUN_ONLY_KEY_BYTES] {
    let mut out = [0u8; RUN_ONLY_KEY_BYTES];
    out[0] = prefix;
    let mut be = [0u8; 8];
    write_u64_be(&mut be, 0, run);
    let mut i = 0;
    while i < 8 {
        out[1 + i] = be[i];
        i += 1;
    }
    out
}

#[verifier::external]
fn sequenced_run_prefixed(
    prefix: u8,
    run: u64,
    seq: u64,
) -> Result<Vec<u8>, SpecKeyEncodeError> {
    if seq == u64::MAX {
        return Err(SpecKeyEncodeError::SequenceOverflow);
    }
    let mut out = [0u8; JOURNAL_KEY_BYTES];
    out[0] = prefix;
    let mut run_be = [0u8; 8];
    let mut seq_be = [0u8; 8];
    write_u64_be(&mut run_be, 0, run);
    write_u64_be(&mut seq_be, 0, seq);
    let mut i = 0;
    while i < 8 {
        out[1 + i] = run_be[i];
        out[9 + i] = seq_be[i];
        i += 1;
    }
    Ok(array_to_vec(&out))
}

#[verifier::external]
fn array_to_vec(arr: &[u8; JOURNAL_KEY_BYTES]) -> Vec<u8> {
    let mut v: Vec<u8> = Vec::with_capacity(JOURNAL_KEY_BYTES);
    let mut i = 0;
    while i < JOURNAL_KEY_BYTES {
        v.push(arr[i]);
        i += 1;
    }
    v
}

#[verifier::external]
fn index_status_layout(state_byte: u8, timestamp: u64, run: u64) -> [u8; INDEX_STATUS_KEY_BYTES] {
    let mut out = [0u8; INDEX_STATUS_KEY_BYTES];
    out[0] = PREFIX_INDEX_STATUS;
    out[1] = state_byte;
    let mut ts_be = [0u8; 8];
    let mut run_be = [0u8; 8];
    write_u64_be(&mut ts_be, 0, timestamp);
    write_u64_be(&mut run_be, 0, run);
    let mut i = 0;
    while i < 8 {
        out[2 + i] = ts_be[i];
        out[10 + i] = run_be[i];
        i += 1;
    }
    out
}

#[verifier::external]
fn index_workflow_layout(workflow: u32, run: u64) -> [u8; INDEX_WORKFLOW_KEY_BYTES] {
    let mut out = [0u8; INDEX_WORKFLOW_KEY_BYTES];
    out[0] = PREFIX_INDEX_WORKFLOW;
    let mut wf_be = [0u8; 4];
    let mut run_be = [0u8; 8];
    write_u32_be(&mut wf_be, 0, workflow);
    write_u64_be(&mut run_be, 0, run);
    let mut i = 0;
    while i < 4 {
        out[1 + i] = wf_be[i];
        i += 1;
    }
    let mut j = 0;
    while j < 8 {
        out[5 + j] = run_be[j];
        j += 1;
    }
    out
}

#[verifier::external]
fn index_action_layout(action: u16, run: u64, step: u16) -> [u8; INDEX_ACTION_KEY_BYTES] {
    let mut out = [0u8; INDEX_ACTION_KEY_BYTES];
    out[0] = PREFIX_INDEX_ACTION;
    let mut act_be = [0u8; 2];
    let mut run_be = [0u8; 8];
    let mut step_be = [0u8; 2];
    write_u16_be(&mut act_be, 0, action);
    write_u64_be(&mut run_be, 0, run);
    write_u16_be(&mut step_be, 0, step);
    let mut i = 0;
    while i < 2 {
        out[1 + i] = act_be[i];
        i += 1;
    }
    let mut j = 0;
    while j < 8 {
        out[3 + j] = run_be[j];
        j += 1;
    }
    let mut k = 0;
    while k < 2 {
        out[11 + k] = step_be[k];
        k += 1;
    }
    out
}

/// Mirror of `decode_storage_key` at `crates/vb_storage/src/keys.rs:346-434`.
/// Decodes all nine `StorageKey` variants; each arm mirrors the
/// production decoder branch:
///   - digest variants  -> 32-byte digest at offset 1
///   - run-header       -> big-endian u64 run at offset 1, rejects run == 0
///   - run-event        -> big-endian u64 run/seq, rejects run == 0 / seq == MAX
///   - run-snapshot     -> big-endian u64 run/seq, rejects run == 0 / seq == MAX
///   - index-status     -> state byte + big-endian timestamp + big-endian run
///   - index-workflow   -> big-endian u32 workflow + big-endian u64 run
///   - index-action     -> big-endian u16 action + big-endian u64 run + u16 step
#[verifier::external]
pub fn decode_storage_key(bytes: &[u8]) -> Result<SpecStorageKey, SpecKeyDecodeError> {
    let prefix = try_key_prefix(bytes)?;
    let expected_len = prefix.expected_key_len();
    if bytes.len() != expected_len {
        return Err(SpecKeyDecodeError::KeyLengthMismatch {
            prefix: prefix.to_u8(),
            expected: expected_len,
            actual: bytes.len(),
        });
    }

    let read_u8 = |idx: usize| -> Result<u8, SpecKeyDecodeError> {
        bytes.get(idx).copied().ok_or(SpecKeyDecodeError::KeyLengthMismatch {
            prefix: prefix.to_u8(),
            expected: expected_len,
            actual: bytes.len(),
        })
    };

    let read_u64 = |start: usize| -> Result<u64, SpecKeyDecodeError> {
        let mut buf = [0u8; 8];
        buf.copy_from_slice(
            bytes
                .get(start..start + 8)
                .ok_or(SpecKeyDecodeError::KeyLengthMismatch {
                    prefix: prefix.to_u8(),
                    expected: expected_len,
                    actual: bytes.len(),
                })?,
        );
        Ok(u64::from_be_bytes(buf))
    };

    match prefix {
        SpecKeyPrefix::WorkflowSource | SpecKeyPrefix::CompiledIr | SpecKeyPrefix::Blob => {
            let mut digest = [0u8; DIGEST_BYTES];
            digest.copy_from_slice(
                bytes
                    .get(1..DIGEST_KEY_BYTES)
                    .ok_or(SpecKeyDecodeError::KeyLengthMismatch {
                        prefix: prefix.to_u8(),
                        expected: expected_len,
                        actual: bytes.len(),
                    })?,
            );
            match prefix {
                SpecKeyPrefix::WorkflowSource => Ok(SpecStorageKey::WorkflowSource { digest }),
                SpecKeyPrefix::CompiledIr => Ok(SpecStorageKey::CompiledIr { digest }),
                _ => Ok(SpecStorageKey::Blob { digest }),
            }
        },
        SpecKeyPrefix::RunHeader => {
            let run_val = read_u64(1)?;
            if run_val == 0 {
                return Err(SpecKeyDecodeError::InvalidRunId);
            }
            Ok(SpecStorageKey::RunHeader { run: run_val })
        },
        SpecKeyPrefix::RunEvent | SpecKeyPrefix::RunSnapshot => {
            let run_val = read_u64(1)?;
            if run_val == 0 {
                return Err(SpecKeyDecodeError::InvalidRunId);
            }
            let seq_val = read_u64(9)?;
            if seq_val == u64::MAX {
                return Err(SpecKeyDecodeError::ReservedSeqSentinel);
            }
            match prefix {
                SpecKeyPrefix::RunEvent => {
                    Ok(SpecStorageKey::RunEvent { run: run_val, seq: seq_val })
                },
                _ => Ok(SpecStorageKey::RunSnapshot { run: run_val, seq: seq_val }),
            }
        },
        SpecKeyPrefix::IndexStatus => {
            let state_byte = read_u8(1)?;
            let state = SpecIndexStatusState::from_u8(state_byte);
            let timestamp = read_u64(2)?;
            let run_val = read_u64(10)?;
            if run_val == 0 {
                return Err(SpecKeyDecodeError::InvalidRunId);
            }
            Ok(SpecStorageKey::IndexStatus { state, timestamp, run: run_val })
        },
        SpecKeyPrefix::IndexWorkflow => {
            let mut wf_buf = [0u8; 4];
            wf_buf.copy_from_slice(
                bytes
                    .get(1..5)
                    .ok_or(SpecKeyDecodeError::KeyLengthMismatch {
                        prefix: prefix.to_u8(),
                        expected: expected_len,
                        actual: bytes.len(),
                    })?,
            );
            let workflow_val = u32::from_be_bytes(wf_buf);
            let run_val = read_u64(5)?;
            if run_val == 0 {
                return Err(SpecKeyDecodeError::InvalidRunId);
            }
            Ok(SpecStorageKey::IndexWorkflow { workflow: workflow_val, run: run_val })
        },
        SpecKeyPrefix::IndexAction => {
            let mut act_buf = [0u8; 2];
            act_buf.copy_from_slice(
                bytes
                    .get(1..3)
                    .ok_or(SpecKeyDecodeError::KeyLengthMismatch {
                        prefix: prefix.to_u8(),
                        expected: expected_len,
                        actual: bytes.len(),
                    })?,
            );
            let action_val = u16::from_be_bytes(act_buf);
            let run_val = read_u64(3)?;
            if run_val == 0 {
                return Err(SpecKeyDecodeError::InvalidRunId);
            }
            let mut step_buf = [0u8; 2];
            step_buf.copy_from_slice(
                bytes
                    .get(11..13)
                    .ok_or(SpecKeyDecodeError::KeyLengthMismatch {
                        prefix: prefix.to_u8(),
                        expected: expected_len,
                        actual: bytes.len(),
                    })?,
            );
            let step_val = u16::from_be_bytes(step_buf);
            Ok(SpecStorageKey::IndexAction { action: action_val, run: run_val, step: step_val })
        },
    }
}