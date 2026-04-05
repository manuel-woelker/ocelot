use crate::trivia_piece::TriviaPiece;

/// Trivia attached to AST nodes or tokens.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Trivia {
    pub leading: Vec<TriviaPiece>,
    pub trailing: Vec<TriviaPiece>,
}

impl Trivia {
    /// Creates trivia from leading and trailing pieces.
    pub fn new(leading: Vec<TriviaPiece>, trailing: Vec<TriviaPiece>) -> Self {
        Self { leading, trailing }
    }

    /// Returns whether both trivia sides are empty.
    pub fn is_empty(&self) -> bool {
        self.leading.is_empty() && self.trailing.is_empty()
    }

    /// Returns whether any trivia piece is a comment.
    pub fn has_comments(&self) -> bool {
        self.leading.iter().any(TriviaPiece::is_comment)
            || self.trailing.iter().any(TriviaPiece::is_comment)
    }
}
