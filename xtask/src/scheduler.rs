#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::ok_expect,
    clippy::as_conversions,
    clippy::arithmetic_side_effects,
    clippy::indexing_slicing,
    clippy::let_underscore_must_use,
    clippy::panic,
    clippy::panic_in_result_fn,
    clippy::todo,
    clippy::unimplemented,
    clippy::assertions_on_constants,
    clippy::needless_range_loop,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::field_reassign_with_default,
    clippy::redundant_guards,
    clippy::redundant_closure,
    clippy::useless_conversion,
    clippy::unnecessary_unwrap,
    clippy::unnecessary_cast,
    clippy::needless_update,
    clippy::bool_comparison,
    clippy::manual_div_ceil,
    clippy::clone_on_copy,
    clippy::len_zero,
    clippy::redundant_clone,
    clippy::collapsible_if,
    clippy::needless_return,
    clippy::needless_borrow,
    clippy::useless_format,
    clippy::redundant_pub_crate,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::missing_safety_doc,
    clippy::wildcard_enum_match_arm,
    clippy::large_futures,
    clippy::unused_async,
    clippy::unused_self,
    let_underscore_drop,
    clippy::filter_map_next,
    clippy::from_iter_instead_of_collect,
    clippy::if_not_else,
    clippy::implicit_clone,
    clippy::inefficient_to_string,
    clippy::inconsistent_struct_constructor,
    clippy::iter_filter_is_ok,
    clippy::iter_filter_is_some,
    clippy::iter_not_returning_iterator,
    clippy::iter_over_hash_type,
    clippy::iter_without_into_iter,
    clippy::large_digit_groups,
    clippy::large_types_passed_by_value,
    clippy::let_and_return,
    clippy::misnamed_getters,
    clippy::mutable_key_type,
    clippy::needless_collect,
    clippy::nonminimal_bool,
    clippy::option_if_let_else,
    clippy::or_fun_call,
    clippy::path_buf_push_overwrite,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::pub_with_shorthand,
    clippy::range_minus_one,
    clippy::range_plus_one,
    clippy::ref_binding_to_reference,
    clippy::ref_option_ref,
    clippy::single_match_else,
    clippy::suspicious_operation_groupings,
    clippy::trivially_copy_pass_by_ref,
    clippy::uninlined_format_args,
    clippy::unnecessary_wraps,
    clippy::unnested_or_patterns,
    clippy::unreadable_literal,
    clippy::unused_io_amount,
    clippy::unused_trait_names,
    clippy::vec_init_then_push,
    clippy::wildcard_imports,
    clippy::absurd_extreme_comparisons,
    clippy::expect_fun_call,
    clippy::useless_vec,
    clippy::redundant_locals,
    clippy::too_many_lines,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_abs_to_unsigned,
    clippy::similar_names,
    clippy::shadow_unrelated,
    clippy::needless_pass_by_value,
    clippy::neg_cmp_op_on_partial_ord,
    clippy::redundant_pattern_matching,
    clippy::unneeded_struct_pattern,
    clippy::single_match,
    clippy::module_inception,
    clippy::match_like_matches_macro,
    clippy::duplicated_attributes,
    clippy::redundant_else,
    clippy::collapsible_match,
    clippy::manual_map,
    clippy::manual_let_else,
    clippy::manual_strip,
    clippy::needless_bool,
    clippy::needless_bool_assign,
    clippy::if_let_mutex,
    unused_imports,
    dead_code,
    unused_variables
)]

//! DAG scheduler with bounded parallelism.
//!
//! Schedules independent crates in parallel levels while respecting
//! dependency order. Each level contains crates that can run concurrently.

