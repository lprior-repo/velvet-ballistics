//! Abrupt process crash/reopen tests for the atomic local register batch.
//! The child aborts after `commit_effect` and before `execute` persists/acknowledges.

use super::test_support::scratch_grant;
use super::{ExecutorError, FjallRegister};
use std::path::Path;
use std::process::{Command, Output};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const CHILD_TEST: &str = "executor::register_crash_tests::child_commit_then_abort";

fn spawn_child(mode: &str) -> Result<(tempfile::TempDir, Output), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("register");
    let output = Command::new(std::env::current_exe()?)
        .args(["--exact", CHILD_TEST, "--nocapture", "--test-threads=1"])
        .env("VB_CRASH_REGISTER_PATH", &path)
        .env("VB_CRASH_MODE", mode)
        .output()?;
    Ok((dir, output))
}

#[test]
fn abrupt_crash_after_commit_reopens_all_rows_and_duplicate_retries() -> TestResult {
    let (dir, output) = spawn_child("commit")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("child-committed"),
        "child must reach the commit marker: {stdout}"
    );
    assert!(!output.status.success(), "child must abort");
    let mut adapter = FjallRegister::open(&dir.path().join("register"), 2)?;
    let grant = scratch_grant();
    assert_eq!(adapter.effect_count()?, 1);
    assert_eq!(adapter.value(grant.scope)?, Some(42));
    let receipt = adapter.reconcile(&grant, grant.ticket, 42)?;
    assert_eq!(receipt.value(), 42);
    let duplicate = adapter.execute(&grant, grant.ticket, 42)?;
    assert_eq!(duplicate, receipt);
    assert_eq!(adapter.effect_count()?, 1);
    Ok(())
}

#[test]
fn abrupt_crash_before_commit_leaves_no_partial_rows() -> TestResult {
    let (dir, output) = spawn_child("before")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("child-ready"),
        "child must open the register before aborting: {stdout}"
    );
    assert!(!output.status.success(), "child must abort");
    let mut adapter = FjallRegister::open(&dir.path().join("register"), 2)?;
    let grant = scratch_grant();
    assert_eq!(adapter.effect_count()?, 0);
    assert_eq!(adapter.value(grant.scope)?, None);
    assert!(matches!(
        adapter.reconcile(&grant, grant.ticket, 42),
        Err(ExecutorError::ReceiptMissing)
    ));
    Ok(())
}

#[test]
fn child_commit_then_abort() {
    let Ok(path) = std::env::var("VB_CRASH_REGISTER_PATH") else {
        return;
    };
    let Ok(mut adapter) = FjallRegister::open(Path::new(&path), 2) else {
        std::process::exit(3);
    };
    println!("child-ready");
    if matches!(std::env::var("VB_CRASH_MODE").as_deref(), Ok("commit")) {
        let grant = scratch_grant();
        if adapter.commit_effect_for_crash_test(&grant, 42).is_ok() {
            println!("child-committed");
        }
    }
    std::process::abort();
}
