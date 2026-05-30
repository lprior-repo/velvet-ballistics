//! Path checker: validates decision table properties.

use super::schema::*;
use super::error::AssurecError;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct PathCheckResult {
    pub total_paths: usize,
    pub grant_paths: usize,
    pub error_paths: usize,
    pub unhandled_paths: usize,
    pub overlap_count: usize,
    pub missing_coverage: Vec<String>,
    pub errors: Vec<PathCheckError>,
}

#[derive(Debug, Clone)]
pub enum PathCheckError {
    ZeroMatches { valuation: String },
    MultipleMatches { valuation: String, clauses: Vec<String> },
    Unreachable { clause_id: String },
    MissingErrorProducer { error_code: String },
    MissingOracleMapping { oracle_id: String },
}

impl PathCheckResult {
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
            && self.unhandled_paths == 0
            && self.overlap_count == 0
            && self.missing_coverage.is_empty()
    }
}

pub fn check_paths(
    paths: &[DecisionPath],
    clauses: &[ContractClause],
    _oracle_bank: &[OracleRecord],
) -> Result<PathCheckResult, AssurecError> {
    let mut errors = Vec::new();
    let mut grant_paths = 0;
    let mut error_paths = 0;
    let mut unhandled_paths = 0;
    let mut overlap_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for path in paths {
        match &path.outcome {
            PathOutcome::Grant => grant_paths += 1,
            PathOutcome::Error { .. } => error_paths += 1,
        }

        if matches_zero_clauses(path, clauses) {
            errors.push(PathCheckError::ZeroMatches {
                valuation: format!("{:?}", path.valuation),
            });
            unhandled_paths += 1;
        }

        let matching = find_matching_clauses(path, clauses);
        if matching.len() > 1 {
            overlap_map
                .entry(valuation_key(&path.valuation))
                .or_default()
                .extend(matching.iter().map(|c| c.id.clone()));
        }
    }

    let overlap_count = overlap_map.len();
    for (_, clause_ids) in &overlap_map {
        if clause_ids.len() > 1 {
            errors.push(PathCheckError::MultipleMatches {
                valuation: "".to_string(),
                clauses: clause_ids.clone(),
            });
        }
    }

    let unreachable = find_unreachable_clauses(paths, clauses);
    for clause_id in unreachable {
        errors.push(PathCheckError::Unreachable { clause_id });
    }

    let error_codes: std::collections::HashSet<_> = paths
        .iter()
        .filter_map(|p| match &p.outcome {
            PathOutcome::Error { code } => Some(code.as_str()),
            _ => None,
        })
        .collect();

    let mut missing_coverage = Vec::new();
    let required_errors = vec![
        "MissingTenantClaim",
        "TenantMismatch",
        "MembershipLookupFailed",
        "NoMembership",
    ];
    for err in required_errors {
        if !error_codes.contains(err) {
            missing_coverage.push(err.to_string());
        }
    }

    Ok(PathCheckResult {
        total_paths: paths.len(),
        grant_paths,
        error_paths,
        unhandled_paths,
        overlap_count,
        missing_coverage,
        errors,
    })
}

fn valuation_key(valuation: &BTreeMap<String, String>) -> String {
    let mut pairs: Vec<_> = valuation.iter().collect();
    pairs.sort();
    pairs.iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join(",")
}

fn matches_zero_clauses(path: &DecisionPath, clauses: &[ContractClause]) -> bool {
    clauses.iter().all(|clause| !clause_matches(path, clause))
}

fn find_matching_clauses<'a>(
    path: &'a DecisionPath,
    clauses: &'a [ContractClause],
) -> Vec<&'a ContractClause> {
    clauses.iter().filter(|clause| clause_matches(path, clause)).collect()
}

fn clause_matches(path: &DecisionPath, clause: &ContractClause) -> bool {
    clause.when.iter().all(|cond| eval_condition(cond, path))
}

fn eval_condition(cond: &TypedExpr, path: &DecisionPath) -> bool {
    match cond {
        TypedExpr::Var { name } => {
            path.valuation.get(name).map(|v| !v.is_empty()).unwrap_or(false)
        }
        TypedExpr::Eq { lhs, rhs } => {
            let lv = eval_expr(lhs, path);
            let rv = eval_expr(rhs, path);
            lv == rv
        }
        TypedExpr::Neq { lhs, rhs } => {
            let lv = eval_expr(lhs, path);
            let rv = eval_expr(rhs, path);
            lv != rv
        }
        TypedExpr::And { lhs, rhs } => eval_condition(lhs, path) && eval_condition(rhs, path),
        TypedExpr::Or { lhs, rhs } => eval_condition(lhs, path) || eval_condition(rhs, path),
        TypedExpr::Not { inner } => !eval_condition(inner, path),
        TypedExpr::Bool(b) => *b,
    }
}

fn eval_expr(expr: &TypedExpr, path: &DecisionPath) -> String {
    match expr {
        TypedExpr::Var { name } => path.valuation.get(name).cloned().unwrap_or_default(),
        TypedExpr::Bool(b) => b.to_string(),
        TypedExpr::Eq { .. } => "eq".to_string(),
        TypedExpr::Neq { .. } => "neq".to_string(),
        TypedExpr::And { .. } => "and".to_string(),
        TypedExpr::Or { .. } => "or".to_string(),
        TypedExpr::Not { .. } => "not".to_string(),
    }
}

fn find_unreachable_clauses<'a>(
    paths: &'a [DecisionPath],
    clauses: &'a [ContractClause],
) -> Vec<String> {
    clauses
        .iter()
        .filter(|clause| !paths.iter().any(|path| clause_matches(path, clause)))
        .map(|c| c.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_rejects_zero_matches() {
        let paths = vec![DecisionPath {
            id: "p1".to_string(),
            valuation: BTreeMap::from([("x".to_string(), "A".to_string())]),
            outcome: PathOutcome::Grant,
        }];
        let clauses = vec![ContractClause::new(
            "c1",
            vec![TypedExpr::eq(TypedExpr::var("y"), TypedExpr::var("B"))],
            EffectOutcome::err("Err"),
        )];
        let result = check_paths(&paths, &clauses, &[]).unwrap();
        assert!(!result.is_valid());
        assert_eq!(result.unhandled_paths, 1);
    }
}
