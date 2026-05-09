#![forbid(unsafe_code)]

use anyhow::Context;
use clap::{Parser, Subcommand};
use std::io::Write;
use std::path::{Path, PathBuf};
use vb_ui_snapshot::{
    BASELINE_HEIGHT, BASELINE_WIDTH, UiSnapshotReport, checks, demo_fixture_names, fixtures,
    report, tokens,
};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Velvet Ballistics xtask commands")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[command(name = "ui-snapshot")]
    Snapshot {
        #[arg(long)]
        all: bool,
        #[arg(long)]
        fixture: Option<String>,
        #[arg(long)]
        emit: Option<String>,
        #[arg(long, default_value = "tests/ui_snapshots")]
        output_dir: String,
    },
    #[command(name = "ui-tokens")]
    Tokens {
        #[arg(long, default_value = "design/tokens/velvet_ui_tokens.toml")]
        input: String,
        #[arg(long, default_value = "crates/vb_ui/src/theme/tokens_generated.rs")]
        output: String,
        #[arg(long)]
        emit: Option<String>,
    },
    #[command(name = "ui-overlap-check")]
    OverlapCheck {
        #[arg(long)]
        all: bool,
        #[arg(long)]
        screen: Option<String>,
        #[arg(long, default_value = "tests/ui_snapshots")]
        input_dir: String,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Snapshot {
            all,
            fixture,
            emit,
            output_dir,
        } => cmd_ui_snapshot(all, fixture, emit, output_dir),
        Commands::Tokens {
            input,
            output,
            emit,
        } => cmd_ui_tokens(&input, &output, emit),
        Commands::OverlapCheck {
            all,
            screen,
            input_dir,
        } => cmd_ui_overlap_check(all, screen, &input_dir),
    }
}

fn write_stdout(args: std::fmt::Arguments<'_>) -> anyhow::Result<()> {
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    handle
        .write_fmt(args)
        .context("Failed to write to stdout")?;
    handle
        .write_all(b"\n")
        .context("Failed to write newline to stdout")?;
    Ok(())
}

fn cmd_ui_snapshot(
    do_all: bool,
    fixture_name: Option<String>,
    emit: Option<String>,
    output_dir: String,
) -> anyhow::Result<()> {
    let output_path = PathBuf::from(&output_dir);
    std::fs::create_dir_all(&output_path)
        .with_context(|| format!("Failed to create output directory: {}", output_dir))?;

    let mut full_report = UiSnapshotReport::new();

    if do_all {
        for name in demo_fixture_names() {
            capture_fixture(name, &output_path, &mut full_report)?;
        }
    } else if let Some(name) = fixture_name {
        capture_fixture(&name, &output_path, &mut full_report)?;
    } else {
        anyhow::bail!("Must specify --all or --fixture <name>");
    }

    full_report.finalize();

    let report_path = output_path.join("ui_snapshot_report.yaml");
    if let Some(emit_fmt) = emit.as_deref() {
        if emit_fmt == "yaml" {
            let yaml = full_report
                .to_yaml()
                .context("Failed to serialize report to YAML")?;
            write_stdout(format_args!("{yaml}"))?;
            std::fs::write(&report_path, &yaml)
                .with_context(|| format!("Failed to write report to {}", report_path.display()))?;
        }
    } else {
        let yaml = full_report
            .to_yaml()
            .context("Failed to serialize report to YAML")?;
        std::fs::write(&report_path, yaml)
            .with_context(|| format!("Failed to write report to {}", report_path.display()))?;
    }

    write_stdout(format_args!(
        "Snapshot report written to: {}",
        report_path.display()
    ))?;
    write_stdout(format_args!("Status: {}", full_report.status))?;
    write_stdout(format_args!(
        "Screens: {}/{} passed",
        full_report.passed_screens, full_report.total_screens
    ))?;

    Ok(())
}

