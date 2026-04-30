//! Compile-time type inference for expression ASTs.

use crate::parser::{ExprAst, ExprHelper, ExprLiteral};
use crate::{ExprError, ExprResult};

/// Inferred expression type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExprType {
    /// Null type.
    Null,
    /// Boolean type.
    Bool,
    /// Signed integer type.
    I64,
    /// Finite floating-point type.
    F64,
    /// String / symbol type.
    Text,
    /// List type.
    List,
    /// Object type.
    Object,
    /// Type could not be determined statically.
    Unknown,
}

/// Type context providing known variable types for type inference.
#[derive(Debug, Clone)]
pub struct TypeContext {
    /// Known variable types keyed by reference name.
    variables: Vec<(Box<str>, ExprType)>,
}

impl TypeContext {
    /// Creates an empty type context.
    #[must_use]
    pub fn new() -> Self {
        Self {
            variables: Vec::new(),
        }
    }

    /// Adds a variable type to the context.
    pub fn add_variable(&mut self, name: Box<str>, ty: ExprType) {
        self.variables.push((name, ty));
    }

    /// Looks up the type of a variable by name.
    pub fn lookup(&self, name: &str) -> ExprType {
        self.variables
            .iter()
            .rev()
            .find(|(n, _)| n.as_ref() == name)
            .map(|(_, ty)| *ty)
            .unwrap_or(ExprType::Unknown)
    }
}

impl Default for TypeContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Infers the result type of an expression AST.
pub fn typecheck_expr(expr: &ExprAst, context: &TypeContext) -> ExprResult<ExprType> {
    match expr {
        ExprAst::Literal(lit) => Ok(infer_literal(lit)),
        ExprAst::Reference(name) => Ok(context.lookup(name)),
        ExprAst::Unary { op, expr: inner } => typecheck_unary(*op, inner, context),
        ExprAst::Binary { op, left, right } => typecheck_binary(*op, left, right, context),
        ExprAst::Helper { name, args } => typecheck_helper(*name, args, context),
    }
}

fn infer_literal(lit: &ExprLiteral) -> ExprType {
    match lit {
        ExprLiteral::Null => ExprType::Null,
        ExprLiteral::Bool(_) => ExprType::Bool,
        ExprLiteral::I64(_) => ExprType::I64,
        ExprLiteral::Text(_) => ExprType::Text,
    }
}

fn typecheck_unary(op: crate::lexer::UnaryOp, inner: &ExprAst, ctx: &TypeContext) -> ExprResult<ExprType> {
    let inner_ty = typecheck_expr(inner, ctx)?;
    match op {
        crate::lexer::UnaryOp::Not => {
            if inner_ty == ExprType::Bool || inner_ty == ExprType::Unknown {
                Ok(ExprType::Bool)
            } else {
                Err(ExprError::TypeMismatch {
                    expected: "boolean".into(),
                    found: type_name(inner_ty),
                })
            }
        }
        crate::lexer::UnaryOp::Neg => {
            if is_numeric(inner_ty) || inner_ty == ExprType::Unknown {
                Ok(inner_ty)
            } else {
                Err(ExprError::TypeMismatch {
                    expected: "number".into(),
                    found: type_name(inner_ty),
                })
            }
        }
    }
}

fn typecheck_binary(
    op: crate::lexer::BinaryOp,
    left: &ExprAst,
    right: &ExprAst,
    ctx: &TypeContext,
) -> ExprResult<ExprType> {
    let left_ty = typecheck_expr(left, ctx)?;
    let right_ty = typecheck_expr(right, ctx)?;
    match op {
        crate::lexer::BinaryOp::And | crate::lexer::BinaryOp::Or => {
            expect_bool_pair(left_ty, right_ty)?;
            Ok(ExprType::Bool)
        }
        crate::lexer::BinaryOp::Eq | crate::lexer::BinaryOp::NotEq => Ok(ExprType::Bool),
        crate::lexer::BinaryOp::Lt
        | crate::lexer::BinaryOp::Lte
        | crate::lexer::BinaryOp::Gt
        | crate::lexer::BinaryOp::Gte => {
            expect_numeric_pair(left_ty, right_ty)?;
            Ok(ExprType::Bool)
        }
        crate::lexer::BinaryOp::Add
        | crate::lexer::BinaryOp::Sub
        | crate::lexer::BinaryOp::Mul
        | crate::lexer::BinaryOp::Div => {
            expect_numeric_pair(left_ty, right_ty)?;
            Ok(coerce_numeric(left_ty, right_ty))
        }
    }
}

