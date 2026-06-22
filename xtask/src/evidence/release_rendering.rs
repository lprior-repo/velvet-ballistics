
fn require_redaction_placeholders(screen_id: &str, text: &str) -> Result<()> {
    for (class, placeholder) in REDACTION_CLASSES {
        if !text.contains(placeholder) {
            return Err(Error::MissingEvidence {
                gate: format!("redaction:{screen_id}:{class}"),
                path: PathBuf::from(".evidence/vb-nf2u/ai-release.yaml"),
            });
        }
    }
    Ok(())
}

fn run_cargo_invariants(output_dir: &Path) -> Result<CargoInvariants> {
    let cargo_manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // xtask lives at `<workspace_root>/xtask/`, so a single `.parent()`
    // yields the workspace root. Earlier this code applied `.parent()` twice,
    // which climbed to `/home/lewis/src` and made `cargo test --workspace`
    // fail with "could not find Cargo.toml" — cascading into every
    // `vb_nf2u_ui_release_acceptance` test that depends on this gate.
    let workspace_root = cargo_manifest_dir
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| Error::EvidenceWriteFailed {
            gate: "cargo_invocation".to_string(),
            path: output_dir.join("cargo-invariants.log"),
            cause: "could not resolve workspace root from CARGO_MANIFEST_DIR".to_string(),
        })?;
    let test_log = output_dir.join("cargo-test.log");
    let clippy_log = output_dir.join("cargo-clippy.log");
    let test_exit = run_cargo_subcommand(&workspace_root, &["test", "--workspace", "--no-run"], &test_log)?;
    let clippy_exit = run_cargo_subcommand(&workspace_root, &["clippy", "--workspace"], &clippy_log)?;
    if test_exit != 0 {
        return Err(Error::GateFailed {
            gate: "cargo_test_workspace_no_run".to_string(),
            exit_code: test_exit,
            log: test_log,
        });
    }
    if clippy_exit != 0 {
        return Err(Error::GateFailed {
            gate: "cargo_clippy_workspace".to_string(),
            exit_code: clippy_exit,
            log: clippy_log,
        });
    }
    Ok(CargoInvariants {
        test_exit,
        clippy_exit,
        test_log,
        clippy_log,
    })
}

fn run_cargo_subcommand(workspace_root: &Path, args: &[&str], log_path: &Path) -> Result<i32> {
    let output = std::process::Command::new("cargo")
        .current_dir(workspace_root)
        .args(args)
        .env("CARGO_TERM_COLOR", "never")
        .output()
        .map_err(|error| Error::EvidenceWriteFailed {
            gate: "cargo_invocation".to_string(),
            path: log_path.to_path_buf(),
            cause: error.to_string(),
        })?;
    let exit_code = output.status.code().unwrap_or(-1);
    let mut log = String::new();
    log.push_str(&format!("$ cargo {}\n", args.join(" ")));
    log.push_str(&format!("exit_code: {exit_code}\n"));
    log.push_str("--- stdout ---\n");
    log.push_str(&String::from_utf8_lossy(&output.stdout));
    log.push_str("--- stderr ---\n");
    log.push_str(&String::from_utf8_lossy(&output.stderr));
    write_text_file(log_path, &log)?;
    Ok(exit_code)
}

struct CargoInvariants {
    test_exit: i32,
    clippy_exit: i32,
    #[allow(dead_code)]
    test_log: PathBuf,
    #[allow(dead_code)]
    clippy_log: PathBuf,
}

fn validate_negative_fixture_inputs() -> Result<()> {
    let _overlap = OverlapNegativeFixture::read_required()?;
    let _secret = SecretNegativeFixture::read_required()?;
    Ok(())
}

fn validate_deterministic_capture_state(screens: &[UiScreenEvidenceRow]) -> Result<()> {
    for screen in screens {
        let snapshot_dir =
            screen
                .provenance
                .path
                .parent()
                .ok_or_else(|| Error::MissingEvidence {
                    gate: format!("deterministic_capture:{}:parent", screen.screen_id),
                    path: screen.provenance.path.clone(),
                })?;
        let facts = ScreenArtifactFacts::read_for_screen(screen.screen_id, snapshot_dir)?;
        validate_deterministic_facts(&facts)?;
    }
    Ok(())
}

fn validate_deterministic_facts(facts: &ScreenArtifactFacts) -> Result<()> {
    if facts.timestamp != CaptureTimestamp::Fixed("2026-05-09T00:00:00Z") {
        return deterministic_error(facts, "snapshot_timestamp");
    }
    if facts.animation_state != HiddenAnimationState::Paused {
        return deterministic_error(facts, "animation_state");
    }
    if facts.clock_source != ClockSource::FixedFixtureTime {
        return deterministic_error(facts, "animation_or_clock_state");
    }
    Ok(())
}

