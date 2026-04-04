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
use ocelot_ast::qualified_identifier::QualifiedIdentifier;
use ocelot_ast::script::Script;
use ocelot_ast::statement::Statement;
use ocelot_ast::statement_kind::StatementKind;
use ocelot_ast::test_item::TestItem;
use ocelot_ast::type_index::TypeIndex;
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

/// Resolves one script as though it were the only loaded module.
pub fn resolve(
    script: &mut Script,
    source_file: &SourceFile,
    compilation_context: &mut CompilationContext,
    environment: &mut ProgramEnvironment,
) -> OcelotResult<()> {
    let module_name = default_module_name(source_file);
    environment.add_module(module_name.clone());
    register_module_functions(
        script,
        &module_name,
        source_file,
        compilation_context,
        environment,
    )?;
    resolve_module_items(
        script,
        &module_name,
        source_file,
        compilation_context,
        environment,
    )?;
    resolve_user_defined_function_definitions(compilation_context, environment)?;
    finish_resolution(compilation_context)
}

/// Registers all function declarations for one module and lowers them out of the item list.
pub fn register_module_functions(
    script: &mut Script,
    module_name: &str,
    source_file: &SourceFile,
    compilation_context: &mut CompilationContext,
    environment: &mut ProgramEnvironment,
) -> OcelotResult<()> {
    Resolver::new(source_file, module_name, compilation_context, environment)
        .register_function_items(script);
    Ok(())
}

/// Resolves all non-function items in one module after registration.
pub fn resolve_module_items(
    script: &mut Script,
    module_name: &str,
    source_file: &SourceFile,
    compilation_context: &mut CompilationContext,
    environment: &mut ProgramEnvironment,
) -> OcelotResult<()> {
    let mut resolver = Resolver::new(source_file, module_name, compilation_context, environment);
    for item in &mut script.items {
        resolver.resolve_item(item);
    }
    Ok(())
}

/// Resolves the bodies of all registered user-defined functions.
pub fn resolve_user_defined_function_definitions(
    compilation_context: &mut CompilationContext,
    environment: &mut ProgramEnvironment,
) -> OcelotResult<()> {
    let function_indices = environment.user_defined_function_indices();

    for function_index in function_indices {
        let (module_name, source_file) = {
            let function_definition = environment.function_definition(function_index)?;
            let FunctionKind::UserDefined { source_file, .. } = &function_definition.kind else {
                ocelot_base::bail!(
                    "internal error: function index did not reference a user-defined function"
                );
            };
            (
                function_definition.module_name.clone(),
                (**source_file).clone(),
            )
        };

        let mut function = environment.take_user_defined_function(function_index)?;
        Resolver::new(
            &source_file,
            module_name.as_str(),
            compilation_context,
            environment,
        )
        .resolve_function_item(&mut function);
        environment.put_user_defined_function(function_index, function)?;
    }

    Ok(())
}

/// Returns a resolver compilation error if diagnostics were produced.
pub fn finish_resolution(compilation_context: &CompilationContext) -> OcelotResult<()> {
    if compilation_context.has_errors() {
        return Err(
            OcelotError::compilation_error(CompilationStage::Resolver).with_source(
                OcelotError::message(render_source_diagnostics(
                    &compilation_context.source_diagnostics.diagnostics,
                )),
            ),
        );
    }

    Ok(())
}

fn default_module_name(source_file: &SourceFile) -> SharedString {
    source_file.path.file_stem().unwrap_or_default().into()
}

struct Resolver<'a> {
    source_file: &'a SourceFile,
    module_name: &'a str,
    compilation_context: &'a mut CompilationContext,
    environment: &'a mut ProgramEnvironment,
}

impl<'a> Resolver<'a> {
    fn new(
        source_file: &'a SourceFile,
        module_name: &'a str,
        compilation_context: &'a mut CompilationContext,
        environment: &'a mut ProgramEnvironment,
    ) -> Self {
        Self {
            source_file,
            module_name,
            compilation_context,
            environment,
        }
    }

