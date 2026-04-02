use ocelot_base::error::OcelotError;

/// How should an engine error be rendered for stable spec comparison?
pub fn render_validation_error(error: &OcelotError) -> String {
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
    use ocelot_base::error::OcelotError;

    #[test]
    fn renders_cause_chain_without_locations() {
        let error = OcelotError::message("outer").with_source(OcelotError::message("inner"));
        assert_eq!(render_validation_error(&error), "outer\ncaused by: inner");
    }
}