fn capture_fixture(
    name: &str,
    output_dir: &Path,
    report: &mut UiSnapshotReport,
) -> anyhow::Result<()> {
    let fixture = fixtures::load_demo_fixture(name)
        .with_context(|| format!("Failed to load fixture: {name}"))?;

    let png_path = output_dir.join(format!("{name}.png"));

    generate_stub_screenshot(&png_path, &fixture.name)?;

    let mut screen_result = report::ScreenResult {
        screen_name: name.to_string(),
        png_path: Some(png_path.to_string_lossy().to_string()),
        checks: Vec::new(),
        passed: true,
    };

    if png_path.exists() {
        let tokens_path = PathBuf::from("design/tokens/velvet_ui_tokens.toml");
        let ui_tokens = tokens_path
            .exists()
            .then(|| tokens::load_tokens_from_file(&tokens_path).ok())
            .flatten();

        let overlap_result = checks::check_overlap(&png_path);
        let overlap_check = match overlap_result {
            Ok(r) if r.overlaps.is_empty() => report::make_pass_result(report::CheckKind::Overlap),
            Ok(r) => report::make_fail_result(
                report::CheckKind::Overlap,
                &format!("{} overlaps detected", r.overlaps.len()),
            ),
            Err(e) => report::make_fail_result(report::CheckKind::Overlap, &e.to_string()),
        };
        screen_result.checks.push(overlap_check);

        let clipping_result = checks::check_clipping(&png_path);
        let clipping_check = match clipping_result {
            Ok(r) if r.clipped_labels.is_empty() => {
                report::make_pass_result(report::CheckKind::Clipping)
            }
            Ok(r) => report::make_fail_result(
                report::CheckKind::Clipping,
                &format!("{} clipped labels", r.clipped_labels.len()),
            ),
            Err(e) => report::make_fail_result(report::CheckKind::Clipping, &e.to_string()),
        };
        screen_result.checks.push(clipping_check);

        let spelling_result = checks::check_spelling(&png_path);
        let spelling_check = match spelling_result {
            Ok(r) if r.violations.is_empty() => {
                report::make_pass_result(report::CheckKind::Spelling)
            }
            Ok(r) => report::make_fail_result(
                report::CheckKind::Spelling,
                &format!("{} spelling violations", r.violations.len()),
            ),
            Err(e) => report::make_fail_result(report::CheckKind::Spelling, &e.to_string()),
        };
        screen_result.checks.push(spelling_check);

        if let Some(ref tok) = ui_tokens {
            let color_result = checks::check_color_drift(&png_path, tok);
            let color_check = match color_result {
                Ok(r) if r.drifts.is_empty() => {
                    report::make_pass_result(report::CheckKind::ColorDrift)
                }
                Ok(r) => report::make_fail_result(
                    report::CheckKind::ColorDrift,
                    &format!("{} color drifts", r.drifts.len()),
                ),
                Err(e) => report::make_fail_result(report::CheckKind::ColorDrift, &e.to_string()),
            };
            screen_result.checks.push(color_check);
        }

        let png_check = match checks::validate_png_dimensions(&png_path) {
            Ok((w, h)) if w == BASELINE_WIDTH && h == BASELINE_HEIGHT => {
                report::make_pass_result(report::CheckKind::PngValidity)
            }
            Ok((w, h)) => report::make_fail_result(
                report::CheckKind::PngValidity,
                &format!("Invalid dimensions: {}x{}", w, h),
            ),
            Err(e) => report::make_fail_result(report::CheckKind::PngValidity, &e.to_string()),
        };
        screen_result.checks.push(png_check);
    }

    screen_result.passed = screen_result.checks.iter().all(|c| c.passed);
    report.add_screen(screen_result);

    write_stdout(format_args!(
        "Captured fixture '{}' -> {}",
        name,
        png_path.display()
    ))?;
    Ok(())
}

fn generate_stub_screenshot(output_path: &Path, _fixture_name: &str) -> anyhow::Result<()> {
    let mut img = image::RgbaImage::new(BASELINE_WIDTH, BASELINE_HEIGHT);

    for pixel in img.pixels_mut() {
        *pixel = image::Rgba([255, 255, 255, 255]);
    }

    img.save(output_path)
        .with_context(|| format!("Failed to save stub PNG to {}", output_path.display()))?;

    Ok(())
}

fn cmd_ui_tokens(input_path: &str, output_path: &str, emit: Option<String>) -> anyhow::Result<()> {
    let tokens_content = std::fs::read_to_string(input_path)
        .with_context(|| format!("Failed to read tokens file: {input_path}"))?;

    let ui_tokens = tokens::parse_tokens_from_toml(&tokens_content)
        .with_context(|| format!("Failed to parse tokens from {input_path}"))?;

    let rust_code = tokens::tokens_to_rust_constants(&ui_tokens);

    let output = PathBuf::from(output_path);
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    if let Some(emit_fmt) = emit.as_deref() {
        if emit_fmt == "rust" {
            write_stdout(format_args!("{}", rust_code))?;
        } else if emit_fmt == "json" {
            let json = serde_json::to_string_pretty(&ui_tokens)
                .context("Failed to serialize tokens to JSON")?;
            write_stdout(format_args!("{}", json))?;
        }
    }

    std::fs::write(&output, &rust_code)
        .with_context(|| format!("Failed to write Rust constants to {}", output.display()))?;

    write_stdout(format_args!(
        "Generated Rust tokens at: {}",
        output.display()
    ))?;
    Ok(())
}

fn cmd_ui_overlap_check(
    do_all: bool,
    screen_name: Option<String>,
    input_dir: &str,
) -> anyhow::Result<()> {
    let input_path = PathBuf::from(input_dir);

    if do_all {
        for name in demo_fixture_names() {
            check_overlap_for_screen(&input_path, name)?;
        }
    } else if let Some(name) = screen_name {
        check_overlap_for_screen(&input_path, &name)?;
    } else {
        anyhow::bail!("Must specify --all or --screen <name>");
    }

    Ok(())
}

fn check_overlap_for_screen(base_dir: &Path, name: &str) -> anyhow::Result<()> {
    let png_path = base_dir.join(format!("{name}.png"));
    if !png_path.exists() {
        write_stdout(format_args!(
            "WARN: {} does not exist, skipping overlap check",
            png_path.display()
        ))?;
        return Ok(());
    }

    let result = checks::check_overlap(&png_path)
        .with_context(|| format!("Overlap check failed for: {name}"))?;

    if result.overlaps.is_empty() {
        write_stdout(format_args!("PASS: {name} — no overlaps detected"))?;
    } else {
        write_stdout(format_args!(
            "FAIL: {name} — {} overlaps detected:",
            result.overlaps.len()
        ))?;
        for ov in &result.overlaps {
            write_stdout(format_args!(
                "  {} overlaps {} ({}px)",
                ov.panel_a, ov.panel_b, ov.overlap_area_px
            ))?;
        }
    }

    Ok(())
}
