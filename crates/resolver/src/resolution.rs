use crate::declaration_indexer::DeclarationIndex;
use crate::declaration_indexer::DeclarationIndexer;
use crate::effect_propagation::propagate_function_effects;
use crate::resolver::Resolver;
use ocelot_ast::compilation_unit::CompilationUnit;
use ocelot_ast::function_item::FunctionItem;
use ocelot_ast::identifier::Identifier;
use ocelot_base::compilation_stage::CompilationStage;
use ocelot_base::diagnostic_level::DiagnosticLevel;
use ocelot_base::error::OcelotError;
use ocelot_base::file_path::FilePath;
use ocelot_base::line_bounds::LineBounds;
use ocelot_base::render_source_diagnostics::render_source_diagnostics;
use ocelot_base::result::OcelotResult;
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_annotation::SourceAnnotation;
use ocelot_base::source_diagnostic::SourceDiagnostic;
use ocelot_base::source_excerpt::SourceExcerpt;
use ocelot_base::source_file::SourceFile;
use ocelot_base::span::Span;
use ocelot_semantic::compilation_context::CompilationContext;
use ocelot_semantic::compilation_inputs::CompilationInputs;
use ocelot_semantic::function_kind::FunctionKind;
use ocelot_semantic::module_imports::ModuleImports;
use ocelot_semantic::parsed_module::ParsedModule;
use ocelot_semantic::resolved_function::ResolvedFunction;
use ocelot_semantic::resolved_program::ResolvedProgram;
use ocelot_semantic::symbol_table::SymbolTable;
use std::collections::HashMap;

const CORE_MODULE_NAME: &str = "core";
const CORE_MODULE_PATH: &str = "crates/engine/resources/core.ocelot";
const CORE_MODULE_SOURCE: &str = include_str!("../../engine/resources/core.ocelot");

/// Resolves one compilation unit as though it were the only loaded module.
pub fn resolve(
    script: &mut CompilationUnit,
    source_file: &SourceFile,
    compilation_context: &mut CompilationContext,
    symbol_table: &mut SymbolTable,
    compilation_inputs: &CompilationInputs,
) -> OcelotResult<()> {
    let mut resolved_symbol_table = SymbolTable::new();
    register_core_module(
        compilation_context,
        &mut resolved_symbol_table,
        compilation_inputs,
    )?;
    let module_name = default_module_name(source_file);
    let mut module_imports = ModuleImports::new();
    resolved_symbol_table.add_module(module_name.clone());
    register_module_effects(
        script,
        source_file,
        compilation_context,
        &mut resolved_symbol_table,
    )?;
    register_module_functions(
        script,
        &module_name,
        source_file,
        compilation_context,
        &mut resolved_symbol_table,
        &mut module_imports,
        compilation_inputs,
    )?;
    register_module_imports(
        script,
        &module_name,
        source_file,
        compilation_context,
        &mut resolved_symbol_table,
        &mut module_imports,
    )?;
    resolve_module_items(
        script,
        &module_name,
        source_file,
        compilation_context,
        &resolved_symbol_table,
        &module_imports,
        compilation_inputs,
    )?;
    let resolved_functions = resolve_user_defined_function_definitions(
        compilation_context,
        &resolved_symbol_table,
        &HashMap::from([(source_file.path.clone(), module_imports)]),
        compilation_inputs,
    )?;
    *symbol_table = resolved_symbol_table;
    symbol_table.apply_resolved_functions(resolved_functions)?;
    finish_resolution(compilation_context)
}

/// Resolves a parsed multi-module program into one semantic result.
pub fn resolve_program(
    entry_path: FilePath,
    mut modules: Vec<ParsedModule>,
    compilation_inputs: &CompilationInputs,
) -> OcelotResult<ResolvedProgram> {
    let mut compilation_context = CompilationContext::default();
    let mut symbol_table = SymbolTable::new();

    if !modules
        .iter()
        .any(|module| module.module_name == CORE_MODULE_NAME)
    {
        register_core_module(
            &mut compilation_context,
            &mut symbol_table,
            compilation_inputs,
        )?;
    }
    validate_loaded_modules(&modules, &mut compilation_context);

    let mut module_imports_by_path: HashMap<FilePath, ModuleImports> = modules
        .iter()
        .map(|module| (module.source_file.path.clone(), ModuleImports::new()))
        .collect();

    for module in &modules {
        symbol_table.add_module(module.module_name.clone());
    }

    for module in &mut modules {
        register_module_effects(
            &mut module.compilation_unit,
            &module.source_file,
            &mut compilation_context,
            &mut symbol_table,
        )?;
    }

    for module in &mut modules {
        register_module_functions(
            &mut module.compilation_unit,
            module.module_name.as_str(),
            &module.source_file,
            &mut compilation_context,
            &mut symbol_table,
            module_imports_by_path
                .get_mut(&module.source_file.path)
                .expect("module imports should exist for parsed module"),
            compilation_inputs,
        )?;
    }

    for module in &mut modules {
        register_module_imports(
            &mut module.compilation_unit,
            module.module_name.as_str(),
            &module.source_file,
            &mut compilation_context,
            &mut symbol_table,
            module_imports_by_path
                .get_mut(&module.source_file.path)
                .expect("module imports should exist for parsed module"),
        )?;
    }

    for module in &mut modules {
        resolve_module_items(
            &mut module.compilation_unit,
            module.module_name.as_str(),
            &module.source_file,
            &mut compilation_context,
            &symbol_table,
            module_imports_by_path
                .get(&module.source_file.path)
                .expect("module imports should exist for parsed module"),
            compilation_inputs,
        )?;
    }

    let resolved_functions = resolve_user_defined_function_definitions(
        &mut compilation_context,
        &symbol_table,
        &module_imports_by_path,
        compilation_inputs,
    )?;
    let mut symbol_table = symbol_table;
    symbol_table.apply_resolved_functions(resolved_functions)?;

    Ok(ResolvedProgram::new(
        entry_path,
        modules,
        compilation_context.source_diagnostics,
        symbol_table,
    ))
}

