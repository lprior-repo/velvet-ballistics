use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug)]
struct Finding {
    kind: &'static str,
    path: String,
    detail: String,
}

struct DeletionAllowance {
    deleted_path: String,
    replacement_path: String,
}

fn program_output(program: &str, args: &[&str], cwd: &Path) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|error| format!("{program} {} failed to start: {error}", args.join(" ")))?;
    if output.status.success() {
        String::from_utf8(output.stdout).map_err(|error| {
            format!(
                "{program} {} returned non-UTF8 stdout: {error}",
                args.join(" ")
            )
        })
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("{program} {} failed: {stderr}", args.join(" ")))
    }
}

fn command_output(args: &[&str], cwd: &Path) -> Result<String, String> {
    program_output("git", args, cwd)
}

fn jj_command_output(args: &[&str], cwd: &Path) -> Result<String, String> {
    program_output("jj", args, cwd)
}

fn program_output_allow_fail(program: &str, args: &[&str], cwd: &Path) -> Option<String> {
    Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
}

fn command_output_allow_fail(args: &[&str], cwd: &Path) -> Option<String> {
    program_output_allow_fail("git", args, cwd)
}

fn jj_command_output_allow_fail(args: &[&str], cwd: &Path) -> Option<String> {
    program_output_allow_fail("jj", args, cwd)
}

fn non_empty_trimmed_path(text: String) -> Option<PathBuf> {
    let trimmed = text.trim();
    (!trimmed.is_empty()).then(|| PathBuf::from(trimmed))
}

fn root_dir() -> Result<PathBuf, String> {
    match command_output(&["rev-parse", "--show-toplevel"], Path::new(".")) {
        Ok(text) => non_empty_trimmed_path(text)
            .ok_or_else(|| "git rev-parse --show-toplevel returned an empty path".to_owned()),
        Err(git_error) => jj_command_output_allow_fail(&["root"], Path::new("."))
            .and_then(non_empty_trimmed_path)
            .ok_or(git_error),
    }
}

fn is_jj_repo(root: &Path) -> bool {
    jj_command_output_allow_fail(&["root"], root).is_some()
}

fn is_test_path(path: &str) -> bool {
    path.ends_with(".rs")
        && [
            "/tests/",
            "/benches/",
            "/examples/",
            "/fuzz/",
            "workspace_tests",
        ]
        .iter()
        .any(|part| format!("/{path}").contains(part))
}

fn is_behavior_test_path(path: &str) -> bool {
    path.ends_with(".rs")
        && ["/tests/", "workspace_tests"]
            .iter()
            .any(|part| format!("/{path}").contains(part))
}

fn has_exact_assertion(text: &str) -> bool {
    [
        "assert_eq!(",
        "assert_ne!(",
        "assert_matches!(",
        "assert_json_",
        "insta::assert_",
        "snapshot!(",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn has_weak_assertion(text: &str) -> bool {
    text.contains("assert!(")
        && [
            ".is_ok(",
            ".is_err(",
            ".is_some(",
            ".is_none(",
            ".is_empty(",
        ]
        .iter()
        .any(|needle| text.contains(needle))
}

fn has_test_decl(text: &str) -> bool {
    text.contains("#[test")
        || text.contains("#[tokio::test")
        || text.contains("fn test_")
        || text.contains("_test(")
}

fn has_ignore_or_skip(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("#[ignore")
        || lower.contains("cfg_attr") && lower.contains("ignore")
        || lower.contains("return;")
        || lower.contains(" skipped")
        || lower.contains(" skip")
        || lower.contains("ignored")
}

fn has_compile_only(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "no_run",
        "compile_only",
        "compile-only",
        "smoke only",
        "compile smoke",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn default_base(root: &Path) -> String {
    if let Ok(value) = std::env::var("TEST_INTEGRITY_BASE") {
        if !value.trim().is_empty() {
            return value;
        }
    }
    let dirty = match command_output(&["status", "--porcelain"], root) {
        Ok(text) => !text.trim().is_empty(),
        Err(_) if is_jj_repo(root) => return "@-".to_owned(),
        Err(_) => true,
    };
    if dirty {
        return "HEAD".to_owned();
    }
    command_output_allow_fail(&["merge-base", "origin/main", "HEAD"], root)
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "HEAD".to_owned())
}

fn parse_git_name_status(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let parts = line.split('\t').collect::<Vec<_>>();
            (parts.len() >= 2).then(|| {
                let status = parts.first().map_or("", |value| *value).to_owned();
                let path = parts.last().map_or("", |value| *value).to_owned();
                (status, path)
            })
        })
        .collect()
}

