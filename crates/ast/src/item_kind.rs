use crate::function_item::FunctionItem;
use crate::statement::Statement;
use crate::test_item::TestItem;

/// Variants of top-level source file items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemKind {
    Function(FunctionItem),
    Statement(Statement),
    Test(TestItem),
}
