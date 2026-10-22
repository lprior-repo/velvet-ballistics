# Exact candidate patch vs own parent

commit 62a2f47c2 (origin/bead/vb-0lx7u, bead/vb-0lx7u)
Author: Lewis <priorlewis43@gmail.com>
Date:   Sun Aug 30 00:00:44 2026 -0500

    fuzz: align YAML target bounds with production max_len
    
    - Add max_len=65536 documentation to yaml_events.rs fuzz target
    - Add max_len=65536 documentation to vb_f04l_yaml_compiler_compile.rs fuzz target
    - Align MAX_FUZZ_PAYLOAD_USIZE in yaml_target.rs from 4096 to 100000
      to match production YamlLimits::max_nodes (100,000)

diff --git a/fuzz/fuzz_targets/vb_f04l_yaml_compiler_compile.rs b/fuzz/fuzz_targets/vb_f04l_yaml_compiler_compile.rs
index cc68897b0..f1868fc05 100644
--- a/fuzz/fuzz_targets/vb_f04l_yaml_compiler_compile.rs
+++ b/fuzz/fuzz_targets/vb_f04l_yaml_compiler_compile.rs
@@ -1,3 +1,7 @@
+//! Fuzz target for YAML compiler with strict profile validation.
+//!
+//! Command: cargo fuzz run vb_f04l_yaml_compiler_compile -- -max_len=65536 -runs=100000
+
 #![no_main]
 
 use libfuzzer_sys::fuzz_target;
diff --git a/fuzz/fuzz_targets/yaml_events.rs b/fuzz/fuzz_targets/yaml_events.rs
index b68482f4c..b135cd5d1 100644
--- a/fuzz/fuzz_targets/yaml_events.rs
+++ b/fuzz/fuzz_targets/yaml_events.rs
@@ -4,6 +4,8 @@
 //! on any UTF-8 input.
 //!
 //! Corpus seeds are maintained in `fuzz/corpus/yaml_events/`.
+//!
+//! Command: cargo fuzz run yaml_events -- -max_len=65536 -runs=100000
 
 #![no_main]
 
diff --git a/fuzz/src/yaml_target.rs b/fuzz/src/yaml_target.rs
index c616bcb35..8739d86b4 100644
--- a/fuzz/src/yaml_target.rs
+++ b/fuzz/src/yaml_target.rs
@@ -13,7 +13,7 @@
 #![allow(clippy::doc_markdown)]
 #![allow(clippy::len_zero)]
 
-const MAX_FUZZ_PAYLOAD_USIZE: usize = 4096;
+const MAX_FUZZ_PAYLOAD_USIZE: usize = 100_000;
 
 fn assert_typed_yaml_error(error: vb_compile::YamlError) {
     use vb_compile::YamlError;


# Actual main fuzz/fuzz_targets/vb_f04l_yaml_compiler_compile.rs

#![no_main]

use libfuzzer_sys::fuzz_target;
use vb_compile::{CompileError, YamlCompiler};

fuzz_target!(|data: &[u8]| {
    let result = YamlCompiler::default().compile(data);
    if let Err(errors) = result {
        for error in errors.iter() {
            match error {
                CompileError::CanonicalYaml { .. }
                | CompileError::EmptySteps
                | CompileError::UnsupportedTopLevelDeclaration { .. }
                | CompileError::UnsupportedTopLevelResult
                | CompileError::UnsupportedStepControlField { .. }
                | CompileError::DuplicateStepId { .. }
                | CompileError::DuplicateOutputName { .. }
                | CompileError::UnknownOutputName { .. }
                | CompileError::StepFieldShape { .. }
                | CompileError::StepIndexOutOfRange { .. }
                | CompileError::SlotIndexOutOfRange { .. }
                | CompileError::PrimitiveLoweringLimitExceeded { .. }
                | CompileError::Workflow(_)
                | CompileError::UnsupportedStepPrimitive { .. } => {}
                _ => {}
            }
        }
    }
});


# Actual main fuzz/fuzz_targets/yaml_events.rs

//! Fuzz target for YAML event parsing.
//!
//! This target verifies that YAML profile validation and event parsing never panic
//! on any UTF-8 input.
//!
//! Corpus seeds are maintained in `fuzz/corpus/yaml_events/`.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    fuzz_lib::fuzz_yaml_events(data);
});


# Actual main fuzz/src/yaml_target.rs

