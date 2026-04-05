use crate::item::Item;
use crate::item_kind::ItemKind;
use crate::statement::Statement;
use crate::trivia::Trivia;
use ocelot_base::span::Span;

/// Root syntax node for one parsed `ocelot` source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilationUnit {
    pub items: Vec<Item>,
    pub trivia: Trivia,
    pub span: Span,
}

impl CompilationUnit {
    /// Creates a compilation unit from its items and source span.
    pub fn new(items: Vec<Item>, span: Span) -> Self {
        Self::with_trivia(items, Trivia::default(), span)
    }

    /// Creates a compilation unit from its items, trivia, and source span.
    pub fn with_trivia(items: Vec<Item>, trivia: Trivia, span: Span) -> Self {
        Self {
            items,
            trivia,
            span,
        }
    }

    /// Returns executable top-level statements in source order.
    pub fn statements(&self) -> impl Iterator<Item = &Statement> {
        self.items.iter().filter_map(|item| match &item.kind {
            ItemKind::Effect(_) => None,
            ItemKind::Function(_) => None,
            ItemKind::Statement(statement) => Some(statement),
            ItemKind::Test(_) => None,
            ItemKind::Use(_) => None,
        })
    }
}

impl Default for CompilationUnit {
    fn default() -> Self {
        Self::new(Vec::new(), Span::default())
    }
}
