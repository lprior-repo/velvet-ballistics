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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Vcs {
    Git,
    Jj,
}

#[derive(Debug, Clone)]
struct RootInfo {
    path: PathBuf,
    vcs: Vcs,
}

fn tool_output(tool: &str, args: &[&str], cwd: &Path) -> Result<String, String> {
    let output = Command::new(tool)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|error| format!("{tool} {} failed to start: {error}", args.join(" ")))?;
    if output.status.success() {
        String::from_utf8(output.stdout).map_err(|error| {
            format!(
                "{tool} {} returned non-UTF8 stdout: {error}",
                args.join(" ")
            )
        })
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("{tool} {} failed: {stderr}", args.join(" ")))
    }
}

fn command_output(args: &[&str], cwd: &Path) -> Result<String, String> {
    tool_output("git", args, cwd)
}

fn jj_output(args: &[&str], cwd: &Path) -> Result<String, String> {
    tool_output("jj", args, cwd)
}

fn command_output_allow_fail(args: &[&str], cwd: &Path) -> Option<String> {
    Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
}

fn root_dir() -> Result<RootInfo, String> {
    if let Ok(text) = command_output(&["rev-parse", "--show-toplevel"], Path::new(".")) {
        return Ok(RootInfo {
            path: PathBuf::from(text.trim()),
            vcs: Vcs::Git,
        });
    }
    jj_output(&["workspace", "root"], Path::new(".")).map(|text| RootInfo {
        path: PathBuf::from(text.trim()),
        vcs: Vcs::Jj,
    })
}

fn is_test_path(path: &str) -> bool {
    path.ends_with(".rs")
        && (is_module_test_path(path)
            || [
                "/tests/",
                "/benches/",
                "/examples/",
                "/fuzz/",
                "workspace_tests",
            ]
            .iter()
            .any(|part| format!("/{path}").contains(part)))
}

fn is_behavior_test_path(path: &str) -> bool {
    path.ends_with(".rs")
        && (is_module_test_path(path)
            || ["/tests/", "workspace_tests"]
                .iter()
                .any(|part| format!("/{path}").contains(part)))
}

fn is_module_test_path(path: &str) -> bool {
    if !path.ends_with(".rs") {
        return false;
    }
    let Some(after_src) = path_after_src(path) else {
        return false;
    };
    is_src_tests_rs_path(after_src) || is_src_tests_child_path(after_src)
}

fn path_after_src(path: &str) -> Option<&str> {
    if let Some(after_src) = path.strip_prefix("src/") {
        return Some(after_src);
    }
    path.split_once("/src/")
        .map(|(_prefix, after_src)| after_src)
}

fn is_src_tests_rs_path(after_src: &str) -> bool {
    after_src == "tests.rs" || after_src.ends_with("/tests.rs")
}

fn is_src_tests_child_path(after_src: &str) -> bool {
    let child = if let Some(child) = after_src.strip_prefix("tests/") {
        child
    } else if let Some((_before_tests, child)) = after_src.rsplit_once("/tests/") {
        child
    } else {
        return false;
    };
    !child.is_empty() && child.ends_with(".rs") && !child.contains('/')
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

fn default_base(root: &Path, vcs: Vcs) -> String {
    if let Ok(value) = std::env::var("TEST_INTEGRITY_BASE") {
        if !value.trim().is_empty() {
            return value;
        }
    }
    if vcs == Vcs::Jj {
        return "@-".to_owned();
    }
    let dirty = command_output(&["status", "--porcelain"], root)
        .map(|text| !text.trim().is_empty())
        .unwrap_or(true);
    if dirty {
        return "HEAD".to_owned();
    }
    command_output_allow_fail(&["merge-base", "origin/main", "HEAD"], root)
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "HEAD".to_owned())
}

fn validate_base_revision(root: &Path, base: &str, vcs: Vcs) -> Result<(), String> {
    if base.trim().is_empty() {
        return Err("empty base revision".to_owned());
    }
    match vcs {
        Vcs::Git => {
            let commit = format!("{base}^{{commit}}");
            command_output(&["rev-parse", "--verify", &commit], root).map(|_| ())
        }
        Vcs::Jj => {
            jj_output(&["log", "--no-graph", "-r", base, "-T", "commit_id"], root).map(|_| ())
        }
    }
    .map_err(|error| format!("invalid base revision {base:?}: {error}"))
}

fn changed_files(root: &Path, base: &str, vcs: Vcs) -> Result<Vec<(String, String)>, String> {
    match vcs {
        Vcs::Git => command_output(
            &["diff", "--name-status", "--find-renames", base, "--"],
            root,
        )
        .map(|text| {
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
        }),
        Vcs::Jj => {
            jj_output(&["diff", "--summary", "--from", base, "--to", "@"], root).map(|text| {
                text.lines()
                    .filter_map(|line| {
                        let trimmed = line.trim();
                        let status = trimmed.chars().next()?.to_string();
                        let path = trimmed.get(1..)?.trim().to_owned();
                        (!path.is_empty()).then_some((status, path))
                    })
                    .collect()
            })
        }
    }
}

fn base_file_has_tests(root: &Path, base: &str, path: &str, vcs: Vcs) -> bool {
    let text = match vcs {
        Vcs::Git => command_output_allow_fail(&["show", &format!("{base}:{path}")], root),
        Vcs::Jj => jj_output(&["file", "show", "-r", base, path], root).ok(),
    };
    text.map(|value| has_test_decl(&value) || has_exact_assertion(&value))
        .unwrap_or(false)
}

