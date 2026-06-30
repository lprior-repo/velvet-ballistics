use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const HOT_CRATES: &[&str] = &["vb_core", "vb_runtime", "vb_storage", "vb_ipc"];
const COLD_MARKERS: &[&str] = &[
    "diagnostic",
    "diagnostics",
    "fixture",
    "fixtures",
    "harness",
    "kani",
    "loom",
    "proof",
    "property",
    "proptest",
    "proptests",
    "support",
    "test_util",
    "tests",
    "verification",
];

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Finding {
    rel_path: String,
    line_no: usize,
    class_id: &'static str,
    text: String,
}

fn without_comment(line: &str) -> &str {
    line.split_once("//").map_or(line, |(prefix, _comment)| prefix)
}

fn compact(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn remove_spaces(line: &str) -> String {
    line.chars().filter(|ch| !ch.is_whitespace()).collect()
}

fn is_cold_path(path: &str) -> bool {
    path.split(['/', '.', '_', '-'])
        .any(|token| COLD_MARKERS.iter().any(|marker| token == *marker))
}

fn line_has_string_map(line: &str) -> bool {
    let normalized = remove_spaces(line);
    [
        "HashMap<String",
        "HashMap<&str",
        "BTreeMap<String",
        "BTreeMap<&str",
        "IndexMap<String",
        "IndexMap<&str",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn classify_line(rel_path: &str, line_no: usize, raw_line: &str) -> Vec<Finding> {
    let stripped = without_comment(raw_line).trim();
    if stripped.is_empty() || stripped.starts_with('#') || stripped.starts_with("use ") {
        return Vec::new();
    }
    let text = compact(stripped);
    let checks = [
        ("FORMAT-PRINT-001", stripped.contains("println!(") || stripped.contains("eprintln!(")),
        ("FORMAT-DBG-001", stripped.contains("dbg!(")),
        (
            "FORMAT-JSON-001",
            stripped.contains("serde_json") || stripped.contains("serde_json::Value"),
        ),
        (
            "FORMAT-YAML-001",
            stripped.contains("serde_saphyr") || stripped.contains("saphyr::") || stripped.contains(" saphyr"),
        ),
        ("MAP-STRING-001", line_has_string_map(stripped)),
        (
            "CHANNEL-UNBOUNDED-001",
            stripped.contains("std::sync::mpsc::channel(")
                || stripped.contains("mpsc::channel(")
                || stripped.contains("unbounded_channel(")
                || stripped.contains("crossbeam_channel::unbounded("),
        ),
    ];

    checks
        .into_iter()
        .filter_map(|(class_id, matched)| {
            matched.then(|| Finding {
                rel_path: rel_path.to_owned(),
                line_no,
                class_id,
                text: text.clone(),
            })
        })
        .collect()
}

fn rust_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    fs::read_dir(root)?.try_fold(Vec::new(), |mut acc, entry| {
        let path = entry?.path();
        if path.is_dir() {
            acc.extend(rust_files(&path)?);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            acc.push(path);
        }
        Ok(acc)
    })
}

fn hot_sources(root: &Path) -> io::Result<Vec<PathBuf>> {
    HOT_CRATES.iter().try_fold(Vec::new(), |mut acc, crate_name| {
        let src = root.join("crates").join(crate_name).join("src");
        acc.extend(rust_files(&src)?);
        Ok(acc)
    })
}

fn load_allow_file(root: &Path) -> Result<BTreeSet<(String, String)>, String> {
    let allow_path = root.join("scripts/hot-cold-forbidden-apis.allow");
    if !allow_path.exists() {
        return Ok(BTreeSet::new());
    }
    let text = fs::read_to_string(&allow_path)
        .map_err(|error| format!("scripts/hot-cold-forbidden-apis.allow: unreadable: {error}"))?;
    text.lines()
        .enumerate()
        .try_fold(BTreeSet::new(), |mut acc, (index, line)| {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                return Ok(acc);
            }
            let parts = trimmed.split('|').collect::<Vec<_>>();
            if parts.len() != 6 {
                return Err(format!(
                    "MalformedException: scripts/hot-cold-forbidden-apis.allow:{} expected path|class|owner=...|reviewed_by=...|test=...|reason=...",
                    index.saturating_add(1)
                ));
            }
            if parts[0].contains('*') || !parts[0].starts_with("crates/") || !parts[0].ends_with(".rs") {
                return Err(format!(
                    "OverbroadException: scripts/hot-cold-forbidden-apis.allow:{} path must be exact crates/*/src/*.rs",
                    index.saturating_add(1)
                ));
            }
            if parts[1] == "ALL" || parts[1].contains('*') {
                return Err(format!(
                    "OverbroadException: scripts/hot-cold-forbidden-apis.allow:{} class must be exact",
                    index.saturating_add(1)
                ));
            }
            if !parts[2].starts_with("owner=")
                || !parts[3].starts_with("reviewed_by=")
                || !parts[4].starts_with("test=")
                || !parts[5].starts_with("reason=")
            {
                return Err(format!(
                    "MalformedException: scripts/hot-cold-forbidden-apis.allow:{} missing owner/reviewed_by/test/reason",
                    index.saturating_add(1)
                ));
            }
            acc.insert((parts[0].to_owned(), parts[1].to_owned()));
            Ok(acc)
        })
}

fn scan(root: &Path) -> Result<(Vec<String>, Vec<Finding>, Vec<Finding>), String> {
    let allowed = load_allow_file(root)?;
    let sources = hot_sources(root).map_err(|error| format!("hot source scan failed: {error}"))?;
    sources.into_iter().try_fold(
        (Vec::new(), Vec::new(), Vec::new()),
        |(mut classified, mut violations, mut justified), source| {
            let rel_path = source
                .strip_prefix(root)
                .map_or_else(|_| source.display().to_string(), |path| path.display().to_string());
            let cold = is_cold_path(&rel_path);
            classified.push(format!("ClassifiedPath|{}|{}", if cold { "cold" } else { "hot" }, rel_path));
            if cold {
                return Ok((classified, violations, justified));
            }
            let text = fs::read_to_string(&source)
                .map_err(|error| format!("{}: unreadable: {error}", source.display()))?;
            let mut cfg_test_pending = false;
            let mut test_depth = 0_i32;
            text.lines().enumerate().for_each(|(index, line)| {
                let trimmed = line.trim();
                if test_depth > 0 {
                    test_depth += line.matches('{').count() as i32;
                    test_depth -= line.matches('}').count() as i32;
                    return;
                }
                if trimmed.starts_with("#[cfg(test)]") {
                    cfg_test_pending = true;
                    return;
                }
                if cfg_test_pending && trimmed.contains("mod ") {
                    test_depth = line.matches('{').count() as i32 - line.matches('}').count() as i32;
                    if test_depth <= 0 {
                        test_depth = 1;
                    }
                    cfg_test_pending = false;
                    return;
                }
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    cfg_test_pending = false;
                }
                classify_line(&rel_path, index.saturating_add(1), line)
                    .into_iter()
                    .for_each(|finding| {
                        if allowed.contains(&(finding.rel_path.clone(), finding.class_id.to_owned())) {
                            justified.push(finding);
                        } else {
                            violations.push(finding);
                        }
                    });
            });
            Ok((classified, violations, justified))
        },
    )
}

