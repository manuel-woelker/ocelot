use ocelot_base::shared_string::SharedString;
use ocelot_base::span::Span;

/// Formatter-relevant trivia attached to tokens and AST nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriviaPiece {
    LineComment { text: SharedString, span: Span },
    BlockComment { text: SharedString, span: Span },
    Newlines { count: usize, span: Span },
}

impl TriviaPiece {
    /// Returns the trivia source span.
    pub fn span(&self) -> &Span {
        match self {
            Self::LineComment { span, .. } => span,
            Self::BlockComment { span, .. } => span,
            Self::Newlines { span, .. } => span,
        }
    }

    /// Returns whether the trivia piece is a comment.
    pub const fn is_comment(&self) -> bool {
        matches!(self, Self::LineComment { .. } | Self::BlockComment { .. })
    }
}
