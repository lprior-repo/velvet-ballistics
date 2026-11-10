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
        state.record_value_node(&event, limits)?;
        state.apply_event(&event, limits)?;
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
    fn record_value_node(
        &mut self,
        event: &saphyr_parser::Event<'_>,
        limits: &YamlLimits,
    ) -> YamlResult<()> {
        let is_value = matches!(event,
            saphyr_parser::Event::Scalar(..)
                | saphyr_parser::Event::MappingStart(..)
                | saphyr_parser::Event::SequenceStart(..));
        let is_key = self.in_mapping.last() == Some(&true)
            && self.expecting_key.last() == Some(&true);
        if is_value && !is_key {
            self.record_node(limits)?;
        }
        Ok(())
    }

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
        self.record_sequence_element(limits)?;
        self.increment_depth(limits)?;
        self.in_mapping.push(true);
        self.expecting_key.push(true);
        self.map_counters.push(0);
        self.found_content = true;
        Ok(())
    }

    fn start_sequence(&mut self, limits: &YamlLimits) -> YamlResult<()> {
        self.record_sequence_element(limits)?;
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
        // A null scalar at document root is saphyr's synthesized placeholder
        // for a contentless document (`---`, `--- ...`, explicit null): it is
        // not workflow content, so it must not satisfy the `finish` content
        // gate. Nested nulls are legitimate values and still count.
        if self.in_mapping.last().is_some() || !is_root_null(value) {
            self.found_content = true;
        }
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

    fn end_mapping(&mut self, _limits: &YamlLimits) -> YamlResult<()> {
        // Mapping entries count per mapping only; a child mapping must not
        // inflate its parent's entry count.
        self.map_counters.pop();
        self.in_mapping.pop();
        self.expecting_key.pop();
        self.depth = self.depth.saturating_sub(1);
        self.complete_parent_mapping_value();
        Ok(())
    }

    fn record_sequence_element(&mut self, limits: &YamlLimits) -> YamlResult<()> {
        // A mapping or sequence nested in a sequence is one element of the
        // parent sequence, counted like a scalar element.
        if self.in_mapping.last() == Some(&false) {
            self.record_sequence_scalar(limits)?;
        }
        Ok(())
    }

    fn end_sequence(&mut self, _limits: &YamlLimits) -> YamlResult<()> {
        self.seq_counters.pop();
        self.in_mapping.pop();
        self.depth = self.depth.saturating_sub(1);
        self.complete_parent_mapping_value();
        Ok(())
    }

    fn complete_parent_mapping_value(&mut self) {
        if self.in_mapping.last() == Some(&true)
            && let Some(expecting_key) = self.expecting_key.last_mut()
        {
            *expecting_key = true;
        }
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

/// Null spellings that carry no workflow content at document root. Kept in
/// sync with the null arm of duplicate-key identity so the profile treats
/// "no content" the same everywhere.
fn is_root_null(value: &str) -> bool {
    matches!(value, "" | "~" | "null" | "Null" | "NULL")
}
