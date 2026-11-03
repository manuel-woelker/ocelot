use crate::function_effect_clause::FunctionEffectClause;
use crate::function_parameter::FunctionParameter;
use crate::identifier::Identifier;
use crate::statement::Statement;
use crate::type_index::TypeIndex;
use ocelot_base::span::Span;

/// Top-level user-defined function declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionItem {
    pub is_native: bool,
    pub identifier: Identifier,
    pub parameters: Vec<FunctionParameter>,
    /// Explicit result type spelling and span, absent when unit is implicit.
    pub return_type_name: Option<Identifier>,
    /// The result type filled in during declaration resolution.
    pub return_type: TypeIndex,
    pub can_clause: Option<FunctionEffectClause>,
    pub cannot_clause: Option<FunctionEffectClause>,
    pub body: Vec<Statement>,
    pub span: Span,
}

impl FunctionItem {
    /// Creates a function item from its identifier, body, and source span.
    pub fn new(
        identifier: Identifier,
        parameters: Vec<FunctionParameter>,
        can_clause: Option<FunctionEffectClause>,
        cannot_clause: Option<FunctionEffectClause>,
        body: Vec<Statement>,
        span: Span,
    ) -> Self {
        Self {
            is_native: false,
            identifier,
            parameters,
            return_type_name: None,
            return_type: TypeIndex::unresolved(),
            can_clause,
            cannot_clause,
            body,
            span,
        }
    }

    /// Creates a native function item from its identifier, signature, and source span.
    pub fn new_native(
        identifier: Identifier,
        parameters: Vec<FunctionParameter>,
        can_clause: Option<FunctionEffectClause>,
        cannot_clause: Option<FunctionEffectClause>,
        span: Span,
    ) -> Self {
        Self {
            is_native: true,
            identifier,
            parameters,
            return_type_name: None,
            return_type: TypeIndex::unresolved(),
            can_clause,
            cannot_clause,
            body: Vec::new(),
            span,
        }
    }
}
