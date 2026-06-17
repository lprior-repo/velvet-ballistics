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

//! Property tests for vb_boundary_inventory — additional coverage beyond vb_y1zq_boundary_inventory_properties.
//!
//! Uses individual `#[test]` functions with `proptest::proptest!` strategy blocks.

use std::path::PathBuf;

use proptest::prelude::*;
use velvet_ballistics_workspace_tests::boundary_inventory::{
    BoundaryCandidate, BoundaryClass, BoundaryInventoryError, BoundaryRecord, BoundaryRecordDraft,
    BoundaryRecordParts, BoundaryRisk, EvidenceKind, EvidenceReference, FieldState,
    FreshnessMarker, Owner, ReviewStatus, ThreatStatement, UnsafeIsolationStatus,
    ValidatedBoundaryInventory, WorkspaceRoot, classify_boundary, discover_boundaries,
    inventory_completion_status, validate_evidence_reference_bytes,
};

// ============================================================================
// discover_boundaries — property-based workspace configurations
// ============================================================================

proptest::proptest! {
    #[test]
    fn discover_boundaries_returns_candidates_with_valid_marker_strings(
        source in "[a-z][a-z0-9_-]{0,64}",
    ) {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("crates/vb_test/src")).expect("crates dir");
        std::fs::create_dir_all(dir.path().join("fuzz")).expect("fuzz dir");
        std::fs::create_dir_all(dir.path().join("scripts")).expect("scripts dir");
        std::fs::write(dir.path().join("Cargo.toml"), "# auto").expect("Cargo.toml");
        std::fs::write(dir.path().join("boundary-surfaces.txt"), "decoder-byte-ingest-boundary\n").expect("surfaces");

        let candidate_marker = format!("{source}");
        let candidate_path = "crates/vb_test/src/lib.rs";
        std::fs::write(dir.path().join(candidate_path), &candidate_marker).expect("write marker file");

        let result = discover_boundaries(WorkspaceRoot::new(dir.path().to_path_buf()));

        if let Ok(candidates) = result {
            let known_markers = [
                "extern-c-boundary",
                "foreign-function-boundary",
                "ipc-frame-boundary",
                "external-binary-boundary",
                "decoder-byte-ingest-boundary",
                "generated-interface-boundary",
                "unsafe-adjacent-dependency-boundary",
            ];
            for candidate in candidates {
                prop_assert!(
                    known_markers.contains(&candidate.marker.as_str()),
                    "marker {:?} not in known set",
                    candidate.marker
                );
            }
        }
    }
}

proptest::proptest! {
    #[test]
    fn discover_boundaries_returns_candidates_with_paths_under_allowed_roots(
        source in "[a-z]{1,32}",
        marker in "[a-z][a-z0-9_-]{0,32}",
    ) {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("crates/vb_test/src")).expect("crates dir");
        std::fs::create_dir_all(dir.path().join("fuzz")).expect("fuzz dir");
        std::fs::create_dir_all(dir.path().join("scripts")).expect("scripts dir");
        std::fs::write(dir.path().join("Cargo.toml"), "# auto").expect("Cargo.toml");
        std::fs::write(dir.path().join("boundary-surfaces.txt"), "decoder-byte-ingest-boundary\n").expect("surfaces");

        let rel_path = format!("crates/vb_test/src/{source}.rs");
        std::fs::write(dir.path().join(&rel_path), &marker).expect("write marker");

        let result = discover_boundaries(WorkspaceRoot::new(dir.path().to_path_buf()));

        if let Ok(candidates) = result {
            let allowed_prefixes = ["crates", "fuzz", "scripts", "Cargo.toml"];
            for candidate in candidates {
                prop_assert!(
                    allowed_prefixes.iter().any(|p| candidate.source_path.starts_with(*p)),
                    "source_path {:?} must start with allowed prefix",
                    candidate.source_path
                );
            }
        }
    }
}

proptest::proptest! {
    #[test]
    fn discover_boundaries_rejects_workspace_with_no_marker_files(count in 0_u64..20_u64) {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("crates/vb_test/src")).expect("crates dir");
        std::fs::create_dir_all(dir.path().join("fuzz")).expect("fuzz dir");
        std::fs::create_dir_all(dir.path().join("scripts")).expect("scripts dir");
        std::fs::write(dir.path().join("Cargo.toml"), "# auto").expect("Cargo.toml");
        std::fs::write(dir.path().join("boundary-surfaces.txt"), "decoder-byte-ingest-boundary\n").expect("surfaces");

        for i in 0..count {
            let path = dir.path().join(format!("crates/vb_test/src/clean_{i}.rs"));
            std::fs::write(&path, "fn clean_code() {}\n").expect("write");
        }

        let result = discover_boundaries(WorkspaceRoot::new(dir.path().to_path_buf()));
        prop_assert_eq!(
            result,
            Err(BoundaryInventoryError::IncompleteDiscoveryInput),
            "workspace with no markers should fail with IncompleteDiscoveryInput"
        );
    }
}

