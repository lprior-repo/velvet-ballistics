//! Phase 1 public contract tests for vb-core foundational types.

use core::str::FromStr;

use vb_core::diagnostic::{Diagnostic, DiagnosticCode, Severity};
use vb_core::errors::{CoreError, CoreResult, EngineError};
use vb_core::ids::{
    AccessorIdx, ActionId, BlobId, CheckedIndex, ConstIdx, ExprIdx, ListId, ObjectId, RunId, SeqNo,
    SlotIdx, StepIdx, SymbolId, WorkflowId,
};
use vb_core::limits::{
    MAX_CONSTANTS, MAX_EXPRESSION_DEPTH, MAX_RUN_NAME_LENGTH, MAX_SLOTS_PER_STEP,
    MAX_STEPS_PER_WORKFLOW,
};
use vb_core::span::{Located, SourceMap, Span, Spanned};
use vb_core::value::{FiniteF64, SlotValue};

#[test]
fn numeric_ids_construct_access_parse_and_serialize() {
    assert_eq!(WorkflowId::new(11).get(), 11);
    assert_eq!(StepIdx::new(12).as_usize(), 12);
    assert_eq!(SlotIdx::new(13).as_usize(), 13);
    assert_eq!(ExprIdx::new(14).as_usize(), 14);
    assert_eq!(ActionId::new(15).get(), 15);
    assert_eq!(AccessorIdx::new(16).as_usize(), 16);
    assert_eq!(ConstIdx::new(17).as_usize(), 17);
    assert_eq!(SymbolId::new(20).get(), 20);
    assert_eq!(ListId::new(21).get(), 21);
    assert_eq!(ObjectId::new(22).get(), 22);
    assert_eq!(BlobId::new(23).as_u64(), 23);
    assert_eq!(RunId::new(18).as_u64(), 18);
    assert_eq!(SeqNo::new(19).as_u64(), 19);

    assert_eq!(WorkflowId::from_str("11"), Ok(WorkflowId::new(11)));
    assert_eq!(StepIdx::from_str("12"), Ok(StepIdx::new(12)));
    assert_eq!(SlotIdx::from_str("13"), Ok(SlotIdx::new(13)));
    assert_eq!(ExprIdx::from_str("14"), Ok(ExprIdx::new(14)));
    assert_eq!(ActionId::from_str("15"), Ok(ActionId::new(15)));
    assert_eq!(AccessorIdx::from_str("16"), Ok(AccessorIdx::new(16)));
    assert_eq!(ConstIdx::from_str("17"), Ok(ConstIdx::new(17)));
    assert_eq!(SymbolId::from_str("20"), Ok(SymbolId::new(20)));
    assert_eq!(ListId::from_str("21"), Ok(ListId::new(21)));
    assert_eq!(ObjectId::from_str("22"), Ok(ObjectId::new(22)));
    assert_eq!(BlobId::from_str("23"), Ok(BlobId::new(23)));
    assert_eq!(RunId::from_str("18"), Ok(RunId::new(18)));
    assert_eq!(SeqNo::from_str("19"), Ok(SeqNo::new(19)));

    assert!(StepIdx::from_str("65536").is_err());
    assert!(RunId::from_str("340282366920938463463374607431768211455").is_err());
    assert_eq!(roundtrip_id(StepIdx::new(12)), StepIdx::new(12));
    assert_eq!(roundtrip_symbol_id(SymbolId::new(20)), SymbolId::new(20));
    assert_eq!(roundtrip_list_id(ListId::new(21)), ListId::new(21));
    assert_eq!(roundtrip_object_id(ObjectId::new(22)), ObjectId::new(22));
    assert_eq!(roundtrip_blob_id(BlobId::new(23)), BlobId::new(23));
}

