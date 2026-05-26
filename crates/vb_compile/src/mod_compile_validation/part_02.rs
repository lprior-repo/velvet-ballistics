#![allow(unused_imports)]
use super::*;
use crate::ast::marks::AstMarks;
use crate::limits::YamlLimits;
use crate::mod_compile_errors::non_string_key_error;
use crate::mod_compile_errors::{CompileError, CompileErrors, SourceMark};
use saphyr::Yaml;
use saphyr_parser::{Event, Parser, Span, StrInput};
use std::collections::HashSet;
use std::str;
use vb_core::{ConstValue, SlotIdx, StepIdx};

/// Context for a YAML node being validated in tree-order traversal.
pub(super) struct NodeCtx<'a> {
    node: &'a Yaml<'a>,
    depth: u16,
    /// Key name of this node within its parent mapping (when applicable).
    key: Option<&'a str>,
    /// Parent mapping key name for nested-mark lookups.
    parent: Option<&'a str>,
}

/// Computes the best available [`SourceMark`] for an error at the given
/// key/parent position.  Consults [`AstMarks`] lookups in order of
/// specificity (`nested_key` → `trigger` → `document`), falling back to
/// [`SourceMark::unavailable`] when no mark can be resolved.
fn best_mark(marks: Option<&AstMarks>, key: Option<&str>, parent: Option<&str>) -> SourceMark {
    let marks = match marks {
        Some(m) => m,
        None => return SourceMark::unavailable(),
    };
    // Field-level: nested_key when we have parent + key
    if let (Some(p), Some(k)) = (parent, key)
        && let Some(m) = marks.nested_key(p, k)
    {
        return m;
    }
    // Trigger-level: key under "when" or known trigger kinds
    if let Some(k) = key
        && let Some(m) = marks.trigger(k)
    {
        return m;
    }
    // Document-level: broad fallback
    if let Some(m) = marks.document() {
        return m;
    }
    SourceMark::unavailable()
}

pub(crate) fn validate_strict_profile(
    root: &Yaml<'_>,
    limits: YamlLimits,
    marks: Option<&AstMarks>,
) -> Result<(), CompileError> {
    if !root.is_mapping() {
        return Err(CompileError::TopLevelNotMapping);
    }

    let mut stack: Vec<NodeCtx<'_>> = vec![NodeCtx {
        node: root,
        depth: 0,
        key: None,
        parent: None,
    }];
    let mut visited = 0_u32;

    while let Some(ctx) = stack.pop() {
        visited = next_visited_count(visited, limits)?;
        validate_depth(ctx.depth, limits)?;
        validate_one_node(
            ctx.node, ctx.depth, limits, &mut stack, marks, ctx.key, ctx.parent,
        )?;
    }

    Ok(())
}

pub(super) fn next_visited_count(visited: u32, limits: YamlLimits) -> Result<u32, CompileError> {
    let next = visited.checked_add(1).ok_or(CompileError::NodeLimit {
        limit: limits.max_nodes,
    })?;
    if next > limits.max_nodes {
        Err(CompileError::NodeLimit {
            limit: limits.max_nodes,
        })
    } else {
        Ok(next)
    }
}

pub(super) fn validate_depth(depth: u16, limits: YamlLimits) -> Result<(), CompileError> {
    if depth > limits.max_depth {
        Err(CompileError::DepthLimit {
            depth,
            limit: limits.max_depth,
        })
    } else {
        Ok(())
    }
}

pub(super) fn validate_one_node<'a>(
    node: &'a Yaml<'a>,
    depth: u16,
    limits: YamlLimits,
    stack: &mut Vec<NodeCtx<'a>>,
    marks: Option<&AstMarks>,
    key: Option<&'a str>,
    parent: Option<&'a str>,
) -> Result<(), CompileError> {
    let fallback_mark = best_mark(marks, key, parent);
    match node {
        Yaml::Mapping(mapping) => push_mapping(mapping, depth, limits, stack, key, marks),
        Yaml::Sequence(sequence) => push_sequence(sequence, depth, limits, stack, key),
        Yaml::Tagged(_, _) => Err(CompileError::TagForbidden {
            mark: fallback_mark,
        }),
        Yaml::Alias(_) => Err(CompileError::AliasForbidden {
            mark: fallback_mark,
        }),
        Yaml::BadValue => Err(CompileError::BadValue),
        Yaml::Value(value) => validate_scalar(value, limits),
        Yaml::Representation(value, _, tag) => {
            validate_representation(value.as_ref(), tag.is_some(), limits, fallback_mark)
        }
    }
}

