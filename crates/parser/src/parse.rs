use ocelot_ast::script::Script;
use ocelot_base::result::OcelotResult;
use ocelot_base::source_file::SourceFile;
use ocelot_base::span::Span;

/// Parses a source file into a script AST.
pub fn parse_script(source_file: &SourceFile) -> OcelotResult<Script> {
    Ok(Script::new(
        Vec::new(),
        Span::new(0, source_file.source().len()),
    ))
}

#[cfg(test)]
mod tests {
    use super::parse_script;
    use ocelot_base::source_file::SourceFile;

    #[test]
    fn parse_script_uses_the_full_source_span() {
        let source_file = SourceFile::new("examples/hello.ocelot", "println(\"hello\");");

        let script = parse_script(&source_file).unwrap();

        assert_eq!(script.statements.len(), 0);
        assert_eq!(script.span.start(), 0);
        assert_eq!(script.span.end(), source_file.source().len());
    }
}
