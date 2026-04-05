use ocelot_ast::boolean_literal_expression::BooleanLiteralExpression;
use ocelot_ast::call_expression::CallExpression;
use ocelot_ast::expression::Expression;
use ocelot_ast::expression_kind::ExpressionKind;
use ocelot_ast::expression_statement::ExpressionStatement;
use ocelot_ast::function_item::FunctionItem;
use ocelot_ast::item_kind::ItemKind;
use ocelot_ast::not_expression::NotExpression;
use ocelot_ast::qualified_identifier::QualifiedIdentifier;
use ocelot_ast::statement::Statement;
use ocelot_ast::statement_kind::StatementKind;
use ocelot_base::diagnostic_level::DiagnosticLevel;
use ocelot_base::error::OcelotError;
use ocelot_base::line_bounds::LineBounds;
use ocelot_base::result::OcelotResult;
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_annotation::SourceAnnotation;
use ocelot_base::source_diagnostic::SourceDiagnostic;
use ocelot_base::source_excerpt::SourceExcerpt;
use ocelot_base::source_file::SourceFile;
use ocelot_base::span::Span;
use ocelot_pal::pal::Pal;
use ocelot_semantic::function_kind::FunctionKind;
use ocelot_semantic::native_function::NativeFunctionContext;
use ocelot_semantic::program_environment::ProgramEnvironment;
use ocelot_semantic::runtime_value::RuntimeValue;
use std::collections::HashMap;

/// Stateful AST-walking interpreter context.
pub struct Interpreter<'a> {
    environment: &'a ProgramEnvironment,
    local_bindings: HashMap<SharedString, RuntimeValue>,
    pal: &'a dyn Pal,
    source_file: &'a SourceFile,
}

impl<'a> Interpreter<'a> {
    /// Creates an interpreter bound to one PAL implementation.
    pub fn new(
        pal: &'a dyn Pal,
        source_file: &'a SourceFile,
        environment: &'a ProgramEnvironment,
    ) -> Self {
        Self::new_with_bindings(pal, source_file, environment, HashMap::new())
    }

    /// Creates an interpreter with explicit local bindings.
    pub fn new_with_bindings(
        pal: &'a dyn Pal,
        source_file: &'a SourceFile,
        environment: &'a ProgramEnvironment,
        local_bindings: HashMap<SharedString, RuntimeValue>,
    ) -> Self {
        Self {
            environment,
            local_bindings,
            pal,
            source_file,
        }
    }

    /// Executes a compilation unit AST.
    pub fn interpret_script(
        &self,
        script: &ocelot_ast::compilation_unit::CompilationUnit,
    ) -> OcelotResult<()> {
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

    fn interpret_item(&self, item: &ocelot_ast::item::Item) -> OcelotResult<()> {
        match &item.kind {
            ItemKind::Effect(_) => Ok(()),
            ItemKind::Function(_) => Ok(()),
            ItemKind::Statement(statement) => self.interpret_statement(statement),
            ItemKind::Test(_) => Ok(()),
            ItemKind::Use(_) => Ok(()),
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
            ExpressionKind::QualifiedIdentifier(identifier) => {
                self.unresolved_qualified_identifier_error(expression, identifier)
            }
            ExpressionKind::Identifier(identifier) => self
                .local_bindings
                .get(identifier.name.as_str())
                .cloned()
                .map_or_else(
                    || {
                        self.runtime_source_error(
                            format!("unresolved identifier `{}`", identifier.name),
                            expression.span.clone(),
                            "not found",
                        )
                    },
                    Ok,
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
            FunctionKind::NativeFunction { native_function } => {
                let arguments = call_expression
                    .arguments
                    .iter()
                    .map(|argument| self.evaluate_expression(argument))
                    .collect::<OcelotResult<Vec<_>>>()?;
                let context =
                    NativeFunctionContext::new(self.pal, self.source_file, expression.span.clone());
                native_function.apply(&arguments, &context)
            }
            FunctionKind::UserDefined {
                function,
                source_file,
            } => self.evaluate_user_defined_call(call_expression, function, source_file),
        }
    }

    fn evaluate_not_expression(
        &self,
        not_expression: &NotExpression,
    ) -> OcelotResult<RuntimeValue> {
        let operand = self.evaluate_expression(&not_expression.operand)?;
        let operand = operand.expect_boolean("type error: `not` expects a bool operand")?;
        Ok(RuntimeValue::boolean(!operand))
    }

    fn evaluate_user_defined_call(
        &self,
        call_expression: &CallExpression,
        function: &FunctionItem,
        source_file: &SourceFile,
    ) -> OcelotResult<RuntimeValue> {
        let mut local_bindings = HashMap::new();

        for (parameter, argument) in function.parameters.iter().zip(&call_expression.arguments) {
            local_bindings.insert(
                parameter.identifier.name.clone(),
                self.evaluate_expression(argument)?,
            );
        }

        Interpreter::new_with_bindings(self.pal, source_file, self.environment, local_bindings)
            .interpret_statements(&function.body)?;
        Ok(RuntimeValue::unit())
    }

    fn unresolved_qualified_identifier_error<T>(
        &self,
        expression: &Expression,
        identifier: &QualifiedIdentifier,
    ) -> OcelotResult<T> {
        self.runtime_source_error(
            format!("unresolved identifier `{}`", identifier.render()),
            expression.span.clone(),
            "not found",
        )
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
        let line_bounds = LineBounds::new(self.source_file.source(), span.start());
        let source_line = &self.source_file.source()[line_bounds.line_start..line_bounds.line_end];
        let relative_start = span.start().saturating_sub(line_bounds.line_start);
        let relative_end = span.end().saturating_sub(line_bounds.line_start);

        SourceDiagnostic::new(DiagnosticLevel::Error, &self.source_file.path, message).with_excerpt(
            SourceExcerpt::new(&self.source_file.path, line_bounds.line_number, source_line)
                .with_annotation(SourceAnnotation::new(
                    Span::new(relative_start, relative_end),
                    annotation,
                )),
        )
    }
}