fn renamed_jj_path(rest: &str) -> &str {
    rest.rsplit_once(" => ")
        .or_else(|| rest.rsplit_once(" -> "))
        .map_or(rest, |(_old, new)| new)
}

fn parse_jj_summary(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let mut chars = line.chars();
            let status = chars.next()?;
            let rest = chars.as_str().trim_start();
            if rest.is_empty() || !matches!(status, 'A' | 'D' | 'M' | 'R') {
                return None;
            }
            let path = if status == 'R' {
                renamed_jj_path(rest)
            } else {
                rest
            };
            Some((status.to_string(), path.to_owned()))
        })
        .collect()
}

fn jj_diff_summary(root: &Path, base: &str) -> Result<Vec<(String, String)>, String> {
    jj_command_output(&["diff", "--summary", "--from", base, "--to", "@"], root)
        .map(|text| parse_jj_summary(&text))
}

fn changed_files(root: &Path, base: &str) -> Result<Vec<(String, String)>, String> {
    match command_output(
        &["diff", "--name-status", "--find-renames", base, "--"],
        root,
    ) {
        Ok(text) => Ok(parse_git_name_status(&text)),
        Err(git_error) => jj_diff_summary(root, base)
            .map_err(|jj_error| format!("{git_error}; fallback {jj_error}")),
    }
}

fn base_file_has_tests(root: &Path, base: &str, path: &str) -> bool {
    command_output_allow_fail(&["show", &format!("{base}:{path}")], root)
        .or_else(|| jj_command_output_allow_fail(&["file", "show", "--revision", base, path], root))
        .map(|text| has_test_decl(&text) || has_exact_assertion(&text))
        .unwrap_or(false)
}

fn deletion_allowances(root: &Path) -> Vec<DeletionAllowance> {
    let path = root.join(".config/test-integrity-deletion-allow.txt");
    let text = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    let mut allowances = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let parts = trimmed.split('|').collect::<Vec<_>>();
        if parts.len() < 4 {
            continue;
        }
        let deleted_path = parts.first().map_or("", |value| *value);
        let replacement_path = parts.get(1).map_or("", |value| *value);
        if deleted_path.is_empty() || replacement_path.is_empty() {
            continue;
        }
        allowances.push(DeletionAllowance {
            deleted_path: deleted_path.to_owned(),
            replacement_path: replacement_path.to_owned(),
        });
    }
    allowances
}

fn replacement_has_coverage(root: &Path, replacement_path: &str) -> bool {
    fs::read_to_string(root.join(replacement_path))
        .map(|text| has_test_decl(&text) && has_exact_assertion(&text))
        .unwrap_or(false)
}

fn deletion_has_allowed_replacement(
    root: &Path,
    path: &str,
    allowances: &[DeletionAllowance],
) -> bool {
    allowances
        .iter()
        .filter(|allowance| allowance.deleted_path == path)
        .any(|allowance| replacement_has_coverage(root, &allowance.replacement_path))
}

fn scan_deleted_files(
    root: &Path,
    base: &str,
    allowances: &[DeletionAllowance],
) -> Result<Vec<Finding>, String> {
    changed_files(root, base).map(|entries| {
        entries
            .into_iter()
            .filter(|(status, path)| {
                status.starts_with('D')
                    && (is_test_path(path) || base_file_has_tests(root, base, path))
            })
            .filter(|(_, path)| !deletion_has_allowed_replacement(root, path, allowances))
            .map(|(_status, path)| Finding {
                kind: "DeletedTestFile",
                path,
                detail: "deleted file contained tests or test assertions".to_owned(),
            })
            .collect()
    })
}