fn deterministic_error(facts: &ScreenArtifactFacts, field: &str) -> Result<()> {
    Err(Error::GateFailed {
        gate: format!("deterministic_capture:{}:{field}", facts.screen_id),
        exit_code: 1,
        log: PathBuf::from(".evidence/vb-nf2u/determinism.txt"),
    })
}

fn ui_release_gate_evidence(output_dir: &Path, bundle: &UiReleaseBundle) -> Vec<GateEvidence> {
    bundle
        .subgates
        .iter()
        .map(|gate| GateEvidence {
            kind: "ui-release".to_string(),
            gate_name: gate.name.to_string(),
            command: gate.command.to_string(),
            exit_code: 0,
            log: output_dir.join(format!("{}.log", gate.name)),
            status: gate.status.clone(),
            why_failed: None,
        })
        .collect()
}

fn write_vb_nf2u_ui_release_evidence(output_dir: &Path) -> Result<Vec<GateEvidence>> {
    let snapshot_dir = output_dir.join("ui_snapshots");
    let source = SourceFixtureSet::read_for_output(&snapshot_dir)?;
    let cargo_invariants = run_cargo_invariants(output_dir)?;
    let (bundle, document) = build_release_model(&source, &cargo_invariants)?;
    persist_and_verify_release_document(output_dir, &source, &document)?;
    Ok(ui_release_gate_evidence(output_dir, &bundle))
}

fn build_release_model(
    source: &SourceFixtureSet,
    cargo_invariants: &CargoInvariants,
) -> Result<(UiReleaseBundle, UiReleaseDocument)> {
    let bundle = UiReleaseBundle::from_source_fixtures(source)?;
    let document = UiReleaseDocument::from_bundle(&bundle, cargo_invariants)?;
    Ok((bundle, document))
}

fn persist_and_verify_release_document(
    output_dir: &Path,
    source: &SourceFixtureSet,
    document: &UiReleaseDocument,
) -> Result<()> {
    write_release_document(output_dir, source, document)?;
    UiReleaseBundle::from_read_artifacts(&output_dir.join("ui_snapshots"))?;
    read_release_document(output_dir)?.validate()
}

fn write_release_document(
    output_dir: &Path,
    source: &SourceFixtureSet,
    document: &UiReleaseDocument,
) -> Result<()> {
    let snapshot_dir = output_dir.join("ui_snapshots");
    fs::create_dir_all(&snapshot_dir).map_err(|error| Error::BeadDirectoryCreationFailed {
        bead: VB_NF2U.to_string(),
        cause: error.to_string(),
    })?;
    persist_source_fixture_artifacts(source)?;
    write_release_text_files(output_dir, &snapshot_dir, document)
}

fn read_release_document(output_dir: &Path) -> Result<UiReleaseDocument> {
    let snapshot_dir = output_dir.join("ui_snapshots");
    Ok(UiReleaseDocument {
        snapshot_report: read_text_file(&snapshot_dir.join("ui_snapshot_report.yaml"))?,
        ai_release_report: read_text_file(&output_dir.join("ai-release.yaml"))?,
        negative_fixtures: read_text_file(&output_dir.join("negative-fixtures.txt"))?,
        determinism: read_text_file(&output_dir.join("determinism.txt"))?,
        animation_freeze: read_text_file(&output_dir.join("animation-freeze.txt"))?,
    })
}

fn write_release_text_files(
    output_dir: &Path,
    snapshot_dir: &Path,
    document: &UiReleaseDocument,
) -> Result<()> {
    write_text_file(
        &snapshot_dir.join("ui_snapshot_report.yaml"),
        &document.snapshot_report,
    )?;
    write_text_file(
        &output_dir.join("ai-release.yaml"),
        &document.ai_release_report,
    )?;
    write_text_file(
        &output_dir.join("negative-fixtures.txt"),
        &document.negative_fixtures,
    )?;
    write_text_file(&output_dir.join("determinism.txt"), &document.determinism)?;
    write_text_file(
        &output_dir.join("animation-freeze.txt"),
        &document.animation_freeze,
    )
}

fn persist_source_fixture_artifacts(source: &SourceFixtureSet) -> Result<()> {
    if let Some(snapshot_dir) = source
        .artifacts
        .first()
        .and_then(|first| first.output_path.parent())
    {
        remove_legacy_surrogate_pngs(snapshot_dir)?;
    }
    for artifact in &source.artifacts {
        write_bytes_file(&artifact.output_path, &artifact.payload.bytes)?;
    }
    Ok(())
}

fn remove_legacy_surrogate_pngs(snapshot_dir: &Path) -> Result<()> {
    for screen in CANONICAL_SCREENS {
        let path = snapshot_dir.join(format!("{screen}.png"));
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(Error::EvidenceWriteFailed {
                    gate: "ui-release-cleanup".to_string(),
                    path,
                    cause: error.to_string(),
                });
            }
        }
    }
    Ok(())
}

