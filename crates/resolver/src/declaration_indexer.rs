use crate::diagnostics::source_diagnostic_for_span;
use crate::diagnostics::source_excerpt_for_span;
use ocelot_ast::effect::Effect;
use ocelot_ast::effect_index::EffectIndex;
use ocelot_ast::effect_item::EffectItem;
use ocelot_ast::function_effect_clause::FunctionEffectClause;
use ocelot_ast::function_item::FunctionItem;
use ocelot_ast::function_parameter::FunctionParameter;
use ocelot_ast::identifier::Identifier;
use ocelot_ast::item_kind::ItemKind;
use ocelot_ast::script::Script;
use ocelot_ast::type_index::TypeIndex;
use ocelot_ast::use_item::UseItem;
use ocelot_base::compilation_context::CompilationContext;
use ocelot_base::result::OcelotResult;
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_file::SourceFile;
use ocelot_base::span::Span;
use ocelot_semantic::compilation_session::CompilationSession;
use ocelot_semantic::function_definition::FunctionDefinition;
use ocelot_semantic::function_kind::FunctionKind;
use ocelot_semantic::module_environment::ModuleEnvironment;
use ocelot_semantic::native_function::native_type_label;
use ocelot_semantic::symbol_table::SymbolTable;
use std::collections::BTreeSet;
use std::collections::HashMap;

pub trait DeclarationIndex {
    fn resolve_effect(&self, name: &str) -> Option<EffectIndex>;
    fn effect_definition(&self, effect_index: EffectIndex) -> OcelotResult<&Effect>;
    fn add_effect(&mut self, effect: Effect) -> EffectIndex;
    fn qualify_function_name(&self, module_name: &str, function_name: &str) -> SharedString;
    fn resolve_function_exact(
        &self,
        name: &str,
    ) -> Option<ocelot_ast::function_index::FunctionIndex>;
    fn function_definition(
        &self,
        function_index: ocelot_ast::function_index::FunctionIndex,
    ) -> OcelotResult<&FunctionDefinition>;
    fn add_function(
        &mut self,
        function: FunctionDefinition,
    ) -> ocelot_ast::function_index::FunctionIndex;
    fn add_module(&mut self, module_name: impl Into<SharedString>);
    fn has_module(&self, module_name: &str) -> bool;
    fn resolve_type(&self, name: &str) -> Option<TypeIndex>;
    fn type_definition(&self, type_index: TypeIndex) -> OcelotResult<&ocelot_ast::ty::Ty>;
    fn any_type_index(&self) -> TypeIndex;
}

impl DeclarationIndex for SymbolTable {
    fn resolve_effect(&self, name: &str) -> Option<EffectIndex> {
        SymbolTable::resolve_effect(self, name)
    }

    fn effect_definition(&self, effect_index: EffectIndex) -> OcelotResult<&Effect> {
        SymbolTable::effect_definition(self, effect_index)
    }

    fn add_effect(&mut self, effect: Effect) -> EffectIndex {
        SymbolTable::add_effect(self, effect)
    }

    fn qualify_function_name(&self, module_name: &str, function_name: &str) -> SharedString {
        SymbolTable::qualify_function_name(self, module_name, function_name)
    }

    fn resolve_function_exact(
        &self,
        name: &str,
    ) -> Option<ocelot_ast::function_index::FunctionIndex> {
        SymbolTable::resolve_function_exact(self, name)
    }

    fn function_definition(
        &self,
        function_index: ocelot_ast::function_index::FunctionIndex,
    ) -> OcelotResult<&FunctionDefinition> {
        SymbolTable::function_definition(self, function_index)
    }

    fn add_function(
        &mut self,
        function: FunctionDefinition,
    ) -> ocelot_ast::function_index::FunctionIndex {
        SymbolTable::add_function(self, function)
    }

    fn add_module(&mut self, module_name: impl Into<SharedString>) {
        SymbolTable::add_module(self, module_name);
    }

    fn has_module(&self, module_name: &str) -> bool {
        SymbolTable::has_module(self, module_name)
    }

    fn resolve_type(&self, name: &str) -> Option<TypeIndex> {
        SymbolTable::resolve_type(self, name)
    }

    fn type_definition(&self, type_index: TypeIndex) -> OcelotResult<&ocelot_ast::ty::Ty> {
        SymbolTable::type_definition(self, type_index)
    }

    fn any_type_index(&self) -> TypeIndex {
        SymbolTable::any_type_index(self)
    }
}

pub(crate) struct DeclarationIndexer<'a, D: DeclarationIndex> {
    source_file: &'a SourceFile,
    module_name: &'a str,
    compilation_context: &'a mut CompilationContext,
    declaration_index: &'a mut D,
    module_environment: &'a mut ModuleEnvironment,
    compilation_session: &'a CompilationSession,
}

impl<'a, D: DeclarationIndex> DeclarationIndexer<'a, D> {
    pub(crate) fn new(
        source_file: &'a SourceFile,
        module_name: &'a str,
        compilation_context: &'a mut CompilationContext,
        declaration_index: &'a mut D,
        module_environment: &'a mut ModuleEnvironment,
        compilation_session: &'a CompilationSession,
    ) -> Self {
        Self {
            source_file,
            module_name,
            compilation_context,
            declaration_index,
            module_environment,
            compilation_session,
        }
    }