use crate::discovery::CrateInfo;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone)]
pub struct ScheduleLevel {
    pub level: usize,
    pub crates: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Schedule {
    pub levels: Vec<ScheduleLevel>,
    pub total_crates: usize,
}

pub fn build_schedule(crates: &[CrateInfo], max_jobs: usize) -> Schedule {
    if crates.is_empty() {
        return Schedule {
            levels: vec![],
            total_crates: 0,
        };
    }

    let graph = build_dependency_graph(crates);
    let levels = topological_levels(&graph, max_jobs);

    Schedule {
        levels,
        total_crates: crates.len(),
    }
}

struct DepGraph {
    nodes: HashSet<String>,
    edges: HashMap<String, Vec<String>>,
    reverse: HashMap<String, Vec<String>>,
}

fn build_dependency_graph(crates: &[CrateInfo]) -> DepGraph {
    let crate_names: HashSet<_> = crates.iter().map(|c| c.name.clone()).collect();
    let mut edges: HashMap<String, Vec<String>> = HashMap::new();
    let mut reverse: HashMap<String, Vec<String>> = HashMap::new();

    for c in crates {
        edges.entry(c.name.clone()).or_default();
        for dep in &c.dependencies {
            if crate_names.contains(dep) {
                edges.entry(c.name.clone()).or_default().push(dep.clone());
                reverse.entry(dep.clone()).or_default().push(c.name.clone());
            }
        }
    }

    DepGraph {
        nodes: crate_names,
        edges,
        reverse,
    }
}

fn topological_levels(graph: &DepGraph, max_jobs: usize) -> Vec<ScheduleLevel> {
    let mut in_degree: HashMap<&str, usize> =
        graph.nodes.iter().map(|n| (n.as_str(), 0usize)).collect();

    for (node, deps) in &graph.edges {
        for _dep in deps {
            if let Some(count) = in_degree.get_mut(node.as_str()) {
                *count = count.saturating_add(1);
            }
        }
    }

    let mut queue: VecDeque<&str> = in_degree
        .iter()
        .filter(|&(_, &deg)| deg == 0)
        .map(|(&name, _)| name)
        .collect();

    let mut levels: Vec<ScheduleLevel> = Vec::new();
    let mut level_num = 0;

    while !queue.is_empty() {
        let batch: Vec<&str> = queue.drain(..).collect();
        let chunked = batch.chunks(max_jobs.max(1));

        for chunk in chunked {
            let crates: Vec<String> = chunk.iter().map(|s| s.to_string()).collect();
            levels.push(ScheduleLevel {
                level: level_num,
                crates,
            });
            level_num = level_num.saturating_add(1);
        }

        for node in &batch {
            if let Some(dependents) = graph.reverse.get(*node) {
                for dep in dependents {
                    if let Some(deg) = in_degree.get_mut(dep.as_str()) {
                        *deg = deg.saturating_sub(1);
                        if *deg == 0 {
                            queue.push_back(dep.as_str());
                        }
                    }
                }
            }
        }
    }

    levels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_schedule() {
        let schedule = build_schedule(&[], 4);
        assert!(schedule.levels.is_empty());
        assert_eq!(schedule.total_crates, 0);
    }

    #[test]
    fn test_single_crate() {
        let crates = vec![make_crate("a", &[])];
        let schedule = build_schedule(&crates, 4);
        assert_eq!(schedule.levels.len(), 1);
        assert_eq!(schedule.levels[0].crates, vec!["a"]);
    }

    #[test]
    fn test_linear_chain() {
        let crates = vec![
            make_crate("a", &[]),
            make_crate("b", &["a"]),
            make_crate("c", &["b"]),
        ];
        let schedule = build_schedule(&crates, 4);
        assert_eq!(schedule.levels.len(), 3);
    }

    #[test]
    fn test_independent_crates() {
        let crates = vec![
            make_crate("a", &[]),
            make_crate("b", &[]),
            make_crate("c", &[]),
        ];
        let schedule = build_schedule(&crates, 4);
        assert_eq!(schedule.levels.len(), 1);
        assert_eq!(schedule.levels[0].crates.len(), 3);
    }

    #[test]
    fn test_bounded_parallelism() {
        let crates = vec![
            make_crate("a", &[]),
            make_crate("b", &[]),
            make_crate("c", &[]),
            make_crate("d", &[]),
        ];
        let schedule = build_schedule(&crates, 2);
        assert_eq!(schedule.levels.len(), 2);
        assert_eq!(schedule.levels[0].crates.len(), 2);
    }

    fn make_crate(name: &str, deps: &[&str]) -> CrateInfo {
        CrateInfo {
            name: name.to_string(),
            manifest_path: std::path::PathBuf::from(format!("crates/{name}/Cargo.toml")),
            dependencies: deps.iter().map(|s| s.to_string()).collect(),
        }
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    fn arb_dag(max_nodes: usize) -> impl Strategy<Value = Vec<CrateInfo>> {
        proptest::collection::vec(0..max_nodes, 1..max_nodes).prop_map(|edges| {
            let n = edges.len();
            let mut crates: Vec<CrateInfo> = (0..n)
                .map(|i| CrateInfo {
                    name: format!("crate_{i}"),
                    manifest_path: std::path::PathBuf::from("Cargo.toml"),
                    dependencies: vec![],
                })
                .collect();

            for (i, &dep_idx) in edges.iter().enumerate() {
                if dep_idx < i {
                    crates[i].dependencies.push(format!("crate_{dep_idx}"));
                }
            }
            crates
        })
    }

    proptest! {
        #[test]
        fn proptest_topological_order(dag in arb_dag(20)) {
            let schedule = build_schedule(&dag, 4);
            let mut scheduled: HashSet<String> = HashSet::new();

            for level in &schedule.levels {
                for name in &level.crates {
                    let crate_info = dag.iter().find(|c| &c.name == name);
                    if let Some(info) = crate_info {
                        for dep in &info.dependencies {
                            prop_assert!(
                                scheduled.contains(dep),
                                "Crate {} scheduled before dependency {}",
                                name, dep
                            );
                        }
                    }
                    scheduled.insert(name.clone());
                }
            }
        }

        #[test]
        fn proptest_dependency_order(dag in arb_dag(20)) {
            let schedule = build_schedule(&dag, 4);
            let mut position: HashMap<String, usize> = HashMap::new();

            for (level_idx, level) in schedule.levels.iter().enumerate() {
                for name in &level.crates {
                    position.insert(name.clone(), level_idx);
                }
            }

            for crate_info in &dag {
                for dep in &crate_info.dependencies {
                    if let (Some(&dep_pos), Some(&self_pos)) =
                        (position.get(dep), position.get(&crate_info.name))
                    {
                        prop_assert!(
                            dep_pos <= self_pos,
                            "Dependency {} (level {}) after {} (level {})",
                            dep, dep_pos, crate_info.name, self_pos
                        );
                    }
        }
    }
        }

        #[test]
        fn proptest_bounded_parallelism(dag in arb_dag(20), max_jobs in 1usize..8) {
            let schedule = build_schedule(&dag, max_jobs);
            for level in &schedule.levels {
                prop_assert!(
                    level.crates.len() <= max_jobs,
                    "Level {} has {} crates, max_jobs={}",
                    level.level, level.crates.len(), max_jobs
                );
            }
        }
    }
}
