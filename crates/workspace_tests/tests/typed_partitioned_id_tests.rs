#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::borrow_deref_ref,
    clippy::map_clone,
    clippy::new_without_default,
    clippy::map_flatten,
    clippy::manual_unwrap_or_default,
    clippy::io_other_error,
    clippy::cmp_owned,
    clippy::derivable_impls,
    clippy::cloned_ref_to_slice_refs,
    clippy::explicit_counter_loop,
    clippy::unnecessary_sort_by,
    clippy::items_after_test_module,
    clippy::unnecessary_cast,
    clippy::manual_saturating_arithmetic,
    clippy::needless_borrows_for_generic_args,
    clippy::manual_unwrap_or,
    clippy::unnecessary_map_or,
    clippy::large_stack_arrays,
    clippy::implicit_saturating_sub,
    clippy::useless_asref,
    clippy::get_first,
    clippy::iter_count,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_fallible_conversions,
    clippy::type_complexity,
    clippy::err_expect,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables
)]
#![forbid(unsafe_code)]

use proptest::prelude::*;
use vb_core::{ActionId, RunId, SeqNo, StepIdx, WorkflowId};
use vb_storage::{JournalError, codec::decode_record_header, keys, types::EventSeq};

fn header_with_kind(kind: u16) -> [u8; vb_storage::constants::RECORD_HEADER_BYTES] {
    let mut header = [0_u8; vb_storage::constants::RECORD_HEADER_BYTES];
    header[0..4].copy_from_slice(&vb_storage::constants::MAGIC_JOURNAL_EVENT.to_le_bytes());
    header[4..6].copy_from_slice(&vb_storage::constants::CURRENT_SCHEMA_VERSION.to_le_bytes());
    header[6..8].copy_from_slice(&kind.to_le_bytes());
    header[8..12].copy_from_slice(&vb_storage::constants::RECORD_HEADER_LEN.to_le_bytes());
    header[12..16].copy_from_slice(&0_u32.to_le_bytes());
    header[16..24].copy_from_slice(&0_u64.to_le_bytes());
    let checksum = crc32c::crc32c(&header[..vb_storage::constants::CRC_OFFSET]);
    header[vb_storage::constants::CRC_OFFSET..vb_storage::constants::CRC_OFFSET + 4]
        .copy_from_slice(&checksum.to_le_bytes());
    header
}

fn unknown_kind(kind: u16) -> bool {
    !matches!(kind, 1 | 2 | 3 | 7 | 10..=29 | 30 | 40 | 50)
}

proptest! {
    #[test]
    fn generated_typed_partitioned_ids_preserve_bytes(
        run in any::<u64>(),
        seq in any::<u64>(),
        workflow in any::<u32>(),
        action in any::<u16>(),
        step in any::<u16>(),
        kind in any::<u16>(),
    ) {
        let header = keys::run_header_key(RunId::new(run))?;
        prop_assert_eq!(header[0], vb_storage::constants::PREFIX_RUN_HEADER);
        prop_assert_eq!(&header[1..9], &run.to_be_bytes());

        let event = keys::run_event_key(RunId::new(run), EventSeq::new(seq))?;
        prop_assert_eq!(event[0], vb_storage::constants::PREFIX_RUN_EVENT);
        prop_assert_eq!(&event[1..9], &run.to_be_bytes());
        prop_assert_eq!(&event[9..17], &seq.to_be_bytes());

        let workflow_key = keys::index_workflow_key(WorkflowId::new(workflow), RunId::new(run))?;
        prop_assert_eq!(workflow_key[0], vb_storage::constants::PREFIX_INDEX_WORKFLOW);
        prop_assert_eq!(&workflow_key[1..5], &workflow.to_be_bytes());
        prop_assert_eq!(&workflow_key[5..13], &run.to_be_bytes());

        let action_key = keys::index_action_key(ActionId::new(action), RunId::new(run), StepIdx::new(step))?;
        prop_assert_eq!(action_key[0], vb_storage::constants::PREFIX_INDEX_ACTION);
        prop_assert_eq!(&action_key[1..3], &action.to_be_bytes());
        prop_assert_eq!(&action_key[3..11], &run.to_be_bytes());
        prop_assert_eq!(&action_key[11..13], &step.to_be_bytes());

        if seq == u64::MAX {
            prop_assert!(SeqNo::new(seq).checked_add(1).is_none());
        } else {
            prop_assert_eq!(SeqNo::new(seq).checked_add(1).map(SeqNo::get), Some(seq + 1));
        }

        if unknown_kind(kind) {
            let decoded = decode_record_header(
                &header_with_kind(kind),
                vb_storage::constants::MAGIC_JOURNAL_EVENT,
                vb_storage::constants::MAX_JOURNAL_EVENT_PAYLOAD_BYTES,
            );
            match decoded {
                Err(JournalError::UnknownRecordKind { kind: found }) => prop_assert_eq!(found, kind),
                other => panic!("expected UnknownRecordKind, got {other:?}"),
            }
        }
    }
}

#[test]
fn explicit_edges_and_stable_record_kinds_hold() -> Result<(), JournalError> {
    for run in [0, 1, 0x0102_0304_0506_0708, u64::MAX - 1, u64::MAX] {
        let header = keys::run_header_key(RunId::new(run))?;
        assert_eq!(&header[1..9], &run.to_be_bytes());
    }
    for workflow in [0, 1, 0x0102_0304, u32::MAX - 1, u32::MAX] {
        let key = keys::index_workflow_key(WorkflowId::new(workflow), RunId::new(7))?;
        assert_eq!(&key[1..5], &workflow.to_be_bytes());
    }
    for value in [0, 1, 0x0102, u16::MAX - 1, u16::MAX] {
        let key = keys::index_action_key(ActionId::new(value), RunId::new(7), StepIdx::new(value))?;
        assert_eq!(&key[1..3], &value.to_be_bytes());
        assert_eq!(&key[11..13], &value.to_be_bytes());
    }
    assert_eq!(vb_storage::records::RecordKind::WorkflowSource.id(), 1);
    assert_eq!(vb_storage::records::RecordKind::CompiledIr.id(), 2);
    assert_eq!(vb_storage::records::RecordKind::RunHeader.id(), 3);
    assert_eq!(vb_storage::records::RecordKind::RecoveryStamp.id(), 7);
    assert_eq!(vb_storage::records::RecordKind::RunAccepted.id(), 10);
    assert_eq!(vb_storage::records::RecordKind::RunAnswered.id(), 27);
    assert_eq!(vb_storage::records::RecordKind::RunKilled.id(), 28);
    assert_eq!(vb_storage::records::RecordKind::StepSucceeded.id(), 29);
    assert_eq!(vb_storage::records::RecordKind::Snapshot.id(), 30);
    assert_eq!(vb_storage::records::RecordKind::Blob.id(), 40);
    assert_eq!(vb_storage::records::RecordKind::IndexUpdate.id(), 50);
    Ok(())
}