//! YAML parsing fuzzing targets.
//
// The strict fuzz clippy denies `indexing_slicing`, `as_conversions`,
// `let_underscore_must_use`, and `arithmetic_side_effects`. The broad
// `#![allow(...)]` lines that previously suppressed those lints have been
// removed so the strict gate is enforceable. The remaining allows are
// documentary lints the strict command does not deny.
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::items_after_statements)]
#![allow(clippy::doc_markdown)]
#![allow(clippy::len_zero)]

const MAX_FUZZ_PAYLOAD_USIZE: usize = 4096;

fn assert_typed_yaml_error(error: vb_compile::YamlError) {
    use vb_compile::YamlError;
    match error {
        YamlError::UnsupportedTrigger { .. }
        | YamlError::UnsupportedFeature { .. }
        | YamlError::DuplicateKey { .. }
        | YamlError::AnchorAliasMerge
        | YamlError::CustomTag { .. }
        | YamlError::BinaryScalar
        | YamlError::MultipleDocuments { .. }
        | YamlError::AmbiguousScalar { .. }
        | YamlError::SourceTooLarge { .. }
        | YamlError::NestingTooDeep { .. }
        | YamlError::NodeLimitExceeded { .. }
        | YamlError::ScalarTooLong { .. }
        | YamlError::SequenceTooLong { .. }
        | YamlError::MappingTooLarge { .. }
        | YamlError::UnknownField { .. }
        | YamlError::EmptySource
        | YamlError::MissingField { .. }
        | YamlError::FieldShape { .. }
        | YamlError::ParseError { .. }
        | YamlError::ForbiddenFeature { .. }
        | YamlError::LegacyPrimitive { .. } => {}
        _ => {}
    }
}

pub fn fuzz_yaml_events(data: &[u8]) {
    if let Ok(text) = std::str::from_utf8(data) {
        let profile_result = vb_compile::validate_yaml_profile(text);
        match profile_result {
            Ok(()) => {}
            Err(e) => {
                assert_typed_yaml_error(e);
            }
        }

        let events_result = vb_compile::parse_yaml_events(text);
        match events_result {
            Ok(events) => {
                if !text.trim().is_empty() {
                    assert!(
                        !events.is_empty(),
                        "non-empty YAML input must produce non-empty events"
                    );
                }
                assert!(
                    events.len() <= MAX_FUZZ_PAYLOAD_USIZE,
                    "event count {} exceeds max payload bound",
                    events.len()
                );
            }
            Err(e) => {
                assert_typed_yaml_error(e);
            }
        }

        let source_map_result = vb_compile::build_source_map(text);
        match source_map_result {
            Ok(source_map) => {
                assert!(
                    source_map.len() <= MAX_FUZZ_PAYLOAD_USIZE,
                    "source map entries {} exceeds max payload bound",
                    source_map.len()
                );
            }
            Err(e) => {
                assert_typed_yaml_error(e);
            }
        }
    }
}

pub fn fuzz_strict_yaml_profile(data: &[u8]) {
    let compile_result = vb_compile::compile_workflow(data);
    if let Ok(ref workflow) = compile_result {
        let text = String::from_utf8_lossy(data);
        let unsupported =
            text.contains("---") || text.contains('&') || text.contains('*') || text.contains('!');
        assert!(
            !unsupported,
            "unsupported YAML features must cause compile error"
        );
        assert!(
            workflow.node_count() >= 1,
            "compiled workflow must have at least 1 node"
        );
    }
}

pub fn fuzz_compile_source_ast_marks(data: &[u8]) {
    use vb_compile::compile_workflow;

    let result = compile_workflow(data);

    match result {
        Ok(_compiled) => {}
        Err(errors) => {
            assert!(
                !errors.is_empty(),
                "CompileErrors must contain at least one error"
            );
        }
    }
}

pub fn fuzz_span_bridge(data: &[u8]) {
    use vb_compile::{SourceSpan, build_semantic_source_map, build_source_map};

    let mut values = [0usize; 6];
    for (slot, byte) in values.iter_mut().zip(data.iter().copied()) {
        *slot = usize::from(byte);
    }

    let span = SourceSpan::new(
        values[0], values[1], values[2], values[3], values[4], values[5],
    );
    assert_eq!(span.start_offset, values[0]);
    assert_eq!(span.end_offset, values[1]);
    assert_eq!(span.start_line, values[2]);
    assert_eq!(span.start_col, values[3]);
    assert_eq!(span.end_line, values[4]);
    assert_eq!(span.end_col, values[5]);

    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };

    if let Ok(map) = build_source_map(text) {
        for (index, mapped_span) in map.iter() {
            assert_eq!(map.span_for_node(index), Some(mapped_span));
        }
    }

    if let Ok(map) = build_semantic_source_map(text) {
        let _root = map.span_for_path("$");
        let _manual = map.span_for_path("$.when.manual");
        let _step_zero = map.span_for_path("$.steps[0]");
        // `_root`, `_manual`, and `_step_zero` are deliberately bound to
        // underscore-prefixed identifiers so the strict
        // `let_underscore_must_use` lint stays triggered only on
        // bare `let _ = must_use_call()`. They prove the lookup table
        // returns a span option without exercising it.
    }
}