pub(super) fn validate_representation(
    value: &str,
    has_tag: bool,
    limits: YamlLimits,
    fallback_mark: SourceMark,
) -> Result<(), CompileError> {
    if has_tag {
        return Err(CompileError::TagForbidden {
            mark: fallback_mark,
        });
    }
    validate_scalar_len(value, limits)
}

pub(super) fn push_mapping<'a>(
    mapping: &'a saphyr::Mapping<'a>,
    depth: u16,
    limits: YamlLimits,
    stack: &mut Vec<NodeCtx<'a>>,
    parent_key: Option<&'a str>,
    marks: Option<&AstMarks>,
) -> Result<(), CompileError> {
    validate_mapping_len(mapping, limits)?;
    let next_depth = depth.checked_add(1).ok_or(CompileError::DepthLimit {
        depth,
        limit: limits.max_depth,
    })?;
    let mut seen = HashSet::with_capacity(mapping.len());
    for (key, value) in mapping {
        let key_str = validate_mapping_key(key, limits)?;
        if !seen.insert(key_str) {
            return Err(CompileError::DuplicateKey {
                key: Box::<str>::from(key_str),
                mark: best_mark(marks, Some(key_str), parent_key),
            });
        }
        stack.push(NodeCtx {
            node: value,
            depth: next_depth,
            key: Some(key_str),
            parent: parent_key,
        });
    }
    Ok(())
}

pub(super) fn validate_mapping_len(
    mapping: &saphyr::Mapping<'_>,
    limits: YamlLimits,
) -> Result<(), CompileError> {
    if mapping.len() > limits.max_mapping_entries {
        Err(CompileError::MappingLimit {
            actual: mapping.len(),
            limit: limits.max_mapping_entries,
        })
    } else {
        Ok(())
    }
}

pub(super) fn push_sequence<'a>(
    sequence: &'a saphyr::Sequence<'a>,
    depth: u16,
    limits: YamlLimits,
    stack: &mut Vec<NodeCtx<'a>>,
    parent_key: Option<&'a str>,
) -> Result<(), CompileError> {
    if sequence.len() > limits.max_sequence_len {
        return Err(CompileError::SequenceLimit {
            actual: sequence.len(),
            limit: limits.max_sequence_len,
        });
    }
    let next_depth = depth.checked_add(1).ok_or(CompileError::DepthLimit {
        depth,
        limit: limits.max_depth,
    })?;
    for item in sequence {
        stack.push(NodeCtx {
            node: item,
            depth: next_depth,
            key: None,
            parent: parent_key,
        });
    }
    Ok(())
}

pub(super) fn validate_mapping_key<'a>(
    key: &'a Yaml<'a>,
    limits: YamlLimits,
) -> Result<&'a str, CompileError> {
    match key.as_str() {
        Some(value) => {
            validate_scalar_len(value, limits)?;
            if value == "<<" {
                Err(CompileError::MergeKeyForbidden {
                    mark: SourceMark::unavailable(),
                })
            } else {
                Ok(value)
            }
        }
        None => Err(CompileError::NonStringKey {
            mark: SourceMark::unavailable(),
        }),
    }
}

pub(super) fn validate_scalar(
    value: &saphyr::Scalar<'_>,
    limits: YamlLimits,
) -> Result<(), CompileError> {
    match value {
        saphyr::Scalar::String(value) => validate_scalar_len(value.as_ref(), limits),
        saphyr::Scalar::FloatingPoint(_) => Err(CompileError::FloatForbidden),
        saphyr::Scalar::Null | saphyr::Scalar::Boolean(_) | saphyr::Scalar::Integer(_) => Ok(()),
    }
}

pub(super) fn validate_scalar_len(value: &str, limits: YamlLimits) -> Result<(), CompileError> {
    if value.len() > limits.max_scalar_bytes {
        Err(CompileError::ScalarLimit {
            actual: value.len(),
            limit: limits.max_scalar_bytes,
        })
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StepPrimitive {
    Set,
    Run,
    Do,
    Save,
    Choose,
    ForEach,
    Parallel,
    Collect,
    Aggregate,
    Repeat,
    Wait,
    Ask,
    Finish,
}
