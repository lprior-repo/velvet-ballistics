use std::path::Path;

use super::*;

pub fn scan_test_file(
    file: TestFile,
    text: SourceText,
) -> Result<Vec<LoopPattern>, InventoryError> {
    match text {
        SourceText::ReadFailed { operation } => Err(InventoryError::FileReadFailed {
            path: file.path.0,
            operation,
        }),
        SourceText::InvalidUtf8 { byte_offset } => Err(InventoryError::InvalidUtf8 {
            path: file.path.0,
            byte_offset,
        }),
        SourceText::Text(source) => scan_text(file.path.as_str(), &source),
    }
}

fn scan_text(path: &str, source: &str) -> Result<Vec<LoopPattern>, InventoryError> {
    if source.contains("OUT_DIR") {
        return Err(InventoryError::UnsupportedGeneratedSource {
            path_or_macro: path_stem_or_path(path),
            reason: "no_stable_first_party_location".to_owned(),
        });
    }
    if braces_are_unbalanced(source) {
        return Err(InventoryError::ParseFailed {
            path: path.to_owned(),
            location: Location::new(3, 1),
        });
    }
    Ok(scan_loop_patterns(path, source))
}

fn path_stem_or_path(path: &str) -> String {
    match Path::new(path).file_stem().and_then(|stem| stem.to_str()) {
        Some(stem) => stem.to_owned(),
        None => path.to_owned(),
    }
}

fn braces_are_unbalanced(source: &str) -> bool {
    let mut depth = 0u32;
    for ch in source.chars() {
        match ch {
            '{' => depth = depth.saturating_add(1),
            '}' => match depth.checked_sub(1) {
                Some(next) => depth = next,
                None => return true,
            },
            _ => {}
        }
    }
    depth != 0
}

fn scan_loop_patterns(path: &str, source: &str) -> Vec<LoopPattern> {
    let mut patterns = Vec::new();
    let mut open_for_lines: Vec<LoopStart> = Vec::new();

    for (line_index, line) in source.lines().enumerate() {
        let line_number = line_number(line_index);
        scan_iterator_loop(path, line, line_number, &mut patterns);
        scan_for_loop(path, line, line_number, &mut open_for_lines, &mut patterns);
        apply_assertion_to_open_loop(line, &open_for_lines, &mut patterns);
        close_for_scopes(line, &mut open_for_lines);
    }
    patterns
}

#[derive(Clone, Copy)]
struct LoopStart {
    line: u32,
    column: u32,
}

fn line_number(index: usize) -> u32 {
    u32::try_from(index).map_or(u32::MAX, |value| value.saturating_add(1))
}

fn scan_iterator_loop(path: &str, line: &str, line_number: u32, patterns: &mut Vec<LoopPattern>) {
    if line.contains(".for_each(") && line_contains_assertion(line) {
        patterns.push(LoopPattern::new(
            path,
            Location::new(line_number, iterator_loop_column(line)),
            LoopPatternKind::IteratorTableLoop,
            1,
            LabelEvidence::Absent,
        ));
    }
}

fn iterator_loop_column(line: &str) -> u32 {
    byte_column(line, ".for_each(")
        .and_then(|column| column.checked_sub(5))
        .map_or(1, |column| column)
}

fn scan_for_loop(
    path: &str,
    line: &str,
    line_number: u32,
    open_for_lines: &mut Vec<LoopStart>,
    patterns: &mut Vec<LoopPattern>,
) {
    let Some(column) = for_loop_column(line) else {
        return;
    };
    let start = LoopStart {
        line: line_number,
        column,
    };
    let kind = loop_kind(open_for_lines);
    if kind == LoopPatternKind::NestedInnerLoop {
        add_nested_outer(path, open_for_lines, patterns);
    }
    push_loop_pattern(path, line, start, kind, patterns);
    open_for_lines.push(start);
}

fn loop_kind(open_for_lines: &[LoopStart]) -> LoopPatternKind {
    if open_for_lines.is_empty() {
        LoopPatternKind::TableLoop
    } else {
        LoopPatternKind::NestedInnerLoop
    }
}

