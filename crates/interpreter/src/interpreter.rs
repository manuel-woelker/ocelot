use crate::runtime_value::RuntimeValue;
use ocelot_ast::boolean_literal_expression::BooleanLiteralExpression;
use ocelot_ast::call_expression::CallExpression;
use ocelot_ast::expression::Expression;
use ocelot_ast::expression_kind::ExpressionKind;
use ocelot_ast::expression_statement::ExpressionStatement;
use ocelot_ast::function_item::FunctionItem;
use ocelot_ast::function_kind::FunctionKind;
use ocelot_ast::item::Item;
use ocelot_ast::item_kind::ItemKind;
use ocelot_ast::not_expression::NotExpression;
use ocelot_ast::program_environment::ProgramEnvironment;
use ocelot_ast::script::Script;
use ocelot_ast::statement::Statement;
use ocelot_ast::statement_kind::StatementKind;
use ocelot_base::assertion_error::AssertionError;
use ocelot_base::diagnostic_level::DiagnosticLevel;
use ocelot_base::error::OcelotError;
use ocelot_base::result::{OcelotResult, OptionExt};
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_annotation::SourceAnnotation;
use ocelot_base::source_diagnostic::SourceDiagnostic;
use ocelot_base::source_excerpt::SourceExcerpt;
use ocelot_base::source_file::SourceFile;
use ocelot_base::span::Span;
use ocelot_pal::pal::Pal;

/// Stateful AST-walking interpreter context.
pub struct Interpreter<'a> {
    environment: &'a ProgramEnvironment,
    pal: &'a dyn Pal,
    script: &'a Script,
    source_file: &'a SourceFile,
}

impl<'a> Interpreter<'a> {
    /// Creates an interpreter bound to one PAL implementation.
    pub fn new(
        pal: &'a dyn Pal,
        script: &'a Script,
        source_file: &'a SourceFile,
        environment: &'a ProgramEnvironment,
    ) -> Self {
        Self {
            environment,
            pal,
            script,
            source_file,
        }
    }

    /// Executes a script AST.
    pub fn interpret_script(&self, script: &Script) -> OcelotResult<()> {
        for item in &script.items {
            self.interpret_item(item)?;
        }
        Ok(())
    }

    /// Executes one ordered sequence of statements.
    pub fn interpret_statements(&self, statements: &[Statement]) -> OcelotResult<()> {
        for statement in statements {
            self.interpret_statement(statement)?;
        }
        Ok(())
    }

    fn interpret_item(&self, item: &Item) -> OcelotResult<()> {
        match &item.kind {
            ItemKind::Function(_) => Ok(()),
            ItemKind::Statement(statement) => self.interpret_statement(statement),
            ItemKind::Test(_) => Ok(()),
        }
    }

    fn interpret_statement(&self, statement: &Statement) -> OcelotResult<()> {
        match &statement.kind {
            StatementKind::Expression(ExpressionStatement { expression }) => {
                self.evaluate_expression(expression)?;
                Ok(())
            }
        }
    }

    fn evaluate_expression(&self, expression: &Expression) -> OcelotResult<RuntimeValue> {
        match &expression.kind {
            ExpressionKind::BooleanLiteral(BooleanLiteralExpression { value }) => {
                Ok(RuntimeValue::boolean(*value))
            }
            ExpressionKind::Call(call_expression) => {
                self.evaluate_call_expression(expression, call_expression)
            }
            ExpressionKind::Not(not_expression) => self.evaluate_not_expression(not_expression),
            ExpressionKind::StringLiteral(string_literal) => {
                Ok(RuntimeValue::string(string_literal.value.clone()))
            }
            ExpressionKind::Identifier(identifier) => self.runtime_source_error(
                format!("unresolved identifier `{}`", identifier.name),
                expression.span.clone(),
                "not found",
            ),
        }
    }

    fn evaluate_call_expression(
        &self,
        expression: &Expression,
        call_expression: &CallExpression,
    ) -> OcelotResult<RuntimeValue> {
        let function_index = call_expression.function_index()?;
        let function = self.environment.function_definition(function_index)?;

        match &function.kind {
            FunctionKind::Native { native_function } => match native_function {
                ocelot_ast::native_function::NativeFunction::Assert => {
                    self.evaluate_assert_call(expression, call_expression)
                }
                ocelot_ast::native_function::NativeFunction::AssertEq => {
                    self.evaluate_assert_eq_call(expression, call_expression)
                }
                ocelot_ast::native_function::NativeFunction::Println => {
                    self.evaluate_println_call(call_expression)
                }
            },
            FunctionKind::UserDefined { item_index } => {
                self.evaluate_user_defined_call(call_expression, *item_index)
            }
        }
    }

