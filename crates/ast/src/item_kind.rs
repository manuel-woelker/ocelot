use crate::statement::Statement;
use crate::test_item::TestItem;

/// Variants of top-level source file items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemKind {
    Statement(Statement),
    Test(TestItem),
}
