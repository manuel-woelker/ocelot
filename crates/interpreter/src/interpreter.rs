use crate::evaluation_flow::EvaluationFlow;
use ocelot_ast::boolean_literal_expression::BooleanLiteralExpression;
use ocelot_ast::call_expression::CallExpression;
use ocelot_ast::expression::Expression;
use ocelot_ast::expression_kind::ExpressionKind;
use ocelot_ast::function_item::FunctionItem;
use ocelot_ast::item_kind::ItemKind;
use ocelot_ast::not_expression::NotExpression;
use ocelot_ast::qualified_identifier::QualifiedIdentifier;
use ocelot_ast::statement::Statement;
use ocelot_ast::statement_kind::StatementKind;
use ocelot_ast::template_string_expression::TemplateStringExpression;
use ocelot_ast::template_string_part::TemplateStringPart;
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
use ocelot_semantic::runtime_value::RuntimeValue;
use ocelot_semantic::symbol_table::SymbolTable;
use std::collections::HashMap;
use std::ops::ControlFlow;

/// Stateful AST-walking interpreter context.
pub struct Interpreter<'a> {
    symbol_table: &'a SymbolTable,
    local_bindings: HashMap<SharedString, RuntimeValue>,
    pal: &'a dyn Pal,
    source_file: &'a SourceFile,
}

impl<'a> Interpreter<'a> {
    /// Creates an interpreter bound to one PAL implementation.
    pub fn new(
        pal: &'a dyn Pal,
        source_file: &'a SourceFile,
        symbol_table: &'a SymbolTable,
    ) -> Self {
        Self::new_with_bindings(pal, source_file, symbol_table, HashMap::new())
    }

