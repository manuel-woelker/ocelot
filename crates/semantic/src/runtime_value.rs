use ocelot_base::result::OcelotResult;
use ocelot_base::shared_string::SharedString;

/* 📖 # Why keep runtime values in `ocelot_semantic`?
Runtime values are no longer just an interpreter concern. Native function
dispatch and semantic function definitions both need a shared value type, but
that type does not belong in the syntax tree.

Keeping it in the semantic crate keeps `ocelot_ast` focused on syntax while
still giving the interpreter and native functions one shared representation.
*/

/// Runtime value used by the tree-walking interpreter and native functions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeValue {
    Boolean(bool),
    String(SharedString),
    Unit,
}

impl RuntimeValue {
    /// Creates a boolean runtime value.
    pub const fn boolean(value: bool) -> Self {
        Self::Boolean(value)
    }

    /// Creates a string runtime value.
    pub fn string(value: impl Into<SharedString>) -> Self {
        Self::String(value.into())
    }

    /// Creates a unit runtime value.
    pub const fn unit() -> Self {
        Self::Unit
    }

    /// Returns the inner string when this is a string value.
    pub fn as_string(&self) -> Option<&SharedString> {
        match self {
            Self::String(value) => Some(value),
            Self::Boolean(_) | Self::Unit => None,
        }
    }

    /// Returns the inner boolean when this is a boolean value.
    pub const fn as_boolean(&self) -> Option<bool> {
        match self {
            Self::Boolean(value) => Some(*value),
            Self::Unit => None,
            Self::String(_) => None,
        }
    }

    /// Returns true when this value is unit.
    pub fn is_unit(&self) -> bool {
        matches!(self, Self::Unit)
    }

    /// Returns a stable user-facing rendering for printed output.
    pub fn render_for_display(&self) -> SharedString {
        match self {
            Self::Boolean(value) => SharedString::from(if *value { "true" } else { "false" }),
            Self::String(value) => value.clone(),
            Self::Unit => SharedString::from("()"),
        }
    }

    /// Returns the inner string or a user-facing type error.
    pub fn expect_string(&self, message: impl AsRef<str>) -> OcelotResult<&SharedString> {
        match self {
            Self::String(value) => Ok(value),
            Self::Boolean(_) | Self::Unit => ocelot_base::bail!("{}", message.as_ref()),
        }
    }

    /// Returns the inner boolean or a user-facing type error.
    pub fn expect_boolean(&self, message: impl AsRef<str>) -> OcelotResult<bool> {
        match self {
            Self::Boolean(value) => Ok(*value),
            Self::String(_) | Self::Unit => ocelot_base::bail!("{}", message.as_ref()),
        }
    }

    /// Returns true when both runtime values compare equal.
    pub fn equals(&self, other: &Self) -> bool {
        self == other
    }

    /// Returns a stable user-facing rendering for assertions.
    pub fn render_for_assertion(&self) -> SharedString {
        match self {
            Self::Boolean(value) => SharedString::from(if *value { "true" } else { "false" }),
            Self::String(value) => SharedString::from(format!("\"{}\"", value)),
            Self::Unit => SharedString::from("()"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RuntimeValue;

    #[test]
    fn string_constructor_builds_a_string_value() {
        let value = RuntimeValue::string("hello");

        assert_eq!(
            value.as_string().map(|string| string.as_str()),
            Some("hello")
        );
        assert!(!value.is_unit());
    }

    #[test]
    fn boolean_constructor_builds_a_boolean_value() {
        let value = RuntimeValue::boolean(true);

        assert_eq!(value.as_boolean(), Some(true));
        assert_eq!(value.as_string(), None);
        assert!(!value.is_unit());
    }

    #[test]
    fn unit_constructor_builds_a_unit_value() {
        let value = RuntimeValue::unit();

        assert!(value.is_unit());
        assert_eq!(value.as_string(), None);
    }

    #[test]
    fn expect_string_returns_the_inner_string() {
        let value = RuntimeValue::string("hello");

        let text = value.expect_string("expected string").unwrap();

        assert_eq!(text, "hello");
    }

    #[test]
    fn expect_string_returns_the_given_error_for_non_strings() {
        let value = RuntimeValue::unit();

        let error = value
            .expect_string("type error: expected string")
            .unwrap_err();

        assert!(
            error
                .to_test_string()
                .contains("type error: expected string")
        );
    }

    #[test]
    fn expect_boolean_returns_the_inner_boolean() {
        let value = RuntimeValue::boolean(true);

        let actual = value.expect_boolean("expected bool").unwrap();

        assert!(actual);
    }

    #[test]
    fn expect_boolean_returns_the_given_error_for_non_booleans() {
        let value = RuntimeValue::string("hello");

        let error = value
            .expect_boolean("type error: expected bool")
            .unwrap_err();

        assert!(error.to_test_string().contains("type error: expected bool"));
    }

    #[test]
    fn render_for_assertion_quotes_strings() {
        let value = RuntimeValue::string("hello");

        assert_eq!(value.render_for_assertion(), "\"hello\"");
    }

    #[test]
    fn render_for_assertion_renders_booleans_as_source_literals() {
        let value = RuntimeValue::boolean(false);

        assert_eq!(value.render_for_assertion(), "false");
    }

    #[test]
    fn render_for_display_renders_booleans_and_strings() {
        assert_eq!(RuntimeValue::boolean(true).render_for_display(), "true");
        assert_eq!(RuntimeValue::string("hello").render_for_display(), "hello");
    }

    #[test]
    fn equals_compares_runtime_values() {
        assert!(RuntimeValue::string("hello").equals(&RuntimeValue::string("hello")));
        assert!(RuntimeValue::boolean(true).equals(&RuntimeValue::boolean(true)));
        assert!(!RuntimeValue::boolean(true).equals(&RuntimeValue::boolean(false)));
        assert!(!RuntimeValue::string("hello").equals(&RuntimeValue::unit()));
    }
}
