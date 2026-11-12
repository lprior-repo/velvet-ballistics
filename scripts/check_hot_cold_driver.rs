use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;

use super::{HOT_CRATES, scan};

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

pub(crate) fn run() -> i32 {
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
