use crate::declaration_indexer::DeclarationIndexer;
use crate::effect_propagation::propagate_function_effects;
use crate::resolver::Resolver;
use ocelot_ast::function_item::FunctionItem;
use ocelot_ast::identifier::Identifier;
use ocelot_ast::script::Script;
use ocelot_base::compilation_context::CompilationContext;
use ocelot_base::compilation_stage::CompilationStage;
use ocelot_base::error::OcelotError;
use ocelot_base::file_path::FilePath;
use ocelot_base::render_source_diagnostics::render_source_diagnostics;
use ocelot_base::result::OcelotResult;
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_file::SourceFile;
use ocelot_base::span::Span;
use ocelot_semantic::compilation_session::CompilationSession;
use ocelot_semantic::function_kind::FunctionKind;
use ocelot_semantic::module_environment::ModuleEnvironment;
use ocelot_semantic::program_environment::ProgramEnvironment;
use ocelot_semantic::program_index::ProgramIndex;
use ocelot_semantic::resolved_function::ResolvedFunction;
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
    compilation_session: &CompilationSession,
) -> OcelotResult<()> {
    register_core_module(compilation_context, environment, compilation_session)?;
    let module_name = default_module_name(source_file);
    let mut module_environment = ModuleEnvironment::new();
    environment.add_module(module_name.clone());
    register_module_effects(script, source_file, compilation_context, environment)?;
    register_module_functions(
        script,
        &module_name,
        source_file,
        compilation_context,
        environment,
        &mut module_environment,
        compilation_session,
    )?;
    register_module_imports(
        script,
        &module_name,
        source_file,
        compilation_context,
        environment,
        &mut module_environment,
    )?;
    let program_index = ProgramIndex::from_environment(environment);
    resolve_module_items(
        script,
        &module_name,
        source_file,
        compilation_context,
        &program_index,
        &module_environment,
        compilation_session,
    )?;
    let resolved_functions = resolve_user_defined_function_definitions(
        compilation_context,
        &program_index,
        &HashMap::from([(source_file.path.clone(), module_environment)]),
        compilation_session,
    )?;
    environment.apply_resolved_functions(resolved_functions)?;
    finish_resolution(compilation_context)
}

/// Registers the compiler-provided `core` module into one program environment.
pub fn register_core_module(
    compilation_context: &mut CompilationContext,
    environment: &mut ProgramEnvironment,
    compilation_session: &CompilationSession,
) -> OcelotResult<()> {
    if environment.has_module(CORE_MODULE_NAME) {
        return Ok(());
    }

    let source_file = SourceFile::new(CORE_MODULE_PATH, CORE_MODULE_SOURCE);
    let mut script = ocelot_parser::parse_script::parse_script(&source_file, compilation_context)?;
    let mut module_environment = ModuleEnvironment::new();

    environment.add_module(CORE_MODULE_NAME);
    register_module_effects(&mut script, &source_file, compilation_context, environment)?;
    register_module_functions(
        &mut script,
        CORE_MODULE_NAME,
        &source_file,
        compilation_context,
        environment,
        &mut module_environment,
        compilation_session,
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
    let compilation_session = CompilationSession::new();
    let mut module_environment = ModuleEnvironment::new();
    DeclarationIndexer::new(
        source_file,
        "",
        compilation_context,
        environment,
        &mut module_environment,
        &compilation_session,
    )
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
    module_environment: &mut ModuleEnvironment,
    compilation_session: &CompilationSession,
) -> OcelotResult<()> {
    DeclarationIndexer::new(
        source_file,
        module_name,
        compilation_context,
        environment,
        module_environment,
        compilation_session,
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
    module_environment: &mut ModuleEnvironment,
) -> OcelotResult<()> {
    let compilation_session = CompilationSession::new();
    DeclarationIndexer::new(
        source_file,
        module_name,
        compilation_context,
        environment,
        module_environment,
        &compilation_session,
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
    program_index: &ProgramIndex,
    module_environment: &ModuleEnvironment,
    _compilation_session: &CompilationSession,
) -> OcelotResult<()> {
    let mut resolver = Resolver::new(
        source_file,
        module_name,
        compilation_context,
        program_index,
        module_environment,
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
    program_index: &ProgramIndex,
    module_environments: &HashMap<FilePath, ModuleEnvironment>,
    _compilation_session: &CompilationSession,
) -> OcelotResult<Vec<ResolvedFunction>> {
    let function_indices = program_index.user_defined_function_indices();
    let mut resolved_functions = Vec::with_capacity(function_indices.len());

    for function_index in function_indices {
        let (module_name, source_file) = {
            let function_definition = program_index.function_definition(function_index)?;
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

        let function_definition = program_index.function_definition(function_index)?;
        let FunctionKind::UserDefined { function, .. } = &function_definition.kind else {
            ocelot_base::bail!(
                "internal error: function index did not reference a user-defined function"
            );
        };
        let module_environment = module_environments
            .get(&source_file.path)
            .expect("module environment should exist for resolved function source file");
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
            program_index,
            module_environment,
            Some(&mut resolved_function),
        )
        .resolve_function_item(&mut function);
        resolved_function.function = function;
        resolved_functions.push(resolved_function);
    }

    propagate_function_effects(compilation_context, program_index, &mut resolved_functions)?;
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
