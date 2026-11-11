use ocelot_ast::equality_operator::EqualityOperator;
use ocelot_ast::expression::Expression;
use ocelot_ast::expression_kind::ExpressionKind;
use ocelot_ast::statement::Statement;
use ocelot_ast::statement_kind::StatementKind;
use ocelot_ast::template_string_part::TemplateStringPart;
use ocelot_base::assertion_error::{AssertionError, render_assertion_error};
use ocelot_base::result::OcelotResult;
use ocelot_base::shared_string::SharedString;
use ocelot_base::source_file::SourceFile;
use ocelot_semantic::function_kind::FunctionKind;
use ocelot_semantic::resolved_program::ResolvedProgram;
use std::collections::HashMap;

/// Stateful C99 emitter for one resolved snapshot and its lexical environments.
pub struct CEmitter<'a> {
    /// Canonical checked snapshot; never re-resolved by the emitter.
    program: &'a ResolvedProgram,
    /// Unique namespace when several snapshots share a scenario translation unit.
    namespace: SharedString,
    /// Generated statements and definitions.
    output: String,
    /// Resolved lexical names mapped to generated C values.
    scopes: Vec<HashMap<SharedString, SharedString>>,
    /// Deterministic temporary counter.
    next: usize,
}

/// Returns the self-contained C runtime, ABI declarations, and frame cleanup helpers.
pub fn runtime_source() -> &'static str {
    concat!(
        include_str!("native_value.h"),
        include_str!("native_string.h"),
        include_str!("native_bytes.h"),
        include_str!("native_io.c"),
        include_str!("native_api.c"),
        include_str!("runtime.h")
    )
}

impl<'a> CEmitter<'a> {
    /// Creates an emitter for one snapshot, without toolchain-specific settings.
    pub fn new(program: &'a ResolvedProgram, namespace: impl Into<SharedString>) -> Self {
        Self {
            program,
            namespace: namespace.into(),
            output: String::new(),
            scopes: vec![HashMap::new()],
            next: 0,
        }
    }

    fn line(&mut self, text: impl AsRef<str>) {
        self.output.push_str(text.as_ref());
        self.output.push('\n');
    }
    fn fresh(&mut self) -> SharedString {
        let name = format!("v{}", self.next).into();
        self.next += 1;
        name
    }
    fn function_name(&self, index: usize) -> String {
        format!("oc_{}_f{}", self.namespace, index)
    }
    fn value(&mut self, expression: impl AsRef<str>) -> SharedString {
        let name = self.fresh();
        self.line(format!(
            "OcValue {name} = oc_track(&frame, {}); (void){name};",
            expression.as_ref()
        ));
        name
    }
    fn string(&mut self, text: &str) -> SharedString {
        let bytes = text
            .bytes()
            .map(|b| format!("\\{:03o}", b))
            .collect::<String>();
        self.value(format!(
            "oc_string((const unsigned char *)\"{bytes}\", {})",
            text.len()
        ))
    }
    fn return_value(&mut self, value: &str) {
        let result = self.fresh();
        self.line(format!(
            "OcValue {result} = oc_keep({value}); oc_drop(&frame); return {result};"
        ));
    }

    /// Emits function definitions and a step entry, with explicit persistent root slots.
    /// Roots are names in declaration order across successful scenario submissions.
    pub fn emit_step(
        mut self,
        statements: &[Statement],
        source: &SourceFile,
        initial_roots: &[SharedString],
        roots: &[SharedString],
        eval: Option<&Expression>,
    ) -> OcelotResult<String> {
        for (index, definition) in self.program.symbol_table.functions.iter().enumerate() {
            if let Some(definition) = definition
                && matches!(definition.kind, FunctionKind::UserDefined { .. })
            {
                let args = (0..definition.parameters.len())
                    .map(|i| format!("OcValue p{i}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                self.line(format!(
                    "OcValue {}({});",
                    self.function_name(index),
                    if args.is_empty() { "void" } else { &args }
                ));
            }
        }
        for (index, definition) in self.program.symbol_table.functions.iter().enumerate() {
            let Some(definition) = definition else {
                continue;
            };
            if let FunctionKind::UserDefined {
                function,
                source_file,
            } = &definition.kind
            {
                let args = (0..function.parameters.len())
                    .map(|i| format!("OcValue p{i}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                self.line(format!(
                    "OcValue {}({}) {{ OcFrame frame = {{0}};",
                    self.function_name(index),
                    if args.is_empty() { "void" } else { &args }
                ));
                for i in 0..function.parameters.len() {
                    self.line(format!("(void)p{i};"));
                }
                self.scopes = vec![
                    function
                        .parameters
                        .iter()
                        .enumerate()
                        .map(|(i, p)| (p.identifier.name.clone(), format!("p{i}").into()))
                        .collect(),
                ];
                let result = self.statements(&function.body, source_file)?;
                self.return_value(&result);
                self.line("}");
            }
        }
        self.line(format!(
            "OcValue oc_{}_step(OcValue *roots) {{ OcFrame frame = {{0}}; (void)roots;",
            self.namespace
        ));
        self.scopes = vec![
            initial_roots
                .iter()
                .enumerate()
                .map(|(i, name)| (name.clone(), format!("roots[{i}]").into()))
                .collect(),
        ];
        let mut result = self.statements(statements, source)?;
        if let Some(eval) = eval {
            result = self.expression(eval, source)?;
        }
        for (index, name) in roots.iter().enumerate() {
            let value = self.lookup(name)?;
            let retained = self.fresh();
            self.line(format!("OcValue {retained} = oc_keep({value}); oc_release(roots[{index}]); roots[{index}] = {retained};"));
        }
        self.return_value(&result);
        self.line("}");
        Ok(self.output)
    }

