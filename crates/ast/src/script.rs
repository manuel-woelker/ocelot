use crate::item::Item;
use crate::item_kind::ItemKind;
use crate::statement::Statement;
use ocelot_base::span::Span;

/// Root syntax node for a script-like `ocelot` source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Script {
    pub items: Vec<Item>,
    pub span: Span,
}

impl Script {
    /// Creates a script from its items and source span.
    pub fn new(items: Vec<Item>, span: Span) -> Self {
        Self { items, span }
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

impl Default for Script {
    fn default() -> Self {
        Self::new(Vec::new(), Span::default())
    }
}
