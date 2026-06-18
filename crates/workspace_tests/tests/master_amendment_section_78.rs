#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

const MASTER_FILE: &str = "velvet-ballistics-MASTER.md";
const TIER_A_HEADER: &str = "## 78. Tier A — Backend / IR Interpreter Complete v1.0";
const WAVE_TABLE_HEADER: &str = "| Wave | Name | Bead count | Critical-path? |";
const REQUIRED_AUDIT_GATE_ROWS: usize = 10;
const REQUIRED_WAVE_TABLE_MARKERS: &[&str] = &[WAVE_TABLE_HEADER];

const REQUIRED_SUBSECTION_HEADERS: &[&str] = &[
    "### Scope",
    "### Tier A waves",
    "### Tier A.0 prerequisites",
    "### Tier A acceptance criteria",
    "### Tier A forbidden-construct audit gates",
];

const REQUIRED_PREREQUISITE_BEADS: &[&str] =
    &["vb-o5zb", "vb-o5zb.1", "vb-o5zb.2", "vb-o5zb.3", "vb-t7srg"];

const REQUIRED_AUDIT_MARKERS: &[&str] = &[
    "cargo +nightly clippy --workspace --all-targets --all-features",
    "moon run :unsafe-audit",
    "moon run :supply-chain",
    "bash scripts/check-spelling-gate.sh",
    "bash scripts/check-workspace-assertions.sh",
    "bash scripts/check-source-length.sh",
    "moon run :nightly-feature-gate",
    "§43",
];

const FORBIDDEN_DEAD_AUDIT_COMMANDS: &[&str] = &[
    "cargo +nightly cargo-geiger --fail-build",
    "cargo +nightly cargo-deny check advisories",
];

const SUPPORTED_DIRECT_CARGO_AUDIT_SUBCOMMANDS: &[&str] = &["fmt", "clippy", "check", "nextest"];

#[test]
fn test_master_amendment_section_78_exists_with_required_subsections() -> Result<(), String> {
    // Given: the canonical master document from the isolated workspace.
    let master_path = master_file_path();

    // When: the test reads the master document as UTF-8 and isolates the Tier A body.
    let body = std::fs::read_to_string(&master_path)
        .map_err(|error| format!("failed to read {} as UTF-8: {error}", master_path.display()))?;
    let tier_a_section = markdown_section(&body, TIER_A_HEADER, "## ");
    let acceptance_section =
        markdown_section(&tier_a_section, "### Tier A acceptance criteria", "### ");
    let audit_section = markdown_section(
        &tier_a_section,
        "### Tier A forbidden-construct audit gates",
        "### ",
    );

    // Then: the authoritative Tier A §78 header is present exactly once.
    let tier_a_header_count = count_exact_lines(&body, TIER_A_HEADER);
    assert_eq!(
        tier_a_header_count, 1,
        "duplicate Tier A §78 collision must be detectable by exact header count"
    );

    // Then: all required Tier A subsection headers and prerequisite beads are present.
    let missing_subsections = missing_markers(&tier_a_section, REQUIRED_SUBSECTION_HEADERS);
    assert_eq!(
        missing_subsections,
        Vec::<&str>::new(),
        "Tier A §78 subsection header(s) missing"
    );

    let missing_prerequisite_beads =
        missing_prerequisite_bullets(&tier_a_section, REQUIRED_PREREQUISITE_BEADS);
    assert_eq!(
        missing_prerequisite_beads,
        Vec::<&str>::new(),
        "Tier A.0 prerequisite bead bullet(s) missing"
    );

    // Then: the 13-wave table contract is present by header plus ordinals 0 through 12.
    let missing_wave_table_markers = missing_markers(&tier_a_section, REQUIRED_WAVE_TABLE_MARKERS);
    assert_eq!(
        missing_wave_table_markers,
        Vec::<&str>::new(),
        "Tier A wave table header missing"
    );

    let missing_wave_ordinals: Vec<String> = (0u8..=12u8)
        .map(|wave| format!("| {wave} |"))
        .filter(|marker| !tier_a_section.contains(marker))
        .collect();
    assert_eq!(
        missing_wave_ordinals,
        Vec::<String>::new(),
        "Tier A wave ordinal row(s) 0 through 12 missing"
    );

    // Then: the acceptance subsection lists all 8 numbered acceptance criteria.
    let missing_acceptance_ordinals: Vec<String> = (1u8..=8u8)
        .map(|criterion| format!("{criterion}. "))
        .filter(|marker| !acceptance_section.contains(marker))
        .collect();
    assert_eq!(
        missing_acceptance_ordinals,
        Vec::<String>::new(),
        "Tier A acceptance criterion ordinal(s) 1 through 8 missing"
    );

    // Then: forbidden-construct audit gates are wired with 10 gate rows and required cross-refs.
    let audit_gate_rows: Vec<&str> = audit_section
        .lines()
        .filter(|line| line.starts_with("- `"))
        .collect();
    assert_eq!(
        audit_gate_rows.len(),
        REQUIRED_AUDIT_GATE_ROWS,
        "Tier A forbidden-construct audit gate row count drifted"
    );

    let missing_audit_markers = missing_markers(&audit_section, REQUIRED_AUDIT_MARKERS);
    assert_eq!(
        missing_audit_markers,
        Vec::<&str>::new(),
        "Tier A forbidden-construct audit marker(s) missing"
    );

    let dead_audit_commands = present_markers(&audit_section, FORBIDDEN_DEAD_AUDIT_COMMANDS);
    assert_eq!(
        dead_audit_commands,
        Vec::<&str>::new(),
        "Tier A forbidden-construct audit command(s) are non-executable dead cargo rows"
    );

    let missing_audit_scripts = missing_audit_gate_wiring(&master_path, &audit_gate_rows);
    assert_eq!(
        missing_audit_scripts,
        Vec::<String>::new(),
        "Tier A audit script command(s) missing on disk or Moon task registration"
    );

    Ok(())
}