#[test]
fn ids_expose_zero_min_max_checked_add_and_checked_index() {
    assert_eq!(RunId::ZERO, RunId::new(0));
    assert_eq!(StepIdx::ZERO, StepIdx::new(0));
    assert_eq!(SlotIdx::ZERO, SlotIdx::new(0));
    assert_eq!(SeqNo::ZERO, SeqNo::new(0));
    assert_eq!(SeqNo::MIN, SeqNo::new(0));
    assert_eq!(SeqNo::MAX, SeqNo::new(u64::MAX));
    assert_eq!(StepIdx::MIN, StepIdx::new(0));
    assert_eq!(StepIdx::MAX, StepIdx::new(u16::MAX));
    assert_eq!(SlotIdx::MIN, SlotIdx::new(0));
    assert_eq!(SlotIdx::MAX, SlotIdx::new(u16::MAX));

    assert_eq!(StepIdx::new(1).checked_add(2), Some(StepIdx::new(3)));
    assert_eq!(SlotIdx::new(1).checked_add(2), Some(SlotIdx::new(3)));
    assert_eq!(ConstIdx::new(1).checked_add(2), Some(ConstIdx::new(3)));
    assert_eq!(SeqNo::new(1).checked_add(2), Some(SeqNo::new(3)));
    assert_eq!(StepIdx::MAX.checked_add(1), None);
    assert_eq!(SlotIdx::MAX.checked_add(1), None);
    assert_eq!(ConstIdx::new(u16::MAX).checked_add(1), None);
    assert_eq!(SeqNo::MAX.checked_add(1), None);

    assert_eq!(CheckedIndex::as_usize(StepIdx::new(1)), 1);
    assert_eq!(CheckedIndex::as_usize(SlotIdx::new(2)), 2);
    assert_eq!(CheckedIndex::as_usize(ExprIdx::new(3)), 3);
    assert_eq!(CheckedIndex::as_usize(AccessorIdx::new(4)), 4);
    assert_eq!(CheckedIndex::as_usize(ConstIdx::new(5)), 5);
}

#[test]
fn limits_match_phase1_contract() {
    assert_eq!(MAX_STEPS_PER_WORKFLOW, 65_535);
    assert_eq!(MAX_SLOTS_PER_STEP, 256);
    assert_eq!(MAX_CONSTANTS, 65_535);
    assert_eq!(MAX_EXPRESSION_DEPTH, 64);
    assert_eq!(MAX_RUN_NAME_LENGTH, 1_024);
}

#[test]
fn spans_locations_and_source_map_are_constructible() {
    let span = Span::new(0, 5);
    let located = Located {
        value: 42_u32,
        span,
    };
    let spanned: Spanned<u32> = located.clone();

    assert!(Span::ZERO.is_empty());
    assert_eq!(span.end, 5);
    assert_eq!(located.value, 42);
    assert_eq!(spanned.span, span);
    assert_eq!(SourceMap::new(), SourceMap::default());
    assert_eq!(roundtrip_span(span), span);
}

#[test]
fn diagnostics_parse_display_and_own_messages() {
    let code = DiagnosticCode::new(0x0101);
    let diagnostic = Diagnostic {
        code,
        message: Box::<str>::from("invalid program counter"),
        severity: Severity::Error,
        span: Span::ZERO,
    };

    assert_eq!(code.code(), 0x0101);
    assert_eq!(code.to_string(), "E0101");
    assert_eq!(DiagnosticCode::from_str("E0101"), Ok(code));
    assert!(DiagnosticCode::from_str("E9999").is_err());
    assert_eq!(diagnostic.severity, Severity::Error);
    assert_eq!(Severity::Warning, Severity::Warning);
    assert_eq!(Severity::Info, Severity::Info);
    assert_eq!(diagnostic.message.as_ref(), "invalid program counter");
}

#[test]
fn core_errors_display_codes_and_engine_alias_convert() {
    let err: EngineError = CoreError::SlotOutOfBounds {
        slot: SlotIdx::ZERO,
    };
    let result: CoreResult<()> = Err(err.clone());

    assert_eq!(
        result,
        Err(CoreError::SlotOutOfBounds {
            slot: SlotIdx::ZERO
        })
    );
    assert_eq!(
        CoreError::InvalidProgramCounter {
            step: StepIdx::ZERO
        }
        .diagnostic_code(),
        DiagnosticCode::new(0x0101)
    );
    assert_eq!(err.diagnostic_code(), DiagnosticCode::new(0x0111));
    assert_eq!(engine_to_core(err.clone()), err);
    assert_eq!(err.to_string(), "slot index out of bounds: SlotIdx(0)");
    assert_eq!(
        CoreError::TypeMismatch {
            expected: "number",
            found: "text"
        }
        .to_string(),
        "type mismatch: expected number, found text"
    );
}