# Actual main crates/vb_yaml/src/yaml_limits.rs

#![forbid(unsafe_code)]

//! Strict YAML profile limits.

/// Strict YAML profile limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YamlLimits {
    /// Maximum source text size in bytes.
    pub max_source_bytes: usize,
    /// Maximum nesting depth.
    pub max_depth: u16,
    /// Maximum total YAML nodes visited.
    pub max_nodes: u32,
    /// Maximum sequence length.
    pub max_sequence_len: usize,
    /// Maximum mapping entry count.
    pub max_mapping_entries: usize,
    /// Maximum scalar value length in bytes.
    pub max_scalar_bytes: usize,
}

impl Default for YamlLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: 1_048_576,
            max_depth: 64,
            max_nodes: 100_000,
            max_sequence_len: 10_000,
            max_mapping_entries: 1_024,
            max_scalar_bytes: 65_536,
        }
    }
}


# Actual main crates/vb_yaml/src/yaml_events_conv.rs

#![forbid(unsafe_code)]
//! Event conversion and collection functions.

use super::events_types::{EventSpan, ScalarStyle, YamlEvent};
use crate::yaml_error::{YamlError, YamlResult};

/// Collect all YAML events from the given source text.
///
/// Returns an owned vector of [`YamlEvent`] values.
pub fn collect_events(text: &str) -> YamlResult<Vec<YamlEvent>> {
    let mut parser = saphyr_parser::Parser::new_from_str(text);
    let mut events = Vec::new();

    while let Some(result) = parser.next_event() {
        let (event, span) = result.map_err(|e| YamlError::ParseError {
            line: e.marker().line(),
            reason: e.info().into(),
        })?;
        events.push(convert_event(event, span));
    }

    Ok(events)
}

/// Convert a saphyr-parser event into our owned YamlEvent.
pub fn convert_event(event: saphyr_parser::Event<'_>, span: saphyr_parser::Span) -> YamlEvent {
    let span = EventSpan::from_parser_span(span);
    match event {
        saphyr_parser::Event::StreamStart => YamlEvent::StreamStart { span },
        saphyr_parser::Event::StreamEnd | saphyr_parser::Event::Nothing => {
            YamlEvent::StreamEnd { span }
        }
        saphyr_parser::Event::DocumentStart(explicit) => {
            YamlEvent::DocumentStart { explicit, span }
        }
        saphyr_parser::Event::DocumentEnd => YamlEvent::DocumentEnd { span },
        saphyr_parser::Event::Alias(anchor_id) => YamlEvent::Alias { anchor_id, span },
        saphyr_parser::Event::Scalar(value, style, anchor_id, tag) => YamlEvent::Scalar {
            value: value.into(),
            style: ScalarStyle::from_parser(style),
            anchor_id,
            tag: tag.map(|t| format_tag(&t)),
            span,
        },
        saphyr_parser::Event::SequenceStart(anchor_id, tag) => YamlEvent::SequenceStart {
            anchor_id,
            tag: tag.map(|t| format_tag(&t)),
            span,
        },
        saphyr_parser::Event::SequenceEnd => YamlEvent::SequenceEnd { span },
        saphyr_parser::Event::MappingStart(anchor_id, tag) => YamlEvent::MappingStart {
            anchor_id,
            tag: tag.map(|t| format_tag(&t)),
            span,
        },
        saphyr_parser::Event::MappingEnd => YamlEvent::MappingEnd { span },
    }
}

/// Format a saphyr-parser Tag into a display string.
fn format_tag(tag: &saphyr_parser::Tag) -> Box<str> {
    format!("{}{}", tag.handle, tag.suffix).into_boxed_str()
}


# Actual main crates/vb_yaml/src/event_collect.rs

#![forbid(unsafe_code)]
//! Event collection with strict YAML profile resource accounting.

