#![forbid(unsafe_code)]
use crate::{CompileError, SourceMark};
use saphyr_parser::{Event, Parser};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AstMarks {
    document: Option<SourceMark>,
    nested: BTreeMap<(Box<str>, Box<str>), SourceMark>,
    trigger: BTreeMap<Box<str>, SourceMark>,
    step: BTreeMap<Box<str>, SourceMark>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Frame {
    Mapping(MappingFrame),
    Sequence(SequenceFrame),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MappingFrame {
    parent: Option<Box<str>>,
    expecting_key: bool,
    pending_key: Option<Box<str>>,
    step_mark: Option<SourceMark>,
    pending_step_id: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SequenceFrame {
    parent: Option<Box<str>>,
    in_steps: bool,
}

impl AstMarks {
    pub(crate) fn new(source: &str) -> Result<Self, CompileError> {
        let mut builder = MarkBuilder::default();
        let mut parser = Parser::new_from_str(source);
        while let Some((event, span)) = parser.next_event().transpose()? {
            builder.accept(event, SourceMark::from_parser_span(span));
        }
        Ok(builder.finish())
    }

    /// Creates an empty AstMarks with no mark lookups.
    ///
    /// Useful for testing and for compilation paths where source mark
    /// tracking is not available.  Gated behind `kani` because the sole
    /// production-crate consumer is the Kani proof harness
    /// (`kani_tree_mark_enrich`).  Integration tests exercise marks
    /// indirectly through the public compiler API.
    #[must_use]
    #[cfg(kani)]
    pub(crate) fn empty() -> Self {
        Self {
            document: None,
            nested: BTreeMap::new(),
            trigger: BTreeMap::new(),
            step: BTreeMap::new(),
        }
    }

    #[must_use]
    pub(crate) const fn document(&self) -> Option<SourceMark> {
        self.document
    }

    #[must_use]
    pub(crate) fn nested_key(&self, parent: &str, key: &str) -> Option<SourceMark> {
        self.nested.get(&(parent.into(), key.into())).copied()
    }

    #[must_use]
    pub(crate) fn trigger(&self, kind: &str) -> Option<SourceMark> {
        self.trigger.get(kind).copied()
    }

    #[must_use]
    pub(crate) fn step(&self, id: &str) -> Option<SourceMark> {
        self.step.get(id).copied()
    }
}

#[derive(Debug, Default)]
struct MarkBuilder {
    document: Option<SourceMark>,
    nested: BTreeMap<(Box<str>, Box<str>), SourceMark>,
    trigger: BTreeMap<Box<str>, SourceMark>,
    step: BTreeMap<Box<str>, SourceMark>,
    stack: Vec<Frame>,
}

impl MarkBuilder {
    fn finish(self) -> AstMarks {
        AstMarks {
            document: self.document,
            nested: self.nested,
            trigger: self.trigger,
            step: self.step,
        }
    }

    fn accept(&mut self, event: Event<'_>, mark: SourceMark) {
        match event {
            Event::DocumentStart(_) => self.document = Some(mark),
            Event::MappingStart(_, _) => self.start_mapping(mark),
            Event::SequenceStart(_, _) => self.start_sequence(),
            Event::MappingEnd | Event::SequenceEnd => self.end_node(),
            Event::Scalar(value, _, _, _) => self.scalar(value.as_ref(), mark),
            _ => {}
        }
    }

    fn start_mapping(&mut self, mark: SourceMark) {
        let parent = self.consume_parent_key();
        let step_mark = self.current_sequence_is_steps().then_some(mark);
        self.stack
            .push(Frame::Mapping(MappingFrame::new(parent, step_mark)));
    }

    fn start_sequence(&mut self) {
        let parent = self.consume_parent_key();
        let in_steps = parent.as_deref() == Some("steps");
        self.stack
            .push(Frame::Sequence(SequenceFrame { parent, in_steps }));
    }

    fn end_node(&mut self) {
        self.stack.pop();
    }

    fn scalar(&mut self, value: &str, mark: SourceMark) {
        let Some(Frame::Mapping(frame)) = self.stack.last_mut() else {
            return;
        };
        if frame.expecting_key {
            frame.capture_key(value, mark, &mut self.nested, &mut self.trigger);
        } else {
            frame.capture_value(value, &mut self.step);
        }
    }

    fn consume_parent_key(&mut self) -> Option<Box<str>> {
        let Some(Frame::Mapping(frame)) = self.stack.last_mut() else {
            return None;
        };
        frame.consume_pending_key()
    }

    fn current_sequence_is_steps(&self) -> bool {
        matches!(self.stack.last(), Some(Frame::Sequence(frame)) if frame.in_steps)
    }
}

impl MappingFrame {
    const fn new(parent: Option<Box<str>>, step_mark: Option<SourceMark>) -> Self {
        Self {
            parent,
            expecting_key: true,
            pending_key: None,
            step_mark,
            pending_step_id: false,
        }
    }

    fn consume_pending_key(&mut self) -> Option<Box<str>> {
        self.expecting_key = true;
        self.pending_step_id = false;
        self.pending_key.take()
    }

    fn capture_key(
        &mut self,
        value: &str,
        mark: SourceMark,
        nested: &mut BTreeMap<(Box<str>, Box<str>), SourceMark>,
        trigger: &mut BTreeMap<Box<str>, SourceMark>,
    ) {
        insert_key_mark(self.parent.as_deref(), value, mark, nested, trigger);
        self.pending_step_id = self.step_mark.is_some() && value == "id";
        self.pending_key = Some(value.into());
        self.expecting_key = false;
    }

    fn capture_value(&mut self, value: &str, steps: &mut BTreeMap<Box<str>, SourceMark>) {
        if let (true, Some(mark)) = (self.pending_step_id, self.step_mark) {
            steps.insert(value.into(), mark);
        }
        self.expecting_key = true;
        self.pending_step_id = false;
        self.pending_key = None;
    }
}

fn insert_key_mark(
    parent: Option<&str>,
    value: &str,
    mark: SourceMark,
    nested: &mut BTreeMap<(Box<str>, Box<str>), SourceMark>,
    trigger: &mut BTreeMap<Box<str>, SourceMark>,
) {
    match parent {
        None => {}
        Some("when") => {
            trigger.insert(value.into(), mark);
        }
        Some(parent) => {
            nested.insert((parent.into(), value.into()), mark);
        }
    }
}