#[test]
fn slot_values_report_contract_type_names_and_roundtrip() {
    let finite = FiniteF64::new(3.5).map_or(SlotValue::Null, SlotValue::F64);
    let values = [
        SlotValue::Null,
        SlotValue::Bool(true),
        SlotValue::I64(42),
        finite,
        SlotValue::Symbol(SymbolId::new(7)),
        SlotValue::List(ListId::new(8)),
        SlotValue::Object(ObjectId::new(9)),
        SlotValue::Blob(BlobId::new(10)),
    ];
    let names = values.each_ref().map(SlotValue::type_name);

    assert_eq!(
        names,
        [
            "null", "boolean", "number", "number", "symbol", "list", "object", "blob"
        ]
    );
    assert_eq!(
        roundtrip_value(SlotValue::Bool(true)),
        SlotValue::Bool(true)
    );
}

#[test]
fn finite_f64_accepts_finite_and_rejects_non_finite_values() {
    assert_eq!(FiniteF64::new(1.25).map(FiniteF64::get), Ok(1.25));
    assert_eq!(FiniteF64::new(f64::NAN), Err(CoreError::NonFiniteNumber));
    assert_eq!(
        FiniteF64::new(f64::INFINITY),
        Err(CoreError::NonFiniteNumber)
    );
    assert_eq!(
        FiniteF64::new(f64::NEG_INFINITY),
        Err(CoreError::NonFiniteNumber)
    );
}

#[test]
fn slot_value_is_copy_compatible() {
    fn assert_copy<T: Copy>() {}

    assert_copy::<SlotValue>();
    assert_copy::<SymbolId>();
    assert_copy::<ListId>();
    assert_copy::<ObjectId>();
    assert_copy::<BlobId>();
    assert_copy::<FiniteF64>();
}

fn roundtrip_id(value: StepIdx) -> StepIdx {
    let bytes = postcard::to_allocvec(&value)
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    postcard::from_bytes::<StepIdx>(&bytes)
        .into_iter()
        .next()
        .map_or(StepIdx::ZERO, |id| id)
}

fn roundtrip_symbol_id(value: SymbolId) -> SymbolId {
    let bytes = postcard::to_allocvec(&value)
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    postcard::from_bytes::<SymbolId>(&bytes)
        .into_iter()
        .next()
        .map_or(SymbolId::new(0), |id| id)
}

fn roundtrip_list_id(value: ListId) -> ListId {
    let bytes = postcard::to_allocvec(&value)
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    postcard::from_bytes::<ListId>(&bytes)
        .into_iter()
        .next()
        .map_or(ListId::new(0), |id| id)
}

fn roundtrip_object_id(value: ObjectId) -> ObjectId {
    let bytes = postcard::to_allocvec(&value)
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    postcard::from_bytes::<ObjectId>(&bytes)
        .into_iter()
        .next()
        .map_or(ObjectId::new(0), |id| id)
}

fn roundtrip_blob_id(value: BlobId) -> BlobId {
    let bytes = postcard::to_allocvec(&value)
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    postcard::from_bytes::<BlobId>(&bytes)
        .into_iter()
        .next()
        .map_or(BlobId::new(0), |id| id)
}

fn engine_to_core(error: EngineError) -> CoreError {
    error
}

fn roundtrip_span(value: Span) -> Span {
    let bytes = postcard::to_allocvec(&value)
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    postcard::from_bytes::<Span>(&bytes)
        .into_iter()
        .next()
        .map_or(Span::ZERO, |span| span)
}

fn roundtrip_value(value: SlotValue) -> SlotValue {
    let bytes = postcard::to_allocvec(&value)
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    postcard::from_bytes::<SlotValue>(&bytes)
        .into_iter()
        .next()
        .map_or(SlotValue::Null, |slot| slot)
}
