use super::{decode_registry, extract_line, read_registry, run, select_targets, write_targets};
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn fixture_path(name: &str) -> io::Result<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.ancestors().nth(2).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "manifest directory has no repository root",
        )
    })?;
    Ok(root
        .join("target/native-tools/verus-registry-targets/test-inputs")
        .join(std::process::id().to_string())
        .join(name))
}

fn write_registry_fixture(name: &str, contents: &[u8]) -> io::Result<PathBuf> {
    let path = fixture_path(name)?;
    let directory = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "fixture path has no parent directory",
        )
    })?;
    fs::create_dir_all(directory)?;
    fs::write(&path, contents)?;
    Ok(path)
}

#[test]
fn selects_every_section_in_first_order_and_deduplicates_exact_targets() {
    let input = "metadata:\n  verus: first.rs\nnotes:\n  verus: second.rs\nverus: first.rs\nverus: first.rs/\n";
    assert_eq!(
        select_targets(input),
        Ok(vec!["first.rs", "second.rs", "first.rs/"])
    );
}

#[test]
fn preserves_quoted_unquoted_whitespace_and_inline_comment_selection() {
    let input = "  verus: \" quoted.rs \"  # note\nverus: 'single.rs'\nverus:\tbare.rs\n";
    assert_eq!(
        select_targets(input),
        Ok(vec!["quoted.rs", "single.rs", "bare.rs"])
    );
}

#[test]
fn matches_independent_optional_quotes_and_rejects_malformed_quote_tails() {
    assert_eq!(
        extract_line("verus: 'mismatched.rs\""),
        Some("mismatched.rs")
    );
    assert_eq!(
        extract_line("verus: \"missing-close.rs"),
        Some("missing-close.rs")
    );
    assert_eq!(
        extract_line("verus: unquoted-close.rs'"),
        Some("unquoted-close.rs")
    );
    assert_eq!(extract_line("verus: trailing.rs\"junk"), None);
    assert_eq!(extract_line("verus: \"tail.rs\"junk"), None);
}

#[test]
fn ignores_comments_other_keys_and_unmatched_lines() {
    let input = "# verus: commented.rs\nproof: other.rs\nnot-verus: no.rs\nverus : spaced-key.rs\n";
    assert_eq!(select_targets(input), Err(super::SelectionError::NoTargets));
}

#[test]
fn rejects_empty_and_targetless_documents() {
    assert_eq!(select_targets(""), Err(super::SelectionError::NoTargets));
    assert_eq!(
        select_targets("verus:   # comment\n"),
        Err(super::SelectionError::NoTargets)
    );
}

#[test]
fn recognizes_python_unicode_whitespace_and_splitlines_boundaries() {
    let input = "\u{1c}verus:\u{a0}one.rs\u{85}verus: \u{2028}verus: two.rs\u{2029}verus:\u{1f}three.rs\r\nverus:\u{b}verus: four.rs\u{c}verus: five.rs";
    assert_eq!(
        select_targets(input),
        Ok(vec!["one.rs", "two.rs", "three.rs", "four.rs", "five.rs"])
    );
}

#[test]
fn reports_usage_and_missing_file_with_failure_status() -> io::Result<()> {
    let mut output = Vec::new();
    let mut diagnostics = Vec::new();
    assert_eq!(
        run(Vec::<OsString>::new(), &mut output, &mut diagnostics),
        ExitCode::from(2)
    );
    assert!(diagnostics.starts_with(b"usage:"));
    diagnostics.clear();
    let missing = fixture_path("missing-registry.yaml")?;
    assert_eq!(
        run([missing.into_os_string()], &mut output, &mut diagnostics),
        ExitCode::FAILURE
    );
    assert!(diagnostics.starts_with(b"failed to read registry:"));
    assert_eq!(output, b"");
    diagnostics.clear();
    let args = [OsString::from("one"), OsString::from("two")];
    assert_eq!(run(args, &mut output, &mut diagnostics), ExitCode::from(2));
    assert!(diagnostics.starts_with(b"usage:"));
    Ok(())
}

