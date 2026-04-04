//! Name resolution for `ocelot`.

use ocelot_ast::call_expression::CallExpression;
use ocelot_ast::effect::Effect;
use ocelot_ast::effect_index::EffectIndex;
use ocelot_ast::effect_item::EffectItem;
use ocelot_ast::expression::Expression;
use ocelot_ast::expression_kind::ExpressionKind;
use ocelot_ast::expression_statement::ExpressionStatement;
use ocelot_ast::function_definition::FunctionDefinition;
use ocelot_ast::function_effect_clause::FunctionEffectClause;
use ocelot_ast::function_index::FunctionIndex;
use ocelot_ast::function_item::FunctionItem;
use ocelot_ast::function_kind::FunctionKind;
use ocelot_ast::function_parameter::FunctionParameter;
use ocelot_ast::identifier::Identifier;
use ocelot_ast::item::Item;
use ocelot_ast::item_kind::ItemKind;
use ocelot_ast::native_function::native_type_label;
use ocelot_ast::program_environment::ProgramEnvironment;
use ocelot_ast::qualified_identifier::QualifiedIdentifier;
use ocelot_ast::script::Script;
use ocelot_ast::statement::Statement;
use ocelot_ast::statement_kind::StatementKind;
use ocelot_ast::test_item::TestItem;
use ocelot_ast::type_index::TypeIndex;
use ocelot_ast::use_item::UseItem;
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
use std::collections::BTreeSet;
use std::collections::HashMap;

const CORE_MODULE_NAME: &str = "core";
const CORE_MODULE_PATH: &str = "crates/engine/resources/core.ocelot";
const CORE_MODULE_SOURCE: &str = include_str!("../../engine/resources/core.ocelot");

/// Resolves one script as though it were the only loaded module.
pub fn resolve(
    script: &mut Script,
    source_file: &SourceFile,
    compilation_context: &mut CompilationContext,
    environment: &mut ProgramEnvironment,
) -> OcelotResult<()> {
    register_core_module(compilation_context, environment)?;
    let module_name = default_module_name(source_file);
    environment.add_module(module_name.clone());
    register_module_effects(script, source_file, compilation_context, environment)?;
    register_module_functions(
        script,
        &module_name,
        source_file,
        compilation_context,
        environment,
    )?;
    register_module_imports(
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

/// Registers the compiler-provided `core` module into one program environment.
pub fn register_core_module(
    compilation_context: &mut CompilationContext,
    environment: &mut ProgramEnvironment,
) -> OcelotResult<()> {
    if environment.has_module(CORE_MODULE_NAME) {
        return Ok(());
    }

    let source_file = SourceFile::new(CORE_MODULE_PATH, CORE_MODULE_SOURCE);
    let mut script = ocelot_parser::parse_script::parse_script(&source_file, compilation_context)?;

    environment.add_module(CORE_MODULE_NAME);
    register_module_effects(&mut script, &source_file, compilation_context, environment)?;
    register_module_functions(
        &mut script,
        CORE_MODULE_NAME,
        &source_file,
        compilation_context,
        environment,
    )?;
    Ok(())
}

/// Registers all effect declarations for one module and lowers them out of the item list.
pub fn register_module_effects(
    script: &mut Script,
    source_file: &SourceFile,
    compilation_context: &mut CompilationContext,
    environment: &mut ProgramEnvironment,
) -> OcelotResult<()> {
    Resolver::new(source_file, "", compilation_context, environment, None)
        .register_effect_items(script);
    Ok(())
}

/// Registers all function declarations for one module and lowers them out of the item list.
pub fn register_module_functions(
    script: &mut Script,
    module_name: &str,
    source_file: &SourceFile,
    compilation_context: &mut CompilationContext,
    environment: &mut ProgramEnvironment,
) -> OcelotResult<()> {
    Resolver::new(
        source_file,
        module_name,
        compilation_context,
        environment,
        None,
    )
    .register_function_items(script)?;
    Ok(())
}

/// Registers all import declarations for one module and lowers them out of the item list.
pub fn register_module_imports(
    script: &mut Script,
    module_name: &str,
    source_file: &SourceFile,
    compilation_context: &mut CompilationContext,
    environment: &mut ProgramEnvironment,
) -> OcelotResult<()> {
    Resolver::new(
        source_file,
        module_name,
        compilation_context,
        environment,
        None,
    )
    .register_use_items(script);
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
    let mut resolver = Resolver::new(
        source_file,
        module_name,
        compilation_context,
        environment,
        None,
    );
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
            Some(function_index),
        )
        .resolve_function_item(&mut function);
        environment.put_user_defined_function(function_index, function)?;
    }

    propagate_function_effects(compilation_context, environment)?;
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
    current_function_index: Option<FunctionIndex>,
    local_value_types: HashMap<SharedString, TypeIndex>,
}

impl<'a> Resolver<'a> {
    fn new(
        source_file: &'a SourceFile,
        module_name: &'a str,
        compilation_context: &'a mut CompilationContext,
        environment: &'a mut ProgramEnvironment,
        current_function_index: Option<FunctionIndex>,
    ) -> Self {
        Self {
            source_file,
            module_name,
            compilation_context,
            environment,
            current_function_index,
            local_value_types: HashMap::new(),
        }
    }

    fn register_effect_items(&mut self, script: &mut Script) {
        let mut retained_items = Vec::with_capacity(script.items.len());

        for item in std::mem::take(&mut script.items) {
            match item.kind {
                ItemKind::Effect(effect_item) => self.register_effect_item(effect_item),
                _ => retained_items.push(item),
            }
        }

        script.items = retained_items;
    }

    fn register_function_items(&mut self, script: &mut Script) -> OcelotResult<()> {
        let mut retained_items = Vec::with_capacity(script.items.len());

        for item in std::mem::take(&mut script.items) {
            match item.kind {
                ItemKind::Effect(effect_item) => self.register_effect_item(effect_item),
                ItemKind::Function(function_item) => self.register_function_item(function_item)?,
                _ => retained_items.push(item),
            }
        }

        script.items = retained_items;
        Ok(())
    }

