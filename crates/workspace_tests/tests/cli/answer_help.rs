use std::error::Error;
use std::process::Command;

type TestResult = Result<(), Box<dyn Error>>;

const ANSWER_USAGE: &str =
    "  answer     <run_id> --slot <N> --value <file> --db <path> [--emit text|yaml|postcard]";

fn successful_stdout(args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new(env!("CARGO_BIN_EXE_velvet-ballistics"))
        .args(args)
        .output()?;
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stderr.is_empty(), "stderr={:?}", output.stderr);
    Ok(String::from_utf8(output.stdout)?)
}

#[test]
fn global_help_shows_canonical_answer_selector() -> TestResult {
    let stdout = successful_stdout(&["help"])?;
    let answer_lines: Vec<&str> = stdout
        .lines()
        .filter(|line| line.split_whitespace().next() == Some("answer"))
        .collect();
    assert_eq!(answer_lines, [ANSWER_USAGE]);
    Ok(())
}

#[test]
fn scoped_answer_help_shows_canonical_selector() -> TestResult {
    let expected = format!(
        "velvet-ballistics usage for 'answer':\n{ANSWER_USAGE}\n\nrun `velvet-ballistics help` for all commands\n"
    );
    [["answer", "--help"], ["answer", "-h"]]
        .into_iter()
        .try_for_each(|args| -> TestResult {
            assert_eq!(successful_stdout(&args)?, expected);
            Ok(())
        })?;
    assert_eq!(
        successful_stdout(&["help", "answer"])?,
        successful_stdout(&["help"])?
    );
    Ok(())
}