#[test]
fn returns_failure_for_a_registry_without_selected_targets() -> io::Result<()> {
    let path = write_registry_fixture("empty-registry.yaml", b"proof: no-target.rs\n")?;
    let mut output = Vec::new();
    let mut diagnostics = Vec::new();
    assert_eq!(
        run([path.into_os_string()], &mut output, &mut diagnostics),
        ExitCode::FAILURE
    );
    assert!(diagnostics.starts_with(b"no Verus targets found"));
    assert_eq!(output, b"");
    Ok(())
}

#[test]
fn returns_failure_for_invalid_utf8_input() -> io::Result<()> {
    let path = write_registry_fixture("invalid-utf8-registry.yaml", &[0xff])?;
    let mut output = Vec::new();
    let mut diagnostics = Vec::new();
    assert_eq!(
        run([path.into_os_string()], &mut output, &mut diagnostics),
        ExitCode::FAILURE
    );
    assert!(diagnostics.starts_with(b"registry is not valid UTF-8:"));
    assert_eq!(output, b"");
    Ok(())
}

#[test]
fn rejects_invalid_utf8_as_a_typed_error() {
    assert!(matches!(
        decode_registry(vec![0xff]),
        Err(super::Failure::InvalidUtf8(_))
    ));
}

struct FailingWriter;

impl Write for FailingWriter {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::from(io::ErrorKind::BrokenPipe))
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::from(io::ErrorKind::BrokenPipe))
    }
}

#[test]
fn propagates_output_write_failure() {
    let result = write_targets(&[String::from("target.rs")], &mut FailingWriter);
    assert_eq!(
        result.map_err(|error| error.kind()),
        Err(io::ErrorKind::BrokenPipe)
    );
}

#[test]
fn returns_failure_when_stdout_rejects_selected_targets() -> io::Result<()> {
    let path = write_registry_fixture("output-failure-registry.yaml", b"verus: target.rs\n")?;
    let mut diagnostics = Vec::new();
    assert_eq!(
        run(
            [path.into_os_string()],
            &mut FailingWriter,
            &mut diagnostics
        ),
        ExitCode::FAILURE
    );
    assert!(diagnostics.starts_with(b"failed to write targets:"));
    Ok(())
}

#[test]
fn writes_each_target_with_a_trailing_newline() {
    let mut output = Vec::new();
    let targets = [String::from("first.rs"), String::from("second.rs")];
    assert_eq!(
        write_targets(&targets, &mut output).map_err(|error| error.kind()),
        Ok(())
    );
    assert_eq!(output, b"first.rs\nsecond.rs\n");
}

#[test]
fn reports_file_read_failure_without_emitting_targets() -> io::Result<()> {
    let missing = fixture_path("missing-read-registry.yaml")?;
    let result = read_registry(&missing);
    assert!(matches!(result, Err(super::Failure::Read(_))));
    Ok(())
}

#[derive(Default)]
struct FlushFailingWriter {
    bytes: Vec<u8>,
}

impl Write for FlushFailingWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "flush failed"))
    }
}

#[test]
fn propagates_flush_failure_after_writing_every_target_byte() {
    let mut output = FlushFailingWriter::default();
    let targets = [String::from("first.rs"), String::from("second.rs")];
    let result = write_targets(&targets, &mut output);
    assert_eq!(output.bytes, b"first.rs\nsecond.rs\n");
    assert_eq!(
        result.map_err(|error| error.kind()),
        Err(io::ErrorKind::BrokenPipe)
    );
}

#[test]
fn returns_failure_when_stdout_flush_fails_after_all_writes() -> io::Result<()> {
    let path = write_registry_fixture(
        "flush-failure-registry.yaml",
        b"verus: first.rs\nverus: second.rs\n",
    )?;
    let mut output = FlushFailingWriter::default();
    let mut diagnostics = Vec::new();
    assert_eq!(
        run([path.into_os_string()], &mut output, &mut diagnostics),
        ExitCode::FAILURE
    );
    assert_eq!(output.bytes, b"first.rs\nsecond.rs\n");
    assert_eq!(diagnostics, b"failed to write targets: flush failed\n");
    Ok(())
}