fn scan_deleted_files(root: &Path, base: &str, vcs: Vcs) -> Result<Vec<Finding>, String> {
    changed_files(root, base, vcs).map(|entries| {
        entries
            .into_iter()
            .filter(|(status, path)| {
                status.starts_with('D')
                    && (is_test_path(path) || base_file_has_tests(root, base, path, vcs))
            })
            .map(|(_status, path)| Finding {
                kind: "DeletedTestFile",
                path,
                detail: "deleted file contained tests or test assertions".to_owned(),
            })
            .collect()
    })
}

fn diff_text(root: &Path, base: &str, vcs: Vcs) -> Result<String, String> {
    match vcs {
        Vcs::Git => command_output(&["diff", "--find-renames", "--unified=0", base, "--"], root),
        Vcs::Jj => jj_output(&["diff", "--git", "--from", base, "--to", "@"], root),
    }
}

fn scan_diff(diff: &str) -> Vec<Finding> {
    let mut current = String::from("<unknown>");
    let mut removed_test_decl: Vec<(String, String)> = Vec::new();
    let mut added_test_decl: Vec<(String, usize)> = Vec::new();
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
            let Some(payload) = line.strip_prefix('-') else {
                return;
            };
            if has_test_decl(payload) {
                removed_test_decl.push((current.clone(), payload.trim().to_owned()));
            }
            if has_exact_assertion(payload) {
                removed_exact.push((current.clone(), 1));
            }
        } else if line.starts_with('+') && !line.starts_with("+++") {
            let Some(payload) = line.strip_prefix('+') else {
                return;
            };
            if has_test_decl(payload) {
                added_test_decl.push((current.clone(), 1));
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

    let paths = removed_exact
        .iter()
        .map(|(path, _count)| path.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let test_decl_paths = removed_test_decl
        .iter()
        .map(|(path, _detail)| path.clone())
        .collect::<std::collections::BTreeSet<_>>();
    test_decl_paths.into_iter().for_each(|path| {
        let removed_count = removed_test_decl
            .iter()
            .filter(|(candidate, _)| candidate == &path)
            .count();
        let added_count = added_test_decl
            .iter()
            .filter(|(candidate, _)| candidate == &path)
            .count();
        if added_count < removed_count {
            findings.push(Finding {
                kind: "DeletedTestDeclaration",
                path,
                detail: format!(
                    "removed_declarations={removed_count} added_declarations={added_count}"
                ),
            });
        }
    });
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

fn check(root: &Path, base: &str, vcs: Vcs) -> Result<i32, String> {
    validate_base_revision(root, base, vcs)?;
    let mut findings = scan_deleted_files(root, base, vcs)?;
    findings.extend(scan_diff(&diff_text(root, base, vcs)?));
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

fn run_self_test_case(label: &str, test_path: &str, mutate: &str, expected: i32) -> bool {
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
    let test_file = root.join(test_path);
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
        "silent_return" => write(
            &test_file,
            "#[test]\nfn behavior_test() {\n    let Ok(value) = Result::<u8, ()>::Ok(4) else { return; };\n    assert_eq!(value, 4);\n}\n",
        ),
        "silent_return_multiline" => write(
            &test_file,
            "#[test]\nfn behavior_test() {\n    let Ok(value) = Result::<u8, ()>::Ok(4) else {\n        return;\n    };\n    assert_eq!(value, 4);\n}\n",
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
    let actual = check(&root, "HEAD", Vcs::Git).unwrap_or(2);
    let passed = actual == expected;
    println!(
        "SelfTest:{label}: {} expected={expected} actual={actual}",
        if passed { "PASS" } else { "FAIL" }
    );
    passed
}

fn self_test() -> i32 {
    let workspace_test_path = "crates/workspace_tests/tests/behavior.rs";
    let module_tests_rs_path = "crates/vb_runtime/src/action/tests.rs";
    let module_tests_child_path = "crates/vb_runtime/src/action/tests/behavior.rs";
    let cases = [
        ("delete", workspace_test_path, "delete", 1),
        ("ignore", workspace_test_path, "ignore", 1),
        ("weaken", workspace_test_path, "weaken", 1),
        ("strengthen", workspace_test_path, "strengthen", 0),
        ("module-rs-delete", module_tests_rs_path, "delete", 1),
        ("module-rs-weaken", module_tests_rs_path, "weaken", 1),
        (
            "module-rs-silent-return",
            module_tests_rs_path,
            "silent_return",
            1,
        ),
        (
            "module-rs-strengthen",
            module_tests_rs_path,
            "strengthen",
            0,
        ),
        ("module-dir-delete", module_tests_child_path, "delete", 1),
        ("module-dir-weaken", module_tests_child_path, "weaken", 1),
        (
            "module-dir-silent-return",
            module_tests_child_path,
            "silent_return_multiline",
            1,
        ),
        (
            "module-dir-strengthen",
            module_tests_child_path,
            "strengthen",
            0,
        ),
    ];
    let ok = cases.iter().all(|(label, test_path, mutate, expected)| {
        run_self_test_case(label, test_path, mutate, *expected)
    });
    if ok { 0 } else { 1 }
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
        Ok(info) => info,
        Err(error) => {
            eprintln!("{error}");
            return 64;
        }
    };
    let base =
        argument_value(&args, "--base").unwrap_or_else(|| default_base(&root.path, root.vcs));
    match check(&root.path, &base, root.vcs) {
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
