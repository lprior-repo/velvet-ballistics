#![forbid(unsafe_code)]

use crate::compat::{CURRENT_IR_VERSION, CompatibilityClass, SchemaVersion};
use crate::constants::MAX_COMPILED_IR_BYTES;
use crate::{JournalError, decode_compiled_ir, encode_compiled_ir};
use vb_core::ids::{ConstIdx, SlotIdx, StepIdx, SymbolId, WorkflowDigest};
use vb_core::value::ConstValue;
use vb_core::workflow::source_metadata::SourceMetadataError;
use vb_core::workflow::{
    AccessorProgram, CompiledNode, CompiledNodeKind, CompiledWorkflow, PathSegment,
    ResourceContract, VersionedWorkflowParts, WorkflowError, WorkflowIrV2, WorkflowParts,
};

#[path = "compiled_ir_test_support.rs"]
mod support;
use support::*;

#[test]
fn v1_zero_symbol_roundtrip() -> TestResult {
    assert_v1(0, None)
}

#[test]
fn v1_none_binding_on_decode() -> TestResult {
    assert_v1(0, None)
}

#[test]
fn v1_outer_version_1_2_0_accepted() -> TestResult {
    assert_v1(0, Some(SchemaVersion::new(1, 2, 0)))
}

#[test]
fn v1_nonzero_symbol_wire_identity() -> TestResult {
    assert_v1(1, None)
}

#[test]
fn v2_empty_symbol_payloads() -> TestResult {
    assert_v2(&[], None)
}

#[test]
fn v2_some_empty_not_downgraded_to_v1() -> TestResult {
    assert_v2(&[], Some(SchemaVersion::new(2, 0, 0)))
}

#[test]
fn v2_ordered_payload_roundtrip() -> TestResult {
    assert_v2(&["key", "é", " "], None)
}

#[path = "compiled_ir_boundary_tests.rs"]
mod boundaries;
#[path = "compiled_ir_digest_tests.rs"]
mod digests;
#[path = "compiled_ir_metadata_tests.rs"]
mod metadata;
#[path = "compiled_ir_version_tests.rs"]
mod versions;