fn write_fixture(path: &Path, text: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)
}

fn self_test() -> i32 {
    let root = std::env::temp_dir().join(format!(
        "hot-cold-scan-{}",
        std::process::id()
    ));
    let cleanup_result = fs::remove_dir_all(&root);
    match cleanup_result {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => {
            eprintln!("FixtureFailure: cleanup failed: {error}");
            return 1;
        }
    }
    let hot = root.join("crates/vb_runtime/src/engine.rs");
    let cold = root.join("crates/vb_runtime/src/diagnostic.rs");
    let writes = write_fixture(
        &hot,
        "pub fn bad() { println!(\"x\"); let _m: HashMap<String, u8> = HashMap::new(); let _c = std::sync::mpsc::channel(); }\n",
    )
    .and_then(|()| write_fixture(&cold, "pub fn ok() { println!(\"diagnostic only\"); }\n"));
    if let Err(error) = writes {
        eprintln!("FixtureFailure: write failed: {error}");
        return 1;
    }
    let result = scan(&root);
    let classes = match result {
        Ok((_classified, violations, _justified)) => violations
            .into_iter()
            .map(|finding| finding.class_id)
            .collect::<BTreeSet<_>>(),
        Err(error) => {
            eprintln!("FixtureFailure: scan failed: {error}");
            return 1;
        }
    };
    let required = ["FORMAT-PRINT-001", "MAP-STRING-001", "CHANNEL-UNBOUNDED-001"];
    let missing = required
        .iter()
        .filter(|class_id| !classes.contains(**class_id))
        .copied()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        eprintln!("FixtureFailure: missing classes {missing:?}");
        return 1;
    }
    println!("FixturePass: hot/cold forbidden API scanner");
    0
}

fn run() -> i32 {
    if std::env::args().any(|arg| arg == "--self-test") {
        return self_test();
    }
    let root = match std::env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            eprintln!("InvalidInvocation: cannot read current directory: {error}");
            return 64;
        }
    };
    match scan(&root) {
        Ok((classified, violations, justified)) => {
            classified.iter().for_each(|line| println!("{line}"));
            justified.iter().for_each(|finding| {
                println!(
                    "JustifiedException|{}|{}|line={}",
                    finding.class_id, finding.rel_path, finding.line_no
                );
            });
            violations.iter().for_each(|finding| {
                println!(
                    "ViolationFound|{}|{}|line={}|{}",
                    finding.class_id, finding.rel_path, finding.line_no, finding.text
                );
            });
            println!(
                "ScanSummary|hot_crates={}|classified={}|violations={}|justified={}",
                HOT_CRATES.join(","),
                classified.len(),
                violations.len(),
                justified.len()
            );
            if violations.is_empty() { 0 } else { 2 }
        }
        Err(error) => {
            eprintln!("{error}");
            3
        }
    }
}

fn main() {
    std::process::exit(run());
}
