use ocelot_base::result::OcelotResult;
use ocelot_base::shared_string::SharedString;

/* 📖 # Why keep interpreter values as a Rust enum for now?
The current interpreter is still a simple tree-walking implementation with
strings as the only real data payload. A plain enum keeps the code easy to
read, test, and extend while hiding representation details inside one module.

If profiling later shows that value tagging is a real hotspot in a lower-level
runtime, the internal representation can change without forcing a broad
interpreter rewrite. Until then, NaN-boxing would be clever in all the wrong
ways.
*/

/// Runtime value used by the tree-walking interpreter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeValue {
    String(SharedString),
    Unit,
}

impl RuntimeValue {
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
            Self::Unit => None,
        }
    }

    /// Returns true when this value is unit.
    pub fn is_unit(&self) -> bool {
        matches!(self, Self::Unit)
    }

    /// Returns the inner string or a user-facing type error.
    pub fn expect_string(&self, message: impl AsRef<str>) -> OcelotResult<&SharedString> {
        match self {
            Self::String(value) => Ok(value),
            Self::Unit => ocelot_base::bail!("{}", message.as_ref()),
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
}