fn diff_text(root: &Path, base: &str) -> Result<String, String> {
    match command_output(&["diff", "--find-renames", "--unified=0", base, "--"], root) {
        Ok(text) => Ok(text),
        Err(git_error) => jj_command_output(
            &[
                "diff",
                "--git",
                "--context",
                "0",
                "--from",
                base,
                "--to",
                "@",
            ],
            root,
        )
        .map_err(|jj_error| format!("{git_error}; fallback {jj_error}")),
    }
}

fn scan_diff(diff: &str) -> Vec<Finding> {
    let mut current = String::from("<unknown>");
    let mut removed_test_decl: Vec<(String, String)> = Vec::new();
    let mut added_test_decl: Vec<String> = Vec::new();
    let mut removed_exact: Vec<(String, usize)> = Vec::new();
    let mut added_exact: Vec<(String, usize)> = Vec::new();
    let mut added_weak: Vec<(String, usize)> = Vec::new();
    let mut findings = Vec::new();

    diff.lines().for_each(|line| {
        if let Some(path) = line.strip_prefix("+++ b/") {
            current = path.to_owned();
            return;
        }
        if let Some(path) = line.strip_prefix("--- a/") {
            if current == "<unknown>" {
                current = path.to_owned();
            }
            return;
        }
        if !is_test_path(&current) {
            return;
        }
        if line.starts_with('-') && !line.starts_with("---") {
            let payload = &line[1..];
            if has_test_decl(payload) {
                removed_test_decl.push((current.clone(), payload.trim().to_owned()));
            }
            if has_exact_assertion(payload) {
                removed_exact.push((current.clone(), 1));
            }
        } else if line.starts_with('+') && !line.starts_with("+++") {
            let payload = &line[1..];
            if has_test_decl(payload) {
                added_test_decl.push(current.clone());
            }
            if is_behavior_test_path(&current) && has_ignore_or_skip(payload) {
                findings.push(Finding {
                    kind: "IgnoredOrSkippedTest",
                    path: current.clone(),
                    detail: payload.trim().to_owned(),
                });
            }
            if is_behavior_test_path(&current) && has_compile_only(payload) {
                findings.push(Finding {
                    kind: "CompileOnlyReplacement",
                    path: current.clone(),
                    detail: payload.trim().to_owned(),
                });
            }
            if has_exact_assertion(payload) {
                added_exact.push((current.clone(), 1));
            }
            if has_weak_assertion(payload) {
                added_weak.push((current.clone(), 1));
            }
        }
    });

    let test_decl_paths = removed_test_decl
        .iter()
        .map(|(path, _detail)| path.clone())
        .collect::<std::collections::BTreeSet<_>>();
    test_decl_paths.into_iter().for_each(|path| {
        let removed_count = removed_test_decl
            .iter()
            .filter(|(candidate, _detail)| candidate == &path)
            .count();
        let added_count = added_test_decl
            .iter()
            .filter(|candidate| *candidate == &path)
            .count();
        if added_count < removed_count {
            findings.push(Finding {
                kind: "DeletedTestDeclaration",
                path,
                detail: format!(
                    "removed_test_decls={removed_count} added_test_decls={added_count}"
                ),
            });
        }
    });

    let paths = removed_exact
        .iter()
        .map(|(path, _count)| path.clone())
        .collect::<std::collections::BTreeSet<_>>();
    paths.into_iter().for_each(|path| {
        let removed_count = removed_exact.iter().filter(|(candidate, _)| candidate == &path).count();
        let added_exact_count = added_exact.iter().filter(|(candidate, _)| candidate == &path).count();
        let added_weak_count = added_weak.iter().filter(|(candidate, _)| candidate == &path).count();
        if added_weak_count > 0 || added_exact_count < removed_count {
            findings.push(Finding {
                kind: "WeakenedAssertion",
                path,
                detail: format!(
                    "removed_exact={removed_count} added_exact={added_exact_count} added_weak={added_weak_count}"
                ),
            });
        }
    });

    findings
}

