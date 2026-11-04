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
use ocelot_ast::template_string_expression::TemplateStringExpression;
use ocelot_ast::template_string_part::TemplateStringPart;
use ocelot_ast::test_item::TestItem;
use ocelot_ast::type_index::TypeIndex;
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_file::SourceFile;
use ocelot_base::span::Span;
use ocelot_semantic::compilation_context::CompilationContext;
use ocelot_semantic::function_kind::FunctionKind;
use ocelot_semantic::module_imports::ModuleImports;
use ocelot_semantic::resolved_function::ResolvedFunction;
use ocelot_semantic::symbol_table::SymbolTable;
use std::collections::HashMap;

pub(crate) struct Resolver<'a> {
    source_file: &'a SourceFile,
    module_name: &'a str,
    compilation_context: &'a mut CompilationContext,
    symbol_table: &'a SymbolTable,
    module_imports: &'a ModuleImports,
    resolved_function: Option<&'a mut ResolvedFunction>,
    local_value_types: HashMap<SharedString, TypeIndex>,
}

impl<'a> Resolver<'a> {
    pub(crate) fn new(
        source_file: &'a SourceFile,
        module_name: &'a str,
        compilation_context: &'a mut CompilationContext,
        symbol_table: &'a SymbolTable,
        module_imports: &'a ModuleImports,
        resolved_function: Option<&'a mut ResolvedFunction>,
    ) -> Self {
        Self {
            source_file,
            module_name,
            compilation_context,
            symbol_table,
            module_imports,
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

        let actual_type = self.resolve_block(&mut function_item.body);
        if !function_item.return_type.is_unresolved()
            && !actual_type.is_unresolved()
            && actual_type != self.symbol_table.never_type_index()
            && actual_type != function_item.return_type
        {
            self.add_diagnostic(
                format!(
                    "block result type mismatch: expected {}, got {}",
                    self.type_label(function_item.return_type),
                    self.type_label(actual_type)
                ),
                function_item
                    .body
                    .last()
                    .map(|s| s.span.clone())
                    .unwrap_or_else(|| function_item.identifier.span.clone()),
                "add a matching result expression or return statement",
            );
        }

        self.local_value_types.clear();
    }

    fn resolve_test_item(&mut self, test_item: &mut TestItem) {
        let ty = self.resolve_block(&mut test_item.body);
        if !ty.is_unresolved()
            && ty != self.symbol_table.unit_type_index()
            && ty != self.symbol_table.never_type_index()
        {
            self.add_diagnostic(
                "test bodies must produce unit",
                test_item.span.clone(),
                "discard the result with `;`",
            );
        }
    }

    fn resolve_block(&mut self, statements: &mut [Statement]) -> TypeIndex {
        let unit = self.symbol_table.unit_type_index();
        let never = self.symbol_table.never_type_index();
        let mut result = unit;
        let mut returns = false;
        for statement in statements {
            self.resolve_statement(statement);
            match &statement.kind {
                StatementKind::Return(_) => returns = true,
                StatementKind::Expression(value) => {
                    returns |= value.expression.ty == never;
                    result = if value.has_semicolon {
                        unit
                    } else {
                        value.expression.ty
                    };
                }
            }
        }
        if returns { never } else { result }
    }

    fn resolve_statement(&mut self, statement: &mut Statement) {
        match &mut statement.kind {
            StatementKind::Return(return_statement) => {
                let actual_type = if let Some(expression) = &mut return_statement.expression {
                    self.resolve_expression(expression);
                    expression.ty
                } else {
                    self.symbol_table.unit_type_index()
                };
                let Some(function) = &self.resolved_function else {
                    self.add_diagnostic(
                        "return is only allowed in function bodies",
                        statement.span.clone(),
                        "not inside a function",
                    );
                    return;
                };
                /* 📖 # Why read the result type from the declaration table?
                Body resolution temporarily moves the function AST out of `ResolvedFunction`,
                leaving a placeholder there. The symbol table retains the complete signature
                throughout this pass, including for forward and recursive calls.
                */
                let expected_type = self
                    .symbol_table
                    .function_definition(function.function_index)
                    .expect("resolved function must have a declaration")
                    .return_type;
                if !expected_type.is_unresolved()
                    && !actual_type.is_unresolved()
                    && actual_type != self.symbol_table.never_type_index()
                    && expected_type != actual_type
                {
                    self.add_diagnostic(
                        format!(
                            "return type mismatch: expected {}, got {}",
                            self.type_label(expected_type),
                            self.type_label(actual_type)
                        ),
                        return_statement
                            .expression
                            .as_ref()
                            .map(|value| value.span.clone())
                            .unwrap_or_else(|| statement.span.clone()),
                        format!("expected {}", self.type_label(expected_type)),
                    );
                }
            }
            StatementKind::Expression(ExpressionStatement { expression, .. }) => {
                self.resolve_expression(expression);
            }
        }
    }

    fn resolve_expression(&mut self, expression: &mut Expression) {
        match &mut expression.kind {
            ExpressionKind::Block(block) => {
                expression.ty = self.resolve_block(&mut block.statements);
            }
            ExpressionKind::BooleanLiteral(_) | ExpressionKind::StringLiteral(_) => {}
            ExpressionKind::Identifier(identifier) => {
                if !self
                    .local_value_types
                    .contains_key(identifier.name.as_str())
                {
                    self.add_diagnostic(
                        format!("unresolved identifier `{}`", identifier.name),
                        identifier.span.clone(),
                        "not found",
                    );
                }
            }
            ExpressionKind::QualifiedIdentifier(identifier) => {
                self.add_diagnostic(
                    format!("unresolved identifier `{}`", identifier.render()),
                    identifier.span(),
                    "not found",
                );
            }
            ExpressionKind::TemplateString(template_string) => {
                self.resolve_template_string_expression(template_string);
            }
            ExpressionKind::Not(not_expression) => {
                self.resolve_expression(&mut not_expression.operand);
            }
            ExpressionKind::Call(call_expression) => self.resolve_call_expression(call_expression),
        }

        self.annotate_expression_type(expression);
    }

    fn resolve_call_expression(&mut self, call_expression: &mut CallExpression) {
        if !matches!(
            call_expression.callee.kind,
            ExpressionKind::Identifier(_) | ExpressionKind::QualifiedIdentifier(_)
        ) {
            self.resolve_expression(&mut call_expression.callee);
        }
        for argument in &mut call_expression.arguments {
            self.resolve_expression(&mut argument.value);
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

        if !self.validate_call_labels(function_name.as_str(), call_expression, function_index) {
            return;
        }

        call_expression.resolve_to(function_index);
        self.record_effect_dependency(function_index, call_expression.callee.span.clone());
        self.validate_call_argument_types(function_name.as_str(), call_expression, function_index);
    }

    fn resolve_template_string_expression(
        &mut self,
        template_string: &mut TemplateStringExpression,
    ) {
        for part in &mut template_string.parts {
            if let TemplateStringPart::Interpolation(expression) = part {
                self.resolve_expression(expression);
            }
        }
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

        if let Some(function_index) = self.module_imports.resolve_imported_function(name) {
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

    fn validate_call_labels(
        &mut self,
        function_name: &str,
        call: &CallExpression,
        function_index: FunctionIndex,
    ) -> bool {
        let Ok(definition) = self.symbol_table.function_definition(function_index) else {
            return false;
        };
        let mut seen_labels = std::collections::HashSet::new();
        let mut saw_label = false;
        let mut valid = true;
        for (argument, parameter) in call.arguments.iter().zip(&definition.parameters) {
            if let Some(label) = &argument.label {
                saw_label = true;
                if !seen_labels.insert(&label.name) {
                    self.add_diagnostic(
                        format!("duplicate argument label `{}`", label.name),
                        label.span.clone(),
                        "argument already supplied",
                    );
                    valid = false;
                } else if label.name != parameter.identifier.name {
                    let known = definition
                        .parameters
                        .iter()
                        .any(|p| p.identifier.name == label.name);
                    self.add_diagnostic(
                        if known {
                            format!("arguments to `{function_name}` must follow declaration order")
                        } else {
                            format!(
                                "unknown argument label `{}` for `{function_name}`",
                                label.name
                            )
                        },
                        label.span.clone(),
                        format!("expected `{}`", parameter.identifier.name),
                    );
                    valid = false;
                }
            } else if !parameter.positional {
                self.add_diagnostic(
                    format!(
                        "argument `{}` to `{function_name}` requires a label",
                        parameter.identifier.name
                    ),
                    argument.value.span.clone(),
                    format!("write `{}: ...`", parameter.identifier.name),
                );
                valid = false;
            } else if saw_label {
                self.add_diagnostic(
                    "unlabeled arguments must precede labeled arguments",
                    argument.value.span.clone(),
                    format!("write `{}: ...`", parameter.identifier.name),
                );
                valid = false;
            }
        }
        valid
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

        for (argument_index, (argument, parameter)) in call_expression
            .arguments
            .iter()
            .zip(function_definition.parameters.iter())
            .enumerate()
        {
            let expected_type = parameter.ty;
            if argument.value.ty.is_unresolved()
                || argument.value.ty == self.symbol_table.never_type_index()
                || expected_type == any_type_index
            {
                continue;
            }

            if argument.value.ty == expected_type {
                continue;
            }

            self.add_diagnostic(
                self.argument_type_error_message(function_name, argument_index, expected_type),
                argument.value.span.clone(),
                format!("expected {}", self.type_label(expected_type)),
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
        let expected = function_definition.parameters.len();
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
            ExpressionKind::Block(_) => {}
            ExpressionKind::BooleanLiteral(_) => expression.ty = boolean_type_index,
            ExpressionKind::Identifier(identifier) => {
                expression.ty = self
                    .local_value_types
                    .get(identifier.name.as_str())
                    .copied()
                    .unwrap_or_else(TypeIndex::unresolved);
            }
            ExpressionKind::StringLiteral(_) => expression.ty = string_type_index,
            ExpressionKind::TemplateString(template) => {
                expression.ty = if template.parts.iter().any(|part| matches!(part, TemplateStringPart::Interpolation(value) if value.ty == self.symbol_table.never_type_index())) {
                    self.symbol_table.never_type_index()
                } else { string_type_index };
            }
            ExpressionKind::Call(call) => {
                if call
                    .arguments
                    .iter()
                    .any(|a| a.value.ty == self.symbol_table.never_type_index())
                {
                    expression.ty = self.symbol_table.never_type_index();
                    return;
                }
                expression.ty = call
                    .function_index()
                    .ok()
                    .and_then(|index| self.symbol_table.function_definition(index).ok())
                    .map(|function| function.return_type)
                    .unwrap_or_else(TypeIndex::unresolved);
            }
            ExpressionKind::QualifiedIdentifier(_) => {
                expression.ty = TypeIndex::unresolved();
            }
            ExpressionKind::Not(not_expression) => {
                if not_expression.operand.ty == self.symbol_table.never_type_index() {
                    expression.ty = self.symbol_table.never_type_index();
                    return;
                }
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
                .value
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
