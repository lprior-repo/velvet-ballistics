#![allow(
    clippy::absurd_extreme_comparisons,
    clippy::approx_constant,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::assertions_on_constants,
    clippy::bool_assert_comparison,
    clippy::bool_comparison,
    clippy::borrow_deref_ref,
    clippy::cast_abs_to_unsigned,
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::clone_on_copy,
    clippy::cloned_ref_to_slice_refs,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::duplicated_attributes,
    clippy::err_expect,
    clippy::expect_fun_call,
    clippy::expect_used,
    clippy::explicit_counter_loop,
    clippy::field_reassign_with_default,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::get_first,
    clippy::if_let_mutex,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::implicit_saturating_sub,
    clippy::inconsistent_struct_constructor,
    clippy::indexing_slicing,
    clippy::inefficient_to_string,
    clippy::io_other_error,
    clippy::items_after_test_module,
    clippy::iter_count,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_futures,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::len_zero,
    clippy::let_and_return,
    clippy::let_underscore_must_use,
    clippy::manual_div_ceil,
    clippy::manual_let_else,
    clippy::manual_map,
    clippy::manual_saturating_arithmetic,
    clippy::manual_strip,
    clippy::manual_unwrap_or,
    clippy::manual_unwrap_or_default,
    clippy::map_clone,
    clippy::map_flatten,
    clippy::match_like_matches_macro,
    clippy::misnamed_getters,
    clippy::missing_safety_doc,
    clippy::module_inception,
    clippy::mutable_key_type,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
    clippy::needless_collect,
    clippy::needless_pass_by_value,
    clippy::needless_range_loop,
    clippy::needless_return,
    clippy::needless_update,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::new_without_default,
    clippy::nonminimal_bool,
    clippy::ok_expect,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::redundant_clone,
    clippy::redundant_closure,
    clippy::redundant_else,
    clippy::redundant_guards,
    clippy::redundant_locals,
    clippy::redundant_pattern_matching,
    clippy::redundant_pub_crate,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::single_match,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::type_complexity,
    clippy::unimplemented,
    clippy::uninlined_format_args,
    clippy::unnecessary_cast,
    clippy::unnecessary_fallible_conversions,
    clippy::unnecessary_map_or,
    clippy::unnecessary_mut_passed,
    clippy::unnecessary_sort_by,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_wraps,
    clippy::unneeded_struct_pattern,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_async,
    clippy::unused_io_amount,
    clippy::unused_self,
    clippy::unused_trait_names,
    clippy::unwrap_used,
    clippy::useless_asref,
    clippy::useless_conversion,
    clippy::useless_format,
    clippy::useless_vec,
    clippy::vec_init_then_push,
    clippy::wildcard_enum_match_arm,
    clippy::wildcard_imports,
    dead_code,
    let_underscore_drop,
    unused_imports,
    unused_variables
)]
#![forbid(unsafe_code)]

use vb_runtime::runtime::Runtime;
use vb_runtime::shard::ShardConfig;

use crate::IpcCommand;
use crate::IpcFrameHeader;
use crate::IpcPayload;
use crate::SubmitRunPayload;
use crate::server::IpcResponse;
use crate::server::IpcServer;
use crate::server::WorkflowResolutionError;
use crate::server::WorkflowResolver;

use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static NEXT_SOCKET_ID: AtomicU64 = AtomicU64::new(0);

fn temp_socket_path(name: &str) -> PathBuf {
    let sequence_result =
        NEXT_SOCKET_ID.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
            current.checked_add(1)
        });
    let sequence = match sequence_result {
        Ok(current) => current,
        Err(_overflowed_current) => 0,
    };
    let suffix = name
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(12)
        .collect::<String>();
    PathBuf::from(format!(
        "/tmp/vbd_{}_{}_{}.sock",
        std::process::id(),
        sequence,
        suffix
    ))
}

struct CleanupPath<'a>(&'a std::path::Path);
impl Drop for CleanupPath<'_> {
    fn drop(&mut self) {
        if let Err(_cleanup_error) = std::fs::remove_file(self.0) {}
    }
}

fn make_runtime() -> Runtime {
    let mut config = ShardConfig::default();
    config.policy = vb_core::policy::RuntimePolicy::Relaxed;
    Runtime::new(NonZeroUsize::MIN, config)
}

#[test]
fn serve_ipc_returns_true_when_server_should_continue() {
    let path = temp_socket_path("continue");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();
    server.set_test_poll_once_result(Ok(true));
    let result = super::dispatch::serve_ipc(&mut server, &mut runtime, Some(Duration::ZERO));
    assert_eq!(result, Ok(true));
}