fn check(root: &Path, base: &str) -> Result<i32, String> {
    let allowances = deletion_allowances(root);
    let mut findings = scan_deleted_files(root, base, &allowances)?;
    findings.extend(scan_diff(&diff_text(root, base)?));
    if findings.is_empty() {
        println!("test integrity: PASS base={base}");
        Ok(0)
    } else {
        eprintln!("test integrity: FAIL");
        findings.iter().for_each(|finding| {
            eprintln!("{}|{}|{}", finding.kind, finding.path, finding.detail);
        });
        eprintln!("Add equal-or-stronger replacement coverage or bead-linked justification.");
        Ok(1)
    }
}

fn write(path: &Path, text: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)
}

fn run_self_test_case(label: &str, mutate: &str, expected: i32) -> bool {
    let root = std::env::temp_dir().join(format!("test-integrity-{}-{label}", std::process::id()));
    let cleanup_result = fs::remove_dir_all(&root);
    match cleanup_result {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => {
            eprintln!("SelfTest:{label}: cleanup failed: {error}");
            return false;
        }
    }
    let test_file = root.join("crates/workspace_tests/tests/behavior.rs");
    let base_text = "#[test]\nfn behavior_test() {\n    assert_eq!(2 + 2, 4);\n}\n";
    let init = fs::create_dir_all(&root)
        .and_then(|()| write(&test_file, base_text))
        .map_err(|error| error.to_string())
        .and_then(|()| command_output(&["init", "-q"], &root).map(|_| ()))
        .and_then(|()| command_output(&["add", "."], &root).map(|_| ()))
        .and_then(|()| {
            command_output(
                &[
                    "-c",
                    "user.name=guard",
                    "-c",
                    "user.email=guard@example.invalid",
                    "commit",
                    "-q",
                    "-m",
                    "base",
                ],
                &root,
            )
            .map(|_| ())
        });
    if let Err(error) = init {
        eprintln!("SelfTest:{label}: setup failed: {error}");
        return false;
    }
    let mutation = match mutate {
        "delete" => fs::remove_file(&test_file),
        "ignore" => write(
            &test_file,
            "#[test]\n#[ignore]\nfn behavior_test() {\n    assert_eq!(2 + 2, 4);\n}\n",
        ),
        "weaken" => write(
            &test_file,
            "#[test]\nfn behavior_test() {\n    assert!((2 + 2).checked_sub(4).is_some());\n}\n",
        ),
        "strengthen" => write(
            &test_file,
            "#[test]\nfn behavior_test() {\n    assert_eq!(2 + 2, 4);\n    assert_ne!(2 + 2, 5);\n}\n",
        ),
        _ => Ok(()),
    };
    if let Err(error) = mutation {
        eprintln!("SelfTest:{label}: mutation failed: {error}");
        return false;
    }
    let actual = check(&root, "HEAD").unwrap_or(2);
    let passed = actual == expected;
    println!(
        "SelfTest:{label}: {} expected={expected} actual={actual}",
        if passed { "PASS" } else { "FAIL" }
    );
    passed
}

fn self_test() -> i32 {
    let cases = [
        ("delete", "delete", 1),
        ("ignore", "ignore", 1),
        ("weaken", "weaken", 1),
        ("strengthen", "strengthen", 0),
    ];
    let ok = cases
        .iter()
        .all(|(label, mutate, expected)| run_self_test_case(label, mutate, *expected));
    if ok {
        0
    } else {
        1
    }
}

fn argument_value(args: &[String], flag: &str) -> Option<String> {
    args.windows(2).find_map(|window| {
        let first = window.first()?;
        let second = window.get(1)?;
        (first == flag).then(|| second.clone())
    })
}

fn run() -> i32 {
    let args = std::env::args().collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--self-test") {
        return self_test();
    }
    let root = match root_dir() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("{error}");
            return 64;
        }
    };
    let base = argument_value(&args, "--base").unwrap_or_else(|| default_base(&root));
    match check(&root, &base) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("test integrity: ERROR {error}");
            2
        }
    }
}

fn main() {
    std::process::exit(run());
}
