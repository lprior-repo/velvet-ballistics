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
    clippy::enum_variant_names,
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

use std::sync::Arc;

use vb_core::{ActionId, Capability, CapabilitySet, RunId, RuntimePolicy, WorkflowDigest};
use vb_runtime::admission::{
    AcceptedArtifactStore, AdmissionError, ArtifactEnvelopeError, REQUIRED_GATE_COUNT,
    StorageArtifactStore, admit_artifact_run,
};
#[cfg(test)]
use vb_storage::__put_compiled_ir_for_testing as put_compiled_ir;
use vb_storage::admission::{AcceptedArtifact, VerificationProof};
use vb_storage::{CompiledIrRecord, EventSeq, FjallJournal, JournalError};

fn digest(byte: u8) -> WorkflowDigest {
    WorkflowDigest::from_bytes([byte; 32])
}

struct ReturningAcceptedArtifactStore {
    artifact: AcceptedArtifact,
}

impl AcceptedArtifactStore for ReturningAcceptedArtifactStore {
    fn load_accepted_artifact(
        &self,
        _artifact_digest: WorkflowDigest,
    ) -> Result<AcceptedArtifact, ArtifactEnvelopeError> {
        Ok(self.artifact.clone())
    }
}

fn required_capability() -> Capability {
    Capability::new("net.fetch".into(), ActionId::new(7))
}

fn granted_capabilities(required: Capability) -> CapabilitySet {
    CapabilitySet::from_grants(Box::new([required]))
}

fn accepted_artifact(proof_digest: WorkflowDigest) -> Result<AcceptedArtifact, String> {
    let workflow = compile_storage_workflow()?;
    let mut parts = workflow.to_parts();
    parts.digest = WorkflowDigest::from_bytes([0u8; 32]);
    let ir = postcard::to_allocvec(&parts).map_err(|error| error.to_string())?;
    let artifact_digest = workflow.digest();
    let policy_digest = vb_storage::admission::compute_policy_digest(&workflow)
        .map_err(|error| error.to_string())?;
    Ok(AcceptedArtifact {
        digest: artifact_digest,
        source_digest: artifact_digest,
        policy_digest,
        ir,
        verification: VerificationProof {
            digest: proof_digest,
            gate_count: REQUIRED_GATE_COUNT,
            durable: true,
            bounded_claimed: true,
            taint_safe_claimed: true,
            retry_safe_claimed: true,
            idempotency_verified_claimed: true,
            replayable_claimed: true,
            idempotency_keyed: Box::new([]),
            idempotency_attested: Box::new([]),
            warnings: Vec::new(),
        },
        accepted_at_seq: EventSeq::new(42),
        required_capabilities: Box::new([required_capability()]),
    })
}

fn compile_storage_workflow() -> Result<vb_core::CompiledWorkflow, String> {
    let yaml = br#"version: velvet-ballistics/v1
name: proof_admission_bdd
when:
  manual: {}
steps:
  - id: make
    set:
      output: answer
      value: "42"
  - id: done
    finish:
      result: answer
"#;
    let workflow = vb_compile::compile_workflow(yaml).map_err(|errors| errors.to_string())?;
    let mut parts = workflow.to_parts();
    parts.digest = WorkflowDigest::from_bytes([0u8; 32]);
    let ir = postcard::to_allocvec(&parts).map_err(|error| error.to_string())?;
    parts.digest = WorkflowDigest::from_bytes(blake3::hash(&ir).into());
    vb_core::CompiledWorkflow::try_from_parts(parts).map_err(|error| error.to_string())
}

fn journal_error_label<T>(result: &Result<T, JournalError>) -> String {
    match result {
        Ok(_) => String::from("Ok"),
        Err(JournalError::ArtifactChecksumMismatch) => String::from("ArtifactChecksumMismatch"),
        Err(other) => format!("Other({other:?})"),
    }
}

fn persist_artifact(journal: &FjallJournal, artifact: &AcceptedArtifact) -> Result<(), String> {
    persist_artifact_as(journal, artifact.digest, artifact)
}

fn persist_artifact_as(
    journal: &FjallJournal,
    record_digest: WorkflowDigest,
    artifact: &AcceptedArtifact,
) -> Result<(), String> {
    let ir = postcard::to_allocvec(artifact).map_err(|error| error.to_string())?;
    put_compiled_ir(
        journal,
        &CompiledIrRecord {
            digest: record_digest,
            ir,
            metadata_hash: None,
        },
    )
    .map_err(|error| error.to_string())
}

#[test]
fn given_matching_proof_digest_when_strict_admission_runs_then_artifact_is_admitted()
-> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|error| error.to_string())?;
    let journal = FjallJournal::open(temp.path(), None).map_err(|error| error.to_string())?;
    let mut artifact = accepted_artifact(digest(0xA1))?;
    artifact.verification.digest = artifact.digest;
    let requested = artifact.digest;
    persist_artifact(&journal, &artifact)?;
    let store = StorageArtifactStore::new(Arc::new(journal));
    let run = RunId::new(9001);
    let caps = granted_capabilities(required_capability());

    let admission = admit_artifact_run(&store, RuntimePolicy::Strict, run, requested, caps.clone())
        .map_err(|error| error.to_string())?;

    assert_eq!(admission.artifact_digest(), requested);
    assert_eq!(admission.policy(), RuntimePolicy::Strict);
    assert_eq!(admission.run_id(), run);
    assert_eq!(admission.granted_capabilities(), &caps);
    Ok(())
}

#[test]
fn given_mismatched_proof_digest_when_strict_admission_runs_then_digest_mismatch_denies()
-> Result<(), String> {
    let found = digest(0xB2);
    let artifact = accepted_artifact(found)?;
    let requested = artifact.digest;
    let store = ReturningAcceptedArtifactStore { artifact };

    let result = admit_artifact_run(
        &store,
        RuntimePolicy::Strict,
        RunId::new(9002),
        requested,
        granted_capabilities(required_capability()),
    );

    assert_eq!(
        result,
        Err(AdmissionError::ArtifactDigestMismatch { requested, found })
    );
    Ok(())
}

#[test]
fn given_storage_record_with_mismatched_artifact_digest_when_stored_then_storage_denies()
-> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|error| error.to_string())?;
    let journal = FjallJournal::open(temp.path(), None).map_err(|error| error.to_string())?;
    let mut artifact = accepted_artifact(digest(0xC2))?;
    artifact.verification.digest = artifact.digest;
    let requested = digest(0xC1);
    let ir = postcard::to_allocvec(&artifact).map_err(|error| error.to_string())?;
    let result = put_compiled_ir(
        &journal,
        &CompiledIrRecord {
            digest: requested,
            ir,
            metadata_hash: None,
        },
    );

    assert_eq!(
        journal_error_label(&result),
        "ArtifactChecksumMismatch",
        "storage must reject record/artifact digest mismatch before runtime admission"
    );
    Ok(())
}