    fn register_use_items(&mut self, script: &mut Script) {
        let mut retained_items = Vec::with_capacity(script.items.len());

        for item in std::mem::take(&mut script.items) {
            match item.kind {
                ItemKind::Use(use_item) => self.register_use_item(use_item),
                _ => retained_items.push(item),
            }
        }

        script.items = retained_items;
    }

    fn resolve_item(&mut self, item: &mut Item) {
        match &mut item.kind {
            ItemKind::Effect(_) => {
                unreachable!("effect items should be lowered before item resolution")
            }
            ItemKind::Statement(statement) => self.resolve_statement(statement),
            ItemKind::Test(test_item) => self.resolve_test_item(test_item),
            ItemKind::Function(_) => {
                unreachable!("function items should be lowered before item resolution")
            }
            ItemKind::Use(_) => {
                unreachable!("use items should be lowered before item resolution")
            }
        }
    }

    fn register_effect_item(&mut self, effect_item: EffectItem) {
        if let Some(effect_index) = self
            .environment
            .resolve_effect(effect_item.identifier.name.as_str())
        {
            let existing_effect = self
                .environment
                .effect_definition(effect_index)
                .expect("resolved effect index should point at a definition");

            if existing_effect.is_builtin {
                self.add_diagnostic(
                    format!(
                        "effect `{}` conflicts with builtin effect",
                        effect_item.identifier.name
                    ),
                    effect_item.identifier.span.clone(),
                    "duplicate effect",
                );
                return;
            }

            let diagnostic = self
                .source_diagnostic(
                    self.source_file,
                    format!("duplicate effect `{}`", effect_item.identifier.name),
                    effect_item.identifier.span.clone(),
                    "duplicate effect",
                )
                .with_excerpt(
                    self.source_excerpt(
                        existing_effect
                            .source_file
                            .as_deref()
                            .expect("user-declared effect should keep its source file"),
                        existing_effect
                            .declaration_span
                            .clone()
                            .expect("user-declared effect should keep its declaration span"),
                        "already defined here",
                    ),
                );
            self.compilation_context.add_diagnostic(diagnostic);
            return;
        }

        self.environment.add_effect(Effect::declared(
            effect_item.identifier.name.clone(),
            effect_item.identifier.span.clone(),
            self.source_file.clone(),
        ));
    }

    fn register_function_item(&mut self, mut function_item: FunctionItem) -> OcelotResult<()> {
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
                FunctionKind::NativeFunction { .. } => None,
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
            return Ok(());
        }

        let can_effects = self.resolve_effect_clause(function_item.can_clause.as_ref());
        let cannot_effects = self.resolve_effect_clause(function_item.cannot_clause.as_ref());
        let argument_types = self.resolve_function_parameter_types(&mut function_item);

        if function_item.is_native {
            if self.module_name != "core" {
                self.add_diagnostic(
                    "native functions may only be declared in `core`",
                    function_item.identifier.span.clone(),
                    "only allowed in `core`",
                );
                return Ok(());
            }

            let Some(native_function) = self
                .environment
                .resolve_native_function_implementation(qualified_name.as_str())
            else {
                ocelot_base::bail!(
                    "internal error: native function `{qualified_name}` has no registered implementation"
                );
            };

            let signature = native_function.signature();
            if signature.argument_types.len() != argument_types.len() {
                ocelot_base::bail!(
                    "internal error: native function `{qualified_name}` declaration does not match its registered signature"
                );
            }

            for (declared_type, expected_type) in
                argument_types.iter().zip(signature.argument_types.iter())
            {
                let declared_kind = self.environment.type_definition(*declared_type)?.kind;
                if declared_kind != *expected_type {
                    ocelot_base::bail!(
                        "internal error: native function `{qualified_name}` declaration type `{}` does not match registered type `{}`",
                        self.type_label(*declared_type),
                        native_type_label(*expected_type)
                    );
                }
            }

            self.environment.add_function(FunctionDefinition::native(
                self.module_name,
                qualified_name,
                argument_types,
                native_function,
                can_effects,
                cannot_effects,
            ));
            return Ok(());
        }

