use crate::expression::Expression;
use crate::function_index::FunctionIndex;
use ocelot_base::result::{OcelotResult, OptionExt};

/// Function call expression payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallExpression {
    pub callee: Box<Expression>,
    pub arguments: Vec<Expression>,
    pub resolved_function_index: Option<FunctionIndex>,
}

impl CallExpression {
    /// Creates a call expression payload.
    pub fn new(callee: Expression, arguments: Vec<Expression>) -> Self {
        Self {
            callee: Box::new(callee),
            arguments,
            resolved_function_index: None,
        }
    }

    /// Records the resolved function index for this call.
    pub fn resolve_to(&mut self, function_index: FunctionIndex) {
        self.resolved_function_index = Some(function_index);
    }

    /// Returns the resolved function index for this call.
    pub fn function_index(&self) -> OcelotResult<FunctionIndex> {
        self.resolved_function_index
            .context("internal error: call expression was not resolved")
    }
}

#[cfg(test)]
mod tests {
    use super::CallExpression;
    use crate::expression::Expression;
    use crate::expression_kind::ExpressionKind;
    use crate::function_index::FunctionIndex;
    use crate::identifier_expression::IdentifierExpression;
    use ocelot_base::span::Span;

    fn identifier(name: &str) -> Expression {
        Expression::new(
            ExpressionKind::Identifier(IdentifierExpression::new(name)),
            Span::new(0, name.len()),
        )
    }

    #[test]
    fn new_call_expression_starts_unresolved() {
        let call_expression = CallExpression::new(identifier("println"), Vec::new());

        assert_eq!(call_expression.resolved_function_index, None);
    }

    #[test]
    fn function_index_returns_the_resolved_index() {
        let mut call_expression = CallExpression::new(identifier("println"), Vec::new());
        call_expression.resolve_to(FunctionIndex::new(1));

        assert_eq!(
            call_expression.function_index().unwrap(),
            FunctionIndex::new(1)
        );
    }

    #[test]
    fn function_index_reports_unresolved_calls() {
        let call_expression = CallExpression::new(identifier("println"), Vec::new());

        assert!(
            call_expression
                .function_index()
                .unwrap_err()
                .to_test_string()
                .contains("call expression was not resolved")
        );
    }
}