/// Registers the compiler-provided `core` module into one symbol table.
pub fn register_core_module(
    compilation_context: &mut CompilationContext,
    declaration_index: &mut impl DeclarationIndex,
    compilation_inputs: &CompilationInputs,
) -> OcelotResult<()> {
    if declaration_index.has_module(CORE_MODULE_NAME) {
        return Ok(());
    }

    let source_file = SourceFile::new(CORE_MODULE_PATH, CORE_MODULE_SOURCE);
    let mut script = ocelot_parser::parse_compilation_unit::parse_compilation_unit(
        &source_file,
        &mut compilation_context.source_diagnostics,
    )?;
    let mut module_imports = ModuleImports::new();

    declaration_index.add_module(CORE_MODULE_NAME);
    register_module_effects(
        &mut script,
        &source_file,
        compilation_context,
        declaration_index,
    )?;
    register_module_functions(
        &mut script,
        CORE_MODULE_NAME,
        &source_file,
        compilation_context,
        declaration_index,
        &mut module_imports,
        compilation_inputs,
    )?;
    Ok(())
}

/// Registers all effect declarations for one module and lowers them out of the item list.
pub fn register_module_effects(
    script: &mut CompilationUnit,
    source_file: &SourceFile,
    compilation_context: &mut CompilationContext,
    declaration_index: &mut impl DeclarationIndex,
) -> OcelotResult<()> {
    let compilation_inputs = CompilationInputs::new();
    let mut module_imports = ModuleImports::new();
    DeclarationIndexer::new(
        source_file,
        "",
        compilation_context,
        declaration_index,
        &mut module_imports,
        &compilation_inputs,
    )
    .register_effect_items(script);
    Ok(())
}

/// Registers all function declarations for one module and lowers them out of the item list.
pub fn register_module_functions(
    script: &mut CompilationUnit,
    module_name: &str,
    source_file: &SourceFile,
    compilation_context: &mut CompilationContext,
    declaration_index: &mut impl DeclarationIndex,
    module_imports: &mut ModuleImports,
    compilation_inputs: &CompilationInputs,
) -> OcelotResult<()> {
    DeclarationIndexer::new(
        source_file,
        module_name,
        compilation_context,
        declaration_index,
        module_imports,
        compilation_inputs,
    )
    .register_function_items(script)?;
    Ok(())
}

/// Registers all import declarations for one module and lowers them out of the item list.
pub fn register_module_imports(
    script: &mut CompilationUnit,
    module_name: &str,
    source_file: &SourceFile,
    compilation_context: &mut CompilationContext,
    declaration_index: &mut impl DeclarationIndex,
    module_imports: &mut ModuleImports,
) -> OcelotResult<()> {
    let compilation_inputs = CompilationInputs::new();
    DeclarationIndexer::new(
        source_file,
        module_name,
        compilation_context,
        declaration_index,
        module_imports,
        &compilation_inputs,
    )
    .register_use_items(script);
    Ok(())
}

