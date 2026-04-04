//! Name resolution for `ocelot`.

use ocelot_ast::call_expression::CallExpression;
use ocelot_ast::expression::Expression;
use ocelot_ast::expression_kind::ExpressionKind;
use ocelot_ast::expression_statement::ExpressionStatement;
use ocelot_ast::function_definition::FunctionDefinition;
use ocelot_ast::function_item::FunctionItem;
use ocelot_ast::function_kind::FunctionKind;
use ocelot_ast::item::Item;
use ocelot_ast::item_kind::ItemKind;
use ocelot_ast::program_environment::ProgramEnvironment;
use ocelot_ast::script::Script;
use ocelot_ast::statement::Statement;
use ocelot_ast::statement_kind::StatementKind;
use ocelot_ast::test_item::TestItem;
use ocelot_base::compilation_context::CompilationContext;
use ocelot_base::compilation_stage::CompilationStage;
use ocelot_base::diagnostic_level::DiagnosticLevel;
use ocelot_base::error::OcelotError;
use ocelot_base::render_source_diagnostics::render_source_diagnostics;
use ocelot_base::result::OcelotResult;
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_annotation::SourceAnnotation;
use ocelot_base::source_diagnostic::SourceDiagnostic;
use ocelot_base::source_excerpt::SourceExcerpt;
use ocelot_base::source_file::SourceFile;
use ocelot_base::span::Span;

/// Resolves names within a parsed program.
pub fn resolve(
    script: &mut Script,
    source_file: &SourceFile,
    compilation_context: &mut CompilationContext,
    environment: &mut ProgramEnvironment,
) -> OcelotResult<()> {
    Resolver::new(source_file, compilation_context, environment).resolve_script(script)
}

struct Resolver<'a> {
    source_file: &'a SourceFile,
    compilation_context: &'a mut CompilationContext,
    environment: &'a mut ProgramEnvironment,
}

impl<'a> Resolver<'a> {
    fn new(
        source_file: &'a SourceFile,
        compilation_context: &'a mut CompilationContext,
        environment: &'a mut ProgramEnvironment,
    ) -> Self {
        Self {
            source_file,
            compilation_context,
            environment,
        }
    }

    fn resolve_script(&mut self, script: &mut Script) -> OcelotResult<()> {
        self.lower_function_items(script);

        for item in &mut script.items {
            self.resolve_item(item);
        }

        self.resolve_user_defined_function_definitions()?;

        if self.compilation_context.has_errors() {
            return Err(
                OcelotError::compilation_error(CompilationStage::Resolver).with_source(
                    OcelotError::message(render_source_diagnostics(
                        &self.compilation_context.source_diagnostics.diagnostics,
                    )),
                ),
            );
        }

        Ok(())
    }

    fn lower_function_items(&mut self, script: &mut Script) {
        let mut retained_items = Vec::with_capacity(script.items.len());

        for item in std::mem::take(&mut script.items) {
            match item.kind {
                ItemKind::Function(function_item) => self.register_function_item(function_item),
                _ => retained_items.push(item),
            }
        }

        script.items = retained_items;
    }

    fn resolve_item(&mut self, item: &mut Item) {
        match &mut item.kind {
            ItemKind::Statement(statement) => self.resolve_statement(statement),
            ItemKind::Test(test_item) => self.resolve_test_item(test_item),
            ItemKind::Function(_) => {
                unreachable!("function items should be lowered before item resolution")
            }
        }
    }

    fn register_function_item(&mut self, function_item: FunctionItem) {
        if let Some(function_index) = self
            .environment
            .resolve_function(&function_item.identifier.name)
        {
            let existing_function = self
                .environment
                .function_definition(function_index)
                .expect("resolved function index should point at a definition");
            let duplicate_with_original = match &existing_function.kind {
                FunctionKind::Native { .. } => None,
                FunctionKind::UserDefined { function } => Some((**function).clone()),
            };

            if let Some(original_function) = duplicate_with_original {
                self.add_duplicate_function_diagnostic(&function_item, &original_function);
            } else {
                self.add_diagnostic(
                    format!(
                        "function `{}` conflicts with native function",
                        function_item.identifier.name
                    ),
                    function_item.identifier.span.clone(),
                    "duplicate function",
                );
            }
            return;
        }

        self.environment
            .add_function(FunctionDefinition::user_defined(function_item));
    }

    fn resolve_function_item(&mut self, function_item: &mut FunctionItem) {
        for statement in &mut function_item.body {
            self.resolve_statement(statement);
        }
    }