#[test]
fn serve_ipc_returns_false_on_shutdown() {
    let path = temp_socket_path("shutdown");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();
    server.set_test_poll_once_result(Ok(false));
    let result = super::dispatch::serve_ipc(&mut server, &mut runtime, Some(Duration::ZERO));
    assert_eq!(result, Ok(false));
}

#[test]
fn serve_ipc_propagates_poll_once_errors() {
    let path = temp_socket_path("error");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();
    server.set_test_poll_once_result(Err(crate::server::IpcServerError::TooManyClients));
    let result = super::dispatch::serve_ipc(&mut server, &mut runtime, Some(Duration::ZERO));
    assert!(matches!(
        result,
        Err(crate::server::IpcServerError::TooManyClients)
    ));
}

#[test]
fn serve_ipc_with_resolver_forwards_to_poll_once_with_resolver() {
    let path = temp_socket_path("resolver");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();
    server.set_test_poll_once_result(Ok(true));
    let result = super::dispatch::serve_ipc_with_resolver(
        &mut server,
        &mut runtime,
        Some(Duration::ZERO),
        None,
    );
    assert_eq!(result, Ok(true));
}

#[test]
fn serve_ipc_with_resolver_returns_false_when_server_should_shutdown() {
    let path = temp_socket_path("resolver_shutdown");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();
    server.set_test_poll_once_result(Ok(false));
    let result = super::dispatch::serve_ipc_with_resolver(
        &mut server,
        &mut runtime,
        Some(Duration::ZERO),
        None,
    );
    assert_eq!(result, Ok(false));
}

#[test]
fn serve_ipc_with_resolver_propagates_poll_once_errors() {
    let path = temp_socket_path("resolver_error");
    let _cleanup = CleanupPath(&path);
    let mut server = IpcServer::bind(&path).expect("bind should succeed");
    let mut runtime = make_runtime();
    server.set_test_poll_once_result(Err(crate::server::IpcServerError::TooManyClients));
    let result = super::dispatch::serve_ipc_with_resolver(
        &mut server,
        &mut runtime,
        Some(Duration::ZERO),
        None,
    );
    assert!(matches!(
        result,
        Err(crate::server::IpcServerError::TooManyClients)
    ));
}

#[test]
fn dispatch_command_wrapper_delegates_to_dispatch_command_with_resolver() {
    let mut runtime = make_runtime();
    let header = IpcFrameHeader::new(IpcCommand::Health, 0, 0, 0);
    let response = super::dispatch::dispatch_command(&header, &[], &mut runtime);
    assert_eq!(response, IpcResponse::Healthy);
}

#[test]
fn dispatch_command_with_resolver_submit_run_inline() {
    let mut runtime = make_runtime();
    let header = IpcFrameHeader::new(IpcCommand::SubmitRunInline, 0, 0, 0);
    let response =
        super::dispatch::dispatch_command_with_resolver(&header, &[], &mut runtime, None);
    match response {
        IpcResponse::PayloadError { .. } | IpcResponse::BadRequest => {}
        other => panic!("expected PayloadError or BadRequest, got {other:?}"),
    }
}

#[test]
fn dispatch_command_with_resolver_cancel_run() {
    let mut runtime = make_runtime();
    let header = IpcFrameHeader::new(IpcCommand::CancelRun, 0, 0, 0);
    let response =
        super::dispatch::dispatch_command_with_resolver(&header, &[], &mut runtime, None);
    assert_eq!(response, IpcResponse::BadRequest);
}

#[test]
fn dispatch_command_with_resolver_inspect_run() {
    let mut runtime = make_runtime();
    let header = IpcFrameHeader::new(IpcCommand::InspectRun, 0, 0, 0);
    let response =
        super::dispatch::dispatch_command_with_resolver(&header, &[], &mut runtime, None);
    assert_eq!(response, IpcResponse::BadRequest);
}

#[test]
fn dispatch_command_with_resolver_list_events() {
    let mut runtime = make_runtime();
    let header = IpcFrameHeader::new(IpcCommand::ListEvents, 0, 0, 0);
    let response =
        super::dispatch::dispatch_command_with_resolver(&header, &[], &mut runtime, None);
    assert_eq!(response, IpcResponse::BadRequest);
}

