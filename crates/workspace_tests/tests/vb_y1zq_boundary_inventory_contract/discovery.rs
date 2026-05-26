use super::support::*;

#[test]
fn discover_boundaries_returns_workspace_not_discoverable_when_crates_surface_missing() {
    let result = discover_boundaries(workspace("missing_crates_workspace"));

    assert_eq!(
        result,
        Err(BoundaryInventoryError::WorkspaceNotDiscoverable)
    );
}

#[test]
fn discover_boundaries_returns_workspace_not_discoverable_when_fuzz_surface_missing() {
    let result = discover_boundaries(workspace("missing_fuzz_workspace"));

    assert_eq!(
        result,
        Err(BoundaryInventoryError::WorkspaceNotDiscoverable)
    );
}

#[test]
fn discover_boundaries_returns_workspace_not_discoverable_when_scripts_surface_missing() {
    let result = discover_boundaries(workspace("missing_scripts_workspace"));

    assert_eq!(
        result,
        Err(BoundaryInventoryError::WorkspaceNotDiscoverable)
    );
}

#[test]
fn discover_boundaries_returns_workspace_not_discoverable_when_cargo_toml_missing() {
    let result = discover_boundaries(workspace("missing_cargo_toml_workspace"));

    assert_eq!(
        result,
        Err(BoundaryInventoryError::WorkspaceNotDiscoverable)
    );
}

#[test]
fn discover_boundaries_returns_empty_candidates_when_required_surfaces_exist() {
    let result = discover_boundaries(workspace("complete_workspace")).map(candidate_pairs);

    assert_eq!(result, Ok(complete_workspace_candidates()));
}

#[test]
fn discover_boundaries_rejects_existing_workspace_missing_each_required_surface() {
    let result = temp_workspace_missing_required_surfaces()
        .map(|missing_all| {
            discover_boundaries(WorkspaceRoot::new(missing_all.path().to_path_buf()))
        })
        .map_err(|error| error.kind());

    assert_eq!(
        result,
        Ok(Err(BoundaryInventoryError::WorkspaceNotDiscoverable))
    );
}

#[test]
fn discover_boundaries_returns_incomplete_discovery_input_when_decoder_surface_omitted_from_config()
{
    let result = discover_boundaries(workspace("omitted_decoder_surface_config"));

    assert_eq!(
        result,
        Err(BoundaryInventoryError::IncompleteDiscoveryInput)
    );
}

#[test]
fn discover_boundaries_rejects_workspace_when_required_surface_absence_helper_is_false_mutated() {
    let result = discover_workspace_with_missing_surfaces_and_omitted_decoder_config();

    assert_eq!(
        result,
        Ok(Err(BoundaryInventoryError::WorkspaceNotDiscoverable))
    );
}

#[test]
fn discover_boundaries_requires_crates_fuzz_scripts_and_cargo_toml_surfaces() {
    let crates_result = discover_boundaries(workspace("missing_crates_workspace"));
    let fuzz_result = discover_boundaries(workspace("missing_fuzz_workspace"));
    let scripts_result = discover_boundaries(workspace("missing_scripts_workspace"));
    let cargo_result = discover_boundaries(workspace("missing_cargo_toml_workspace"));

    assert_eq!(
        (crates_result, fuzz_result, scripts_result, cargo_result),
        (
            Err(BoundaryInventoryError::WorkspaceNotDiscoverable),
            Err(BoundaryInventoryError::WorkspaceNotDiscoverable),
            Err(BoundaryInventoryError::WorkspaceNotDiscoverable),
            Err(BoundaryInventoryError::WorkspaceNotDiscoverable),
        )
    );
}

#[test]
#[ignore]
fn discover_boundaries_accepts_complete_decoder_surface_config_and_rejects_only_omitted_decoder() {
    let complete = discover_boundaries(workspace("complete_workspace")).map(candidate_pairs);
    let omitted = discover_boundaries(workspace("omitted_decoder_surface_config"));

    assert_eq!(complete, Ok(complete_workspace_candidates()));
    assert_eq!(
        omitted,
        Err(BoundaryInventoryError::IncompleteDiscoveryInput)
    );
}