    fn lookup(&self, name: &str) -> OcelotResult<SharedString> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).cloned())
            .ok_or_else(|| {
                ocelot_base::error::OcelotError::message(format!(
                    "C backend: missing resolved value `{name}`"
                ))
            })
    }

    fn statements(
        &mut self,
        statements: &[Statement],
        source: &SourceFile,
    ) -> OcelotResult<SharedString> {
        let mut result = SharedString::from("oc_unit()");
        for statement in statements {
            match &statement.kind {
                StatementKind::Let(binding) => {
                    let value = self.expression(&binding.initializer, source)?;
                    self.scopes
                        .last_mut()
                        .expect("scope")
                        .insert(binding.identifier.name.clone(), value);
                    result = "oc_unit()".into();
                }
                StatementKind::Return(value) => {
                    let value = if let Some(expr) = &value.expression {
                        self.expression(expr, source)?
                    } else {
                        "oc_unit()".into()
                    };
                    self.return_value(&value);
                    result = "oc_unit()".into();
                }
                StatementKind::Expression(value) => {
                    let expr = self.expression(&value.expression, source)?;
                    result = if value.has_semicolon {
                        "oc_unit()".into()
                    } else {
                        expr
                    };
                }
            }
        }
        Ok(result)
    }

    fn expression(
        &mut self,
        expression: &Expression,
        source: &SourceFile,
    ) -> OcelotResult<SharedString> {
        Ok(match &expression.kind {
            ExpressionKind::BooleanLiteral(value) => {
                self.value(format!("oc_boolean({})", i32::from(value.value)))
            }
            ExpressionKind::StringLiteral(value) => self.string(&value.value),
            ExpressionKind::Identifier(identifier) => {
                self.value(format!("oc_keep({})", self.lookup(&identifier.name)?))
            }
            ExpressionKind::Not(value) => {
                let operand = self.expression(&value.operand, source)?;
                self.value(format!("oc_boolean(!{operand}.payload)"))
            }
            ExpressionKind::Equality(value) => {
                let left = self.expression(&value.left, source)?;
                let right = self.expression(&value.right, source)?;
                let not = if value.operator == EqualityOperator::NotEqual {
                    "!"
                } else {
                    ""
                };
                self.value(format!("oc_boolean({not}oc_equal({left}, {right}))"))
            }
            ExpressionKind::Block(block) => {
                let mark = self.fresh();
                self.line(format!("size_t {mark} = frame.len;"));
                self.scopes.push(HashMap::new());
                let result = self.statements(&block.statements, source)?;
                self.scopes.pop();
                let saved = self.fresh();
                self.line(format!(
                    "OcValue {saved} = oc_keep({result}); oc_clear(&frame, {mark});"
                ));
                self.value(saved)
            }
            ExpressionKind::If(value) => {
                let condition = self.expression(&value.condition, source)?;
                let result = self.fresh();
                self.line(format!(
                    "OcValue {result} = oc_unit(); if ({condition}.payload) {{"
                ));
                let branch = self.expression(&value.then_branch, source)?;
                self.line(format!("{result} = oc_keep({branch}); }} else {{"));
                if let Some(branch) = &value.else_branch {
                    let branch = self.expression(branch, source)?;
                    self.line(format!("{result} = oc_keep({branch});"));
                }
                self.line("}");
                self.value(result)
            }
            ExpressionKind::TemplateString(template) => {
                let mut result = self.string("");
                for part in &template.parts {
                    let value = match part {
                        TemplateStringPart::Text(text) => self.string(text),
                        TemplateStringPart::Interpolation(expr) => self.expression(expr, source)?,
                    };
                    result = self.value(format!("oc_concat({result}, {value})"));
                }
                result
            }
            ExpressionKind::Call(call) => {
                let index = call.function_index()?;
                let definition = self.program.symbol_table.function_definition(index)?;
                let mut args = Vec::new();
                for argument in &call.arguments {
                    args.push(self.expression(&argument.value, source)?);
                }
                match &definition.kind {
                    FunctionKind::UserDefined { .. } => self.value(format!(
                        "{}({})",
                        self.function_name(index.as_usize()),
                        args.iter()
                            .map(|s| s.as_str())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )),
                    FunctionKind::NativeFunction { .. } => match definition.name.as_str() {
                        "core::println" => self.value(format!("oc_print({})", args[0])),
                        "core::assert" | "core::assert_eq" => {
                            let is_equal = definition.name == "core::assert_eq";
                            let message = if is_equal {
                                "assert_eq values differ"
                            } else {
                                "assert condition was false"
                            };
                            let diagnostic = ocelot_base::unansi(&render_assertion_error(
                                &AssertionError::new_without_diff(
                                    source,
                                    expression.span.clone(),
                                    message,
                                ),
                            ));
                            let diagnostic = self.string(&diagnostic);
                            args.push(diagnostic);
                            self.value(format!(
                                "{}({})",
                                if is_equal {
                                    "oc_assert_eq"
                                } else {
                                    "oc_assert"
                                },
                                args.iter()
                                    .map(|s| s.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ))
                        }
                        name => ocelot_base::bail!(
                            "C backend does not support native function `{name}`"
                        ),
                    },
                }
            }
            ExpressionKind::QualifiedIdentifier(_) => {
                ocelot_base::bail!("C backend received an unresolved value")
            }
        })
    }
}