        self.environment
            .add_function(FunctionDefinition::user_defined(
                self.module_name,
                qualified_name,
                function_item,
                argument_types,
                can_effects,
                cannot_effects,
                self.source_file.clone(),
            ));
        Ok(())
    }

    fn register_use_item(&mut self, use_item: UseItem) {
        let module_name = use_item.module_path.render();

        if !self.environment.has_module(module_name.as_str()) {
            self.add_diagnostic(
                format!("unknown module `{module_name}`"),
                use_item.module_path.span(),
                "unknown module",
            );
            return;
        }

        for imported_name in use_item.imported_names {
            self.register_imported_name(&module_name, imported_name);
        }
    }

    fn register_imported_name(&mut self, module_name: &str, imported_name: Identifier) {
        let local_name = imported_name.name.clone();
        let qualified_name = self
            .environment
            .qualify_function_name(module_name, local_name.as_str());

        let Some(function_index) = self
            .environment
            .resolve_function_exact(qualified_name.as_str())
        else {
            self.add_diagnostic(
                format!("module `{module_name}` has no function `{local_name}`"),
                imported_name.span,
                "unknown function",
            );
            return;
        };

        if self
            .environment
            .resolve_function_exact(
                self.environment
                    .qualify_function_name(self.module_name, local_name.as_str())
                    .as_str(),
            )
            .is_some()
        {
            self.add_diagnostic(
                format!(
                    "imported function `{local_name}` conflicts with local function `{}`",
                    self.module_name
                ),
                imported_name.span,
                "conflicting import",
            );
            return;
        }

        if self
            .environment
            .resolve_imported_function(&self.source_file.path, local_name.as_str())
            .is_some()
        {
            self.add_diagnostic(
                format!("duplicate import `{local_name}`"),
                imported_name.span,
                "duplicate import",
            );
            return;
        }

        self.environment.add_imported_function(
            self.source_file.path.clone(),
            local_name,
            function_index,
        );
    }

    fn resolve_function_item(&mut self, function_item: &mut FunctionItem) {
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
                } else {
                    let Some(function_index) = self.environment.resolve_local_function(
                        &self.source_file.path,
                        self.module_name,
                        identifier.name.as_str(),
                    ) else {
                        self.add_diagnostic(
                            format!("unknown function `{}`", identifier.name),
                            identifier.span.clone(),
                            "unknown function",
                        );
                        return;
                    };

                    Some((function_index, identifier.name.clone()))
                }
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
            ExpressionKind::Identifier(identifier) => {
                expression.ty = self
                    .local_value_types
                    .get(identifier.name.as_str())
                    .copied()
                    .unwrap_or_else(TypeIndex::unresolved);
            }
            ExpressionKind::StringLiteral(_) => {
                expression.ty = string_type_index;
            }
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

    fn resolve_effect_clause(
        &mut self,
        clause: Option<&FunctionEffectClause>,
    ) -> BTreeSet<EffectIndex> {
        let Some(clause) = clause else {
            return BTreeSet::new();
        };

        let Some(effect_index) = self.environment.resolve_effect(clause.effect.name.as_str())
        else {
            self.add_diagnostic(
                format!("unknown effect `{}`", clause.effect.name),
                clause.effect.span.clone(),
                "unknown effect",
            );
            return BTreeSet::new();
        };

        BTreeSet::from([effect_index])
    }

    fn record_effect_dependency(&mut self, called_function_index: FunctionIndex, span: Span) {
        let Some(current_function_index) = self.current_function_index else {
            return;
        };

        let Ok(called_function) = self.environment.function_definition(called_function_index)
        else {
            return;
        };
        let called_kind = called_function.kind.clone();
        let called_effects = called_function.inferred_effects.clone();

        let Ok(current_function) = self
            .environment
            .function_definition_mut(current_function_index)
        else {
            return;
        };

        match called_kind {
            FunctionKind::NativeFunction { .. } => {
                for effect_index in called_effects {
                    current_function.direct_effects.insert(effect_index);
                    current_function
                        .direct_effect_sources
                        .entry(effect_index)
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

    fn validate_call_arity(
        &mut self,
        function_name: &str,
        call_expression: &CallExpression,
        function_index: FunctionIndex,
    ) -> bool {
        let Ok(function_definition) = self.environment.function_definition(function_index) else {
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

    fn add_duplicate_parameter_diagnostic(
        &mut self,
        duplicate_parameter: &FunctionParameter,
        original_parameter: &FunctionParameter,
    ) {
        let diagnostic = self
            .source_diagnostic(
                self.source_file,
                format!(
                    "duplicate parameter `{}`",
                    duplicate_parameter.identifier.name
                ),
                duplicate_parameter.identifier.span.clone(),
                "duplicate parameter",
            )
            .with_excerpt(self.source_excerpt(
                self.source_file,
                original_parameter.identifier.span.clone(),
                "already defined here",
            ));
        self.compilation_context.add_diagnostic(diagnostic);
    }

    fn resolve_function_parameter_types(
        &mut self,
        function_item: &mut FunctionItem,
    ) -> Vec<TypeIndex> {
        let mut seen_parameters = HashMap::<SharedString, FunctionParameter>::new();

        for parameter in &function_item.parameters {
            if let Some(original_parameter) =
                seen_parameters.insert(parameter.identifier.name.clone(), parameter.clone())
            {
                self.add_duplicate_parameter_diagnostic(parameter, &original_parameter);
            }
        }

        function_item
            .parameters
            .iter_mut()
            .map(|parameter| {
                let Some(type_index) = self
                    .environment
                    .resolve_type(parameter.type_name.name.as_str())
                else {
                    self.add_diagnostic(
                        format!("unknown type `{}`", parameter.type_name.name),
                        parameter.type_name.span.clone(),
                        "unknown type",
                    );
                    parameter.ty = TypeIndex::unresolved();
                    return TypeIndex::unresolved();
                };

                parameter.ty = type_index;
                if !function_item.is_native && type_index == self.environment.any_type_index() {
                    self.add_diagnostic(
                        "`any` may only be used in native function signatures",
                        parameter.type_name.span.clone(),
                        "`any` is only allowed here for native functions",
                    );
                    parameter.ty = TypeIndex::unresolved();
                    return TypeIndex::unresolved();
                }

                type_index
            })
            .collect()
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

fn propagate_function_effects(
    compilation_context: &mut CompilationContext,
    environment: &mut ProgramEnvironment,
) -> OcelotResult<()> {
    let function_indices = environment.user_defined_function_indices();

    let mut changed = true;
    while changed {
        changed = false;

        for function_index in &function_indices {
            let (direct_effects, called_functions, current_inferred_effects) = {
                let function = environment.function_definition(*function_index)?;
                (
                    function.direct_effects.clone(),
                    function.called_functions.clone(),
                    function.inferred_effects.clone(),
                )
            };

            let mut next_effects = direct_effects;
            for called_function_index in called_functions.keys() {
                let called_function = environment.function_definition(*called_function_index)?;
                next_effects.extend(called_function.inferred_effects.iter().copied());
            }

            if next_effects != current_inferred_effects {
                environment
                    .function_definition_mut(*function_index)?
                    .inferred_effects = next_effects;
                changed = true;
            }
        }
    }

    for function_index in function_indices {
        let function = environment.function_definition(function_index)?.clone();

        for forbidden_effect in &function.cannot_effects {
            if !function.inferred_effects.contains(forbidden_effect) {
                continue;
            }

            let effect_name = environment
                .effect_definition(*forbidden_effect)?
                .name
                .clone();
            let Some((span, annotation)) =
                violation_source(environment, &function, *forbidden_effect)?
            else {
                continue;
            };

            let FunctionKind::UserDefined { source_file, .. } = &function.kind else {
                continue;
            };

            let mut diagnostic = source_diagnostic_for_span(
                source_file,
                format!(
                    "effect error: function `{}` cannot perform effect `{}`",
                    function.name, effect_name
                ),
                span,
                annotation,
            );

            if let Some(cannot_clause_span) = function.cannot_clause_span.clone() {
                diagnostic = diagnostic.with_excerpt(source_excerpt_for_span(
                    source_file,
                    cannot_clause_span,
                    "forbidden here",
                ));
            }

            compilation_context.add_diagnostic(diagnostic);
        }
    }

    Ok(())
}

fn violation_source(
    environment: &ProgramEnvironment,
    function: &FunctionDefinition,
    effect_index: EffectIndex,
) -> OcelotResult<Option<(Span, SharedString)>> {
    if function.can_effects.contains(&effect_index)
        && let Some(span) = function.can_clause_span.clone()
    {
        return Ok(Some((span, "effect declared here".into())));
    }

    for (called_function_index, span) in &function.called_functions {
        let called_function = environment.function_definition(*called_function_index)?;
        if called_function.inferred_effects.contains(&effect_index) {
            return Ok(Some((
                span.clone(),
                format!(
                    "this has a `{}` effect",
                    effect_label(environment, effect_index)?
                )
                .into(),
            )));
        }
    }

    if let Some(span) = function.direct_effect_sources.get(&effect_index) {
        return Ok(Some((
            span.clone(),
            format!(
                "this has a `{}` effect",
                effect_label(environment, effect_index)?
            )
            .into(),
        )));
    }

    if let Some(span) = function.cannot_clause_span.clone() {
        return Ok(Some((span, "forbidden effect".into())));
    }

    Ok(None)
}

fn effect_label(
    environment: &ProgramEnvironment,
    effect_index: EffectIndex,
) -> OcelotResult<SharedString> {
    Ok(environment.effect_definition(effect_index)?.name.clone())
}

fn source_diagnostic_for_span(
    source_file: &SourceFile,
    message: impl Into<SharedString>,
    span: Span,
    annotation: impl Into<SharedString>,
) -> SourceDiagnostic {
    let message = message.into();
    SourceDiagnostic::new(DiagnosticLevel::Error, &source_file.path, message)
        .with_excerpt(source_excerpt_for_span(source_file, span, annotation))
}

fn source_excerpt_for_span(
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
    use super::register_core_module;
    use super::register_module_effects;
    use super::register_module_functions;
    use super::register_module_imports;
    use super::resolve;
    use super::resolve_module_items;
    use super::resolve_user_defined_function_definitions;
    use ocelot_ast::call_expression::CallExpression;
    use ocelot_ast::effect_item::EffectItem;
    use ocelot_ast::expression::Expression;
    use ocelot_ast::expression_kind::ExpressionKind;
    use ocelot_ast::expression_statement::ExpressionStatement;
    use ocelot_ast::function_effect_clause::FunctionEffectClause;
    use ocelot_ast::function_item::FunctionItem;
    use ocelot_ast::function_kind::FunctionKind;
    use ocelot_ast::function_parameter::FunctionParameter;
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
    use ocelot_ast::use_item::UseItem;
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

    fn effect_clause(name: &str, span: Span) -> FunctionEffectClause {
        FunctionEffectClause::new(Identifier::new(name, span.clone()), span)
    }

    fn parameter(name: &str, type_name: &str, span: Span) -> FunctionParameter {
        FunctionParameter::new(
            Identifier::new(name, Span::new(span.start(), span.start() + name.len())),
            Identifier::new(
                type_name,
                Span::new(span.end() - type_name.len(), span.end()),
            ),
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
        let println_index = {
            register_core_module(&mut CompilationContext::default(), &mut environment).unwrap();
            environment.resolve_function("core::println").unwrap()
        };
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
    fn resolves_parameter_references_inside_function_bodies() {
        let mut script = Script::new(
            vec![
                Item::new(
                    ItemKind::Function(FunctionItem::new(
                        Identifier::new("greet", Span::new(4, 9)),
                        vec![parameter("name", "string", Span::new(10, 22))],
                        None,
                        None,
                        vec![Statement::new(
                            StatementKind::Expression(ExpressionStatement::new(call(
                                identifier("println", Span::new(27, 34)),
                                vec![identifier("name", Span::new(35, 39))],
                                Span::new(27, 40),
                            ))),
                            Span::new(27, 41),
                        )],
                        Span::new(0, 43),
                    )),
                    Span::new(0, 43),
                ),
                Item::new(
                    ItemKind::Statement(Statement::new(
                        StatementKind::Expression(ExpressionStatement::new(call(
                            identifier("greet", Span::new(44, 49)),
                            vec![string_literal("hello", Span::new(50, 57))],
                            Span::new(44, 58),
                        ))),
                        Span::new(44, 59),
                    )),
                    Span::new(44, 59),
                ),
            ],
            Span::new(0, 59),
        );
        let source_file = SourceFile::new(
            "examples/functions.ocelot",
            "fun greet(name: string) { println(name); } greet(\"hello\");",
        );
        let mut environment = ProgramEnvironment::new();
        let mut context = CompilationContext::default();

        resolve(&mut script, &source_file, &mut context, &mut environment).unwrap();

        let function_definition = environment
            .function_definition(environment.resolve_function("functions::greet").unwrap())
            .unwrap();
        assert_eq!(
            function_definition.argument_types,
            vec![environment.string_type_index()]
        );

        let FunctionKind::UserDefined { function, .. } = &function_definition.kind else {
            panic!("expected user-defined function");
        };
        let StatementKind::Expression(ExpressionStatement { expression }) = &function.body[0].kind;
        let ExpressionKind::Call(call_expression) = &expression.kind else {
            panic!("expected call expression");
        };
        assert_eq!(
            call_expression.arguments[0].ty,
            environment.string_type_index()
        );
    }

    #[test]
    fn reports_duplicate_function_parameter_names() {
        let mut script = Script::new(
            vec![Item::new(
                ItemKind::Function(FunctionItem::new(
                    Identifier::new("greet", Span::new(4, 9)),
                    vec![
                        parameter("name", "string", Span::new(10, 22)),
                        parameter("name", "bool", Span::new(24, 34)),
                    ],
                    None,
                    None,
                    Vec::new(),
                    Span::new(0, 38),
                )),
                Span::new(0, 38),
            )],
            Span::new(0, 38),
        );
        let source_file = SourceFile::new(
            "examples/functions.ocelot",
            "fun greet(name: string, name: bool) {}",
        );
        let mut environment = ProgramEnvironment::new();
        let mut context = CompilationContext::default();

        register_module_functions(
            &mut script,
            "functions",
            &source_file,
            &mut context,
            &mut environment,
        )
        .unwrap();

        let error = finish_resolution(&context).unwrap_err();
        assert!(
            error
                .to_test_string()
                .contains("duplicate parameter `name`")
        );
    }

    #[test]
    fn reports_unknown_function_parameter_types() {
        let mut script = Script::new(
            vec![Item::new(
                ItemKind::Function(FunctionItem::new(
                    Identifier::new("greet", Span::new(4, 9)),
                    vec![parameter("name", "number", Span::new(10, 22))],
                    None,
                    None,
                    Vec::new(),
                    Span::new(0, 26),
                )),
                Span::new(0, 26),
            )],
            Span::new(0, 26),
        );
        let source_file =
            SourceFile::new("examples/functions.ocelot", "fun greet(name: number) {}");
        let mut environment = ProgramEnvironment::new();
        let mut context = CompilationContext::default();

        register_module_functions(
            &mut script,
            "functions",
            &source_file,
            &mut context,
            &mut environment,
        )
        .unwrap();

        let error = finish_resolution(&context).unwrap_err();
        assert!(error.to_test_string().contains("unknown type `number`"));
    }

    #[test]
    fn reports_any_in_user_defined_function_signatures() {
        let mut script = Script::new(
            vec![Item::new(
                ItemKind::Function(FunctionItem::new(
                    Identifier::new("greet", Span::new(4, 9)),
                    vec![parameter("value", "any", Span::new(10, 20))],
                    None,
                    None,
                    Vec::new(),
                    Span::new(0, 24),
                )),
                Span::new(0, 24),
            )],
            Span::new(0, 24),
        );
        let source_file = SourceFile::new("examples/functions.ocelot", "fun greet(value: any) {}");
        let mut environment = ProgramEnvironment::new();
        let mut context = CompilationContext::default();

        register_module_functions(
            &mut script,
            "functions",
            &source_file,
            &mut context,
            &mut environment,
        )
        .unwrap();

        let error = finish_resolution(&context).unwrap_err();
        assert!(
            error
                .to_test_string()
                .contains("`any` may only be used in native function signatures")
        );
    }

    #[test]
    fn reports_native_functions_outside_core() {
        let mut script = Script::new(
            vec![Item::new(
                ItemKind::Function(FunctionItem::new_native(
                    Identifier::new("println", Span::new(11, 18)),
                    vec![parameter("value", "any", Span::new(19, 29))],
                    None,
                    None,
                    Span::new(0, 30),
                )),
                Span::new(0, 30),
            )],
            Span::new(0, 30),
        );
        let source_file =
            SourceFile::new("examples/helper.ocelot", "native fun println(value: any);");
        let mut environment = ProgramEnvironment::new();
        let mut context = CompilationContext::default();

        register_module_functions(
            &mut script,
            "helper",
            &source_file,
            &mut context,
            &mut environment,
        )
        .unwrap();

        let error = finish_resolution(&context).unwrap_err();
        assert!(
            error
                .to_test_string()
                .contains("native functions may only be declared in `core`")
        );
    }

    #[test]
    fn reports_wrong_user_defined_call_arity() {
        let mut script = Script::new(
            vec![
                Item::new(
                    ItemKind::Function(FunctionItem::new(
                        Identifier::new("greet", Span::new(4, 9)),
                        vec![parameter("name", "string", Span::new(10, 22))],
                        None,
                        None,
                        Vec::new(),
                        Span::new(0, 26),
                    )),
                    Span::new(0, 26),
                ),
                Item::new(
                    ItemKind::Statement(Statement::new(
                        StatementKind::Expression(ExpressionStatement::new(call(
                            identifier("greet", Span::new(27, 32)),
                            Vec::new(),
                            Span::new(27, 34),
                        ))),
                        Span::new(27, 35),
                    )),
                    Span::new(27, 35),
                ),
            ],
            Span::new(0, 35),
        );
        let source_file = SourceFile::new(
            "examples/functions.ocelot",
            "fun greet(name: string) {} greet();",
        );
        let mut environment = ProgramEnvironment::new();
        let mut context = CompilationContext::default();

        resolve(&mut script, &source_file, &mut context, &mut environment).unwrap_err();

        let error = finish_resolution(&context).unwrap_err();
        assert!(
            error
                .to_test_string()
                .contains("type error: `greet` expects exactly one argument")
        );
    }

    #[test]
    fn reports_wrong_user_defined_call_argument_types() {
        let mut script = Script::new(
            vec![
                Item::new(
                    ItemKind::Function(FunctionItem::new(
                        Identifier::new("greet", Span::new(4, 9)),
                        vec![parameter("excited", "bool", Span::new(10, 23))],
                        None,
                        None,
                        Vec::new(),
                        Span::new(0, 27),
                    )),
                    Span::new(0, 27),
                ),
                Item::new(
                    ItemKind::Statement(Statement::new(
                        StatementKind::Expression(ExpressionStatement::new(call(
                            identifier("greet", Span::new(28, 33)),
                            vec![string_literal("hello", Span::new(34, 41))],
                            Span::new(28, 42),
                        ))),
                        Span::new(28, 43),
                    )),
                    Span::new(28, 43),
                ),
            ],
            Span::new(0, 43),
        );
        let source_file = SourceFile::new(
            "examples/functions.ocelot",
            "fun greet(excited: bool) {} greet(\"hello\");",
        );
        let mut environment = ProgramEnvironment::new();
        let mut context = CompilationContext::default();

        resolve(&mut script, &source_file, &mut context, &mut environment).unwrap_err();

        let error = finish_resolution(&context).unwrap_err();
        assert!(
            error
                .to_test_string()
                .contains("type error: argument 1 to `greet` must be bool")
        );
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
                    None,
                    None,
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
    fn resolves_imported_function_calls() {
        let mut main_script = Script::new(
            vec![
                Item::new(
                    ItemKind::Use(UseItem::new(
                        QualifiedIdentifier::new(vec![Identifier::new("helper", Span::new(4, 10))]),
                        vec![Identifier::new("greet", Span::new(12, 17))],
                        Span::new(0, 18),
                    )),
                    Span::new(0, 18),
                ),
                Item::new(
                    ItemKind::Statement(Statement::new(
                        StatementKind::Expression(ExpressionStatement::new(call(
                            identifier("greet", Span::new(19, 24)),
                            Vec::new(),
                            Span::new(19, 26),
                        ))),
                        Span::new(19, 27),
                    )),
                    Span::new(19, 27),
                ),
            ],
            Span::new(0, 27),
        );
        let mut helper_script = Script::new(
            vec![Item::new(
                ItemKind::Function(FunctionItem::new(
                    Identifier::new("greet", Span::new(4, 9)),
                    Vec::new(),
                    None,
                    None,
                    Vec::new(),
                    Span::new(0, 14),
                )),
                Span::new(0, 14),
            )],
            Span::new(0, 14),
        );
        let main_source_file =
            SourceFile::new("main.ocelot-script", "use helper::greet;\ngreet();");
        let helper_source_file = SourceFile::new("helper.ocelot", "fun greet() {}");
        let mut environment = ProgramEnvironment::new();
        environment.add_module("main");
        environment.add_module("helper");
        let mut context = CompilationContext::default();

        register_module_functions(
            &mut helper_script,
            "helper",
            &helper_source_file,
            &mut context,
            &mut environment,
        )
        .unwrap();
        register_module_imports(
            &mut main_script,
            "main",
            &main_source_file,
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
            environment.resolve_function("helper::greet").unwrap()
        );
    }

    #[test]
    fn imported_names_are_available_inside_function_bodies() {
        let mut main_script = Script::new(
            vec![
                Item::new(
                    ItemKind::Use(UseItem::new(
                        QualifiedIdentifier::new(vec![Identifier::new("helper", Span::new(4, 10))]),
                        vec![Identifier::new("greet", Span::new(12, 17))],
                        Span::new(0, 18),
                    )),
                    Span::new(0, 18),
                ),
                Item::new(
                    ItemKind::Function(FunctionItem::new(
                        Identifier::new("run", Span::new(23, 26)),
                        Vec::new(),
                        None,
                        None,
                        vec![Statement::new(
                            StatementKind::Expression(ExpressionStatement::new(call(
                                identifier("greet", Span::new(33, 38)),
                                Vec::new(),
                                Span::new(33, 40),
                            ))),
                            Span::new(33, 41),
                        )],
                        Span::new(19, 43),
                    )),
                    Span::new(19, 43),
                ),
            ],
            Span::new(0, 43),
        );
        let mut helper_script = Script::new(
            vec![Item::new(
                ItemKind::Function(FunctionItem::new(
                    Identifier::new("greet", Span::new(4, 9)),
                    Vec::new(),
                    None,
                    None,
                    Vec::new(),
                    Span::new(0, 14),
                )),
                Span::new(0, 14),
            )],
            Span::new(0, 14),
        );
        let main_source_file =
            SourceFile::new("main.ocelot", "use helper::greet;\nfun run() { greet(); }");
        let helper_source_file = SourceFile::new("helper.ocelot", "fun greet() {}");
        let mut environment = ProgramEnvironment::new();
        environment.add_module("main");
        environment.add_module("helper");
        let mut context = CompilationContext::default();

        register_module_functions(
            &mut main_script,
            "main",
            &main_source_file,
            &mut context,
            &mut environment,
        )
        .unwrap();
        register_module_functions(
            &mut helper_script,
            "helper",
            &helper_source_file,
            &mut context,
            &mut environment,
        )
        .unwrap();
        register_module_imports(
            &mut main_script,
            "main",
            &main_source_file,
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

        let run = environment
            .function_definition(environment.resolve_function("main::run").unwrap())
            .unwrap();
        let FunctionKind::UserDefined { function, .. } = &run.kind else {
            panic!("expected user-defined function");
        };
        let StatementKind::Expression(ExpressionStatement { expression }) = &function.body[0].kind;
        let ExpressionKind::Call(call_expression) = &expression.kind else {
            panic!("expected call expression");
        };
        assert_eq!(
            call_expression.function_index().unwrap(),
            environment.resolve_function("helper::greet").unwrap()
        );
    }

    #[test]
    fn local_functions_win_over_imported_names() {
        let mut main_script = Script::new(
            vec![
                Item::new(
                    ItemKind::Use(UseItem::new(
                        QualifiedIdentifier::new(vec![Identifier::new("helper", Span::new(4, 10))]),
                        vec![Identifier::new("greet", Span::new(12, 17))],
                        Span::new(0, 18),
                    )),
                    Span::new(0, 18),
                ),
                Item::new(
                    ItemKind::Function(FunctionItem::new(
                        Identifier::new("greet", Span::new(23, 28)),
                        Vec::new(),
                        None,
                        None,
                        Vec::new(),
                        Span::new(19, 31),
                    )),
                    Span::new(19, 31),
                ),
                Item::new(
                    ItemKind::Statement(Statement::new(
                        StatementKind::Expression(ExpressionStatement::new(call(
                            identifier("greet", Span::new(32, 37)),
                            Vec::new(),
                            Span::new(32, 39),
                        ))),
                        Span::new(32, 40),
                    )),
                    Span::new(32, 40),
                ),
            ],
            Span::new(0, 40),
        );
        let mut helper_script = Script::new(
            vec![Item::new(
                ItemKind::Function(FunctionItem::new(
                    Identifier::new("greet", Span::new(4, 9)),
                    Vec::new(),
                    None,
                    None,
                    Vec::new(),
                    Span::new(0, 14),
                )),
                Span::new(0, 14),
            )],
            Span::new(0, 14),
        );
        let main_source_file = SourceFile::new(
            "main.ocelot-script",
            "use helper::greet;\nfun greet() {}\ngreet();",
        );
        let helper_source_file = SourceFile::new("helper.ocelot", "fun greet() {}");
        let mut environment = ProgramEnvironment::new();
        environment.add_module("main");
        environment.add_module("helper");
        let mut context = CompilationContext::default();

        register_module_functions(
            &mut main_script,
            "main",
            &main_source_file,
            &mut context,
            &mut environment,
        )
        .unwrap();
        register_module_functions(
            &mut helper_script,
            "helper",
            &helper_source_file,
            &mut context,
            &mut environment,
        )
        .unwrap();
        register_module_imports(
            &mut main_script,
            "main",
            &main_source_file,
            &mut context,
            &mut environment,
        )
        .unwrap();

        let error = finish_resolution(&context).unwrap_err();
        assert!(
            error
                .to_test_string()
                .contains("conflicts with local function")
        );
    }

    #[test]
    fn reports_duplicate_imports() {
        let mut main_script = Script::new(
            vec![
                Item::new(
                    ItemKind::Use(UseItem::new(
                        QualifiedIdentifier::new(vec![Identifier::new("helper", Span::new(4, 10))]),
                        vec![Identifier::new("greet", Span::new(12, 17))],
                        Span::new(0, 18),
                    )),
                    Span::new(0, 18),
                ),
                Item::new(
                    ItemKind::Use(UseItem::new(
                        QualifiedIdentifier::new(vec![Identifier::new(
                            "helper",
                            Span::new(23, 29),
                        )]),
                        vec![Identifier::new("greet", Span::new(31, 36))],
                        Span::new(19, 37),
                    )),
                    Span::new(19, 37),
                ),
            ],
            Span::new(0, 37),
        );
        let mut helper_script = Script::new(
            vec![Item::new(
                ItemKind::Function(FunctionItem::new(
                    Identifier::new("greet", Span::new(4, 9)),
                    Vec::new(),
                    None,
                    None,
                    Vec::new(),
                    Span::new(0, 14),
                )),
                Span::new(0, 14),
            )],
            Span::new(0, 14),
        );
        let main_source_file = SourceFile::new(
            "main.ocelot-script",
            "use helper::greet;\nuse helper::greet;",
        );
        let helper_source_file = SourceFile::new("helper.ocelot", "fun greet() {}");
        let mut environment = ProgramEnvironment::new();
        environment.add_module("main");
        environment.add_module("helper");
        let mut context = CompilationContext::default();

        register_module_functions(
            &mut helper_script,
            "helper",
            &helper_source_file,
            &mut context,
            &mut environment,
        )
        .unwrap();
        register_module_imports(
            &mut main_script,
            "main",
            &main_source_file,
            &mut context,
            &mut environment,
        )
        .unwrap();

        let error = finish_resolution(&context).unwrap_err();
        assert!(error.to_test_string().contains("duplicate import `greet`"));
    }

    #[test]
    fn reports_unknown_functions_in_use_items() {
        let mut main_script = Script::new(
            vec![Item::new(
                ItemKind::Use(UseItem::new(
                    QualifiedIdentifier::new(vec![Identifier::new("helper", Span::new(4, 10))]),
                    vec![Identifier::new("greet", Span::new(12, 17))],
                    Span::new(0, 18),
                )),
                Span::new(0, 18),
            )],
            Span::new(0, 18),
        );
        let mut helper_script = Script::new(
            vec![Item::new(
                ItemKind::Function(FunctionItem::new(
                    Identifier::new("wave", Span::new(4, 8)),
                    Vec::new(),
                    None,
                    None,
                    Vec::new(),
                    Span::new(0, 13),
                )),
                Span::new(0, 13),
            )],
            Span::new(0, 13),
        );
        let main_source_file = SourceFile::new("main.ocelot-script", "use helper::greet;");
        let helper_source_file = SourceFile::new("helper.ocelot", "fun wave() {}");
        let mut environment = ProgramEnvironment::new();
        environment.add_module("main");
        environment.add_module("helper");
        let mut context = CompilationContext::default();

        register_module_functions(
            &mut helper_script,
            "helper",
            &helper_source_file,
            &mut context,
            &mut environment,
        )
        .unwrap();
        register_module_imports(
            &mut main_script,
            "main",
            &main_source_file,
            &mut context,
            &mut environment,
        )
        .unwrap();

        let error = finish_resolution(&context).unwrap_err();
        assert!(
            error
                .to_test_string()
                .contains("module `helper` has no function `greet`")
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
                        None,
                        None,
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

    #[test]
    fn registers_effect_items_before_function_resolution() {
        let mut script = Script::new(
            vec![Item::new(
                ItemKind::Effect(EffectItem::new(
                    Identifier::new("exec", Span::new(7, 11)),
                    Span::new(0, 12),
                )),
                Span::new(0, 12),
            )],
            Span::new(0, 12),
        );
        let source_file = SourceFile::new("main.ocelot", "effect exec;");
        let mut environment = ProgramEnvironment::new();
        let mut context = CompilationContext::default();

        register_module_effects(&mut script, &source_file, &mut context, &mut environment).unwrap();

        assert!(script.items.is_empty());
        assert!(environment.resolve_effect("exec").is_some());
    }

    #[test]
    fn propagates_explicit_can_effects_to_callers() {
        let mut script = Script::new(
            vec![
                Item::new(
                    ItemKind::Effect(EffectItem::new(
                        Identifier::new("exec", Span::new(7, 11)),
                        Span::new(0, 12),
                    )),
                    Span::new(0, 12),
                ),
                Item::new(
                    ItemKind::Function(FunctionItem::new(
                        Identifier::new("child", Span::new(17, 22)),
                        Vec::new(),
                        Some(effect_clause("exec", Span::new(25, 33))),
                        None,
                        Vec::new(),
                        Span::new(13, 36),
                    )),
                    Span::new(13, 36),
                ),
                Item::new(
                    ItemKind::Function(FunctionItem::new(
                        Identifier::new("parent", Span::new(41, 47)),
                        Vec::new(),
                        None,
                        None,
                        vec![Statement::new(
                            StatementKind::Expression(ExpressionStatement::new(call(
                                identifier("child", Span::new(53, 58)),
                                Vec::new(),
                                Span::new(53, 60),
                            ))),
                            Span::new(53, 61),
                        )],
                        Span::new(37, 63),
                    )),
                    Span::new(37, 63),
                ),
            ],
            Span::new(0, 63),
        );
        let source_file = SourceFile::new(
            "main.ocelot",
            "effect exec; fun child() can exec {} fun parent() { child(); }",
        );
        let mut environment = ProgramEnvironment::new();
        let mut context = CompilationContext::default();

        register_module_effects(&mut script, &source_file, &mut context, &mut environment).unwrap();
        register_module_functions(
            &mut script,
            "main",
            &source_file,
            &mut context,
            &mut environment,
        )
        .unwrap();
        resolve_module_items(
            &mut script,
            "main",
            &source_file,
            &mut context,
            &mut environment,
        )
        .unwrap();
        resolve_user_defined_function_definitions(&mut context, &mut environment).unwrap();
        finish_resolution(&context).unwrap();

        let exec_effect = environment.resolve_effect("exec").unwrap();
        let parent = environment
            .function_definition(environment.resolve_function("main::parent").unwrap())
            .unwrap();

        assert!(parent.inferred_effects.contains(&exec_effect));
    }

    #[test]
    fn reports_transitive_forbidden_effects() {
        let mut script = Script::new(
            vec![
                Item::new(
                    ItemKind::Effect(EffectItem::new(
                        Identifier::new("exec", Span::new(7, 11)),
                        Span::new(0, 12),
                    )),
                    Span::new(0, 12),
                ),
                Item::new(
                    ItemKind::Function(FunctionItem::new(
                        Identifier::new("child", Span::new(17, 22)),
                        Vec::new(),
                        Some(effect_clause("exec", Span::new(25, 33))),
                        None,
                        Vec::new(),
                        Span::new(13, 36),
                    )),
                    Span::new(13, 36),
                ),
                Item::new(
                    ItemKind::Function(FunctionItem::new(
                        Identifier::new("parent", Span::new(41, 47)),
                        Vec::new(),
                        None,
                        Some(effect_clause("exec", Span::new(50, 61))),
                        vec![Statement::new(
                            StatementKind::Expression(ExpressionStatement::new(call(
                                identifier("child", Span::new(65, 70)),
                                Vec::new(),
                                Span::new(65, 72),
                            ))),
                            Span::new(65, 73),
                        )],
                        Span::new(37, 75),
                    )),
                    Span::new(37, 75),
                ),
            ],
            Span::new(0, 75),
        );
        let source_file = SourceFile::new(
            "main.ocelot",
            "effect exec; fun child() can exec {} fun parent() cannot exec { child(); }",
        );
        let mut environment = ProgramEnvironment::new();
        let mut context = CompilationContext::default();

        register_module_effects(&mut script, &source_file, &mut context, &mut environment).unwrap();
        register_module_functions(
            &mut script,
            "main",
            &source_file,
            &mut context,
            &mut environment,
        )
        .unwrap();
        resolve_module_items(
            &mut script,
            "main",
            &source_file,
            &mut context,
            &mut environment,
        )
        .unwrap();
        resolve_user_defined_function_definitions(&mut context, &mut environment).unwrap();

        let error = finish_resolution(&context).unwrap_err();

        assert!(matches!(
            error.kind(),
            ocelot_base::error::ErrorKind::CompilationError(CompilationStage::Resolver)
        ));
        assert!(
            error
                .to_test_string()
                .contains("effect error: function `main::parent` cannot perform effect `exec`")
        );
    }

    #[test]
    fn reports_direct_builtin_effect_violations_at_the_call_site() {
        let mut script = Script::new(
            vec![Item::new(
                ItemKind::Function(FunctionItem::new(
                    Identifier::new("quiet", Span::new(4, 9)),
                    Vec::new(),
                    None,
                    Some(effect_clause("write_stdout", Span::new(12, 32))),
                    vec![Statement::new(
                        StatementKind::Expression(ExpressionStatement::new(call(
                            identifier("println", Span::new(35, 42)),
                            vec![string_literal("hello", Span::new(43, 50))],
                            Span::new(35, 51),
                        ))),
                        Span::new(35, 52),
                    )],
                    Span::new(0, 54),
                )),
                Span::new(0, 54),
            )],
            Span::new(0, 54),
        );
        let source_file = SourceFile::new(
            "main.ocelot",
            "fun quiet() cannot write_stdout { println(\"hello\"); }",
        );
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
        resolve_user_defined_function_definitions(&mut context, &mut environment).unwrap();

        let error = finish_resolution(&context).unwrap_err();

        assert!(error.to_test_string().contains("println"));
    }

    #[test]
    fn reports_unknown_effect_names_in_function_annotations() {
        let mut script = Script::new(
            vec![Item::new(
                ItemKind::Function(FunctionItem::new(
                    Identifier::new("quiet", Span::new(4, 9)),
                    Vec::new(),
                    Some(effect_clause("exec", Span::new(12, 20))),
                    None,
                    Vec::new(),
                    Span::new(0, 23),
                )),
                Span::new(0, 23),
            )],
            Span::new(0, 23),
        );
        let source_file = SourceFile::new("main.ocelot", "fun quiet() can exec {}");
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

        let error = finish_resolution(&context).unwrap_err();

        assert!(error.to_test_string().contains("unknown effect `exec`"));
    }

    #[test]
    fn reports_duplicate_effect_declarations() {
        let mut script = Script::new(
            vec![
                Item::new(
                    ItemKind::Effect(EffectItem::new(
                        Identifier::new("exec", Span::new(7, 11)),
                        Span::new(0, 12),
                    )),
                    Span::new(0, 12),
                ),
                Item::new(
                    ItemKind::Effect(EffectItem::new(
                        Identifier::new("exec", Span::new(20, 24)),
                        Span::new(13, 25),
                    )),
                    Span::new(13, 25),
                ),
            ],
            Span::new(0, 25),
        );
        let source_file = SourceFile::new("main.ocelot", "effect exec;\neffect exec;");
        let mut environment = ProgramEnvironment::new();
        let mut context = CompilationContext::default();

        register_module_effects(&mut script, &source_file, &mut context, &mut environment).unwrap();

        let error = finish_resolution(&context).unwrap_err();

        assert!(error.to_test_string().contains("duplicate effect `exec`"));
    }
}
