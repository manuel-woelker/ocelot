use crate::diagnostics::source_diagnostic_for_span;
use ocelot_ast::call_expression::CallExpression;
use ocelot_ast::expression::Expression;
use ocelot_ast::expression_kind::ExpressionKind;
use ocelot_ast::expression_statement::ExpressionStatement;
use ocelot_ast::function_index::FunctionIndex;
use ocelot_ast::function_item::FunctionItem;
use ocelot_ast::item::Item;
use ocelot_ast::item_kind::ItemKind;
use ocelot_ast::qualified_identifier::QualifiedIdentifier;
use ocelot_ast::statement::Statement;
use ocelot_ast::statement_kind::StatementKind;
use ocelot_ast::test_item::TestItem;
use ocelot_ast::type_index::TypeIndex;
use ocelot_base::compilation_context::CompilationContext;
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_file::SourceFile;
use ocelot_base::span::Span;
use ocelot_semantic::function_kind::FunctionKind;
use ocelot_semantic::module_environment::ModuleEnvironment;
use ocelot_semantic::resolved_function::ResolvedFunction;
use ocelot_semantic::symbol_table::SymbolTable;
use std::collections::HashMap;

pub(crate) struct Resolver<'a> {
    source_file: &'a SourceFile,
    module_name: &'a str,
    compilation_context: &'a mut CompilationContext,
    symbol_table: &'a SymbolTable,
    module_environment: &'a ModuleEnvironment,
    resolved_function: Option<&'a mut ResolvedFunction>,
    local_value_types: HashMap<SharedString, TypeIndex>,
}

impl<'a> Resolver<'a> {
    pub(crate) fn new(
        source_file: &'a SourceFile,
        module_name: &'a str,
        compilation_context: &'a mut CompilationContext,
        symbol_table: &'a SymbolTable,
        module_environment: &'a ModuleEnvironment,
        resolved_function: Option<&'a mut ResolvedFunction>,
    ) -> Self {
        Self {
            source_file,
            module_name,
            compilation_context,
            symbol_table,
            module_environment,
            resolved_function,
            local_value_types: HashMap::new(),
        }
    }

    pub(crate) fn resolve_item(&mut self, item: &mut Item) {
        match &mut item.kind {
            ItemKind::Effect(_) => {
                unreachable!("effect items should be lowered before item resolution")
            }
            ItemKind::Statement(statement) => self.resolve_statement(statement),
            ItemKind::Test(test_item) => self.resolve_test_item(test_item),
            ItemKind::Function(_) => {
                unreachable!("function items should be lowered before item resolution")
            }
            ItemKind::Use(_) => unreachable!("use items should be lowered before item resolution"),
        }
    }

    pub(crate) fn resolve_function_item(&mut self, function_item: &mut FunctionItem) {
        self.local_value_types.clear();

        for parameter in &function_item.parameters {
            self.local_value_types
                .insert(parameter.identifier.name.clone(), parameter.ty);
        }

        for statement in &mut function_item.body {
            self.resolve_statement(statement);
        }

        self.local_value_types.clear();
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
                if self
                    .local_value_types
                    .contains_key(identifier.name.as_str())
                {
                    self.add_diagnostic(
                        format!("`{}` is a value, not a function", identifier.name),
                        identifier.span.clone(),
                        "callable function required",
                    );
                    return;
                }

                let Some(function_index) = self.resolve_local_function(identifier.name.as_str())
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

        if !self.validate_call_arity(function_name.as_str(), call_expression, function_index) {
            return;
        }

        call_expression.resolve_to(function_index);
        self.record_effect_dependency(function_index, call_expression.callee.span.clone());
        self.validate_call_argument_types(function_name.as_str(), call_expression, function_index);
    }