fn typecheck_helper(
    helper: ExprHelper,
    args: &[ExprAst],
    ctx: &TypeContext,
) -> ExprResult<ExprType> {
    let _arg_types: Vec<ExprType> = args
        .iter()
        .map(|a| typecheck_expr(a, ctx))
        .collect::<ExprResult<Vec<_>>>()?;
    match helper {
        ExprHelper::Contains | ExprHelper::StartsWith | ExprHelper::EndsWith => {
            Ok(ExprType::Bool)
        }
        ExprHelper::Has => Ok(ExprType::Bool),
        ExprHelper::Exists => Ok(ExprType::Bool),
        ExprHelper::Length => Ok(ExprType::I64),
        ExprHelper::Empty => Ok(ExprType::Bool),
        ExprHelper::Append => Ok(ExprType::List),
        ExprHelper::AppendIf => Ok(ExprType::List),
        ExprHelper::Merge => Ok(ExprType::List),
        ExprHelper::Sum => Ok(ExprType::I64),
        ExprHelper::Count => Ok(ExprType::I64),
        ExprHelper::Unique => Ok(ExprType::List),
    }
}

fn is_numeric(ty: ExprType) -> bool {
    matches!(ty, ExprType::I64 | ExprType::F64)
}

fn coerce_numeric(left: ExprType, right: ExprType) -> ExprType {
    if left == ExprType::F64 || right == ExprType::F64 {
        ExprType::F64
    } else {
        ExprType::I64
    }
}

fn expect_bool_pair(left: ExprType, right: ExprType) -> ExprResult<()> {
    if (left == ExprType::Bool || left == ExprType::Unknown)
        && (right == ExprType::Bool || right == ExprType::Unknown)
    {
        Ok(())
    } else {
        Err(ExprError::TypeMismatch {
            expected: "boolean".into(),
            found: type_name(if left != ExprType::Bool && left != ExprType::Unknown {
                left
            } else {
                right
            }),
        })
    }
}

fn expect_numeric_pair(left: ExprType, right: ExprType) -> ExprResult<()> {
    if (is_numeric(left) || left == ExprType::Unknown)
        && (is_numeric(right) || right == ExprType::Unknown)
    {
        Ok(())
    } else {
        Err(ExprError::TypeMismatch {
            expected: "number".into(),
            found: type_name(if !is_numeric(left) && left != ExprType::Unknown {
                left
            } else {
                right
            }),
        })
    }
}

fn type_name(ty: ExprType) -> String {
    match ty {
        ExprType::Null => "null".into(),
        ExprType::Bool => "boolean".into(),
        ExprType::I64 => "i64".into(),
        ExprType::F64 => "f64".into(),
        ExprType::Text => "text".into(),
        ExprType::List => "list".into(),
        ExprType::Object => "object".into(),
        ExprType::Unknown => "unknown".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(source: &str) -> ExprResult<ExprType> {
        let tokens = crate::lexer::lex_expr(source)?;
        let ast = crate::parser::parse_expr(&tokens)?;
        typecheck_expr(&ast, &TypeContext::new())
    }

    #[test]
    fn infers_literal_types() -> ExprResult<()> {
        assert_eq!(check("42")?, ExprType::I64);
        assert_eq!(check("true")?, ExprType::Bool);
        assert_eq!(check("null")?, ExprType::Null);
        assert_eq!(check("\"hello\"")?, ExprType::Text);
        Ok(())
    }

    #[test]
    fn infers_arithmetic_result() -> ExprResult<()> {
        assert_eq!(check("1 + 2")?, ExprType::I64);
        Ok(())
    }

    #[test]
    fn infers_comparison_result() -> ExprResult<()> {
        assert_eq!(check("1 < 2")?, ExprType::Bool);
        assert_eq!(check("1 == 2")?, ExprType::Bool);
        Ok(())
    }

    #[test]
    fn infers_logical_result() -> ExprResult<()> {
        assert_eq!(check("true and false")?, ExprType::Bool);
        assert_eq!(check("true or false")?, ExprType::Bool);
        Ok(())
    }

    #[test]
    fn infers_helper_result() -> ExprResult<()> {
        assert_eq!(check("length($x)")?, ExprType::I64);
        assert_eq!(check("empty($x)")?, ExprType::Bool);
        assert_eq!(check("contains($x, $y)")?, ExprType::Bool);
        Ok(())
    }

    #[test]
    fn infers_unary_not() -> ExprResult<()> {
        assert_eq!(check("not true")?, ExprType::Bool);
        Ok(())
    }

    #[test]
    fn infers_negation_preserves_type() -> ExprResult<()> {
        assert_eq!(check("-42")?, ExprType::I64);
        Ok(())
    }

    #[test]
    fn unknown_type_for_unresolved_reference() -> ExprResult<()> {
        assert_eq!(check("$unknown")?, ExprType::Unknown);
        Ok(())
    }

    #[test]
    fn context_resolves_known_variables() -> ExprResult<()> {
        let mut ctx = TypeContext::new();
        ctx.add_variable(Box::from("$x"), ExprType::I64);
        let tokens = crate::lexer::lex_expr("$x + 1")?;
        let ast = crate::parser::parse_expr(&tokens)?;
        let ty = typecheck_expr(&ast, &ctx)?;
        assert_eq!(ty, ExprType::I64);
        Ok(())
    }
}