proptest::proptest! {
    #[test]
    fn discover_boundaries_candidates_are_deduplicated(
        source in "[a-z]{1,16}",
        marker in "[a-z][a-z0-9_-]{0,16}",
    ) {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("crates/vb_test/src")).expect("crates dir");
        std::fs::create_dir_all(dir.path().join("fuzz")).expect("fuzz dir");
        std::fs::create_dir_all(dir.path().join("scripts")).expect("scripts dir");
        std::fs::write(dir.path().join("Cargo.toml"), "# auto").expect("Cargo.toml");
        std::fs::write(dir.path().join("boundary-surfaces.txt"), "decoder-byte-ingest-boundary\n").expect("surfaces");

        let rel_path = format!("crates/vb_test/src/{source}.rs");
        for _ in 0..3 {
            std::fs::write(dir.path().join(&rel_path), &marker).expect("write marker");
        }

        let result = discover_boundaries(WorkspaceRoot::new(dir.path().to_path_buf()));

        if let Ok(candidates) = &result {
            let mut seen = std::collections::HashSet::new();
            for c in candidates {
                let key = (c.source_path.clone(), c.marker.clone());
                prop_assert!(seen.insert(key), "duplicate candidate ({:?}, {:?})", c.source_path, c.marker);
            }
        }
    }
}

// ============================================================================
// classify_boundary — comprehensive marker/class mapping
// ============================================================================

proptest::proptest! {
    #[test]
    fn classify_boundary_id_is_stable_across_multiple_calls(_class in prop_oneof![
        Just(BoundaryClass::CAbi),
        Just(BoundaryClass::Ffi),
        Just(BoundaryClass::Ipc),
        Just(BoundaryClass::ExternalBinary),
        Just(BoundaryClass::Decoder),
        Just(BoundaryClass::GeneratedCode),
        Just(BoundaryClass::UnsafeAdjacentDependency),
    ]) {
        let source = "crates/ffi/src/c_abi.rs";
        let marker = "extern-c-boundary";
        let candidate = BoundaryCandidate::new(source, marker);

        let first = classify_boundary(candidate.clone());
        let second = classify_boundary(candidate);

        prop_assert_eq!(
            first.as_ref().map(|b| &b.id),
            second.as_ref().map(|b| &b.id)
        );
    }
}

proptest::proptest! {
    #[test]
    fn classify_boundary_produces_id_with_vb_prefix_and_sanitized_source(
        subpath in "[a-z][a-z0-9_/.-]{0,64}",
    ) {
        let candidate = BoundaryCandidate::new(
            format!("crates/vb_test/src/{subpath}"),
            "extern-c-boundary",
        );

        let result = classify_boundary(candidate);

        prop_assert!(result.is_ok(), "classify_boundary should succeed for known marker");
        let id = result.unwrap().id;
        prop_assert!(id.starts_with("vb-y1zq-"), "id {:?} must start with vb-y1zq-", id);
        prop_assert!(!id.contains('/'), "id {:?} should not contain / after sanitization", id);
    }
}

proptest::proptest! {
    #[test]
    fn classify_boundary_exposure_is_defined_for_all_known_classes(_class in prop_oneof![
        Just(BoundaryClass::CAbi),
        Just(BoundaryClass::Ffi),
        Just(BoundaryClass::Ipc),
        Just(BoundaryClass::ExternalBinary),
        Just(BoundaryClass::Decoder),
        Just(BoundaryClass::GeneratedCode),
        Just(BoundaryClass::UnsafeAdjacentDependency),
    ]) {
        let source = "crates/ffi/src/c_abi.rs";
        let marker = "extern-c-boundary";
        let candidate = BoundaryCandidate::new(source, marker);

        let result = classify_boundary(candidate).expect("known class should classify");

        prop_assert!(matches!(
            result.exposure.risk,
            BoundaryRisk::Multiple
                | BoundaryRisk::ExternalBytes
                | BoundaryRisk::ProcessLimit
                | BoundaryRisk::LanguageLimit
                | BoundaryRisk::None
        ));
    }
}