    fn resolve_qualified_call(
        &mut self,
        identifier: &QualifiedIdentifier,
    ) -> Option<(FunctionIndex, SharedString)> {
        let qualified_name = identifier.render();
        let module_name = identifier
            .module_segments()
            .iter()
            .map(|segment| segment.name.as_str())
            .collect::<Vec<_>>()
            .join("::");

        if !self.symbol_table.has_module(&module_name) {
            self.add_diagnostic(
                format!("unknown module `{module_name}`"),
                identifier.span(),
                "unknown module",
            );
            return None;
        }

        let Some(function_index) = self
            .symbol_table
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

    fn resolve_local_function(&self, name: &str) -> Option<FunctionIndex> {
        if !self.module_name.is_empty() {
            let qualified_name = self
                .symbol_table
                .qualify_function_name(self.module_name, name);
            if let Some(function_index) = self.symbol_table.resolve_function_exact(&qualified_name)
            {
                return Some(function_index);
            }
        }

        if let Some(function_index) = self.module_environment.resolve_imported_function(name) {
            return Some(function_index);
        }

        let core_function_name = self.symbol_table.qualify_function_name("core", name);
        self.symbol_table
            .resolve_function_exact(&core_function_name)
    }

    fn record_effect_dependency(&mut self, called_function_index: FunctionIndex, span: Span) {
        let Some(current_function) = self.resolved_function.as_mut() else {
            return;
        };

        let Ok(called_function) = self.symbol_table.function_definition(called_function_index)
        else {
            return;
        };

        match &called_function.kind {
            FunctionKind::NativeFunction { .. } => {
                for effect_index in &called_function.inferred_effects {
                    current_function.direct_effects.insert(*effect_index);
                    current_function
                        .direct_effect_sources
                        .entry(*effect_index)
                        .or_insert(span.clone());
                }
            }
            FunctionKind::UserDefined { .. } => {
                current_function
                    .called_functions
                    .entry(called_function_index)
                    .or_insert(span);
            }
        }
    }

    fn validate_call_argument_types(
        &mut self,
        function_name: &str,
        call_expression: &CallExpression,
        function_index: FunctionIndex,
    ) {
        let Ok(function_definition) = self.symbol_table.function_definition(function_index) else {
            return;
        };
        let any_type_index = self.symbol_table.any_type_index();

        for (argument_index, (argument, expected_type)) in call_expression
            .arguments
            .iter()
            .zip(function_definition.argument_types.iter())
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

    fn validate_call_arity(
        &mut self,
        function_name: &str,
        call_expression: &CallExpression,
        function_index: FunctionIndex,
    ) -> bool {
        let Ok(function_definition) = self.symbol_table.function_definition(function_index) else {
            return true;
        };
        let expected = function_definition.argument_types.len();
        let actual = call_expression.arguments.len();

        if expected == actual {
            return true;
        }

        self.add_diagnostic(
            self.argument_arity_error_message(function_name, expected),
            self.call_arity_span(call_expression, actual, expected),
            if actual < expected {
                "missing argument"
            } else {
                "extra argument"
            },
        );
        false
    }

    fn annotate_expression_type(&mut self, expression: &mut Expression) {
        let boolean_type_index = self.symbol_table.boolean_type_index();
        let string_type_index = self.symbol_table.string_type_index();

        match &expression.kind {
            ExpressionKind::BooleanLiteral(_) => expression.ty = boolean_type_index,
            ExpressionKind::Identifier(identifier) => {
                expression.ty = self
                    .local_value_types
                    .get(identifier.name.as_str())
                    .copied()
                    .unwrap_or_else(TypeIndex::unresolved);
            }
            ExpressionKind::StringLiteral(_) => expression.ty = string_type_index,
            ExpressionKind::QualifiedIdentifier(_) | ExpressionKind::Call(_) => {
                expression.ty = TypeIndex::unresolved();
            }
            ExpressionKind::Not(not_expression) => {
                if not_expression.operand.ty == boolean_type_index {
                    expression.ty = boolean_type_index;
                    return;
                }

                if !not_expression.operand.ty.is_unresolved() {
                    self.add_diagnostic(
                        "operator `not` expects a bool operand",
                        not_expression.operand.span.clone(),
                        "bool operand required",
                    );
                }

                expression.ty = TypeIndex::unresolved();
            }
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
            && expected_type == self.symbol_table.boolean_type_index()
        {
            return "type error: `assert` expects a bool argument".into();
        }

        format!(
            "type error: argument {} to `{}` must be {}",
            argument_index + 1,
            function_name,
            self.type_label(expected_type)
        )
        .into()
    }

    fn argument_arity_error_message(
        &self,
        function_name: &str,
        expected_argument_count: usize,
    ) -> SharedString {
        match expected_argument_count {
            0 => format!("type error: `{function_name}` expects no arguments").into(),
            1 => format!("type error: `{function_name}` expects exactly one argument").into(),
            2 => format!("type error: `{function_name}` expects exactly two arguments").into(),
            _ => format!(
                "type error: `{function_name}` expects exactly {expected_argument_count} arguments"
            )
            .into(),
        }
    }

    fn call_arity_span(
        &self,
        call_expression: &CallExpression,
        actual_argument_count: usize,
        expected_argument_count: usize,
    ) -> Span {
        if actual_argument_count > expected_argument_count {
            return call_expression.arguments[expected_argument_count]
                .span
                .clone();
        }

        call_expression.callee.span.clone()
    }

    fn type_label(&self, type_index: TypeIndex) -> SharedString {
        self.symbol_table
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
        let diagnostic = source_diagnostic_for_span(self.source_file, message, span, annotation);
        self.compilation_context.add_diagnostic(diagnostic);
    }
}
