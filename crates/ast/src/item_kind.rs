use crate::effect_item::EffectItem;
use crate::function_item::FunctionItem;
use crate::statement::Statement;
use crate::test_item::TestItem;
use crate::use_item::UseItem;

/// Variants of top-level source file items.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemKind {
    Effect(EffectItem),
    Function(FunctionItem),
    Statement(Statement),
    Test(TestItem),
    Use(UseItem),
}