    pub(crate) fn register_effect_items(&mut self, script: &mut Script) {
        let mut retained_items = Vec::with_capacity(script.items.len());

        for item in std::mem::take(&mut script.items) {
            match item.kind {
                ItemKind::Effect(effect_item) => self.register_effect_item(effect_item),
                _ => retained_items.push(item),
            }
        }

        script.items = retained_items;
    }

    pub(crate) fn register_function_items(&mut self, script: &mut Script) -> OcelotResult<()> {
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

    pub(crate) fn register_use_items(&mut self, script: &mut Script) {
        let mut retained_items = Vec::with_capacity(script.items.len());

        for item in std::mem::take(&mut script.items) {
            match item.kind {
                ItemKind::Use(use_item) => self.register_use_item(use_item),
                _ => retained_items.push(item),
            }
        }

        script.items = retained_items;
    }

    fn register_effect_item(&mut self, effect_item: EffectItem) {
        if let Some(effect_index) = self
            .declaration_index
            .resolve_effect(effect_item.identifier.name.as_str())
        {
            let existing_effect = self
                .declaration_index
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

            let diagnostic = source_diagnostic_for_span(
                self.source_file,
                format!("duplicate effect `{}`", effect_item.identifier.name),
                effect_item.identifier.span.clone(),
                "duplicate effect",
            )
            .with_excerpt(source_excerpt_for_span(
                existing_effect
                    .source_file
                    .as_deref()
                    .expect("user-declared effect should keep its source file"),
                existing_effect
                    .declaration_span
                    .clone()
                    .expect("user-declared effect should keep its declaration span"),
                "already defined here",
            ));
            self.compilation_context.add_diagnostic(diagnostic);
            return;
        }

        self.declaration_index.add_effect(Effect::declared(
            effect_item.identifier.name.clone(),
            effect_item.identifier.span.clone(),
            self.source_file.clone(),
        ));
    }

    fn register_function_item(&mut self, mut function_item: FunctionItem) -> OcelotResult<()> {
        let qualified_name = self
            .declaration_index
            .qualify_function_name(self.module_name, function_item.identifier.name.as_str());

        if let Some(function_index) = self
            .declaration_index
            .resolve_function_exact(qualified_name.as_str())
        {
            let existing_function = self
                .declaration_index
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
                .compilation_session
                .native_function_registry()
                .resolve(qualified_name.as_str())
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
                let declared_kind = self.declaration_index.type_definition(*declared_type)?.kind;
                if declared_kind != *expected_type {
                    ocelot_base::bail!(
                        "internal error: native function `{qualified_name}` declaration type `{}` does not match registered type `{}`",
                        self.type_label(*declared_type),
                        native_type_label(*expected_type)
                    );
                }
            }

            self.declaration_index
                .add_function(FunctionDefinition::native(
                    self.module_name,
                    qualified_name,
                    argument_types,
                    native_function,
                    can_effects,
                    cannot_effects,
                ));
            return Ok(());
        }

        self.declaration_index
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

        if !self.declaration_index.has_module(module_name.as_str()) {
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
            .declaration_index
            .qualify_function_name(module_name, local_name.as_str());

        let Some(function_index) = self
            .declaration_index
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
            .declaration_index
            .resolve_function_exact(
                self.declaration_index
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
            .module_environment
            .resolve_imported_function(local_name.as_str())
            .is_some()
        {
            self.add_diagnostic(
                format!("duplicate import `{local_name}`"),
                imported_name.span,
                "duplicate import",
            );
            return;
        }

        self.module_environment
            .add_imported_function(local_name, function_index);
    }

    fn resolve_effect_clause(
        &mut self,
        clause: Option<&FunctionEffectClause>,
    ) -> BTreeSet<EffectIndex> {
        let Some(clause) = clause else {
            return BTreeSet::new();
        };

        let Some(effect_index) = self
            .declaration_index
            .resolve_effect(clause.effect.name.as_str())
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

    fn add_diagnostic(
        &mut self,
        message: impl Into<SharedString>,
        span: Span,
        annotation: impl Into<SharedString>,
    ) {
        let diagnostic = source_diagnostic_for_span(self.source_file, message, span, annotation);
        self.compilation_context.add_diagnostic(diagnostic);
    }

    fn add_duplicate_function_diagnostic(
        &mut self,
        duplicate_function: &FunctionItem,
        original_function: &FunctionItem,
        original_source_file: &SourceFile,
    ) {
        let diagnostic = source_diagnostic_for_span(
            self.source_file,
            format!(
                "duplicate function `{}`",
                duplicate_function.identifier.name
            ),
            duplicate_function.identifier.span.clone(),
            "duplicate function",
        )
        .with_excerpt(source_excerpt_for_span(
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
        let diagnostic = source_diagnostic_for_span(
            self.source_file,
            format!(
                "duplicate parameter `{}`",
                duplicate_parameter.identifier.name
            ),
            duplicate_parameter.identifier.span.clone(),
            "duplicate parameter",
        )
        .with_excerpt(source_excerpt_for_span(
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
                    .declaration_index
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
                if !function_item.is_native && type_index == self.declaration_index.any_type_index()
                {
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

    fn type_label(&self, type_index: TypeIndex) -> SharedString {
        self.declaration_index
            .type_definition(type_index)
            .map(|ty| ty.name.clone())
            .unwrap_or_else(|_| "unknown".into())
    }
}