    /// Creates an interpreter with explicit local bindings.
    pub fn new_with_bindings(
        pal: &'a dyn Pal,
        source_file: &'a SourceFile,
        symbol_table: &'a SymbolTable,
        local_bindings: HashMap<SharedString, RuntimeValue>,
    ) -> Self {
        Self {
            symbol_table,
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

    /// Executes statements, stopping at a return and discarding its result.
    pub fn interpret_statements(&self, statements: &[Statement]) -> OcelotResult<()> {
        self.interpret_function_body(statements).map(|_| ())
    }

    /// Executes a function body, consuming explicit returns or producing its block result.
    pub fn interpret_function_body(&self, statements: &[Statement]) -> OcelotResult<RuntimeValue> {
        match self.evaluate_block(statements)? {
            ControlFlow::Continue(value) | ControlFlow::Break(value) => Ok(value),
        }
    }

    fn evaluate_block(&self, statements: &[Statement]) -> OcelotResult<EvaluationFlow> {
        let mut result = RuntimeValue::unit();
        for statement in statements {
            match &statement.kind {
                StatementKind::Return(value) => {
                    let flow = match &value.expression {
                        Some(expression) => self.evaluate_expression_flow(expression)?,
                        None => ControlFlow::Continue(RuntimeValue::unit()),
                    };
                    return Ok(ControlFlow::Break(match flow {
                        ControlFlow::Continue(value) | ControlFlow::Break(value) => value,
                    }));
                }
                StatementKind::Expression(value) => {
                    result = match self.evaluate_expression_flow(&value.expression)? {
                        ControlFlow::Break(value) => return Ok(ControlFlow::Break(value)),
                        ControlFlow::Continue(result) if !value.has_semicolon => result,
                        ControlFlow::Continue(_) => RuntimeValue::unit(),
                    };
                }
            }
        }
        Ok(ControlFlow::Continue(result))
    }

    fn interpret_item(&self, item: &ocelot_ast::item::Item) -> OcelotResult<()> {
        match &item.kind {
            ItemKind::Effect(_) => Ok(()),
            ItemKind::Function(_) => Ok(()),
            ItemKind::Statement(statement) => {
                self.interpret_statements(std::slice::from_ref(statement))
            }
            ItemKind::Test(_) => Ok(()),
            ItemKind::Use(_) => Ok(()),
        }
    }

    /// Evaluates one resolved expression and returns its runtime value.
    pub fn evaluate_expression(&self, expression: &Expression) -> OcelotResult<RuntimeValue> {
        match self.evaluate_expression_flow(expression)? {
            ControlFlow::Continue(value) => Ok(value),
            ControlFlow::Break(_) => self.runtime_source_error(
                "return outside function",
                expression.span.clone(),
                "not inside a function",
            ),
        }
    }

    fn evaluate_expression_flow(&self, expression: &Expression) -> OcelotResult<EvaluationFlow> {
        let value = match &expression.kind {
            ExpressionKind::Block(block) => return self.evaluate_block(&block.statements),
            ExpressionKind::BooleanLiteral(BooleanLiteralExpression { value }) => {
                RuntimeValue::boolean(*value)
            }
            ExpressionKind::StringLiteral(value) => RuntimeValue::string(value.value.clone()),
            ExpressionKind::Call(call) => return self.evaluate_call_expression(expression, call),
            ExpressionKind::Not(value) => return self.evaluate_not_expression(value),
            ExpressionKind::TemplateString(value) => {
                return self.evaluate_template_string_expression(value);
            }
            ExpressionKind::QualifiedIdentifier(value) => {
                return self.unresolved_qualified_identifier_error(expression, value);
            }
            ExpressionKind::Identifier(identifier) => {
                match self.local_bindings.get(identifier.name.as_str()) {
                    Some(value) => value.clone(),
                    None => {
                        return self.runtime_source_error(
                            format!("unresolved identifier `{}`", identifier.name),
                            expression.span.clone(),
                            "not found",
                        );
                    }
                }
            }
        };
        Ok(ControlFlow::Continue(value))
    }

    fn evaluate_call_expression(
        &self,
        expression: &Expression,
        call_expression: &CallExpression,
    ) -> OcelotResult<EvaluationFlow> {
        let function_index = call_expression.function_index()?;
        let function = self.symbol_table.function_definition(function_index)?;
        let mut arguments = Vec::with_capacity(call_expression.arguments.len());
        for argument in &call_expression.arguments {
            match self.evaluate_expression_flow(&argument.value)? {
                ControlFlow::Break(value) => return Ok(ControlFlow::Break(value)),
                ControlFlow::Continue(value) => arguments.push(value),
            }
        }
        let result = match &function.kind {
            FunctionKind::NativeFunction { native_function } => {
                let context =
                    NativeFunctionContext::new(self.pal, self.source_file, expression.span.clone());
                native_function.apply(&arguments, &context)?
            }
            FunctionKind::UserDefined {
                function,
                source_file,
            } => self.evaluate_user_defined_call(&arguments, function, source_file)?,
        };
        Ok(ControlFlow::Continue(result))
    }

    fn evaluate_not_expression(
        &self,
        not_expression: &NotExpression,
    ) -> OcelotResult<EvaluationFlow> {
        let operand = match self.evaluate_expression_flow(&not_expression.operand)? {
            ControlFlow::Break(value) => return Ok(ControlFlow::Break(value)),
            ControlFlow::Continue(value) => value,
        };
        let operand = operand.expect_boolean("type error: `not` expects a bool operand")?;
        Ok(ControlFlow::Continue(RuntimeValue::boolean(!operand)))
    }

    fn evaluate_template_string_expression(
        &self,
        template_string: &TemplateStringExpression,
    ) -> OcelotResult<EvaluationFlow> {
        let mut output = String::new();

        for part in &template_string.parts {
            match part {
                TemplateStringPart::Interpolation(expression) => {
                    let value = match self.evaluate_expression_flow(expression)? {
                        ControlFlow::Break(value) => return Ok(ControlFlow::Break(value)),
                        ControlFlow::Continue(value) => value,
                    };
                    output.push_str(value.render_for_display().as_str());
                }
                TemplateStringPart::Text(text) => output.push_str(text.as_str()),
            }
        }

        Ok(ControlFlow::Continue(RuntimeValue::string(output)))
    }

    fn evaluate_user_defined_call(
        &self,
        arguments: &[RuntimeValue],
        function: &FunctionItem,
        source_file: &SourceFile,
    ) -> OcelotResult<RuntimeValue> {
        let mut local_bindings = HashMap::new();

        for (parameter, argument) in function.parameters.iter().zip(arguments) {
            local_bindings.insert(parameter.identifier.name.clone(), argument.clone());
        }

        Interpreter::new_with_bindings(self.pal, source_file, self.symbol_table, local_bindings)
            .interpret_function_body(&function.body)
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

#[cfg(test)]
mod tests {
    use super::Interpreter;
    use ocelot_ast::boolean_literal_expression::BooleanLiteralExpression;
    use ocelot_ast::expression::Expression;
    use ocelot_ast::expression_kind::ExpressionKind;
    use ocelot_ast::identifier::Identifier;
    use ocelot_base::shared_string::SharedString;
    use ocelot_base::source_file::SourceFile;
    use ocelot_base::span::Span;
    use ocelot_pal::pal_mock::PalMock;
    use ocelot_semantic::runtime_value::RuntimeValue;
    use ocelot_semantic::symbol_table::SymbolTable;
    use std::collections::HashMap;

    #[test]
    fn evaluate_expression_returns_runtime_values() {
        let pal = PalMock::new();
        let source_file = SourceFile::new("examples/main.ocelot-script", "true;");
        let symbol_table = SymbolTable::new();
        let expression = Expression::new(
            ExpressionKind::BooleanLiteral(BooleanLiteralExpression::new(true)),
            Span::new(0, 4),
        );

        let result = Interpreter::new(&pal, &source_file, &symbol_table)
            .evaluate_expression(&expression)
            .unwrap();

        assert_eq!(result, RuntimeValue::boolean(true));
    }

    #[test]
    fn evaluate_expression_reads_local_bindings() {
        let pal = PalMock::new();
        let source_file = SourceFile::new("examples/main.ocelot-script", "name;");
        let symbol_table = SymbolTable::new();
        let expression = Expression::new(
            ExpressionKind::Identifier(Identifier::new("name", Span::new(0, 4))),
            Span::new(0, 4),
        );
        let interpreter = Interpreter::new_with_bindings(
            &pal,
            &source_file,
            &symbol_table,
            HashMap::from([(SharedString::from("name"), RuntimeValue::string("Ada"))]),
        );

        let result = interpreter.evaluate_expression(&expression).unwrap();

        assert_eq!(result, RuntimeValue::string("Ada"));
    }
}