#[test]
fn dispatch_command_with_resolver_answer_ask() {
    let mut runtime = make_runtime();
    let header = IpcFrameHeader::new(IpcCommand::AnswerAsk, 0, 0, 0);
    let response =
        super::dispatch::dispatch_command_with_resolver(&header, &[], &mut runtime, None);
    assert_eq!(response, IpcResponse::BadRequest);
}

#[test]
fn dispatch_command_with_resolver_complete_action() {
    let mut runtime = make_runtime();
    let header = IpcFrameHeader::new(IpcCommand::CompleteAction, 0, 0, 0);
    let response =
        super::dispatch::dispatch_command_with_resolver(&header, &[], &mut runtime, None);
    assert_eq!(response, IpcResponse::BadRequest);
}

#[test]
fn dispatch_command_with_resolver_fail_action() {
    let mut runtime = make_runtime();
    let header = IpcFrameHeader::new(IpcCommand::FailAction, 0, 0, 0);
    let response =
        super::dispatch::dispatch_command_with_resolver(&header, &[], &mut runtime, None);
    assert_eq!(response, IpcResponse::BadRequest);
}

#[test]
fn dispatch_command_with_resolver_drain_trace() {
    let mut runtime = make_runtime();
    let header = IpcFrameHeader::new(IpcCommand::DrainTrace, 0, 0, 0);
    let response =
        super::dispatch::dispatch_command_with_resolver(&header, &[], &mut runtime, None);
    assert_eq!(response, IpcResponse::BadRequest);
}

#[test]
fn dispatch_unknown_command_returns_bad_request() {
    let mut runtime = make_runtime();
    let header = IpcFrameHeader::new(IpcCommand::UnknownCommand(99), 0, 0, 0);
    let response =
        super::dispatch::dispatch_command_with_resolver(&header, &[], &mut runtime, None);
    assert_eq!(response, IpcResponse::BadRequest);
}

#[test]
fn dispatch_unknown_command_with_resolver_present_returns_bad_request() {
    let mut runtime = make_runtime();
    struct NoopResolver;
    impl WorkflowResolver for NoopResolver {
        fn resolve_workflow(
            &mut self,
            _digest: vb_core::WorkflowDigest,
        ) -> Result<vb_core::workflow::CompiledWorkflow, WorkflowResolutionError> {
            Err(WorkflowResolutionError::NotFound)
        }
    }
    let mut resolver = NoopResolver;
    let header = IpcFrameHeader::new(IpcCommand::UnknownCommand(42), 0, 0, 0);
    let response = super::dispatch::dispatch_command_with_resolver(
        &header,
        &[],
        &mut runtime,
        Some(&mut resolver),
    );
    assert_eq!(
        response,
        IpcResponse::BadRequest,
        "UnknownCommand must return BadRequest even when resolver is present"
    );
}

#[test]
fn dispatch_unknown_command_with_various_ids_returns_bad_request() {
    let mut runtime = make_runtime();
    let unknown_ids = [0u16, 12, 13, 14, 15, 16, 99, u16::MAX];
    for &id in &unknown_ids {
        let header = IpcFrameHeader::new(IpcCommand::UnknownCommand(id), 0, 0, 0);
        let response =
            super::dispatch::dispatch_command_with_resolver(&header, &[], &mut runtime, None);
        assert_eq!(
            response,
            IpcResponse::BadRequest,
            "UnknownCommand({id}) must return BadRequest"
        );
    }
}

#[test]
fn dispatch_submit_run_without_resolver_returns_workflow_resolution_required() {
    let mut runtime = make_runtime();
    let header = IpcFrameHeader::new(IpcCommand::SubmitRun, 0, 0, 0);
    let submit_payload = SubmitRunPayload {
        run_id: vb_core::ids::RunId::new(1),
        workflow: vb_core::ids::WorkflowDigest::from_bytes([0xAB; 32]),
        input: Vec::new(),
    };
    let encoded = postcard::to_allocvec(&IpcPayload::SubmitRun(submit_payload))
        .expect("test payload must encode");
    let response =
        super::dispatch::dispatch_command_with_resolver(&header, &encoded, &mut runtime, None);
    assert_eq!(response, IpcResponse::WorkflowResolutionRequired);
}

#[test]
fn dispatch_shutdown_returns_shutting_down() {
    let mut runtime = make_runtime();
    let header = IpcFrameHeader::new(IpcCommand::Shutdown, 0, 0, 0);
    let response =
        super::dispatch::dispatch_command_with_resolver(&header, &[], &mut runtime, None);
    assert_eq!(response, IpcResponse::ShuttingDown);
}