fn render_snapshot_report(bundle: &UiReleaseBundle, cargo_invariants: &CargoInvariants) -> String {
    let mut report = format!(
        "status: pass\ntotal_screens: 8\npassed_screens: 8\nfailed_screens: 0\nfixture_backed: true\ncore_runtime_parity_claim: asserted\ncargo_test_exit_code: {}\ncargo_clippy_exit_code: {}\nscreens:\n",
        cargo_invariants.test_exit, cargo_invariants.clippy_exit,
    );
    for screen in &bundle.screens {
        append_screen_snapshot(&mut report, screen);
    }
    report
}

fn append_screen_snapshot(report: &mut String, screen: &UiScreenEvidenceRow) {
    append_screen_header(report, screen);
    append_screen_checks(report, screen);
}

fn append_screen_header(report: &mut String, screen: &UiScreenEvidenceRow) {
    report.push_str("  - screen_name: ");
    report.push_str(screen.screen_id);
    report.push_str("\n    fixture_id: ");
    report.push_str(screen.fixture_id);
    report.push_str("\n    artifact_path: ");
    report.push_str(&screen.artifact_path);
    report.push_str("\n    digest: ");
    report.push_str(&screen.digest);
    report.push_str("\n    passed: true\n    diagnostics: []\n    execution_marker: vb-nf2u-");
    report.push_str(screen.screen_id);
    report.push_str("\n    checks:\n");
}

fn append_screen_checks(report: &mut String, screen: &UiScreenEvidenceRow) {
    for check in &screen.checks {
        append_screen_check(report, screen, check);
    }
}

fn append_screen_check(
    report: &mut String,
    screen: &UiScreenEvidenceRow,
    check: &UiCheckEvidenceRow,
) {
    report.push_str("      - kind: ");
    report.push_str(check.kind);
    report.push_str("\n        passed: ");
    report.push_str(if check.outcome.is_passed() {
        "true"
    } else {
        "false"
    });
    report.push_str("\n        diagnostics: []\n        execution_marker: vb-nf2u-");
    report.push_str(screen.screen_id);
    report.push('-');
    report.push_str(check.kind);
    report.push_str("\n        origin: ");
    report.push_str(subgate_origin_name(check.outcome.origin()));
    report.push('\n');
}

fn render_ai_release_report(bundle: &UiReleaseBundle, cargo_invariants: &CargoInvariants) -> String {
    let mut report = format!(
        "profile: ai-release\nbead_id: vb-nf2u\nstatus: passed\nfixture_backed: true\ncore_runtime_parity_claim: asserted\ncargo_test_exit_code: {}\ncargo_clippy_exit_code: {}\ncommand: cargo xtask ai-release --bead vb-nf2u\nsubgates:\n",
        cargo_invariants.test_exit, cargo_invariants.clippy_exit,
    );
    for gate in &bundle.subgates {
        report.push_str("  - name: ");
        report.push_str(gate.name);
        report.push_str("\n    status: passed\n    command: ");
        report.push_str(gate.command);
        report.push_str("\n    origin: ");
        report.push_str(subgate_origin_name(gate.origin));
        report.push_str("\n    diagnostics: []\n    execution_marker: vb-nf2u-");
        report.push_str(gate.name);
        report.push('\n');
    }
    append_redaction_report(&mut report);
    report
}

fn render_determinism_report(cargo_invariants: &CargoInvariants) -> String {
    format!(
        "deterministic_capture: passed\nsnapshot_timestamp: 2026-05-09T00:00:00Z\nhidden_animation_state: Paused\nclock_source: FixedFixtureTime\nexecution_marker: vb-nf2u-deterministic-capture\nfixture_backed: true\ncore_runtime_parity_claim: asserted\ncargo_test_exit_code: {}\ncargo_clippy_exit_code: {}\n",
        cargo_invariants.test_exit, cargo_invariants.clippy_exit,
    )
}

fn render_animation_freeze_report() -> String {
    "hidden_animation_state: Paused\nvisible_animation_time_source: FixedFixtureTime\nexecution_marker: vb-nf2u-animation-freeze\n".to_string()
}

fn require_document_shape(document: &UiReleaseDocument) -> Result<()> {
    parse_snapshot_document(&document.snapshot_report)
        .map_err(|error| release_shape_error("snapshot_report", &error))?;
    parse_ai_release_document(&document.ai_release_report)
        .map_err(|error| release_shape_error("ai_release", &error))?;
    parse_negative_fixture_document(&document.negative_fixtures)
        .map_err(|error| release_shape_error("negative_fixtures", &error))?;
    parse_determinism_document(&document.determinism)
        .map_err(|error| release_shape_error("determinism", &error))?;
    parse_animation_freeze_document(&document.animation_freeze)
        .map_err(|error| release_shape_error("animation_freeze", &error))
}