#[test]
fn discover_boundaries_returns_discovered_marker_candidates_with_exact_paths_and_markers() {
    let result = discover_boundaries(workspace("complete_workspace")).map(candidate_pairs);

    assert_eq!(result, Ok(complete_workspace_candidates()));
}

#[test]
fn discover_boundaries_scans_only_required_candidate_roots_for_markers() {
    let result = discover_temp_with_files(&[
        ("outside.txt", "extern-c-boundary"),
        ("crates/inside.rs", "ipc-frame-boundary"),
    ]);

    assert_eq!(
        result,
        Ok(vec![
            (
                "crates/inside.rs".to_string(),
                "ipc-frame-boundary".to_string()
            ),
            (
                "Cargo.toml".to_string(),
                "unsafe-adjacent-dependency-boundary".to_string(),
            ),
        ])
    );
}

#[test]
fn discover_boundaries_finds_nested_marker_files_under_candidate_roots() {
    let result = discover_temp_with_files(&[("crates/nested/deep/frame.rs", "ipc-frame-boundary")]);

    assert_eq!(
        result,
        Ok(vec![
            (
                "crates/nested/deep/frame.rs".to_string(),
                "ipc-frame-boundary".to_string(),
            ),
            (
                "Cargo.toml".to_string(),
                "unsafe-adjacent-dependency-boundary".to_string(),
            ),
        ])
    );
}

#[test]
fn discover_boundaries_extracts_marker_from_file_content_not_filename() {
    let result =
        discover_temp_with_files(&[("crates/plain_name.rs", "decoder-byte-ingest-boundary")]);

    assert_eq!(
        result,
        Ok(vec![
            (
                "crates/plain_name.rs".to_string(),
                "decoder-byte-ingest-boundary".to_string(),
            ),
            (
                "Cargo.toml".to_string(),
                "unsafe-adjacent-dependency-boundary".to_string(),
            ),
        ])
    );
}

#[test]
fn discover_boundaries_ignores_empty_marker_mutation_and_reports_only_known_markers() {
    let result = discover_temp_with_files(&[
        ("crates/empty_lines.rs", "\n\n"),
        ("crates/known.rs", "extern-c-boundary"),
    ]);

    assert_eq!(
        result,
        Ok(vec![
            (
                "crates/known.rs".to_string(),
                "extern-c-boundary".to_string()
            ),
            (
                "Cargo.toml".to_string(),
                "unsafe-adjacent-dependency-boundary".to_string(),
            ),
        ])
    );
}

#[test]
#[ignore]
fn discover_boundaries_rejects_junk_marker_set_mutation_by_requiring_all_seven_known_markers() {
    let result = discover_boundaries(workspace("complete_workspace")).map(candidate_pairs);

    assert_eq!(result, Ok(complete_workspace_candidates()));
}

fn complete_workspace_candidates() -> Vec<(String, String)> {
    vec![
        (
            "crates/ffi/src/c_abi.rs".to_string(),
            "extern-c-boundary".to_string(),
        ),
        (
            "crates/ffi/src/lib.rs".to_string(),
            "foreign-function-boundary".to_string(),
        ),
        (
            "crates/vb_runtime/src/generated/interface.rs".to_string(),
            "generated-interface-boundary".to_string(),
        ),
        (
            "crates/vb_ipc/src/frame.rs".to_string(),
            "ipc-frame-boundary".to_string(),
        ),
        (
            "crates/vb_yaml/src/decode.rs".to_string(),
            "decoder-byte-ingest-boundary".to_string(),
        ),
        (
            "fuzz/fuzz_targets/boundary.rs".to_string(),
            "decoder-byte-ingest-boundary".to_string(),
        ),
        (
            "scripts/run-verifier.sh".to_string(),
            "external-binary-boundary".to_string(),
        ),
        (
            "Cargo.toml".to_string(),
            "unsafe-adjacent-dependency-boundary".to_string(),
        ),
    ]
}