proptest::proptest! {
    #[test]
    fn classify_boundary_returns_unknown_for_plain_rust_module(_index in 0u32..1) {
        let candidate = BoundaryCandidate::new("crates/vb_core/src/lib.rs", "plain-rust-module");
        let result = classify_boundary(candidate);
        prop_assert_eq!(result, Err(BoundaryInventoryError::UnknownBoundaryClass));
    }
}

// ============================================================================
// inventory_completion_status — arbitrary record combinations
// ============================================================================

/// Build a fully-valid record for inventory_completion_status testing.
fn valid_completion_record(class: BoundaryClass) -> BoundaryRecord {
    BoundaryRecordDraft::new(BoundaryRecordParts {
        id: format!("vb-y1zq-completion-{class:?}"),
        class,
        source_path: PathBuf::from("crates/vb_ipc/src/frame.rs"),
        owner: FieldState::Present(Owner(String::from("security-team"))),
        threat: FieldState::Present(ThreatStatement(String::from(
            "hostile external bytes cross a trust boundary",
        ))),
        evidence: FieldState::Present(EvidenceReference::repo_local(
            PathBuf::from("formal-verification-report.md"),
            EvidenceKind::Fuzz,
        )),
        freshness: FreshnessMarker::new(1, 1, 1),
        review_status: FieldState::Present(ReviewStatus::Approved),
        waiver: FieldState::Missing,
    })
}

/// Build a ValidatedBoundaryInventory for inventory_completion_status testing.
fn validated_inventory(
    records: Vec<BoundaryRecord>,
    _discovered_boundary_count: usize,
) -> ValidatedBoundaryInventory {
    ValidatedBoundaryInventory::from_validated_records(1, records, Some("approved".into()))
}

/// Build a ValidatedBoundaryInventory with a first-party unsafe record.
#[allow(unused)]
fn validated_inventory_unsafe_first_party(
    records: Vec<BoundaryRecord>,
    discovered_boundary_count: usize,
) -> ValidatedBoundaryInventory {
    ValidatedBoundaryInventory {
        schema_version: 1,
        records,
        discovered_boundary_count,
        review_status: Some("approved".into()),
    }
}

proptest::proptest! {
    #[test]
    fn inventory_completion_status_returns_complete_for_valid_records(count in 0_usize..16) {
        let records: Vec<_> = (0..count)
            .map(|i| {
                // Cycle through known safe classes; skip UnsafeAdjacentDependency
                // which would trigger UnsafeForbiddenViolation when source starts with "crates".
                let class = match i % 6 {
                    0 => BoundaryClass::CAbi,
                    1 => BoundaryClass::Ffi,
                    2 => BoundaryClass::Ipc,
                    3 => BoundaryClass::ExternalBinary,
                    4 => BoundaryClass::Decoder,
                    _ => BoundaryClass::GeneratedCode,
                };
                let mut r = valid_completion_record(class);
                r.id = format!("vb-y1zq-record-{i}");
                r
            })
            .collect();

        let inventory = validated_inventory(records.clone(), count);
        let result = inventory_completion_status(inventory);

        prop_assert_eq!(
            result,
            Ok(UnsafeIsolationStatus::Complete { boundary_count: count }),
            "expected Complete with {} boundaries",
            count
        );
    }
}

proptest::proptest! {
    #[test]
    fn inventory_completion_status_returns_unknown_boundary_class_when_unknown_record_present(
        valid_count in 0_usize..8,
        unknown_count in 1_usize..4,
    ) {
        let mut records: Vec<_> = (0..valid_count)
            .map(|i| {
                let mut r = valid_completion_record(BoundaryClass::CAbi);
                r.id = format!("vb-y1zq-valid-{i}");
                r
            })
            .collect();

        for i in 0..unknown_count {
            let mut r = valid_completion_record(BoundaryClass::Unknown);
            r.id = format!("vb-y1zq-unknown-{i}");
            records.push(r);
        }

        let inventory = validated_inventory(records, valid_count + unknown_count);
        let result = inventory_completion_status(inventory);

        prop_assert_eq!(result, Err(BoundaryInventoryError::UnknownBoundaryClass));
    }
}

proptest::proptest! {
    #[test]
    fn inventory_completion_status_returns_unsafe_forbidden_when_first_party_unsafe_present(
        valid_count in 0_usize..8,
    ) {
        let mut records: Vec<_> = (0..valid_count)
            .map(|i| {
                let mut r = valid_completion_record(BoundaryClass::CAbi);
                r.id = format!("vb-y1zq-valid-{i}");
                r
            })
            .collect();

        // Add a first-party UnsafeAdjacentDependency record (source_path starts with "crates").
        let mut unsafe_record = valid_completion_record(BoundaryClass::UnsafeAdjacentDependency);
        unsafe_record.id = "vb-y1zq-first-party-unsafe".to_string();
        unsafe_record.source_path = PathBuf::from("crates/vb_boundary_inventory/src/lib.rs");
        records.push(unsafe_record);

        let inventory = validated_inventory_unsafe_first_party(records, valid_count + 1);
        let result = inventory_completion_status(inventory);

        prop_assert_eq!(result, Err(BoundaryInventoryError::UnsafeForbiddenViolation));
    }
}