use super::{check_null_bytes, check_scalar_length};
use crate::yaml_error::{YamlError, YamlResult};
use crate::yaml_events::{YamlEvent, convert_event};
use crate::yaml_limits::YamlLimits;

/// Collect events from the parser while tracking depth, node counts, and
/// sequence/mapping entry counts for enforcing YamlLimits.
pub(crate) fn collect_and_validate_events(
    text: &str,
    limits: &YamlLimits,
) -> YamlResult<Vec<YamlEvent>> {
    let mut parser = saphyr_parser::Parser::new_from_str(text);
    let mut state = EventCollectionState::new();
    while let Some(result) = parser.next_event() {
        let (event, span) = result.map_err(|e| YamlError::ParseError {
            line: e.marker().line(),
            reason: e.info().into(),
        })?;
        state.apply_event(&event, limits)?;
        state.record_node(limits)?;
        state.events.push(convert_event(event, span));
    }
    state.finish()
}

struct EventCollectionState {
    events: Vec<YamlEvent>,
    depth: u16,
    node_count: u32,
    document_count: usize,
    found_content: bool,
    seq_counters: Vec<usize>,
    map_counters: Vec<usize>,
    in_mapping: Vec<bool>,
    expecting_key: Vec<bool>,
}

impl EventCollectionState {
    fn new() -> Self {
        Self {
            events: Vec::new(),
            depth: 0,
            node_count: 0,
            document_count: 0,
            found_content: false,
            seq_counters: Vec::new(),
            map_counters: Vec::new(),
            in_mapping: Vec::new(),
            expecting_key: Vec::new(),
        }
    }

    fn apply_event(
        &mut self,
        event: &saphyr_parser::Event<'_>,
        limits: &YamlLimits,
    ) -> YamlResult<()> {
        match event {
            saphyr_parser::Event::StreamStart
            | saphyr_parser::Event::StreamEnd
            | saphyr_parser::Event::DocumentEnd
            | saphyr_parser::Event::Alias(_)
            | saphyr_parser::Event::Nothing => Ok(()),
            saphyr_parser::Event::DocumentStart(_) => self.record_document(),
            saphyr_parser::Event::MappingStart(_, _) => self.start_mapping(limits),
            saphyr_parser::Event::SequenceStart(_, _) => self.start_sequence(limits),
            saphyr_parser::Event::Scalar(value, _, _, _) => self.record_scalar(value, limits),
            saphyr_parser::Event::MappingEnd => self.end_mapping(limits),
            saphyr_parser::Event::SequenceEnd => self.end_sequence(limits),
        }
    }

    fn record_document(&mut self) -> YamlResult<()> {
        self.document_count = self
            .document_count
            .checked_add(1)
            .ok_or(YamlError::MultipleDocuments { count: usize::MAX })?;
        Ok(())
    }

    fn start_mapping(&mut self, limits: &YamlLimits) -> YamlResult<()> {
        self.increment_depth(limits)?;
        self.in_mapping.push(true);
        self.expecting_key.push(true);
        self.map_counters.push(0);
        self.found_content = true;
        Ok(())
    }

    fn start_sequence(&mut self, limits: &YamlLimits) -> YamlResult<()> {
        self.increment_depth(limits)?;
        self.in_mapping.push(false);
        self.seq_counters.push(0);
        self.found_content = true;
        Ok(())
    }

    fn increment_depth(&mut self, limits: &YamlLimits) -> YamlResult<()> {
        self.depth = self.depth.checked_add(1).ok_or(YamlError::NestingTooDeep {
            depth: self.depth,
            max: limits.max_depth,
        })?;
        if self.depth > limits.max_depth {
            return Err(YamlError::NestingTooDeep {
                depth: self.depth,
                max: limits.max_depth,
            });
        }
        Ok(())
    }

    fn record_scalar(&mut self, value: &str, limits: &YamlLimits) -> YamlResult<()> {
        check_scalar_length(value, limits.max_scalar_bytes)?;
        check_null_bytes(value)?;
        if let Some(&true) = self.in_mapping.last() {
            self.record_mapping_scalar(limits)?;
        } else if self.in_mapping.last().is_some() {
            self.record_sequence_scalar(limits)?;
        }
        self.found_content = true;
        Ok(())
    }

