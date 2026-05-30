//! IR builder: contract -> typed IR with finite facts and decision paths.

use super::schema::*;
use super::error::AssurecError;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct AssuranceIr {
    pub domains: BTreeMap<String, DomainType>,
    pub clauses: Vec<ContractClause>,
    pub decision_paths: Vec<DecisionPath>,
    pub variables: Vec<String>,
}

impl AssuranceIr {
    pub fn from_contract(
        clauses: &[ContractClause],
        domains: &[DomainType],
    ) -> Result<Self, AssurecError> {
        let mut variables = BTreeMap::new();
        for clause in clauses {
            for expr in &clause.when {
                collect_variables(expr, &mut variables);
            }
        }

        let variables: Vec<String> = variables.into_keys().collect();
        let decision_paths = build_decision_paths(clauses, &variables, domains)?;

        Ok(Self {
            domains: domains.iter().map(|d| (d.name.clone(), d.clone())).collect(),
            clauses: clauses.to_vec(),
            decision_paths,
            variables,
        })
    }

    pub fn digest(&self) -> String {
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(b"assurance_ir_v1");
        for clause in &self.clauses {
            hasher.update(clause.id.as_bytes());
        }
        for path in &self.decision_paths {
            hasher.update(path.id.as_bytes());
        }
        format!("{}", hasher.finalize().to_hex())
    }
}

fn collect_variables(expr: &TypedExpr, vars: &mut BTreeMap<String, ()>) {
    match expr {
        TypedExpr::Var { name } => {
            vars.insert(name.clone(), ());
        }
        TypedExpr::Eq { lhs, rhs } => {
            collect_variables(lhs, vars);
            collect_variables(rhs, vars);
        }
        TypedExpr::Neq { lhs, rhs } => {
            collect_variables(lhs, vars);
            collect_variables(rhs, vars);
        }
        TypedExpr::And { lhs, rhs } => {
            collect_variables(lhs, vars);
            collect_variables(rhs, vars);
        }
        TypedExpr::Or { lhs, rhs } => {
            collect_variables(lhs, vars);
            collect_variables(rhs, vars);
        }
        TypedExpr::Not { inner } => {
            collect_variables(inner, vars);
        }
        TypedExpr::Bool(_) => {}
    }
}

fn build_decision_paths(
    clauses: &[ContractClause],
    variables: &[String],
    domains: &[DomainType],
) -> Result<Vec<DecisionPath>, AssurecError> {
    let domain_map: BTreeMap<&str, &DomainType> =
        domains.iter().map(|d| (d.name.as_str(), d)).collect();

    let valuations: Vec<BTreeMap<String, String>> = enumerate_valuations(variables, &domain_map)?;

    let mut paths = Vec::new();
    for (idx, valuation) in valuations.into_iter().enumerate() {
        let outcome = evaluate_valuation(&valuation, clauses);
        paths.push(DecisionPath {
            id: format!("path_{:03}", idx),
            valuation,
            outcome,
        });
    }

    Ok(paths)
}

fn enumerate_valuations(
    variables: &[String],
    domains: &BTreeMap<&str, &DomainType>,
) -> Result<Vec<BTreeMap<String, String>>, AssurecError> {
    if variables.is_empty() {
        return Ok(vec![BTreeMap::new()]);
    }

    let first_var = &variables[0];
    let rest_vars = &variables[1..];

    let domain = domains
        .get(first_var.as_str())
        .ok_or_else(|| AssurecError::Ir(format!("unknown domain for variable: {}", first_var)))?;

    let mut results = Vec::new();
    for variant in &domain.variants {
        let domain_map = domains.clone();
        let sub_valuations = enumerate_valuations(rest_vars, &domain_map)?;

        for mut valuation in sub_valuations {
            valuation.insert(first_var.clone(), variant.clone());
            results.push(valuation);
        }
    }

    Ok(results)
}

fn evaluate_valuation(
    valuation: &BTreeMap<String, String>,
    clauses: &[ContractClause],
) -> PathOutcome {
    for clause in clauses {
        if evaluate_when(&clause.when, valuation) {
            return clause.then.clone().into();
        }
    }
    PathOutcome::Error { code: "Unhandled".to_string() }
}

fn evaluate_when(conditions: &[TypedExpr], valuation: &BTreeMap<String, String>) -> bool {
    conditions.iter().all(|cond| evaluate_expr(cond, valuation))
}

fn evaluate_expr(expr: &TypedExpr, valuation: &BTreeMap<String, String>) -> bool {
    match expr {
        TypedExpr::Bool(b) => *b,
        TypedExpr::Var { name } => {
            valuation.get(name).map(|v| v.as_str() == "true" || !v.is_empty()).unwrap_or(false)
        }
        TypedExpr::Eq { lhs, rhs } => {
            let lv = evaluate_expr(lhs, valuation);
            let rv = evaluate_expr(rhs, valuation);
            lv == rv
        }
        TypedExpr::Neq { lhs, rhs } => {
            let lv = evaluate_expr(lhs, valuation);
            let rv = evaluate_expr(rhs, valuation);
            lv != rv
        }
        TypedExpr::And { lhs, rhs } => {
            evaluate_expr(lhs, valuation) && evaluate_expr(rhs, valuation)
        }
        TypedExpr::Or { lhs, rhs } => {
            evaluate_expr(lhs, valuation) || evaluate_expr(rhs, valuation)
        }
        TypedExpr::Not { inner } => !evaluate_expr(inner, valuation),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ir_from_v1_contract() {
        let domains = vec![
            crate::assure::contract::tenant_claim_rel_domain(),
            crate::assure::contract::membership_fact_domain(),
        ];
        let clauses = crate::assure::contract::v1_contract();
        let ir = AssuranceIr::from_contract(&clauses, &domains).unwrap();

        assert_eq!(ir.domains.len(), 2);
        assert_eq!(ir.clauses.len(), 5);
        assert!(!ir.variables.is_empty());
    }
}