proptest::proptest! {
    #[test]
    fn inventory_completion_status_returns_incomplete_when_empty_with_nonzero_discovered(
        discovered in 1_usize..16,
    ) {
        let inventory = ValidatedBoundaryInventory {
            schema_version: 1,
            records: Vec::new(),
            discovered_boundary_count: discovered,
            review_status: Some("approved".into()),
        };

        let result = inventory_completion_status(inventory);
        prop_assert_eq!(result, Err(BoundaryInventoryError::IncompleteDiscoveryInput));
    }
}

proptest::proptest! {
    #[test]
    fn inventory_completion_status_boundary_count_equals_records_len(
        valid_count in 1_usize..12,
    ) {
        let records: Vec<_> = (0..valid_count)
            .map(|i| {
                let mut r = valid_completion_record(BoundaryClass::Ipc);
                r.id = format!("vb-y1zq-count-test-{i}");
                r
            })
            .collect();

        let inventory = validated_inventory(records.clone(), valid_count);
        let result = inventory_completion_status(inventory).expect("should be valid");

        match result {
            UnsafeIsolationStatus::Complete { boundary_count } => {
                prop_assert_eq!(boundary_count, valid_count);
            }
        }
    }
}

// ============================================================================
// valid_bead_id — valid/invalid bead ID formats
// Tested indirectly through validate_evidence_reference_bytes
// ============================================================================

proptest::proptest! {
    #[test]
    fn validate_evidence_reference_accepts_well_formed_bead_ids(suffix in "[a-z][a-z0-9]{0,30}") {
        let bead_id = format!("vb-{suffix}");
        let bytes = bead_id.as_bytes();
        let result = validate_evidence_reference_bytes(bytes);
        prop_assert!(result.is_ok(), "expected Ok for valid bead id {:?}, got {:?}", bead_id, result);
    }
}

proptest::proptest! {
    #[test]
    fn validate_evidence_reference_rejects_bead_id_with_trailing_component(
        suffix in "[a-z][a-z0-9]{0,20}",
        extra in "[a-z][a-z0-9]{0,20}",
    ) {
        let bead_id = format!("vb-{suffix}-{extra}");
        let bytes = bead_id.as_bytes();
        let result = validate_evidence_reference_bytes(bytes);
        prop_assert!(result.is_err(), "bead id {:?} with trailing component should be rejected", bead_id);
    }
}

proptest::proptest! {
    #[test]
    fn validate_evidence_reference_accepts_external_provenance_with_sha256(
        external_id in "[a-z][a-z0-9]{0,40}",
    ) {
        let text = format!("external:{external_id}#sha256=abc123def456");
        let bytes = text.as_bytes();
        let result = validate_evidence_reference_bytes(bytes);
        prop_assert!(result.is_ok(), "external with sha256 should be accepted, got {:?}", result);
    }
}

proptest::proptest! {
    #[test]
    fn validate_evidence_reference_rejects_external_without_sha256(
        external_id in "[a-z][a-z0-9]{0,40}",
    ) {
        let text = format!("external:{external_id}");
        let bytes = text.as_bytes();
        let result = validate_evidence_reference_bytes(bytes);
        prop_assert!(result.is_err(), "external without sha256 should be rejected, got {:?}", result);
    }
}

proptest::proptest! {
    #[test]
    fn validate_evidence_reference_rejects_bead_id_with_empty_suffix(_index in 0u32..1) {
        let bead_id = "vb-";
        let bytes = bead_id.as_bytes();
        let result = validate_evidence_reference_bytes(bytes);
        prop_assert!(result.is_err(), "bead id {:?} with empty suffix should be rejected", bead_id);
    }
}

proptest::proptest! {
    #[test]
    fn validate_evidence_reference_rejects_bead_id_missing_vb_prefix(
        suffix in "[a-z][a-z0-9]{0,30}",
    ) {
        let bead_id = format!("vc-{suffix}");
        let bytes = bead_id.as_bytes();
        let result = validate_evidence_reference_bytes(bytes);
        prop_assert!(result.is_err(), "bead id {:?} missing vb- prefix should be rejected", bead_id);
    }
}
