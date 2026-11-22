use std::ops::Range;

#[path = "check_test_ownership_scope.rs"]
mod check_test_ownership_scope;
#[path = "check_test_ownership_lexer.rs"]
mod lexer;
use check_test_ownership_scope::{closing_brace, leading_file_cfg_test};
use lexer::{Token, tokens};

pub(crate) fn lexer_tokens(source: &str) -> Vec<Token> {
    tokens(source)
}

#[derive(Clone, Debug)]
pub(crate) struct Edge {
    pub(crate) path: String,
    pub(crate) offset: usize,
}

pub(crate) fn test_scopes(source: &str) -> Vec<Range<usize>> {
    let lexed = tokens(source);
    let mut ranges = Vec::new();
    if leading_file_cfg_test(&lexed) {
        ranges.push(0..source.len());
    }
    for index in 0..lexed.len() {
        let module_body = lexed
            .get(index..index.saturating_add(3))
            .is_some_and(|part| {
                part.first().is_some_and(|token| token.text == "mod")
                    && part.get(1).is_some_and(|token| !token.text.is_empty())
                    && part.get(2).is_some_and(|token| token.text == "{")
            });
        let inner_cfg_test = lexed
            .get(index.saturating_add(3)..index.saturating_add(11))
            .is_some_and(|part| {
                ["#", "!", "[", "cfg", "(", "test", ")", "]"]
                    .iter()
                    .zip(part)
                    .all(|(expected, actual)| actual.text == *expected)
            });
        if module_body && inner_cfg_test {
            let open = index.saturating_add(2);
            if let Some(close) = closing_brace(&lexed, open)
                && let (Some(start), Some(end)) = (lexed.get(open), lexed.get(close))
            {
                ranges.push(start.end..end.end);
            }
        }
    }
    let mut index = 0usize;
    while index.saturating_add(5) < lexed.len() {
        let cfg_test = lexed
            .get(index..index.saturating_add(6))
            .is_some_and(|part| {
                ["#", "[", "cfg", "(", "test", ")"]
                    .iter()
                    .zip(part)
                    .all(|(expected, actual)| actual.text == *expected)
            })
            && lexed
                .get(index.saturating_add(6))
                .is_some_and(|token| token.text == "]");
        let test_attr = lexed
            .get(index..index.saturating_add(4))
            .is_some_and(|part| {
                ["#", "[", "test", "]"]
                    .iter()
                    .zip(part)
                    .all(|(expected, actual)| actual.text == *expected)
            });
        let kani_attr = lexed
            .get(index..index.saturating_add(7))
            .is_some_and(|part| {
                ["#", "[", "kani", ":", ":", "proof", "]"]
                    .iter()
                    .zip(part)
                    .all(|(expected, actual)| actual.text == *expected)
            });
        if cfg_test || test_attr || kani_attr {
            let start = lexed.get(index).map_or(0, |token| token.start);
            let attr_end = if cfg_test || kani_attr {
                index.saturating_add(6)
            } else {
                index.saturating_add(3)
            };
            let mut end = attr_end;
            let mut scan = attr_end.saturating_add(1);
            while scan < lexed.len() {
                let Some(current) = lexed.get(scan) else {
                    break;
                };
                match current.text.as_str() {
                    "{" => {
                        if let Some(close) = closing_brace(&lexed, scan) {
                            end = close;
                        }
                        break;
                    }
                    ";" => {
                        end = scan;
                        break;
                    }
                    _ => scan = scan.saturating_add(1),
                }
            }
            if let Some(last) = lexed.get(end) {
                ranges.push(start..last.end);
            }
            index = end.saturating_add(1);
        } else {
            index = index.saturating_add(1);
        }
    }
    ranges
}

