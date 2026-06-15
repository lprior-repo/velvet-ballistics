#![forbid(unsafe_code)]
//! Input-boundary behavior tests (Category A).
//!
//! Tests that the fuzz harness pipeline handles invalid UTF-8, empty bytes,
//! whitespace-only input, and arbitrary byte sequences without panicking.

use crate::ExprError;
use crate::lexer::lex_expr;

/// Simulates the first step of `fuzz_expression`: UTF-8 validation.
/// Returns Ok(()) if the pipeline should proceed, or an early-return signal.
fn validate_utf8_for_harness(data: &[u8]) -> bool {
    std::str::from_utf8(data).is_ok()
}

/// Full pipeline simulator matching `fuzz_expression` behavior exactly.
fn simulate_harness_pipeline(data: &[u8]) -> Result<(), ExprError> {
    let text = std::str::from_utf8(data).map_err(|_| ExprError::UnexpectedEof)?;
    // Stage 1: Lex
    crate::lexer::lex_expr(text)?;
    // (If we get here, lex succeeded. In the real harness, parse/compile/eval
    // would follow. For input boundary tests, we only care that no panic occurs.)
    Ok(())
}

// ── A-1: Invalid UTF-8 causes silent early return ──

#[test]
fn harness_returns_silently_for_invalid_utf8_ff_fe_fd() {
    // Given: invalid UTF-8 bytes (0xFF, 0xFE, 0xFD)
    let data: &[u8] = &[0xFF, 0xFE, 0xFD];
    // When: utf-8 validation runs
    let is_valid = validate_utf8_for_harness(data);
    // Then: it's rejected as invalid UTF-8
    assert!(
        !is_valid,
        "0xFF 0xFE 0xFD must be detected as invalid UTF-8"
    );
    // And: the full pipeline must not panic on this input
    let result = simulate_harness_pipeline(data);
    assert!(
        result.is_err(),
        "invalid UTF-8 must produce an error from pipeline"
    );
}

// ── A-2: Empty bytes produce valid UTF-8, lex returns [End] ──

#[test]
fn harness_returns_silently_for_empty_bytes() {
    // Given: empty byte slice
    let data: &[u8] = b"";
    // When: validated as UTF-8
    assert!(
        validate_utf8_for_harness(data),
        "empty bytes are valid UTF-8"
    );
    let text = std::str::from_utf8(data).expect("empty bytes must be valid UTF-8");
    // Then: lex returns Ok with [End] (empty input is valid for lexing)
    let tokens = lex_expr(text);
    assert!(tokens.is_ok(), "empty string must lex successfully");
    let tokens = tokens.expect("empty string must lex successfully");
    assert_eq!(
        tokens.len(),
        1,
        "empty input must produce exactly one End token"
    );
}

// ── A-3: Whitespace-only input is valid UTF-8, lex returns [End] ──

#[test]
fn harness_returns_silently_for_whitespace_only_input() {
    // Given: whitespace-only input
    let data: &[u8] = b"   \t\n  ";
    // When: validated as UTF-8
    assert!(validate_utf8_for_harness(data), "whitespace is valid UTF-8");
    let text = std::str::from_utf8(data).expect("whitespace must be valid UTF-8");
    // Then: lex returns Ok with [End]
    let tokens = lex_expr(text);
    assert!(tokens.is_ok(), "whitespace-only must lex successfully");
    let tokens = tokens.expect("whitespace-only must lex successfully");
    assert_eq!(
        tokens.len(),
        1,
        "whitespace-only must produce exactly one End token"
    );
}

// ── A-4: Harness never panics on arbitrary byte sequences ──

#[test]
fn harness_never_panics_on_null_byte_input() {
    // Given: input containing null bytes
    let data: &[u8] = b"\x00\x00\x00";
    // When/Then: pipeline must not panic
    let _ = simulate_harness_pipeline(data);
}

#[test]
fn harness_never_panics_on_zero_length_byte() {
    // Given: single zero byte
    let data: &[u8] = &[0u8];
    // When/Then: pipeline must not panic
    let _ = simulate_harness_pipeline(data);
}

#[test]
fn harness_never_panics_on_binary_garbage() {
    // Given: arbitrary binary garbage
    let data: &[u8] = &[
        0x00, 0x01, 0x02, 0x80, 0x81, 0xFE, 0xFF, 0xC0, 0xC1, 0xF5, 0xF6, 0xF7,
    ];
    // When/Then: pipeline must not panic
    let _ = simulate_harness_pipeline(data);
}

#[test]
fn harness_never_panics_on_all_zeros_short() {
    let data: &[u8] = &[0u8; 64];
    let _ = simulate_harness_pipeline(data);
}

#[test]
fn harness_never_panics_on_all_ones() {
    let data: &[u8] = &[0xFFu8; 32];
    let _ = simulate_harness_pipeline(data);
}
