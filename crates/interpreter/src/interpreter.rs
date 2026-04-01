use ocelot_ast::expression::Expression;
use ocelot_ast::expression_kind::ExpressionKind;
use ocelot_ast::script::Script;
use ocelot_ast::statement::Statement;
use ocelot_ast::statement_kind::StatementKind;
use ocelot_base::result::OcelotResult;
use ocelot_pal::pal::Pal;

/// Stateful AST-walking interpreter context.
pub struct Interpreter<'a> {
    pal: &'a dyn Pal,
}

impl<'a> Interpreter<'a> {
    /// Creates an interpreter bound to one PAL implementation.
    pub fn new(pal: &'a dyn Pal) -> Self {
        Self { pal }
    }

    /// Executes a script AST.
    pub fn interpret_script(&self, script: &Script) -> OcelotResult<()> {
        for statement in &script.statements {
            self.interpret_statement(statement)?;
        }
        Ok(())
    }

    fn interpret_statement(&self, statement: &Statement) -> OcelotResult<()> {
        match &statement.kind {
            StatementKind::Println(println_statement) => {
                let value = self.evaluate_expression(&println_statement.argument)?;
                self.pal.print(&format!("{value}\n"))?;
                Ok(())
            }
        }
    }

    fn evaluate_expression(&self, expression: &Expression) -> OcelotResult<String> {
        match &expression.kind {
            ExpressionKind::StringLiteral(string_literal) => Ok(string_literal.value.clone()),
            ExpressionKind::Identifier(identifier) => {
                ocelot_base::bail!("unresolved identifier `{}`", identifier.name)
            }
        }
    }
}
