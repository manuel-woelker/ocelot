use crate::builtin_module::BuiltinModule;
use crate::module_name_from_path::module_name_from_path;
use ocelot_base::compilation_stage::CompilationStage;
use ocelot_base::error::OcelotError;
use ocelot_base::file_path::FilePath;
use ocelot_base::result::OcelotResult;
use ocelot_base::source_diagnostics::SourceDiagnostics;
use ocelot_base::source_file::SourceFile;
use ocelot_pal::pal::Pal;
use ocelot_semantic::compilation_inputs::CompilationInputs;
use ocelot_semantic::parsed_module::ParsedModule;
use ocelot_semantic::resolved_program::ResolvedProgram;
use ocelot_semantic::source_file_kind::SourceFileKind;

const RESERVED_BUILTIN_SOURCE_DIRECTORY: &str = "crates/engine/resources/builtin";

/// Compiles one program from disk-backed and synthetic source files.
pub fn compile_program(
    pal: &dyn Pal,
    base_path: &FilePath,
    entry_path: &FilePath,
    builtin_modules: &[BuiltinModule],
    synthetic_source_files: &[SourceFile],
    source_diagnostics: &mut SourceDiagnostics,
) -> OcelotResult<ResolvedProgram> {
    *source_diagnostics = SourceDiagnostics::default();

    let mut parsed_modules = parse_disk_modules(
        pal,
        base_path,
        entry_path,
        synthetic_source_files,
        source_diagnostics,
    )?;
    for source_file in synthetic_source_files {
        if let Some(module) = parse_source_file(base_path, source_file.clone(), source_diagnostics)?
        {
            parsed_modules.push(module);
        }
    }

    for builtin_module in builtin_modules {
        if let Some(module) = parse_builtin_module(builtin_module.clone(), source_diagnostics)? {
            parsed_modules.push(module);
        }
    }

    if source_diagnostics.has_errors() {
        return Err(OcelotError::compilation_error(CompilationStage::Parser));
    }

    let resolved_program = ocelot_resolver::resolution::resolve_program(
        entry_path.clone(),
        parsed_modules,
        &CompilationInputs::with_default_native_functions(),
    )?;
    *source_diagnostics = resolved_program.source_diagnostics.clone();

    if source_diagnostics.has_errors() {
        return Err(OcelotError::compilation_error(CompilationStage::Resolver));
    }

    Ok(resolved_program)
}

fn parse_disk_modules(
    pal: &dyn Pal,
    base_path: &FilePath,
    entry_path: &FilePath,
    synthetic_source_files: &[SourceFile],
    source_diagnostics: &mut SourceDiagnostics,
) -> OcelotResult<Vec<ParsedModule>> {
    let mut file_paths = pal
        .walk_directory(base_path, &[String::from("*.ocelot")])?
        .collect::<OcelotResult<Vec<_>>>()?;
    file_paths.retain(|path| !is_reserved_builtin_source_path(path));
    let synthetic_paths = synthetic_source_files
        .iter()
        .map(|source_file| &source_file.path)
        .collect::<Vec<_>>();
    if !file_paths.contains(entry_path) && !synthetic_paths.contains(&entry_path) {
        file_paths.push(entry_path.clone());
    }
    file_paths.sort();

    let mut parsed_modules = Vec::new();
    for path in file_paths {
        let source = pal.read_file_to_string(&path)?;
        let source_file = SourceFile::new(path, source);
        if let Some(module) = parse_source_file(base_path, source_file, source_diagnostics)? {
            parsed_modules.push(module);
        }
    }

    Ok(parsed_modules)
}

fn parse_builtin_module(
    builtin_module: BuiltinModule,
    source_diagnostics: &mut SourceDiagnostics,
) -> OcelotResult<Option<ParsedModule>> {
    let source_file = SourceFile::new(builtin_module.source_file_path(), builtin_module.source);
    let compilation_unit = match ocelot_parser::parse_compilation_unit::parse_compilation_unit(
        &source_file,
        source_diagnostics,
    ) {
        Ok(compilation_unit) => compilation_unit,
        Err(error) if is_parser_compilation_error(&error) => return Ok(None),
        Err(error) => return Err(error),
    };

    Ok(Some(ParsedModule::new(
        builtin_module.module_name,
        SourceFileKind::Module,
        source_file,
        compilation_unit,
    )))
}

fn parse_source_file(
    base_path: &FilePath,
    source_file: SourceFile,
    source_diagnostics: &mut SourceDiagnostics,
) -> OcelotResult<Option<ParsedModule>> {
    let path = source_file.path.clone();
    let compilation_unit = match ocelot_parser::parse_compilation_unit::parse_compilation_unit(
        &source_file,
        source_diagnostics,
    ) {
        Ok(compilation_unit) => compilation_unit,
        Err(error) if is_parser_compilation_error(&error) => return Ok(None),
        Err(error) => return Err(error),
    };

    let source_kind = SourceFileKind::from_path(&path)?;
    let module_name = if base_path.as_str().is_empty() {
        path.file_stem().unwrap_or_default().into()
    } else {
        module_name_from_path(base_path, &path)?
    };

    Ok(Some(ParsedModule::new(
        module_name,
        source_kind,
        source_file,
        compilation_unit,
    )))
}

fn is_parser_compilation_error(error: &OcelotError) -> bool {
    matches!(
        error.kind(),
        ocelot_base::error::ErrorKind::CompilationError(CompilationStage::Parser)
    )
}

fn is_reserved_builtin_source_path(path: &FilePath) -> bool {
    path.as_path()
        .starts_with(FilePath::from(RESERVED_BUILTIN_SOURCE_DIRECTORY).as_path())
}

#[cfg(test)]
mod tests {
    use super::is_reserved_builtin_source_path;
    use ocelot_base::file_path::FilePath;

    #[test]
    fn recognizes_the_reserved_builtin_source_directory() {
        assert!(is_reserved_builtin_source_path(&FilePath::from(
            "crates/engine/resources/builtin/core.ocelot"
        )));
        assert!(!is_reserved_builtin_source_path(&FilePath::from(
            "crates/engine/resources/core.ocelot"
        )));
        assert!(!is_reserved_builtin_source_path(&FilePath::from(
            "examples/core.ocelot"
        )));
    }
}
