use crate::expression_kind::ExpressionKind;
use crate::type_index::TypeIndex;
use ocelot_base::span::Span;

/// Expression node with an explicit source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expression {
    pub kind: ExpressionKind,
    pub span: Span,
    pub ty: TypeIndex,
}

impl Expression {
    /// Removes explicit grouping wrappers for syntactic direct-call lookup.
    pub fn ungrouped(&self) -> &Self {
        let mut expression = self;
        while let ExpressionKind::Parenthesized(group) = &expression.kind {
            expression = &group.expression;
        }
        expression
    }
    /// Creates an expression from its kind and source span.
    pub fn new(kind: ExpressionKind, span: Span) -> Self {
        Self {
            kind,
            span,
            ty: TypeIndex::unresolved(),
        }
    }
}