    fn record_mapping_scalar(&mut self, limits: &YamlLimits) -> YamlResult<()> {
        if self.expecting_key.last() == Some(&true) {
            if let Some(counter) = self.map_counters.last_mut() {
                *counter = counter.checked_add(1).ok_or(YamlError::NodeLimitExceeded {
                    count: u32::MAX,
                    max: limits.max_nodes,
                })?;
                if *counter > limits.max_mapping_entries {
                    return Err(YamlError::MappingTooLarge {
                        count: *counter,
                        max: limits.max_mapping_entries,
                    });
                }
            }
            if let Some(expecting) = self.expecting_key.last_mut() {
                *expecting = false;
            }
        } else if let Some(expecting) = self.expecting_key.last_mut() {
            *expecting = true;
        }
        Ok(())
    }

    fn record_sequence_scalar(&mut self, limits: &YamlLimits) -> YamlResult<()> {
        if let Some(counter) = self.seq_counters.last_mut() {
            *counter = counter.checked_add(1).ok_or(YamlError::NodeLimitExceeded {
                count: u32::MAX,
                max: limits.max_nodes,
            })?;
            if *counter > limits.max_sequence_len {
                return Err(YamlError::SequenceTooLong {
                    len: *counter,
                    max: limits.max_sequence_len,
                });
            }
        }
        Ok(())
    }

    fn end_mapping(&mut self, limits: &YamlLimits) -> YamlResult<()> {
        if let Some((count, parent_count)) =
            self.map_counters.pop().zip(self.map_counters.last_mut())
        {
            *parent_count =
                parent_count
                    .checked_add(count)
                    .ok_or(YamlError::NodeLimitExceeded {
                        count: u32::MAX,
                        max: limits.max_nodes,
                    })?;
        }
        self.in_mapping.pop();
        self.expecting_key.pop();
        self.depth = self.depth.saturating_sub(1);
        Ok(())
    }

    fn end_sequence(&mut self, limits: &YamlLimits) -> YamlResult<()> {
        if let Some((count, parent_count)) =
            self.seq_counters.pop().zip(self.seq_counters.last_mut())
        {
            *parent_count =
                parent_count
                    .checked_add(count)
                    .ok_or(YamlError::NodeLimitExceeded {
                        count: u32::MAX,
                        max: limits.max_nodes,
                    })?;
        }
        self.in_mapping.pop();
        self.depth = self.depth.saturating_sub(1);
        Ok(())
    }

    fn record_node(&mut self, limits: &YamlLimits) -> YamlResult<()> {
        self.node_count = self
            .node_count
            .checked_add(1)
            .ok_or(YamlError::NodeLimitExceeded {
                count: u32::MAX,
                max: limits.max_nodes,
            })?;
        if self.node_count > limits.max_nodes {
            return Err(YamlError::NodeLimitExceeded {
                count: self.node_count,
                max: limits.max_nodes,
            });
        }
        Ok(())
    }

    fn finish(self) -> YamlResult<Vec<YamlEvent>> {
        if !self.found_content || self.document_count == 0 {
            return Err(YamlError::EmptySource);
        }
        Ok(self.events)
    }
}


# Actual main crates/vb_yaml/src/yaml_source_map_build.rs

#![forbid(unsafe_code)]
//! Source map building functions.

use super::source_map_types::{SemanticSourceMap, SourceMap, SourceSpan};
use crate::yaml_error::YamlResult;
use crate::yaml_events::{EventSpan, YamlEvent};

#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingKey {
    value: Box<str>,
    span: EventSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PathContext {
    Mapping {
        path: String,
        pending_key: Option<PendingKey>,
    },
    Sequence {
        path: String,
        next_index: usize,
    },
}

/// Build a source map from YAML text by parsing the event stream.
pub fn build_source_map(text: &str) -> YamlResult<SourceMap> {
    crate::yaml_profile::validate_yaml_profile(text)?;
    let events = crate::yaml_events::collect_events(text)?;
    Ok(source_map_from_events(text, &events))
}

/// Build a source map from a pre-collected event list.
fn source_map_from_events(text: &str, events: &[YamlEvent]) -> SourceMap {
    let mut spans = Vec::new();

    for event in events {
        let span = event.span();
        match event {
            YamlEvent::MappingStart { .. }
            | YamlEvent::SequenceStart { .. }
            | YamlEvent::Scalar { .. } => {
                spans.push(event_span_to_source_span(text, span));
            }
            _ => {}
        }
    }

    SourceMap { spans }
}

/// Convert an EventSpan to a SourceSpan.
fn event_span_to_source_span(text: &str, span: EventSpan) -> SourceSpan {
    let (end_line, end_col) = line_col(text, span.end);
    SourceSpan::new(
        span.start,
        span.end,
        span.line,
        span.column,
        end_line,
        end_col,
    )
}