pub(crate) fn self_test() -> bool {
    let source = r##"
// #[cfg(test)] mod forged { include!("no.rs"); }
let text = r#"#[cfg(test)] include!("fake.rs");"#;
#[cfg(test)] mod tests {
    include!("child.rs");
    #[path = "child_module.rs"] mod child_module;
}
#[kani::proof]
fn proof_case() { assert_eq!(1, 1); }
fn production() { assert_eq!(1, 1); }
"##;
    let ranges = test_scopes(source);
    let owned = ranges
        .iter()
        .filter_map(|range| source.get(range.clone()))
        .collect::<Vec<_>>();
    let found_edges = edges(source, &ranges);
    let test_scope = owned.iter().any(|scope| scope.contains("mod tests"));
    let kani_scope = owned.iter().any(|scope| scope.contains("proof_case"));
    let production = source.find("fn production");
    let production_outside =
        production.is_some_and(|offset| !ranges.iter().any(|range| range.contains(&offset)));
    let macro_hits = panic_hits(source);
    let passed = test_scope
        && kani_scope
        && production_outside
        && macro_hits.len() == 2
        && found_edges.len() == 2
        && found_edges.iter().any(|edge| edge.path == "child.rs")
        && found_edges
            .iter()
            .any(|edge| edge.path == "child_module.rs");
    let nested_source = "mod tests { #![cfg(test)] assert_eq!(1, 1); }\nassert_eq!(2, 2);";
    let nested_ranges = test_scopes(nested_source);
    let nested_hits = panic_hits(nested_source);
    let nested_scope_only = nested_hits.len() == 2
        && nested_hits.first().is_some_and(|(offset, _)| {
            nested_ranges.iter().any(|range| range.contains(offset))
        })
        && nested_hits.get(1).is_some_and(|(offset, _)| {
            nested_ranges.iter().all(|range| !range.contains(offset))
        });
    let lifetime_hits = panic_hits("fn lifetime(_: &'static str) { assert!(true); }");
    let char_literal_hits =
        panic_hits("fn character() { let _ = '\\''; let _ = '\\u{7d}'; assert!(true); }");
    let root_cfg = "#![cfg(test)]\nfn root_test() { assert!(true); }";
    let root_cfg_hits = panic_hits(root_cfg);
    let root_cfg_scopes = test_scopes(root_cfg);
    let root_cfg_owned = root_cfg_hits.len() == 1
        && root_cfg_hits.first().is_some_and(|(offset, _)| {
            root_cfg_scopes.iter().any(|range| range.contains(offset))
        });
    let lexical_cases = lifetime_hits.len() == 1 && char_literal_hits.len() == 1;
    if !passed || !nested_scope_only || !lexical_cases || !root_cfg_owned {
        let found_tokens = lexer_tokens(source)
            .into_iter()
            .filter(|token| token.text.contains("include") || token.text.contains("child"))
            .collect::<Vec<_>>();
        eprintln!(
            "ownership lexer details: scopes={owned:?} edges={found_edges:?} tokens={found_tokens:?}"
        );
    }
    passed && nested_scope_only && lexical_cases && root_cfg_owned
}

pub(crate) fn edges(source: &str, ranges: &[Range<usize>]) -> Vec<Edge> {
    all_edges(source)
        .into_iter()
        .filter(|edge| ranges.iter().any(|range| range.contains(&edge.offset)))
        .collect()
}

pub(crate) fn all_edges(source: &str) -> Vec<Edge> {
    let lexed = tokens(source);
    let mut out = Vec::new();
    for index in 0..lexed.len() {
        let Some(token) = lexed.get(index) else {
            continue;
        };
        if token.text == "include"
            && lexed
                .get(index.saturating_add(1))
                .is_some_and(|next| next.text == "!")
            && lexed
                .get(index.saturating_add(2))
                .is_some_and(|next| next.text == "(")
            && let Some(path) = lexed
                .get(index.saturating_add(3))
                .and_then(|next| string_literal(&next.text))
        {
            out.push(Edge {
                path,
                offset: token.start,
            });
        }
        if token.text == "path"
            && lexed
                .get(index.saturating_add(1))
                .is_some_and(|next| next.text == "=")
            && let Some(path) = lexed
                .get(index.saturating_add(2))
                .and_then(|next| string_literal(&next.text))
        {
            let end = index.saturating_add(9).min(lexed.len());
            if lexed
                .get(index.saturating_add(3)..end)
                .is_some_and(|tail| tail.iter().any(|item| item.text == "mod"))
            {
                out.push(Edge {
                    path,
                    offset: token.start,
                });
            }
        }
    }
    out
}

pub(crate) fn panic_hits(source: &str) -> Vec<(usize, String)> {
    let lexed = tokens(source);
    let mut hits = Vec::new();
    for index in 0..lexed.len() {
        let Some(token) = lexed.get(index) else {
            continue;
        };
        if ["assert", "assert_eq", "assert_ne", "unreachable"]
            .iter()
            .any(|name| token.text == *name)
            && lexed
                .get(index.saturating_add(1))
                .is_some_and(|next| next.text == "!")
        {
            hits.push((token.start, token.text.clone()));
        }
    }
    hits
}

fn string_literal(token: &str) -> Option<String> {
    if token.starts_with('"') && token.ends_with('"') && token.len() >= 2 {
        token
            .get(1..token.len().saturating_sub(1))
            .map(str::to_owned)
    } else if token.starts_with("r\"") && token.ends_with('"') {
        token
            .get(2..token.len().saturating_sub(1))
            .map(str::to_owned)
    } else {
        None
    }
}
