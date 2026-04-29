//! Strict YAML profile event checks for the cold compiler boundary.
//!
//! This module intentionally lives in `vb-compiler`, not in runtime crates.
//! Runtime crates never depend on YAML parser crates.

use crate::{CompileError, SourceMark};
use saphyr_parser::{Event, Parser, Span};

/// Rejects YAML stream/profile features before semantic compilation.
pub fn reject_unsupported_profile_events(text: &str) -> Result<(), CompileError> {
    event_profile(text).and_then(StrictYamlProfile::validate)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StrictYamlProfile {
    document_count: usize,
}

impl StrictYamlProfile {
    fn validate(self) -> Result<(), CompileError> {
        match self.document_count {
            0 => Err(CompileError::EmptySource),
            1 => Ok(()),
            count => Err(CompileError::DocumentCount { count }),
        }
    }
}

fn event_profile(text: &str) -> Result<StrictYamlProfile, CompileError> {
    let mut parser = Parser::new_from_str(text);
    std::iter::from_fn(|| match parser.next_event().transpose() {
        Ok(Some(event)) => Some(Ok(event)),
        Ok(None) => None,
        Err(error) => Some(Err(error)),
    })
    .try_fold(0_usize, |count, item| {
        let (event, mark) = item?;
        validate_event(&event, mark)?;
        count_document(count, &event)
    })
    .map(|document_count| StrictYamlProfile { document_count })
}

fn validate_event(event: &Event<'_>, mark: Span) -> Result<(), CompileError> {
    match event {
        Event::Alias(_) => Err(CompileError::AliasForbidden {
            mark: SourceMark::from_parser_span(mark),
        }),
        Event::MappingStart(anchor, tag) | Event::SequenceStart(anchor, tag) => {
            reject_anchor(*anchor, mark)?;
            reject_tag(tag.as_ref(), mark)
        }
        Event::Scalar(_, _, anchor, tag) => {
            reject_anchor(*anchor, mark)?;
            reject_tag(tag.as_ref(), mark)
        }
        _ => Ok(()),
    }
}

fn count_document(count: usize, event: &Event<'_>) -> Result<usize, CompileError> {
    if matches!(event, Event::DocumentStart(_)) {
        count
            .checked_add(1)
            .ok_or(CompileError::DocumentCount { count: usize::MAX })
    } else {
        Ok(count)
    }
}

fn reject_anchor(anchor: usize, mark: Span) -> Result<(), CompileError> {
    if anchor == 0 {
        Ok(())
    } else {
        Err(CompileError::AnchorForbidden {
            mark: SourceMark::from_parser_span(mark),
        })
    }
}

fn reject_tag<T>(tag: Option<&T>, mark: Span) -> Result<(), CompileError> {
    if tag.is_some() {
        Err(CompileError::TagForbidden {
            mark: SourceMark::from_parser_span(mark),
        })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use saphyr_parser::Marker;

    #[test]
    fn classifies_alias_events_as_alias_diagnostics() {
        let mark = Span::empty(Marker::new(4, 1, 5));
        let result = validate_event(&Event::Alias(1), mark);

        assert!(matches!(
            result,
            Err(CompileError::AliasForbidden { mark })
                if mark == SourceMark::from_parser_span(Span::empty(Marker::new(4, 1, 5)))
        ));
    }

    #[test]
    fn classifies_anchored_nodes_as_anchor_diagnostics() {
        let mark = Span::empty(Marker::new(2, 1, 3));
        let result = validate_event(&Event::SequenceStart(7, None), mark);

        assert!(matches!(
            result,
            Err(CompileError::AnchorForbidden { mark })
                if mark == SourceMark::from_parser_span(Span::empty(Marker::new(2, 1, 3)))
        ));
    }
}