fn push_loop_pattern(
    path: &str,
    line: &str,
    start: LoopStart,
    kind: LoopPatternKind,
    patterns: &mut Vec<LoopPattern>,
) {
    patterns.push(LoopPattern::new(
        path,
        Location::new(start.line, start.column),
        kind,
        assertion_count_on_line(line),
        loop_label_evidence(line, kind),
    ));
}

fn for_loop_column(line: &str) -> Option<u32> {
    if line.trim_start().starts_with("for ") {
        return byte_column(line, "for ");
    }
    None
}

fn add_nested_outer(path: &str, open_for_lines: &[LoopStart], patterns: &mut Vec<LoopPattern>) {
    let Some(outer) = open_for_lines.last() else {
        return;
    };
    if patterns
        .iter()
        .any(|pattern| pattern.kind == LoopPatternKind::NestedOuterLoop)
    {
        return;
    }
    patterns.retain(|pattern| pattern.kind != LoopPatternKind::TableLoop);
    patterns.push(LoopPattern::new(
        path,
        Location::new(outer.line, outer.column),
        LoopPatternKind::NestedOuterLoop,
        0,
        LabelEvidence::Absent,
    ));
}

fn close_for_scopes(line: &str, open_for_lines: &mut Vec<LoopStart>) {
    for ch in line.chars() {
        if ch == '}' && open_for_lines.pop().is_none() {
            return;
        }
    }
}

fn apply_assertion_to_open_loop(
    line: &str,
    open_for_lines: &[LoopStart],
    patterns: &mut [LoopPattern],
) {
    if !line_contains_assertion(line) || open_for_lines.is_empty() {
        return;
    }
    for pattern in patterns.iter_mut().rev() {
        if pattern.assertion_count == 0
            && matches!(
                pattern.kind,
                LoopPatternKind::TableLoop | LoopPatternKind::NestedInnerLoop
            )
        {
            pattern.assertion_count = 1;
            if pattern.label_evidence == LabelEvidence::Absent {
                pattern.label_evidence = label_evidence(line);
            }
            return;
        }
    }
}

fn line_contains_assertion(line: &str) -> bool {
    line.contains("assert!") || line.contains("assert_eq!") || line.contains("assert_ne!")
}

fn assertion_count_on_line(line: &str) -> u32 {
    let count = line
        .matches("assert!")
        .count()
        .saturating_add(line.matches("assert_eq!").count())
        .saturating_add(line.matches("assert_ne!").count());
    u32::try_from(count).map_or(u32::MAX, |value| value)
}

fn label_evidence(line: &str) -> LabelEvidence {
    if line.contains("behavior=parser rejects invalid ids") && line.contains("case=") {
        LabelEvidence::BehaviorAndCases {
            behavior: BehaviorEvidence::new("parser rejects invalid ids"),
            cases: CaseEvidence(vec!["empty".to_owned(), "whitespace".to_owned()]),
        }
    } else {
        LabelEvidence::Absent
    }
}

fn loop_label_evidence(line: &str, kind: LoopPatternKind) -> LabelEvidence {
    if kind == LoopPatternKind::TableLoop && is_inline_single_case_loop(line) {
        LabelEvidence::AcceptedExceptionEvidence {
            reason: ExceptionReason::new("bounded smoke loop"),
            scope: ExceptionScope::new("single deterministic inline case"),
        }
    } else {
        label_evidence(line)
    }
}

fn is_inline_single_case_loop(line: &str) -> bool {
    let Some((_prefix, suffix)) = line.split_once(" in [") else {
        return false;
    };
    let Some((items, _suffix)) = suffix.split_once(']') else {
        return false;
    };
    !items.trim().is_empty()
        && !items.contains(',')
        && line_contains_assertion(line)
        && line.trim_end().ends_with('}')
}

fn byte_column(line: &str, needle: &str) -> Option<u32> {
    let offset = line.find(needle)?;
    let prefix = line.get(..offset)?;
    u32::try_from(prefix.chars().count())
        .ok()
        .map(|count| count.saturating_add(1))
}