    fn resolve_user_defined_function_definitions(&mut self) -> OcelotResult<()> {
        let function_indices = self.environment.user_defined_function_indices();

        for function_index in function_indices {
            let mut function = self
                .environment
                .take_user_defined_function(function_index)?;
            self.resolve_function_item(&mut function);
            self.environment
                .put_user_defined_function(function_index, function)?;
        }

        Ok(())
    }

    fn resolve_test_item(&mut self, test_item: &mut TestItem) {
        for statement in &mut test_item.body {
            self.resolve_statement(statement);
        }
    }

    fn resolve_statement(&mut self, statement: &mut Statement) {
        match &mut statement.kind {
            StatementKind::Expression(ExpressionStatement { expression }) => {
                self.resolve_expression(expression);
            }
        }
    }

    fn resolve_expression(&mut self, expression: &mut Expression) {
        match &mut expression.kind {
            ExpressionKind::BooleanLiteral(_)
            | ExpressionKind::Identifier(_)
            | ExpressionKind::StringLiteral(_) => {}
            ExpressionKind::Not(not_expression) => {
                self.resolve_expression(&mut not_expression.operand);
            }
            ExpressionKind::Call(call_expression) => self.resolve_call_expression(call_expression),
        }
    }

    fn resolve_call_expression(&mut self, call_expression: &mut CallExpression) {
        self.resolve_expression(&mut call_expression.callee);
        for argument in &mut call_expression.arguments {
            self.resolve_expression(argument);
        }

        let ExpressionKind::Identifier(identifier) = &call_expression.callee.kind else {
            self.add_diagnostic(
                "only identifier calls are supported",
                call_expression.callee.span.clone(),
                "callee must be an identifier",
            );
            return;
        };

        let Some(function_index) = self.environment.resolve_function(&identifier.name) else {
            self.add_diagnostic(
                format!("unknown function `{}`", identifier.name),
                call_expression.callee.span.clone(),
                "unknown function",
            );
            return;
        };

        call_expression.resolve_to(function_index);
    }

    fn add_diagnostic(
        &mut self,
        message: impl Into<SharedString>,
        span: Span,
        annotation: impl Into<SharedString>,
    ) {
        self.compilation_context
            .add_diagnostic(self.source_diagnostic(message, span, annotation));
    }

    fn add_duplicate_function_diagnostic(
        &mut self,
        duplicate_function: &FunctionItem,
        original_function: &FunctionItem,
    ) {
        let diagnostic = self
            .source_diagnostic(
                format!(
                    "duplicate function `{}`",
                    duplicate_function.identifier.name
                ),
                duplicate_function.identifier.span.clone(),
                "duplicate function",
            )
            .with_excerpt(self.source_excerpt(
                original_function.identifier.span.clone(),
                "already defined here",
            ));
        self.compilation_context.add_diagnostic(diagnostic);
    }

    fn source_diagnostic(
        &self,
        message: impl Into<SharedString>,
        span: Span,
        annotation: impl Into<SharedString>,
    ) -> SourceDiagnostic {
        let message = message.into();
        SourceDiagnostic::new(DiagnosticLevel::Error, &self.source_file.path, message)
            .with_excerpt(self.source_excerpt(span, annotation))
    }

