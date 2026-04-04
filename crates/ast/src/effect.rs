use ocelot_base::shared_string::SharedString;
use ocelot_base::source_file::SourceFile;
use ocelot_base::span::Span;

/// One nominal effect definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Effect {
    pub name: SharedString,
    pub is_builtin: bool,
    pub declaration_span: Option<Span>,
    pub source_file: Option<Box<SourceFile>>,
}

impl Effect {
    /// Creates one builtin effect definition.
    pub fn builtin(name: impl Into<SharedString>) -> Self {
        Self {
            name: name.into(),
            is_builtin: true,
            declaration_span: None,
            source_file: None,
        }
    }

    /// Creates one user-declared effect definition.
    pub fn declared(
        name: impl Into<SharedString>,
        declaration_span: Span,
        source_file: SourceFile,
    ) -> Self {
        Self {
            name: name.into(),
            is_builtin: false,
            declaration_span: Some(declaration_span),
            source_file: Some(Box::new(source_file)),
        }
    }
}