fn master_file_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("..");
    path.push("..");
    path.push(MASTER_FILE);
    path
}

fn count_exact_lines(body: &str, exact_line: &str) -> usize {
    body.lines().filter(|line| *line == exact_line).count()
}

fn missing_markers<'a>(body: &str, markers: &'a [&'a str]) -> Vec<&'a str> {
    markers
        .iter()
        .copied()
        .filter(|marker| !body.contains(marker))
        .collect()
}

fn present_markers<'a>(body: &str, markers: &'a [&'a str]) -> Vec<&'a str> {
    markers
        .iter()
        .copied()
        .filter(|marker| body.contains(marker))
        .collect()
}

fn missing_prerequisite_bullets<'a>(body: &str, beads: &'a [&'a str]) -> Vec<&'a str> {
    beads
        .iter()
        .copied()
        .filter(|bead| !has_prerequisite_bullet(body, bead))
        .collect()
}

fn has_prerequisite_bullet(body: &str, bead: &str) -> bool {
    let prefix = format!("- `{bead}`");
    body.lines().any(|line| {
        let trimmed = line.trim();
        trimmed == prefix
            || trimmed
                .strip_prefix(&prefix)
                .is_some_and(|tail| tail.starts_with(' '))
    })
}

fn missing_audit_gate_wiring(master_path: &Path, audit_rows: &[&str]) -> Vec<String> {
    let repo_root = master_path.parent().unwrap_or_else(|| Path::new("."));
    let task_path = repo_root.join(".moon/tasks/all.yml");
    let task_text = match std::fs::read_to_string(&task_path) {
        Ok(text) => text,
        Err(error) => return vec![format!("{}: {error}", task_path.display())],
    };

    let mut missing = Vec::new();
    for row in audit_rows {
        match audit_command(row) {
            Some(command) => {
                if let Some(finding) = missing_audit_command(repo_root, &task_text, command) {
                    missing.push(finding);
                }
            }
            None => missing.push(format!("{row}: missing backtick-delimited command")),
        }
    }
    missing
}

fn missing_audit_command(repo_root: &Path, task_text: &str, command: &str) -> Option<String> {
    if let Some(finding) = dead_cargo_command(command) {
        return Some(finding);
    }

    if let Some(script_path) = script_path_from_command(command) {
        return missing_script_command(repo_root, task_text, command, script_path);
    }

    if let Some(task) = moon_task_from_command(command) {
        return missing_moon_task(task_text, task);
    }

    if supported_direct_cargo_audit_command(command) {
        return None;
    }

    Some(format!("{command}: unsupported Tier A audit command form"))
}

fn dead_cargo_command(command: &str) -> Option<String> {
    direct_cargo_subcommand(command)
        .filter(|subcommand| subcommand.starts_with("cargo-"))
        .map(|subcommand| {
            format!(
                "{command}: dead cargo subcommand `{subcommand}`; use a repository Moon task or a runnable cargo subcommand"
            )
        })
}

fn supported_direct_cargo_audit_command(command: &str) -> bool {
    direct_cargo_subcommand(command).is_some_and(|subcommand| {
        SUPPORTED_DIRECT_CARGO_AUDIT_SUBCOMMANDS
            .iter()
            .any(|supported| *supported == subcommand)
    })
}

fn direct_cargo_subcommand(command: &str) -> Option<&str> {
    let mut parts = command.split_whitespace();
    if parts.next()? != "cargo" {
        return None;
    }
    if parts.next()? != "+nightly" {
        return None;
    }
    parts.next()
}

fn missing_script_command(
    repo_root: &Path,
    task_text: &str,
    command: &str,
    script_path: &str,
) -> Option<String> {
    if !repo_root.join(script_path).is_file() {
        return Some(format!("{script_path}: missing script"));
    }

    let exact_task_command = format!("command: '{command}'");
    (!task_text.contains(&exact_task_command)).then(|| format!("{command}: missing Moon command"))
}

fn missing_moon_task(task_text: &str, task: &str) -> Option<String> {
    let exact_task_header = format!("  {task}:");
    (!task_text.lines().any(|line| line == exact_task_header))
        .then(|| format!("moon run :{task}: missing Moon task"))
}

fn audit_command(row: &str) -> Option<&str> {
    row.strip_prefix("- `")?
        .split_once('`')
        .map(|(command, _tail)| command)
}

fn script_path_from_command(command: &str) -> Option<&str> {
    let candidate = command.strip_prefix("bash ").unwrap_or(command);
    candidate
        .split_whitespace()
        .next()
        .filter(|part| part.starts_with("scripts/"))
}

fn moon_task_from_command(command: &str) -> Option<&str> {
    command.strip_prefix("moon run :")
}

fn markdown_section(body: &str, start_heading: &str, next_heading_prefix: &str) -> String {
    let mut section = String::new();
    let mut in_section = false;

    for line in body.lines() {
        if !in_section {
            if line == start_heading || line.starts_with(start_heading) {
                in_section = true;
                append_markdown_line(&mut section, line);
            }
            continue;
        }

        if line.starts_with(next_heading_prefix) && !line.starts_with(start_heading) {
            break;
        }

        append_markdown_line(&mut section, line);
    }

    section
}

fn append_markdown_line(buffer: &mut String, line: &str) {
    if !buffer.is_empty() {
        buffer.push('\n');
    }
    buffer.push_str(line);
}