/// Resolves all non-function items in one module after registration.
pub fn resolve_module_items(
    script: &mut CompilationUnit,
    module_name: &str,
    source_file: &SourceFile,
    compilation_context: &mut CompilationContext,
    symbol_table: &SymbolTable,
    module_imports: &ModuleImports,
    _compilation_inputs: &CompilationInputs,
) -> OcelotResult<()> {
    let mut resolver = Resolver::new(
        source_file,
        module_name,
        compilation_context,
        symbol_table,
        module_imports,
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
    symbol_table: &SymbolTable,
    module_imports_by_path: &HashMap<FilePath, ModuleImports>,
    _compilation_inputs: &CompilationInputs,
) -> OcelotResult<Vec<ResolvedFunction>> {
    let function_indices = symbol_table.user_defined_function_indices();
    let mut resolved_functions = Vec::with_capacity(function_indices.len());

    for function_index in function_indices {
        let (module_name, source_file) = {
            let function_definition = symbol_table.function_definition(function_index)?;
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

        let function_definition = symbol_table.function_definition(function_index)?;
        let FunctionKind::UserDefined { function, .. } = &function_definition.kind else {
            ocelot_base::bail!(
                "internal error: function index did not reference a user-defined function"
            );
        };
        let module_imports = module_imports_by_path
            .get(&source_file.path)
            .expect("module imports should exist for resolved function source file");
        let mut resolved_function = ResolvedFunction::new(
            function_index,
            (**function).clone(),
            function_definition.direct_effects.clone(),
        );
        let mut function = std::mem::replace(
            &mut resolved_function.function,
            Box::new(FunctionItem::new(
                Identifier::new("", Span::default()),
                Vec::new(),
                None,
                None,
                Vec::new(),
                Span::default(),
            )),
        );
        Resolver::new(
            &source_file,
            module_name.as_str(),
            compilation_context,
            symbol_table,
            module_imports,
            Some(&mut resolved_function),
        )
        .resolve_function_item(&mut function);
        resolved_function.function = function;
        resolved_functions.push(resolved_function);
    }

    propagate_function_effects(compilation_context, symbol_table, &mut resolved_functions)?;
    Ok(resolved_functions)
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

fn validate_loaded_modules(modules: &[ParsedModule], compilation_context: &mut CompilationContext) {
    for module in modules {
        validate_loaded_module(module, compilation_context);
    }

    let mut modules_by_name: HashMap<SharedString, Vec<&SourceFile>> = HashMap::new();
    for module in modules {
        modules_by_name
            .entry(module.module_name.clone())
            .or_default()
            .push(&module.source_file);
    }

    for (module_name, source_files) in modules_by_name {
        if source_files.len() < 2 {
            continue;
        }

        let builtin_source_file = source_files
            .iter()
            .copied()
            .find(|source_file| source_file.path.as_str().starts_with("<builtin:"));
        let original_source_file = source_files[0];

        for source_file in source_files {
            if builtin_source_file.is_some() && source_file.path.as_str().starts_with("<builtin:") {
                continue;
            }

            compilation_context.add_diagnostic(module_name_conflict_diagnostic(
                source_file,
                builtin_source_file.unwrap_or(original_source_file),
                module_name.as_str(),
            ));
        }
    }
}

fn validate_loaded_module(module: &ParsedModule, compilation_context: &mut CompilationContext) {
    if module.kind.allows_top_level_statements() {
        return;
    }

    let Some(statement) = module.compilation_unit.statements().next() else {
        return;
    };

    compilation_context.add_diagnostic(module_statement_diagnostic(
        &module.source_file,
        statement.span.clone(),
    ));
}

fn module_name_conflict_diagnostic(
    source_file: &SourceFile,
    original_source_file: &SourceFile,
    module_name: &str,
) -> SourceDiagnostic {
    let message = if original_source_file.path.as_str().starts_with("<builtin:") {
        format!("module name `{module_name}` is reserved for a builtin module")
    } else {
        format!("module name `{module_name}` is already defined")
    };

    SourceDiagnostic::new(DiagnosticLevel::Error, &source_file.path, message).with_excerpt(
        source_excerpt_for_path(source_file, "rename this file or module path"),
    )
}

fn module_statement_diagnostic(source_file: &SourceFile, span: Span) -> SourceDiagnostic {
    let line_bounds = LineBounds::new(source_file.source(), span.start());
    let source_line = &source_file.source()[line_bounds.line_start..line_bounds.line_end];
    let relative_start = span.start().saturating_sub(line_bounds.line_start);
    let relative_end = span.end().saturating_sub(line_bounds.line_start);

    SourceDiagnostic::new(
        DiagnosticLevel::Error,
        &source_file.path,
        "top-level statements are only allowed in `.ocelot-script` files",
    )
    .with_excerpt(
        SourceExcerpt::new(&source_file.path, line_bounds.line_number, source_line)
            .with_annotation(SourceAnnotation::new(
                Span::new(relative_start, relative_end),
                "move this statement into `main()` or rename the file to `.ocelot-script`",
            )),
    )
}

fn source_excerpt_for_path(
    source_file: &SourceFile,
    annotation: impl Into<SharedString>,
) -> SourceExcerpt {
    let annotation = annotation.into();
    let source_line = source_file.source().lines().next().unwrap_or_default();

    SourceExcerpt::new(&source_file.path, 1, source_line)
        .with_annotation(SourceAnnotation::new(Span::new(0, 0), annotation))
}