    fn register_function_items(&mut self, script: &mut Script) {
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
        let qualified_name = self
            .environment
            .qualify_function_name(self.module_name, function_item.identifier.name.as_str());

        if let Some(function_index) = self
            .environment
            .resolve_function_exact(qualified_name.as_str())
        {
            let existing_function = self
                .environment
                .function_definition(function_index)
                .expect("resolved function index should point at a definition");
            let duplicate_with_original = match &existing_function.kind {
                FunctionKind::Native { .. } => None,
                FunctionKind::UserDefined {
                    function,
                    source_file,
                } => Some(((**function).clone(), (**source_file).clone())),
            };

            if let Some((original_function, original_source_file)) = duplicate_with_original {
                self.add_duplicate_function_diagnostic(
                    &function_item,
                    &original_function,
                    &original_source_file,
                );
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
            .add_function(FunctionDefinition::user_defined(
                self.module_name,
                qualified_name,
                function_item,
                self.source_file.clone(),
            ));
    }

    fn resolve_function_item(&mut self, function_item: &mut FunctionItem) {
        for statement in &mut function_item.body {
            self.resolve_statement(statement);
        }
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
            | ExpressionKind::QualifiedIdentifier(_)
            | ExpressionKind::StringLiteral(_) => {}
            ExpressionKind::Not(not_expression) => {
                self.resolve_expression(&mut not_expression.operand);
            }
            ExpressionKind::Call(call_expression) => self.resolve_call_expression(call_expression),
        }

        self.annotate_expression_type(expression);
    }

    fn resolve_call_expression(&mut self, call_expression: &mut CallExpression) {
        self.resolve_expression(&mut call_expression.callee);
        for argument in &mut call_expression.arguments {
            self.resolve_expression(argument);
        }

        let resolved = match &call_expression.callee.kind {
            ExpressionKind::Identifier(identifier) => {
                let Some(function_index) = self
                    .environment
                    .resolve_local_function(self.module_name, identifier.name.as_str())
                else {
                    self.add_diagnostic(
                        format!("unknown function `{}`", identifier.name),
                        identifier.span.clone(),
                        "unknown function",
                    );
                    return;
                };

                Some((function_index, identifier.name.clone()))
            }
            ExpressionKind::QualifiedIdentifier(identifier) => {
                self.resolve_qualified_call(identifier)
            }
            _ => {
                self.add_diagnostic(
                    "only identifier calls are supported",
                    call_expression.callee.span.clone(),
                    "callee must be an identifier",
                );
                None
            }
        };

        let Some((function_index, function_name)) = resolved else {
            return;
        };

        call_expression.resolve_to(function_index);
        self.validate_call_argument_types(function_name.as_str(), call_expression, function_index);
    }

    fn resolve_qualified_call(
        &mut self,
        identifier: &QualifiedIdentifier,
    ) -> Option<(ocelot_ast::function_index::FunctionIndex, SharedString)> {
        let qualified_name = identifier.render();
        let module_name = identifier
            .module_segments()
            .iter()
            .map(|segment| segment.name.as_str())
            .collect::<Vec<_>>()
            .join("::");

        if !self.environment.has_module(&module_name) {
            self.add_diagnostic(
                format!("unknown module `{module_name}`"),
                identifier.span(),
                "unknown module",
            );
            return None;
        }

        let Some(function_index) = self
            .environment
            .resolve_function_exact(qualified_name.as_str())
        else {
            let function_name = identifier
                .last()
                .map(|segment| segment.name.clone())
                .unwrap_or_else(SharedString::empty);
            self.add_diagnostic(
                format!("module `{module_name}` has no function `{function_name}`"),
                identifier.span(),
                "unknown function",
            );
            return None;
        };

        Some((function_index, qualified_name))
    }

    fn annotate_expression_type(&mut self, expression: &mut Expression) {
        let boolean_type_index = self.environment.boolean_type_index();
        let string_type_index = self.environment.string_type_index();

        match &expression.kind {
            ExpressionKind::BooleanLiteral(_) => {
                expression.ty = boolean_type_index;
            }
            ExpressionKind::StringLiteral(_) => {
                expression.ty = string_type_index;
            }
            ExpressionKind::Identifier(_)
            | ExpressionKind::QualifiedIdentifier(_)
            | ExpressionKind::Call(_) => {
                expression.ty = TypeIndex::unresolved();
            }
            ExpressionKind::Not(not_expression) => {
                if not_expression.operand.ty == boolean_type_index {
                    expression.ty = boolean_type_index;
                    return;
                }

                if !not_expression.operand.ty.is_unresolved() {
                    self.add_diagnostic(
                        "operator `not` expects a boolean operand",
                        not_expression.operand.span.clone(),
                        "boolean operand required",
                    );
                }

                expression.ty = TypeIndex::unresolved();
            }
        }
    }

    fn validate_call_argument_types(
        &mut self,
        function_name: &str,
        call_expression: &CallExpression,
        function_index: ocelot_ast::function_index::FunctionIndex,
    ) {
        let Ok(function_definition) = self.environment.function_definition(function_index) else {
            return;
        };
        let argument_types = function_definition.argument_types.clone();
        let any_type_index = self.environment.any_type_index();

        for (argument_index, (argument, expected_type)) in call_expression
            .arguments
            .iter()
            .zip(argument_types.iter())
            .enumerate()
        {
            if argument.ty.is_unresolved() || *expected_type == any_type_index {
                continue;
            }

            if argument.ty == *expected_type {
                continue;
            }

            self.add_diagnostic(
                self.argument_type_error_message(function_name, argument_index, *expected_type),
                argument.span.clone(),
                format!("expected {}", self.type_label(*expected_type)),
            );
        }
    }

    fn argument_type_error_message(
        &self,
        function_name: &str,
        argument_index: usize,
        expected_type: TypeIndex,
    ) -> SharedString {
        if argument_index == 0
            && function_name == "assert"
            && expected_type == self.environment.boolean_type_index()
        {
            return "type error: `assert` expects a boolean argument".into();
        }

        format!(
            "type error: argument {} to `{}` must be {}",
            argument_index + 1,
            function_name,
            self.type_label(expected_type)
        )
        .into()
    }

    fn type_label(&self, type_index: TypeIndex) -> SharedString {
        self.environment
            .type_definition(type_index)
            .map(|ty| ty.name.clone())
            .unwrap_or_else(|_| "unknown".into())
    }

    fn add_diagnostic(
        &mut self,
        message: impl Into<SharedString>,
        span: Span,
        annotation: impl Into<SharedString>,
    ) {
        self.compilation_context
            .add_diagnostic(self.source_diagnostic(self.source_file, message, span, annotation));
    }

    fn add_duplicate_function_diagnostic(
        &mut self,
        duplicate_function: &FunctionItem,
        original_function: &FunctionItem,
        original_source_file: &SourceFile,
    ) {
        let diagnostic = self
            .source_diagnostic(
                self.source_file,
                format!(
                    "duplicate function `{}`",
                    duplicate_function.identifier.name
                ),
                duplicate_function.identifier.span.clone(),
                "duplicate function",
            )
            .with_excerpt(self.source_excerpt(
                original_source_file,
                original_function.identifier.span.clone(),
                "already defined here",
            ));
        self.compilation_context.add_diagnostic(diagnostic);
    }

    fn source_diagnostic(
        &self,
        source_file: &SourceFile,
        message: impl Into<SharedString>,
        span: Span,
        annotation: impl Into<SharedString>,
    ) -> SourceDiagnostic {
        let message = message.into();
        SourceDiagnostic::new(DiagnosticLevel::Error, &source_file.path, message)
            .with_excerpt(self.source_excerpt(source_file, span, annotation))
    }

    fn source_excerpt(
        &self,
        source_file: &SourceFile,
        span: Span,
        annotation: impl Into<SharedString>,
    ) -> SourceExcerpt {
        let annotation = annotation.into();
        let (line_number, line_start, line_end) = line_bounds(source_file.source(), span.start());
        let source_line = &source_file.source()[line_start..line_end];
        let relative_start = span.start().saturating_sub(line_start);
        let relative_end = span.end().saturating_sub(line_start);

        SourceExcerpt::new(&source_file.path, line_number, source_line).with_annotation(
            SourceAnnotation::new(Span::new(relative_start, relative_end), annotation),
        )
    }
}

fn line_bounds(source: &str, index: usize) -> (usize, usize, usize) {
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

#[cfg(test)]
mod tests {
    use super::finish_resolution;
    use super::register_module_functions;
    use super::resolve;
    use super::resolve_module_items;
    use super::resolve_user_defined_function_definitions;
    use ocelot_ast::call_expression::CallExpression;
    use ocelot_ast::expression::Expression;
    use ocelot_ast::expression_kind::ExpressionKind;
    use ocelot_ast::expression_statement::ExpressionStatement;
    use ocelot_ast::function_item::FunctionItem;
    use ocelot_ast::function_kind::FunctionKind;
    use ocelot_ast::identifier::Identifier;
    use ocelot_ast::item::Item;
    use ocelot_ast::item_kind::ItemKind;
    use ocelot_ast::program_environment::ProgramEnvironment;
    use ocelot_ast::qualified_identifier::QualifiedIdentifier;
    use ocelot_ast::script::Script;
    use ocelot_ast::statement::Statement;
    use ocelot_ast::statement_kind::StatementKind;
    use ocelot_ast::string_literal_expression::StringLiteralExpression;
    use ocelot_ast::test_item::TestItem;
    use ocelot_ast::type_index::TypeIndex;
    use ocelot_base::compilation_context::CompilationContext;
    use ocelot_base::compilation_stage::CompilationStage;
    use ocelot_base::source_file::SourceFile;
    use ocelot_base::span::Span;

    fn identifier(name: &str, span: Span) -> Expression {
        Expression::new(
            ExpressionKind::Identifier(Identifier::new(name, span.clone())),
            span,
        )
    }

    fn qualified_identifier(names: &[&str], spans: &[Span]) -> Expression {
        Expression::new(
            ExpressionKind::QualifiedIdentifier(QualifiedIdentifier::new(
                names
                    .iter()
                    .zip(spans.iter())
                    .map(|(name, span)| Identifier::new(*name, span.clone()))
                    .collect(),
            )),
            Span::new(spans[0].start(), spans[spans.len() - 1].end()),
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
        let mut environment = ProgramEnvironment::new();
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
        assert_eq!(expression.ty, TypeIndex::unresolved());
    }

    #[test]
    fn resolves_module_qualified_calls() {
        let mut main_script = Script::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call(
                        qualified_identifier(
                            &["math", "greet", "hello"],
                            &[Span::new(0, 4), Span::new(6, 11), Span::new(13, 18)],
                        ),
                        Vec::new(),
                        Span::new(0, 20),
                    ))),
                    Span::new(0, 21),
                )),
                Span::new(0, 21),
            )],
            Span::new(0, 21),
        );
        let mut module_script = Script::new(
            vec![Item::new(
                ItemKind::Function(FunctionItem::new(
                    Identifier::new("hello", Span::new(4, 9)),
                    Vec::new(),
                    Span::new(0, 14),
                )),
                Span::new(0, 14),
            )],
            Span::new(0, 14),
        );
        let main_source_file = SourceFile::new("main.ocelot", "math::greet::hello();");
        let module_source_file = SourceFile::new("math/greet.ocelot", "fun hello() {}");
        let mut environment = ProgramEnvironment::new();
        environment.add_module("main");
        environment.add_module("math::greet");
        let mut context = CompilationContext::default();

        register_module_functions(
            &mut module_script,
            "math::greet",
            &module_source_file,
            &mut context,
            &mut environment,
        )
        .unwrap();
        resolve_module_items(
            &mut main_script,
            "main",
            &main_source_file,
            &mut context,
            &mut environment,
        )
        .unwrap();
        resolve_user_defined_function_definitions(&mut context, &mut environment).unwrap();
        finish_resolution(&context).unwrap();

        let ItemKind::Statement(statement) = &main_script.items[0].kind else {
            panic!("expected statement");
        };
        let StatementKind::Expression(ExpressionStatement { expression }) = &statement.kind;
        let ExpressionKind::Call(call_expression) = &expression.kind else {
            panic!("expected call expression");
        };
        assert_eq!(
            call_expression.function_index().unwrap(),
            environment.resolve_function("math::greet::hello").unwrap()
        );
    }

    #[test]
    fn reports_unknown_modules() {
        let mut script = Script::new(
            vec![Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call(
                        qualified_identifier(
                            &["math", "greet", "hello"],
                            &[Span::new(0, 4), Span::new(6, 11), Span::new(13, 18)],
                        ),
                        Vec::new(),
                        Span::new(0, 20),
                    ))),
                    Span::new(0, 21),
                )),
                Span::new(0, 21),
            )],
            Span::new(0, 21),
        );
        let source_file = SourceFile::new("main.ocelot", "math::greet::hello();");
        let mut environment = ProgramEnvironment::new();
        environment.add_module("main");
        let mut context = CompilationContext::default();

        resolve_module_items(
            &mut script,
            "main",
            &source_file,
            &mut context,
            &mut environment,
        )
        .unwrap();
        let error = finish_resolution(&context).unwrap_err();

        assert!(matches!(
            error.kind(),
            ocelot_base::error::ErrorKind::CompilationError(CompilationStage::Resolver)
        ));
        assert!(
            error
                .to_test_string()
                .contains("unknown module `math::greet`")
        );
    }

    #[test]
    fn lowers_function_items_before_resolving_tests() {
        let mut script = Script::new(
            vec![
                Item::new(
                    ItemKind::Function(FunctionItem::new(
                        Identifier::new("helper", Span::new(4, 10)),
                        Vec::new(),
                        Span::new(0, 15),
                    )),
                    Span::new(0, 15),
                ),
                Item::new(
                    ItemKind::Test(TestItem::new("works", Vec::new(), Span::new(16, 30))),
                    Span::new(16, 30),
                ),
            ],
            Span::new(0, 30),
        );
        let source_file = SourceFile::new("main.ocelot", "fun helper() {} test \"works\" {}");
        let mut environment = ProgramEnvironment::new();
        let mut context = CompilationContext::default();

        register_module_functions(
            &mut script,
            "main",
            &source_file,
            &mut context,
            &mut environment,
        )
        .unwrap();

        assert_eq!(script.items.len(), 1);
        assert!(matches!(script.items[0].kind, ItemKind::Test(_)));
        assert!(matches!(
            environment
                .function_definition(environment.resolve_function("main::helper").unwrap())
                .unwrap()
                .kind,
            FunctionKind::UserDefined { .. }
        ));
    }
}