    fn evaluate_not_expression(
        &self,
        not_expression: &NotExpression,
    ) -> OcelotResult<RuntimeValue> {
        let operand = self.evaluate_expression(&not_expression.operand)?;
        let operand = operand.expect_boolean("type error: `not` expects a boolean operand")?;
        Ok(RuntimeValue::boolean(!operand))
    }

    fn evaluate_println_call(
        &self,
        call_expression: &CallExpression,
    ) -> OcelotResult<RuntimeValue> {
        if call_expression.arguments.len() != 1 {
            ocelot_base::bail!("type error: `println` expects exactly one argument");
        }

        let value = self.evaluate_expression(&call_expression.arguments[0])?;
        let text = match &value {
            RuntimeValue::Boolean(_) | RuntimeValue::String(_) => value.render_for_display(),
            RuntimeValue::Unit => {
                ocelot_base::bail!("type error: `println` expects a string or boolean argument")
            }
        };

        self.pal.print(&format!("{text}\n"))?;
        Ok(RuntimeValue::unit())
    }

    fn evaluate_assert_call(
        &self,
        expression: &Expression,
        call_expression: &CallExpression,
    ) -> OcelotResult<RuntimeValue> {
        if call_expression.arguments.len() != 1 {
            ocelot_base::bail!("type error: `assert` expects exactly one argument");
        }

        let condition = self.evaluate_expression(&call_expression.arguments[0])?;
        let condition =
            condition.expect_boolean("type error: `assert` expects a boolean argument")?;

        if condition {
            return Ok(RuntimeValue::unit());
        }

        Err(OcelotError::assertion_error(
            AssertionError::new_without_diff(
                self.source_file,
                expression.span.clone(),
                "assert condition was false",
            ),
        ))
    }

    fn evaluate_assert_eq_call(
        &self,
        expression: &Expression,
        call_expression: &CallExpression,
    ) -> OcelotResult<RuntimeValue> {
        if call_expression.arguments.len() != 2 {
            ocelot_base::bail!("type error: `assert_eq` expects exactly two arguments");
        }

        let expected = self.evaluate_expression(&call_expression.arguments[0])?;
        let actual = self.evaluate_expression(&call_expression.arguments[1])?;

        if expected.equals(&actual) {
            return Ok(RuntimeValue::unit());
        }

        Err(OcelotError::assertion_error(AssertionError::new(
            self.source_file,
            expression.span.clone(),
            "assert_eq values differ",
            expected.render_for_assertion(),
            actual.render_for_assertion(),
        )))
    }

    fn evaluate_user_defined_call(
        &self,
        call_expression: &CallExpression,
        item_index: usize,
    ) -> OcelotResult<RuntimeValue> {
        if !call_expression.arguments.is_empty() {
            ocelot_base::bail!("type error: user-defined functions do not accept arguments yet");
        }

        let function_item = self.lookup_function_item(item_index)?;
        self.interpret_statements(&function_item.body)?;
        Ok(RuntimeValue::unit())
    }

    fn lookup_function_item(&self, item_index: usize) -> OcelotResult<&FunctionItem> {
        let item = self
            .script
            .items
            .get(item_index)
            .context("internal error: function item index points outside the script")?;

        let ItemKind::Function(function_item) = &item.kind else {
            ocelot_base::bail!("internal error: function item index did not reference a function");
        };

        Ok(function_item)
    }

    fn runtime_source_error<T>(
        &self,
        message: impl Into<SharedString>,
        span: Span,
        annotation: impl Into<SharedString>,
    ) -> OcelotResult<T> {
        let diagnostic = self.source_diagnostic(message, span, annotation);
        Err(OcelotError::runtime_error(diagnostic))
    }

    fn source_diagnostic(
        &self,
        message: impl Into<SharedString>,
        span: Span,
        annotation: impl Into<SharedString>,
    ) -> SourceDiagnostic {
        let message = message.into();
        let annotation = annotation.into();
        let (line_number, line_start, line_end) = self.line_bounds(span.start());
        let source_line = &self.source_file.source()[line_start..line_end];
        let relative_start = span.start().saturating_sub(line_start);
        let relative_end = span.end().saturating_sub(line_start);

        SourceDiagnostic::new(DiagnosticLevel::Error, &self.source_file.path, message).with_excerpt(
            SourceExcerpt::new(&self.source_file.path, line_number, source_line).with_annotation(
                SourceAnnotation::new(Span::new(relative_start, relative_end), annotation),
            ),
        )
    }

    fn line_bounds(&self, index: usize) -> (usize, usize, usize) {
        let source = self.source_file.source();
        let line_start = source[..index].rfind('\n').map_or(0, |offset| offset + 1);
        let line_end = source[index..]
            .find('\n')
            .map_or(source.len(), |offset| index + offset);
        let line_number = source[..line_start]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count()
            + 1;

        (line_number, line_start, line_end)
    }
}