/// Build a cold semantic source map for commonly diagnosed author paths.
pub fn build_semantic_source_map(text: &str) -> YamlResult<SemanticSourceMap> {
    crate::yaml_profile::validate_yaml_profile(text)?;
    let events = crate::yaml_events::collect_events(text)?;
    Ok(semantic_source_map_from_events(text, &events))
}

fn semantic_source_map_from_events(text: &str, events: &[YamlEvent]) -> SemanticSourceMap {
    let mut map = SemanticSourceMap::default();
    let mut stack = Vec::<PathContext>::new();

    for event in events {
        match event {
            YamlEvent::MappingStart { span, .. } => {
                let path = begin_container_path(text, &mut stack, *span, &mut map);
                stack.push(PathContext::Mapping {
                    path,
                    pending_key: None,
                });
            }
            YamlEvent::SequenceStart { span, .. } => {
                let path = begin_container_path(text, &mut stack, *span, &mut map);
                stack.push(PathContext::Sequence {
                    path,
                    next_index: 0,
                });
            }
            YamlEvent::MappingEnd { .. } | YamlEvent::SequenceEnd { .. } => {
                let _popped = stack.pop();
            }
            YamlEvent::Scalar { value, span, .. } => {
                visit_scalar(text, &mut stack, value.as_ref(), *span, &mut map);
            }
            YamlEvent::StreamStart { .. }
            | YamlEvent::StreamEnd { .. }
            | YamlEvent::DocumentStart { .. }
            | YamlEvent::DocumentEnd { .. }
            | YamlEvent::Alias { .. } => {}
        }
    }

    map
}

fn begin_container_path(
    text: &str,
    stack: &mut [PathContext],
    span: EventSpan,
    map: &mut SemanticSourceMap,
) -> String {
    let Some(parent) = stack.last_mut() else {
        return "$".to_string();
    };
    match parent {
        PathContext::Mapping { path, pending_key } => {
            let Some(key) = pending_key.take() else {
                return path.clone();
            };
            let child = append_mapping_path(path, key.value.as_ref());
            if is_trigger_container_path(&child) {
                map.push(child.clone(), event_span_to_source_span(text, key.span));
            } else {
                map.push(child.clone(), event_span_to_source_span(text, span));
            }
            child
        }
        PathContext::Sequence { path, next_index } => {
            let child = append_sequence_path(path, *next_index);
            *next_index = next_index.saturating_add(1);
            map.push(child.clone(), event_span_to_source_span(text, span));
            child
        }
    }
}

fn visit_scalar(
    text: &str,
    stack: &mut [PathContext],
    value: &str,
    span: EventSpan,
    map: &mut SemanticSourceMap,
) {
    let Some(parent) = stack.last_mut() else {
        map.push("$".to_string(), event_span_to_source_span(text, span));
        return;
    };
    match parent {
        PathContext::Mapping { path, pending_key } => {
            if let Some(key) = pending_key.take() {
                let child = append_mapping_path(path, key.value.as_ref());
                map.push(child, event_span_to_source_span(text, span));
            } else {
                *pending_key = Some(PendingKey {
                    value: Box::<str>::from(value),
                    span,
                });
            }
        }
        PathContext::Sequence { path, next_index } => {
            let child = append_sequence_path(path, *next_index);
            *next_index = next_index.saturating_add(1);
            map.push(child, event_span_to_source_span(text, span));
        }
    }
}

fn append_mapping_path(parent: &str, key: &str) -> String {
    if parent == "$" {
        format!("$.{key}")
    } else {
        format!("{parent}.{key}")
    }
}

fn append_sequence_path(parent: &str, index: usize) -> String {
    format!("{parent}[{index}]")
}

fn is_trigger_container_path(path: &str) -> bool {
    matches!(
        path,
        "$.when.manual" | "$.when.schedule" | "$.when.event" | "$.when.webhook"
    )
}

fn line_col(text: &str, offset: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut col = 1usize;
    let mut cursor = 0usize;
    for ch in text.chars() {
        if cursor >= offset {
            break;
        }
        if ch == '\n' {
            line = line.saturating_add(1);
            col = 1;
        } else {
            col = col.saturating_add(1);
        }
        cursor = match cursor.checked_add(ch.len_utf8()) {
            Some(next) => next,
            None => break,
        };
    }
    (line, col)
}

/// Look up the span for a node by index in a pre-built source map.
pub fn span_for_node(map: &SourceMap, node_index: u32) -> Option<SourceSpan> {
    map.span_for_node(node_index)
}
