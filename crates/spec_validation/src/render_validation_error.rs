use ocelot_base::error::ErrorKind;
use ocelot_base::error::OcelotError;
use ocelot_base::unansi;

/// Renders an engine error into stable text for spec comparison.
pub fn render_validation_error(error: &OcelotError) -> String {
    if matches!(error.kind(), ErrorKind::CompilationError(_))
        && let Some(source) = error.source()
    {
        return unansi(&source.kind().to_string());
    }

    let mut rendered = error.kind().to_string();
    let mut current = error.source();

    while let Some(cause) = current {
        rendered.push('\n');
        rendered.push_str("caused by: ");
        rendered.push_str(&cause.kind().to_string());
        current = cause.source();
    }

    rendered
}

#[cfg(test)]
mod tests {
    use super::render_validation_error;
    use ocelot_base::compilation_stage::CompilationStage;
    use ocelot_base::error::OcelotError;

    #[test]
    fn renders_cause_chain_without_locations() {
        let error = OcelotError::message("outer").with_source(OcelotError::message("inner"));
        assert_eq!(render_validation_error(&error), "outer\ncaused by: inner");
    }

    #[test]
    fn unwraps_compilation_error_to_full_rendered_diagnostic() {
        let error = OcelotError::compilation_error(CompilationStage::Parser).with_source(
            OcelotError::message(
                "\u{1b}[1m\u{1b}[91merror\u{1b}[0m\u{1b}[1m: type error: `println` expects exactly one argument\u{1b}[0m\n  \u{1b}[1m\u{1b}[94m╭▸ \u{1b}[0mexamples/test.ocelot:1:1\nat examples/test.ocelot:1:1",
            ),
        );

        assert_eq!(
            render_validation_error(&error),
            "error: type error: `println` expects exactly one argument\n  ╭▸ examples/test.ocelot:1:1\nat examples/test.ocelot:1:1"
        );
    }
}