    fn source_excerpt(&self, span: Span, annotation: impl Into<SharedString>) -> SourceExcerpt {
        let annotation = annotation.into();
        let (line_number, line_start, line_end) = self.line_bounds(span.start());
        let source_line = &self.source_file.source()[line_start..line_end];
        let relative_start = span.start().saturating_sub(line_start);
        let relative_end = span.end().saturating_sub(line_start);

        SourceExcerpt::new(&self.source_file.path, line_number, source_line).with_annotation(
            SourceAnnotation::new(Span::new(relative_start, relative_end), annotation),
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

#[cfg(test)]
mod tests {
    use super::resolve;
    use ocelot_ast::call_expression::CallExpression;
    use ocelot_ast::expression::Expression;
    use ocelot_ast::expression_kind::ExpressionKind;
    use ocelot_ast::expression_statement::ExpressionStatement;
    use ocelot_ast::function_definition::FunctionDefinition;
    use ocelot_ast::function_item::FunctionItem;
    use ocelot_ast::function_kind::FunctionKind;
    use ocelot_ast::identifier::Identifier;
    use ocelot_ast::identifier_expression::IdentifierExpression;
    use ocelot_ast::item::Item;
    use ocelot_ast::item_kind::ItemKind;
    use ocelot_ast::native_function::NativeFunction;
    use ocelot_ast::program_environment::ProgramEnvironment;
    use ocelot_ast::script::Script;
    use ocelot_ast::statement::Statement;
    use ocelot_ast::statement_kind::StatementKind;
    use ocelot_ast::string_literal_expression::StringLiteralExpression;
    use ocelot_ast::test_item::TestItem;
    use ocelot_base::compilation_context::CompilationContext;
    use ocelot_base::compilation_stage::CompilationStage;
    use ocelot_base::source_file::SourceFile;
    use ocelot_base::span::Span;

    fn identifier(name: &str, span: Span) -> Expression {
        Expression::new(
            ExpressionKind::Identifier(IdentifierExpression::new(name)),
            span,
        )
    }

    fn string_literal(value: &str, span: Span) -> Expression {
        Expression::new(
            ExpressionKind::StringLiteral(StringLiteralExpression::new(value)),
            span,
        )
    }

    fn call(callee: Expression, arguments: Vec<Expression>, span: Span) -> Expression {
        Expression::new(
            ExpressionKind::Call(CallExpression::new(callee, arguments)),
            span,
        )
    }

    fn test_program_environment() -> ProgramEnvironment {
        ProgramEnvironment::new(vec![
            FunctionDefinition::native("println", NativeFunction::Println),
            FunctionDefinition::native("assert", NativeFunction::Assert),
            FunctionDefinition::native("assert_eq", NativeFunction::AssertEq),
        ])
    }

    #[test]
    fn resolves_native_call_expressions() {
        let mut script = Script::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call(
                        identifier("println", Span::new(0, 7)),
                        vec![string_literal("hello", Span::new(8, 15))],
                        Span::new(0, 16),
                    ))),
                    Span::new(0, 17),
                )),
                Span::new(0, 17),
            )],
            Span::new(0, 17),
        );
        let source_file = SourceFile::new("examples/hello.ocelot", "println(\"hello\");");
        let mut environment = test_program_environment();
        let println_index = environment.resolve_function("println").unwrap();
        let mut context = CompilationContext::default();

        resolve(&mut script, &source_file, &mut context, &mut environment).unwrap();

        let ItemKind::Statement(statement) = &script.items[0].kind else {
            panic!("expected statement");
        };
        let StatementKind::Expression(ExpressionStatement { expression }) = &statement.kind;
        let ExpressionKind::Call(call_expression) = &expression.kind else {
            panic!("expected call expression");
        };
        assert_eq!(call_expression.function_index().unwrap(), println_index);
    }

    #[test]
    fn resolves_calls_inside_test_items() {
        let mut script = Script::new(
            vec![Item::new(
                ItemKind::Test(TestItem::new(
                    "prints",
                    vec![Statement::new(
                        StatementKind::Expression(ExpressionStatement::new(call(
                            identifier("assert", Span::new(17, 23)),
                            vec![identifier("true", Span::new(24, 28))],
                            Span::new(17, 29),
                        ))),
                        Span::new(17, 30),
                    )],
                    Span::new(0, 32),
                )),
                Span::new(0, 32),
            )],
            Span::new(0, 32),
        );
        let source_file =
            SourceFile::new("examples/tests.ocelot", "test \"prints\" { assert(true); }");
        let mut environment = test_program_environment();
        let assert_index = environment.resolve_function("assert").unwrap();
        let mut context = CompilationContext::default();

        resolve(&mut script, &source_file, &mut context, &mut environment).unwrap();

        let ItemKind::Test(test_item) = &script.items[0].kind else {
            panic!("expected test item");
        };
        let StatementKind::Expression(ExpressionStatement { expression }) = &test_item.body[0].kind;
        let ExpressionKind::Call(call_expression) = &expression.kind else {
            panic!("expected call expression");
        };
        assert_eq!(call_expression.function_index().unwrap(), assert_index);
    }

    #[test]
    fn resolves_nested_calls_recursively() {
        let mut script = Script::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call(
                        identifier("println", Span::new(0, 7)),
                        vec![call(
                            identifier("println", Span::new(8, 15)),
                            vec![string_literal("hello", Span::new(16, 23))],
                            Span::new(8, 24),
                        )],
                        Span::new(0, 25),
                    ))),
                    Span::new(0, 26),
                )),
                Span::new(0, 26),
            )],
            Span::new(0, 26),
        );
        let source_file = SourceFile::new("examples/nested.ocelot", "println(println(\"hello\"));");
        let mut environment = test_program_environment();
        let println_index = environment.resolve_function("println").unwrap();
        let mut context = CompilationContext::default();

        resolve(&mut script, &source_file, &mut context, &mut environment).unwrap();

        let ItemKind::Statement(statement) = &script.items[0].kind else {
            panic!("expected statement");
        };
        let StatementKind::Expression(ExpressionStatement { expression }) = &statement.kind;
        let ExpressionKind::Call(outer_call) = &expression.kind else {
            panic!("expected outer call");
        };
        assert_eq!(outer_call.function_index().unwrap(), println_index);

        let ExpressionKind::Call(inner_call) = &outer_call.arguments[0].kind else {
            panic!("expected nested call");
        };
        assert_eq!(inner_call.function_index().unwrap(), println_index);
    }

    #[test]
    fn reports_unknown_function_calls_as_resolver_errors() {
        let mut script = Script::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call(
                        identifier("printline", Span::new(0, 9)),
                        vec![string_literal("hello", Span::new(10, 17))],
                        Span::new(0, 18),
                    ))),
                    Span::new(0, 19),
                )),
                Span::new(0, 19),
            )],
            Span::new(0, 19),
        );
        let source_file = SourceFile::new("examples/broken.ocelot", "printline(\"hello\");");
        let mut environment = test_program_environment();
        let mut context = CompilationContext::default();

        let error = resolve(&mut script, &source_file, &mut context, &mut environment).unwrap_err();

        assert!(matches!(
            error.kind(),
            ocelot_base::error::ErrorKind::CompilationError(CompilationStage::Resolver)
        ));
        assert!(
            error
                .to_test_string()
                .contains("unknown function `printline`")
        );
        assert!(error.to_test_string().contains("printline(\"hello\");"));
        assert!(
            error
                .to_test_string()
                .contains("at examples/broken.ocelot:1:1")
        );
    }

    #[test]
    fn reports_non_identifier_callees_as_resolver_errors() {
        let mut script = Script::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call(
                        string_literal("hello", Span::new(0, 7)),
                        Vec::new(),
                        Span::new(0, 9),
                    ))),
                    Span::new(0, 10),
                )),
                Span::new(0, 10),
            )],
            Span::new(0, 10),
        );
        let source_file = SourceFile::new("examples/broken.ocelot", "\"hello\"();");
        let mut environment = test_program_environment();
        let mut context = CompilationContext::default();

        let error = resolve(&mut script, &source_file, &mut context, &mut environment).unwrap_err();

        assert!(
            error
                .to_test_string()
                .contains("only identifier calls are supported")
        );
    }

    #[test]
    fn resolves_calls_to_user_defined_functions() {
        let mut script = Script::new(
            vec![
                Item::new(
                    ItemKind::Function(FunctionItem::new(
                        Identifier::new("greet", Span::new(4, 9)),
                        vec![Statement::new(
                            StatementKind::Expression(ExpressionStatement::new(call(
                                identifier("println", Span::new(14, 21)),
                                vec![string_literal("hello", Span::new(22, 29))],
                                Span::new(14, 30),
                            ))),
                            Span::new(14, 31),
                        )],
                        Span::new(0, 33),
                    )),
                    Span::new(0, 33),
                ),
                Item::new(
                    ItemKind::Statement(Statement::new(
                        StatementKind::Expression(ExpressionStatement::new(call(
                            identifier("greet", Span::new(34, 39)),
                            Vec::new(),
                            Span::new(34, 41),
                        ))),
                        Span::new(34, 42),
                    )),
                    Span::new(34, 42),
                ),
            ],
            Span::new(0, 42),
        );
        let source_file = SourceFile::new(
            "examples/functions.ocelot",
            "fun greet() { println(\"hello\"); } greet();",
        );
        let mut environment = test_program_environment();
        let mut context = CompilationContext::default();

        resolve(&mut script, &source_file, &mut context, &mut environment).unwrap();

        let greet_index = environment.resolve_function("greet").unwrap();
        assert_eq!(script.items.len(), 1);
        let ItemKind::Statement(statement) = &script.items[0].kind else {
            panic!("expected statement");
        };
        let StatementKind::Expression(ExpressionStatement { expression }) = &statement.kind;
        let ExpressionKind::Call(call_expression) = &expression.kind else {
            panic!("expected call expression");
        };

        assert_eq!(call_expression.function_index().unwrap(), greet_index);
    }

    #[test]
    fn resolves_forward_references_to_later_function_definitions() {
        let mut script = Script::new(
            vec![
                Item::new(
                    ItemKind::Statement(Statement::new(
                        StatementKind::Expression(ExpressionStatement::new(call(
                            identifier("greet", Span::new(0, 5)),
                            Vec::new(),
                            Span::new(0, 7),
                        ))),
                        Span::new(0, 8),
                    )),
                    Span::new(0, 8),
                ),
                Item::new(
                    ItemKind::Function(FunctionItem::new(
                        Identifier::new("greet", Span::new(13, 18)),
                        Vec::new(),
                        Span::new(9, 22),
                    )),
                    Span::new(9, 22),
                ),
            ],
            Span::new(0, 22),
        );
        let source_file = SourceFile::new("examples/functions.ocelot", "greet(); fun greet() {}");
        let mut environment = test_program_environment();
        let mut context = CompilationContext::default();

        resolve(&mut script, &source_file, &mut context, &mut environment).unwrap();

        assert_eq!(script.items.len(), 1);
        let greet_index = environment.resolve_function("greet").unwrap();
        let ItemKind::Statement(statement) = &script.items[0].kind else {
            panic!("expected statement");
        };
        let StatementKind::Expression(ExpressionStatement { expression }) = &statement.kind;
        let ExpressionKind::Call(call_expression) = &expression.kind else {
            panic!("expected call expression");
        };

        assert_eq!(call_expression.function_index().unwrap(), greet_index);
    }

    #[test]
    fn resolves_calls_inside_function_bodies() {
        let mut script = Script::new(
            vec![Item::new(
                ItemKind::Function(FunctionItem::new(
                    Identifier::new("greet", Span::new(4, 9)),
                    vec![Statement::new(
                        StatementKind::Expression(ExpressionStatement::new(call(
                            identifier("println", Span::new(14, 21)),
                            vec![string_literal("hello", Span::new(22, 29))],
                            Span::new(14, 30),
                        ))),
                        Span::new(14, 31),
                    )],
                    Span::new(0, 33),
                )),
                Span::new(0, 33),
            )],
            Span::new(0, 33),
        );
        let source_file = SourceFile::new(
            "examples/functions.ocelot",
            "fun greet() { println(\"hello\"); }",
        );
        let mut environment = test_program_environment();
        let println_index = environment.resolve_function("println").unwrap();
        let mut context = CompilationContext::default();

        resolve(&mut script, &source_file, &mut context, &mut environment).unwrap();

        assert!(script.items.is_empty());
        let greet_index = environment.resolve_function("greet").unwrap();
        let function_definition = environment.function_definition(greet_index).unwrap();
        let FunctionKind::UserDefined { function } = &function_definition.kind else {
            panic!("expected user-defined function");
        };
        let StatementKind::Expression(ExpressionStatement { expression }) = &function.body[0].kind;
        let ExpressionKind::Call(call_expression) = &expression.kind else {
            panic!("expected call expression");
        };

        assert_eq!(call_expression.function_index().unwrap(), println_index);
    }

    #[test]
    fn reports_duplicate_user_defined_function_names() {
        let mut script = Script::new(
            vec![
                Item::new(
                    ItemKind::Function(FunctionItem::new(
                        Identifier::new("greet", Span::new(4, 9)),
                        Vec::new(),
                        Span::new(0, 13),
                    )),
                    Span::new(0, 13),
                ),
                Item::new(
                    ItemKind::Function(FunctionItem::new(
                        Identifier::new("greet", Span::new(18, 23)),
                        Vec::new(),
                        Span::new(14, 27),
                    )),
                    Span::new(14, 27),
                ),
            ],
            Span::new(0, 27),
        );
        let source_file =
            SourceFile::new("examples/functions.ocelot", "fun greet() {} fun greet() {}");
        let mut environment = test_program_environment();
        let mut context = CompilationContext::default();

        let error = resolve(&mut script, &source_file, &mut context, &mut environment).unwrap_err();

        assert!(
            error
                .to_test_string()
                .contains("duplicate function `greet`")
        );
        assert!(error.to_test_string().contains("already defined here"));
        assert!(
            error
                .to_test_string()
                .contains("fun greet() {} fun greet() {}")
        );
    }

    #[test]
    fn reports_collisions_with_native_function_names() {
        let mut script = Script::new(
            vec![Item::new(
                ItemKind::Function(FunctionItem::new(
                    Identifier::new("println", Span::new(4, 11)),
                    Vec::new(),
                    Span::new(0, 15),
                )),
                Span::new(0, 15),
            )],
            Span::new(0, 15),
        );
        let source_file = SourceFile::new("examples/functions.ocelot", "fun println() {}");
        let mut environment = test_program_environment();
        let mut context = CompilationContext::default();

        let error = resolve(&mut script, &source_file, &mut context, &mut environment).unwrap_err();

        assert!(
            error
                .to_test_string()
                .contains("function `println` conflicts with native function")
        );
    }
}
