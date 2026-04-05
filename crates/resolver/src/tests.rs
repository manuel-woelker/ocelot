use crate::resolution::finish_resolution;
use crate::resolution::register_core_module;
use crate::resolution::register_module_effects;
use crate::resolution::register_module_functions;
use crate::resolution::register_module_imports;
use crate::resolution::resolve;
use crate::resolution::resolve_module_items;
use crate::resolution::resolve_program;
use crate::resolution::resolve_user_defined_function_definitions;
use ocelot_ast::call_expression::CallExpression;
use ocelot_ast::compilation_unit::CompilationUnit;
use ocelot_ast::effect_item::EffectItem;
use ocelot_ast::expression::Expression;
use ocelot_ast::expression_kind::ExpressionKind;
use ocelot_ast::expression_statement::ExpressionStatement;
use ocelot_ast::function_effect_clause::FunctionEffectClause;
use ocelot_ast::function_item::FunctionItem;
use ocelot_ast::function_parameter::FunctionParameter;
use ocelot_ast::identifier::Identifier;
use ocelot_ast::item::Item;
use ocelot_ast::item_kind::ItemKind;
use ocelot_ast::qualified_identifier::QualifiedIdentifier;
use ocelot_ast::statement::Statement;
use ocelot_ast::statement_kind::StatementKind;
use ocelot_ast::string_literal_expression::StringLiteralExpression;
use ocelot_ast::test_item::TestItem;
use ocelot_ast::type_index::TypeIndex;
use ocelot_ast::use_item::UseItem;
use ocelot_base::compilation_stage::CompilationStage;
use ocelot_base::source_file::SourceFile;
use ocelot_base::span::Span;
use ocelot_semantic::compilation_context::CompilationContext;
use ocelot_semantic::compilation_session::CompilationSession;
use ocelot_semantic::function_kind::FunctionKind;
use ocelot_semantic::module_environment::ModuleEnvironment;
use ocelot_semantic::parsed_module::ParsedModule;
use ocelot_semantic::program_environment::ProgramEnvironment;
use ocelot_semantic::source_file_kind::SourceFileKind;
use ocelot_semantic::symbol_table::SymbolTable;
use std::collections::HashMap;

fn identifier(name: &str, span: Span) -> Expression {
    Expression::new(
        ExpressionKind::Identifier(Identifier::new(name, span.clone())),
        span,
    )
}

fn qualified_identifier(names: &[&str], spans: &[Span]) -> Expression {
    Expression::new(
        ExpressionKind::QualifiedIdentifier(QualifiedIdentifier::new(
            names
                .iter()
                .zip(spans.iter())
                .map(|(name, span)| Identifier::new(*name, span.clone()))
                .collect(),
        )),
        Span::new(spans[0].start(), spans[spans.len() - 1].end()),
    )
}

fn string_literal(value: &str, span: Span) -> Expression {
    Expression::new(
        ExpressionKind::StringLiteral(StringLiteralExpression::new(value)),
        span,
    )
}

fn call(callee: Expression, arguments: Vec<Expression>, span: Span) -> Expression {
    Expression::new(
        ExpressionKind::Call(CallExpression::new(callee, arguments)),
        span,
    )
}

fn effect_clause(name: &str, span: Span) -> FunctionEffectClause {
    FunctionEffectClause::new(Identifier::new(name, span.clone()), span)
}

fn parameter(name: &str, type_name: &str, span: Span) -> FunctionParameter {
    FunctionParameter::new(
        Identifier::new(name, Span::new(span.start(), span.start() + name.len())),
        Identifier::new(
            type_name,
            Span::new(span.end() - type_name.len(), span.end()),
        ),
        span,
    )
}

fn compilation_session() -> CompilationSession {
    CompilationSession::with_default_native_functions()
}

fn create_module_environment() -> ModuleEnvironment {
    ModuleEnvironment::new()
}

fn create_symbol_table() -> SymbolTable {
    SymbolTable::new()
}

fn parse_module(module_name: &str, path: &str, kind: SourceFileKind, source: &str) -> ParsedModule {
    let source_file = SourceFile::new(path, source);
    let compilation_unit = ocelot_parser::parse_compilation_unit::parse_compilation_unit(
        &source_file,
        &mut Default::default(),
    )
    .unwrap();

    ParsedModule::new(module_name, kind, source_file, compilation_unit)
}

#[test]
fn resolves_native_call_expressions() {
    let mut script = CompilationUnit::new(
        vec![Item::new(
            ItemKind::Statement(Statement::new(
                StatementKind::Expression(ExpressionStatement::new(call(
                    identifier("println", Span::new(0, 7)),
                    vec![string_literal("hello", Span::new(8, 15))],
                    Span::new(0, 16),
                ))),
                Span::new(0, 17),
            )),
            Span::new(0, 17),
        )],
        Span::new(0, 17),
    );
    let source_file = SourceFile::new("examples/hello.ocelot", "println(\"hello\");");
    let mut symbol_table = create_symbol_table();
    let compilation_session = compilation_session();
    let println_index = {
        register_core_module(
            &mut CompilationContext::default(),
            &mut symbol_table,
            &compilation_session,
        )
        .unwrap();
        symbol_table
            .resolve_function_exact("core::println")
            .unwrap()
    };
    let mut context = CompilationContext::default();
    let mut environment = ProgramEnvironment::new();

    resolve(
        &mut script,
        &source_file,
        &mut context,
        &mut environment,
        &compilation_session,
    )
    .unwrap();

    let ItemKind::Statement(statement) = &script.items[0].kind else {
        panic!("expected statement");
    };
    let StatementKind::Expression(ExpressionStatement { expression }) = &statement.kind;
    let ExpressionKind::Call(call_expression) = &expression.kind else {
        panic!("expected call expression");
    };
    assert_eq!(call_expression.function_index().unwrap(), println_index);
    assert_eq!(expression.ty, TypeIndex::unresolved());
}

#[test]
fn resolve_program_returns_resolved_modules_and_symbol_table() {
    let modules = vec![
        parse_module(
            "main",
            "examples/main.ocelot-script",
            SourceFileKind::Script,
            "use helper::greet;\ngreet();",
        ),
        parse_module(
            "helper",
            "examples/helper.ocelot",
            SourceFileKind::Module,
            "fun greet() { println(\"hello\"); }",
        ),
    ];

    let resolved_program = resolve_program(modules, &compilation_session()).unwrap();

    assert!(!resolved_program.source_diagnostics.has_errors());
    assert!(
        resolved_program
            .symbol_table
            .resolve_function_exact("helper::greet")
            .is_some()
    );

    let main_module = resolved_program
        .modules
        .iter()
        .find(|module| module.module_name == "main")
        .unwrap();
    let statement = main_module
        .compilation_unit
        .items
        .iter()
        .find_map(|item| match &item.kind {
            ItemKind::Statement(statement) => Some(statement),
            _ => None,
        })
        .expect("expected statement");
    let StatementKind::Expression(ExpressionStatement { expression }) = &statement.kind;
    let ExpressionKind::Call(call_expression) = &expression.kind else {
        panic!("expected call expression");
    };
    assert_eq!(
        call_expression.function_index().unwrap(),
        resolved_program
            .symbol_table
            .resolve_function_exact("helper::greet")
            .unwrap()
    );
}

#[test]
fn resolve_program_reports_builtin_module_name_conflicts() {
    let modules = vec![
        parse_module(
            "helpers",
            "examples/helpers.ocelot",
            SourceFileKind::Module,
            "fun greet() {}",
        ),
        parse_module(
            "helpers",
            "<builtin:helpers>",
            SourceFileKind::Module,
            "fun greet() { core::println(\"builtin\"); }",
        ),
    ];

    let resolved_program = resolve_program(modules, &compilation_session()).unwrap();

    assert!(resolved_program.source_diagnostics.has_errors());
    assert!(
        resolved_program
            .source_diagnostics
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message
                == "module name `helpers` is reserved for a builtin module")
    );
}

#[test]
fn resolves_parameter_references_inside_function_bodies() {
    let mut script = CompilationUnit::new(
        vec![
            Item::new(
                ItemKind::Function(FunctionItem::new(
                    Identifier::new("greet", Span::new(4, 9)),
                    vec![parameter("name", "string", Span::new(10, 22))],
                    None,
                    None,
                    vec![Statement::new(
                        StatementKind::Expression(ExpressionStatement::new(call(
                            identifier("println", Span::new(27, 34)),
                            vec![identifier("name", Span::new(35, 39))],
                            Span::new(27, 40),
                        ))),
                        Span::new(27, 41),
                    )],
                    Span::new(0, 43),
                )),
                Span::new(0, 43),
            ),
            Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call(
                        identifier("greet", Span::new(44, 49)),
                        vec![string_literal("hello", Span::new(50, 57))],
                        Span::new(44, 58),
                    ))),
                    Span::new(44, 59),
                )),
                Span::new(44, 59),
            ),
        ],
        Span::new(0, 59),
    );
    let source_file = SourceFile::new(
        "examples/functions.ocelot",
        "fun greet(name: string) { println(name); } greet(\"hello\");",
    );
    let mut environment = ProgramEnvironment::new();
    let compilation_session = compilation_session();
    let mut context = CompilationContext::default();

    resolve(
        &mut script,
        &source_file,
        &mut context,
        &mut environment,
        &compilation_session,
    )
    .unwrap();

    let function_definition = environment
        .function_definition(environment.resolve_function("functions::greet").unwrap())
        .unwrap();
    assert_eq!(
        function_definition.argument_types,
        vec![environment.string_type_index()]
    );

    let FunctionKind::UserDefined { function, .. } = &function_definition.kind else {
        panic!("expected user-defined function");
    };
    let StatementKind::Expression(ExpressionStatement { expression }) = &function.body[0].kind;
    let ExpressionKind::Call(call_expression) = &expression.kind else {
        panic!("expected call expression");
    };
    assert_eq!(
        call_expression.arguments[0].ty,
        environment.string_type_index()
    );
}

#[test]
fn reports_duplicate_function_parameter_names() {
    let mut script = CompilationUnit::new(
        vec![Item::new(
            ItemKind::Function(FunctionItem::new(
                Identifier::new("greet", Span::new(4, 9)),
                vec![
                    parameter("name", "string", Span::new(10, 22)),
                    parameter("name", "bool", Span::new(24, 34)),
                ],
                None,
                None,
                Vec::new(),
                Span::new(0, 38),
            )),
            Span::new(0, 38),
        )],
        Span::new(0, 38),
    );
    let source_file = SourceFile::new(
        "examples/functions.ocelot",
        "fun greet(name: string, name: bool) {}",
    );
    let mut symbol_table = create_symbol_table();
    let compilation_session = compilation_session();
    let mut module_environment = create_module_environment();
    let mut context = CompilationContext::default();

    register_module_functions(
        &mut script,
        "functions",
        &source_file,
        &mut context,
        &mut symbol_table,
        &mut module_environment,
        &compilation_session,
    )
    .unwrap();

    let error = finish_resolution(&context).unwrap_err();
    assert!(
        error
            .to_test_string()
            .contains("duplicate parameter `name`")
    );
}

#[test]
fn reports_unknown_function_parameter_types() {
    let mut script = CompilationUnit::new(
        vec![Item::new(
            ItemKind::Function(FunctionItem::new(
                Identifier::new("greet", Span::new(4, 9)),
                vec![parameter("name", "number", Span::new(10, 22))],
                None,
                None,
                Vec::new(),
                Span::new(0, 26),
            )),
            Span::new(0, 26),
        )],
        Span::new(0, 26),
    );
    let source_file = SourceFile::new("examples/functions.ocelot", "fun greet(name: number) {}");
    let mut symbol_table = create_symbol_table();
    let compilation_session = compilation_session();
    let mut module_environment = create_module_environment();
    let mut context = CompilationContext::default();

    register_module_functions(
        &mut script,
        "functions",
        &source_file,
        &mut context,
        &mut symbol_table,
        &mut module_environment,
        &compilation_session,
    )
    .unwrap();

    let error = finish_resolution(&context).unwrap_err();
    assert!(error.to_test_string().contains("unknown type `number`"));
}

#[test]
fn reports_any_in_user_defined_function_signatures() {
    let mut script = CompilationUnit::new(
        vec![Item::new(
            ItemKind::Function(FunctionItem::new(
                Identifier::new("greet", Span::new(4, 9)),
                vec![parameter("value", "any", Span::new(10, 20))],
                None,
                None,
                Vec::new(),
                Span::new(0, 24),
            )),
            Span::new(0, 24),
        )],
        Span::new(0, 24),
    );
    let source_file = SourceFile::new("examples/functions.ocelot", "fun greet(value: any) {}");
    let mut symbol_table = create_symbol_table();
    let compilation_session = compilation_session();
    let mut module_environment = create_module_environment();
    let mut context = CompilationContext::default();

    register_module_functions(
        &mut script,
        "functions",
        &source_file,
        &mut context,
        &mut symbol_table,
        &mut module_environment,
        &compilation_session,
    )
    .unwrap();

    let error = finish_resolution(&context).unwrap_err();
    assert!(
        error
            .to_test_string()
            .contains("`any` may only be used in native function signatures")
    );
}

#[test]
fn reports_native_functions_outside_core() {
    let mut script = CompilationUnit::new(
        vec![Item::new(
            ItemKind::Function(FunctionItem::new_native(
                Identifier::new("println", Span::new(11, 18)),
                vec![parameter("value", "any", Span::new(19, 29))],
                None,
                None,
                Span::new(0, 30),
            )),
            Span::new(0, 30),
        )],
        Span::new(0, 30),
    );
    let source_file = SourceFile::new("examples/helper.ocelot", "native fun println(value: any);");
    let mut symbol_table = create_symbol_table();
    let compilation_session = compilation_session();
    let mut module_environment = create_module_environment();
    let mut context = CompilationContext::default();

    register_module_functions(
        &mut script,
        "helper",
        &source_file,
        &mut context,
        &mut symbol_table,
        &mut module_environment,
        &compilation_session,
    )
    .unwrap();

    let error = finish_resolution(&context).unwrap_err();
    assert!(
        error
            .to_test_string()
            .contains("native functions may only be declared in `core`")
    );
}

#[test]
fn reports_wrong_user_defined_call_arity() {
    let mut script = CompilationUnit::new(
        vec![
            Item::new(
                ItemKind::Function(FunctionItem::new(
                    Identifier::new("greet", Span::new(4, 9)),
                    vec![parameter("name", "string", Span::new(10, 22))],
                    None,
                    None,
                    Vec::new(),
                    Span::new(0, 26),
                )),
                Span::new(0, 26),
            ),
            Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call(
                        identifier("greet", Span::new(27, 32)),
                        Vec::new(),
                        Span::new(27, 34),
                    ))),
                    Span::new(27, 35),
                )),
                Span::new(27, 35),
            ),
        ],
        Span::new(0, 35),
    );
    let source_file = SourceFile::new(
        "examples/functions.ocelot",
        "fun greet(name: string) {} greet();",
    );
    let mut environment = ProgramEnvironment::new();
    let compilation_session = compilation_session();
    let mut context = CompilationContext::default();

    resolve(
        &mut script,
        &source_file,
        &mut context,
        &mut environment,
        &compilation_session,
    )
    .unwrap_err();

    let error = finish_resolution(&context).unwrap_err();
    assert!(
        error
            .to_test_string()
            .contains("type error: `greet` expects exactly one argument")
    );
}

#[test]
fn reports_wrong_user_defined_call_argument_types() {
    let mut script = CompilationUnit::new(
        vec![
            Item::new(
                ItemKind::Function(FunctionItem::new(
                    Identifier::new("greet", Span::new(4, 9)),
                    vec![parameter("excited", "bool", Span::new(10, 23))],
                    None,
                    None,
                    Vec::new(),
                    Span::new(0, 27),
                )),
                Span::new(0, 27),
            ),
            Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call(
                        identifier("greet", Span::new(28, 33)),
                        vec![string_literal("hello", Span::new(34, 41))],
                        Span::new(28, 42),
                    ))),
                    Span::new(28, 43),
                )),
                Span::new(28, 43),
            ),
        ],
        Span::new(0, 43),
    );
    let source_file = SourceFile::new(
        "examples/functions.ocelot",
        "fun greet(excited: bool) {} greet(\"hello\");",
    );
    let mut environment = ProgramEnvironment::new();
    let compilation_session = compilation_session();
    let mut context = CompilationContext::default();

    resolve(
        &mut script,
        &source_file,
        &mut context,
        &mut environment,
        &compilation_session,
    )
    .unwrap_err();

    let error = finish_resolution(&context).unwrap_err();
    assert!(
        error
            .to_test_string()
            .contains("type error: argument 1 to `greet` must be bool")
    );
}

#[test]
fn resolves_module_qualified_calls() {
    let mut main_script = CompilationUnit::new(
        vec![Item::new(
            ItemKind::Statement(Statement::new(
                StatementKind::Expression(ExpressionStatement::new(call(
                    qualified_identifier(
                        &["math", "greet", "hello"],
                        &[Span::new(0, 4), Span::new(6, 11), Span::new(13, 18)],
                    ),
                    Vec::new(),
                    Span::new(0, 20),
                ))),
                Span::new(0, 21),
            )),
            Span::new(0, 21),
        )],
        Span::new(0, 21),
    );
    let mut module_script = CompilationUnit::new(
        vec![Item::new(
            ItemKind::Function(FunctionItem::new(
                Identifier::new("hello", Span::new(4, 9)),
                Vec::new(),
                None,
                None,
                Vec::new(),
                Span::new(0, 14),
            )),
            Span::new(0, 14),
        )],
        Span::new(0, 14),
    );
    let main_source_file = SourceFile::new("main.ocelot", "math::greet::hello();");
    let module_source_file = SourceFile::new("math/greet.ocelot", "fun hello() {}");
    let mut symbol_table = create_symbol_table();
    symbol_table.add_module("main");
    symbol_table.add_module("math::greet");
    let compilation_session = compilation_session();
    let mut module_environment = create_module_environment();
    let mut module_environments = HashMap::from([
        (main_source_file.path.clone(), create_module_environment()),
        (module_source_file.path.clone(), create_module_environment()),
    ]);
    let mut context = CompilationContext::default();

    register_module_functions(
        &mut module_script,
        "math::greet",
        &module_source_file,
        &mut context,
        &mut symbol_table,
        &mut module_environment,
        &compilation_session,
    )
    .unwrap();
    resolve_module_items(
        &mut main_script,
        "main",
        &main_source_file,
        &mut context,
        &symbol_table,
        module_environments
            .get_mut(&main_source_file.path)
            .expect("module environment should exist"),
        &compilation_session,
    )
    .unwrap();
    let resolved_functions = resolve_user_defined_function_definitions(
        &mut context,
        &symbol_table,
        &module_environments,
        &compilation_session,
    )
    .unwrap();
    let mut environment = ProgramEnvironment::from_symbol_table(&symbol_table);
    environment
        .apply_resolved_functions(resolved_functions)
        .unwrap();
    finish_resolution(&context).unwrap();

    let ItemKind::Statement(statement) = &main_script.items[0].kind else {
        panic!("expected statement");
    };
    let StatementKind::Expression(ExpressionStatement { expression }) = &statement.kind;
    let ExpressionKind::Call(call_expression) = &expression.kind else {
        panic!("expected call expression");
    };
    assert_eq!(
        call_expression.function_index().unwrap(),
        environment.resolve_function("math::greet::hello").unwrap()
    );
}

#[test]
fn resolves_imported_function_calls() {
    let mut main_script = CompilationUnit::new(
        vec![
            Item::new(
                ItemKind::Use(UseItem::new(
                    QualifiedIdentifier::new(vec![Identifier::new("helper", Span::new(4, 10))]),
                    vec![Identifier::new("greet", Span::new(12, 17))],
                    Span::new(0, 18),
                )),
                Span::new(0, 18),
            ),
            Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call(
                        identifier("greet", Span::new(19, 24)),
                        Vec::new(),
                        Span::new(19, 26),
                    ))),
                    Span::new(19, 27),
                )),
                Span::new(19, 27),
            ),
        ],
        Span::new(0, 27),
    );
    let mut helper_script = CompilationUnit::new(
        vec![Item::new(
            ItemKind::Function(FunctionItem::new(
                Identifier::new("greet", Span::new(4, 9)),
                Vec::new(),
                None,
                None,
                Vec::new(),
                Span::new(0, 14),
            )),
            Span::new(0, 14),
        )],
        Span::new(0, 14),
    );
    let main_source_file = SourceFile::new("main.ocelot-script", "use helper::greet;\ngreet();");
    let helper_source_file = SourceFile::new("helper.ocelot", "fun greet() {}");
    let mut symbol_table = create_symbol_table();
    symbol_table.add_module("main");
    symbol_table.add_module("helper");
    let compilation_session = compilation_session();
    let mut helper_module_environment = create_module_environment();
    let mut module_environments = HashMap::from([
        (main_source_file.path.clone(), create_module_environment()),
        (helper_source_file.path.clone(), create_module_environment()),
    ]);
    let mut context = CompilationContext::default();

    register_module_functions(
        &mut helper_script,
        "helper",
        &helper_source_file,
        &mut context,
        &mut symbol_table,
        &mut helper_module_environment,
        &compilation_session,
    )
    .unwrap();
    register_module_imports(
        &mut main_script,
        "main",
        &main_source_file,
        &mut context,
        &mut symbol_table,
        module_environments
            .get_mut(&main_source_file.path)
            .expect("module environment should exist"),
    )
    .unwrap();
    resolve_module_items(
        &mut main_script,
        "main",
        &main_source_file,
        &mut context,
        &symbol_table,
        module_environments
            .get_mut(&main_source_file.path)
            .expect("module environment should exist"),
        &compilation_session,
    )
    .unwrap();
    let resolved_functions = resolve_user_defined_function_definitions(
        &mut context,
        &symbol_table,
        &module_environments,
        &compilation_session,
    )
    .unwrap();
    let mut environment = ProgramEnvironment::from_symbol_table(&symbol_table);
    environment
        .apply_resolved_functions(resolved_functions)
        .unwrap();
    finish_resolution(&context).unwrap();

    let ItemKind::Statement(statement) = &main_script.items[0].kind else {
        panic!("expected statement");
    };
    let StatementKind::Expression(ExpressionStatement { expression }) = &statement.kind;
    let ExpressionKind::Call(call_expression) = &expression.kind else {
        panic!("expected call expression");
    };
    assert_eq!(
        call_expression.function_index().unwrap(),
        environment.resolve_function("helper::greet").unwrap()
    );
}

#[test]
fn imported_names_are_available_inside_function_bodies() {
    let mut main_script = CompilationUnit::new(
        vec![
            Item::new(
                ItemKind::Use(UseItem::new(
                    QualifiedIdentifier::new(vec![Identifier::new("helper", Span::new(4, 10))]),
                    vec![Identifier::new("greet", Span::new(12, 17))],
                    Span::new(0, 18),
                )),
                Span::new(0, 18),
            ),
            Item::new(
                ItemKind::Function(FunctionItem::new(
                    Identifier::new("run", Span::new(23, 26)),
                    Vec::new(),
                    None,
                    None,
                    vec![Statement::new(
                        StatementKind::Expression(ExpressionStatement::new(call(
                            identifier("greet", Span::new(33, 38)),
                            Vec::new(),
                            Span::new(33, 40),
                        ))),
                        Span::new(33, 41),
                    )],
                    Span::new(19, 43),
                )),
                Span::new(19, 43),
            ),
        ],
        Span::new(0, 43),
    );
    let mut helper_script = CompilationUnit::new(
        vec![Item::new(
            ItemKind::Function(FunctionItem::new(
                Identifier::new("greet", Span::new(4, 9)),
                Vec::new(),
                None,
                None,
                Vec::new(),
                Span::new(0, 14),
            )),
            Span::new(0, 14),
        )],
        Span::new(0, 14),
    );
    let main_source_file =
        SourceFile::new("main.ocelot", "use helper::greet;\nfun run() { greet(); }");
    let helper_source_file = SourceFile::new("helper.ocelot", "fun greet() {}");
    let mut symbol_table = create_symbol_table();
    symbol_table.add_module("main");
    symbol_table.add_module("helper");
    let compilation_session = compilation_session();
    let mut main_module_environment = create_module_environment();
    let mut helper_module_environment = create_module_environment();
    let mut module_environments = HashMap::from([
        (
            main_source_file.path.clone(),
            main_module_environment.clone(),
        ),
        (
            helper_source_file.path.clone(),
            helper_module_environment.clone(),
        ),
    ]);
    let mut context = CompilationContext::default();

    register_module_functions(
        &mut main_script,
        "main",
        &main_source_file,
        &mut context,
        &mut symbol_table,
        &mut main_module_environment,
        &compilation_session,
    )
    .unwrap();
    register_module_functions(
        &mut helper_script,
        "helper",
        &helper_source_file,
        &mut context,
        &mut symbol_table,
        &mut helper_module_environment,
        &compilation_session,
    )
    .unwrap();
    register_module_imports(
        &mut main_script,
        "main",
        &main_source_file,
        &mut context,
        &mut symbol_table,
        module_environments
            .get_mut(&main_source_file.path)
            .expect("module environment should exist"),
    )
    .unwrap();
    resolve_module_items(
        &mut main_script,
        "main",
        &main_source_file,
        &mut context,
        &symbol_table,
        module_environments
            .get_mut(&main_source_file.path)
            .expect("module environment should exist"),
        &compilation_session,
    )
    .unwrap();
    let resolved_functions = resolve_user_defined_function_definitions(
        &mut context,
        &symbol_table,
        &module_environments,
        &compilation_session,
    )
    .unwrap();
    let mut environment = ProgramEnvironment::from_symbol_table(&symbol_table);
    environment
        .apply_resolved_functions(resolved_functions)
        .unwrap();
    finish_resolution(&context).unwrap();

    let run = environment
        .function_definition(environment.resolve_function("main::run").unwrap())
        .unwrap();
    let FunctionKind::UserDefined { function, .. } = &run.kind else {
        panic!("expected user-defined function");
    };
    let StatementKind::Expression(ExpressionStatement { expression }) = &function.body[0].kind;
    let ExpressionKind::Call(call_expression) = &expression.kind else {
        panic!("expected call expression");
    };
    assert_eq!(
        call_expression.function_index().unwrap(),
        environment.resolve_function("helper::greet").unwrap()
    );
}

#[test]
fn local_functions_win_over_imported_names() {
    let mut main_script = CompilationUnit::new(
        vec![
            Item::new(
                ItemKind::Use(UseItem::new(
                    QualifiedIdentifier::new(vec![Identifier::new("helper", Span::new(4, 10))]),
                    vec![Identifier::new("greet", Span::new(12, 17))],
                    Span::new(0, 18),
                )),
                Span::new(0, 18),
            ),
            Item::new(
                ItemKind::Function(FunctionItem::new(
                    Identifier::new("greet", Span::new(23, 28)),
                    Vec::new(),
                    None,
                    None,
                    Vec::new(),
                    Span::new(19, 31),
                )),
                Span::new(19, 31),
            ),
            Item::new(
                ItemKind::Statement(Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call(
                        identifier("greet", Span::new(32, 37)),
                        Vec::new(),
                        Span::new(32, 39),
                    ))),
                    Span::new(32, 40),
                )),
                Span::new(32, 40),
            ),
        ],
        Span::new(0, 40),
    );
    let mut helper_script = CompilationUnit::new(
        vec![Item::new(
            ItemKind::Function(FunctionItem::new(
                Identifier::new("greet", Span::new(4, 9)),
                Vec::new(),
                None,
                None,
                Vec::new(),
                Span::new(0, 14),
            )),
            Span::new(0, 14),
        )],
        Span::new(0, 14),
    );
    let main_source_file = SourceFile::new(
        "main.ocelot-script",
        "use helper::greet;\nfun greet() {}\ngreet();",
    );
    let helper_source_file = SourceFile::new("helper.ocelot", "fun greet() {}");
    let mut symbol_table = create_symbol_table();
    symbol_table.add_module("main");
    symbol_table.add_module("helper");
    let compilation_session = compilation_session();
    let mut main_module_environment = create_module_environment();
    let mut helper_module_environment = create_module_environment();
    let mut context = CompilationContext::default();

    register_module_functions(
        &mut main_script,
        "main",
        &main_source_file,
        &mut context,
        &mut symbol_table,
        &mut main_module_environment,
        &compilation_session,
    )
    .unwrap();
    register_module_functions(
        &mut helper_script,
        "helper",
        &helper_source_file,
        &mut context,
        &mut symbol_table,
        &mut helper_module_environment,
        &compilation_session,
    )
    .unwrap();
    register_module_imports(
        &mut main_script,
        "main",
        &main_source_file,
        &mut context,
        &mut symbol_table,
        &mut main_module_environment,
    )
    .unwrap();

    let error = finish_resolution(&context).unwrap_err();
    assert!(
        error
            .to_test_string()
            .contains("conflicts with local function")
    );
}

#[test]
fn reports_duplicate_imports() {
    let mut main_script = CompilationUnit::new(
        vec![
            Item::new(
                ItemKind::Use(UseItem::new(
                    QualifiedIdentifier::new(vec![Identifier::new("helper", Span::new(4, 10))]),
                    vec![Identifier::new("greet", Span::new(12, 17))],
                    Span::new(0, 18),
                )),
                Span::new(0, 18),
            ),
            Item::new(
                ItemKind::Use(UseItem::new(
                    QualifiedIdentifier::new(vec![Identifier::new("helper", Span::new(23, 29))]),
                    vec![Identifier::new("greet", Span::new(31, 36))],
                    Span::new(19, 37),
                )),
                Span::new(19, 37),
            ),
        ],
        Span::new(0, 37),
    );
    let mut helper_script = CompilationUnit::new(
        vec![Item::new(
            ItemKind::Function(FunctionItem::new(
                Identifier::new("greet", Span::new(4, 9)),
                Vec::new(),
                None,
                None,
                Vec::new(),
                Span::new(0, 14),
            )),
            Span::new(0, 14),
        )],
        Span::new(0, 14),
    );
    let main_source_file = SourceFile::new(
        "main.ocelot-script",
        "use helper::greet;\nuse helper::greet;",
    );
    let helper_source_file = SourceFile::new("helper.ocelot", "fun greet() {}");
    let mut symbol_table = create_symbol_table();
    symbol_table.add_module("main");
    symbol_table.add_module("helper");
    let compilation_session = compilation_session();
    let mut helper_module_environment = create_module_environment();
    let mut main_module_environment = create_module_environment();
    let mut context = CompilationContext::default();

    register_module_functions(
        &mut helper_script,
        "helper",
        &helper_source_file,
        &mut context,
        &mut symbol_table,
        &mut helper_module_environment,
        &compilation_session,
    )
    .unwrap();
    register_module_imports(
        &mut main_script,
        "main",
        &main_source_file,
        &mut context,
        &mut symbol_table,
        &mut main_module_environment,
    )
    .unwrap();

    let error = finish_resolution(&context).unwrap_err();
    assert!(error.to_test_string().contains("duplicate import `greet`"));
}

#[test]
fn reports_unknown_functions_in_use_items() {
    let mut main_script = CompilationUnit::new(
        vec![Item::new(
            ItemKind::Use(UseItem::new(
                QualifiedIdentifier::new(vec![Identifier::new("helper", Span::new(4, 10))]),
                vec![Identifier::new("greet", Span::new(12, 17))],
                Span::new(0, 18),
            )),
            Span::new(0, 18),
        )],
        Span::new(0, 18),
    );
    let mut helper_script = CompilationUnit::new(
        vec![Item::new(
            ItemKind::Function(FunctionItem::new(
                Identifier::new("wave", Span::new(4, 8)),
                Vec::new(),
                None,
                None,
                Vec::new(),
                Span::new(0, 13),
            )),
            Span::new(0, 13),
        )],
        Span::new(0, 13),
    );
    let main_source_file = SourceFile::new("main.ocelot-script", "use helper::greet;");
    let helper_source_file = SourceFile::new("helper.ocelot", "fun wave() {}");
    let mut symbol_table = create_symbol_table();
    symbol_table.add_module("main");
    symbol_table.add_module("helper");
    let compilation_session = compilation_session();
    let mut helper_module_environment = create_module_environment();
    let mut main_module_environment = create_module_environment();
    let mut context = CompilationContext::default();

    register_module_functions(
        &mut helper_script,
        "helper",
        &helper_source_file,
        &mut context,
        &mut symbol_table,
        &mut helper_module_environment,
        &compilation_session,
    )
    .unwrap();
    register_module_imports(
        &mut main_script,
        "main",
        &main_source_file,
        &mut context,
        &mut symbol_table,
        &mut main_module_environment,
    )
    .unwrap();

    let error = finish_resolution(&context).unwrap_err();
    assert!(
        error
            .to_test_string()
            .contains("module `helper` has no function `greet`")
    );
}

#[test]
fn reports_unknown_modules() {
    let mut script = CompilationUnit::new(
        vec![Item::new(
            ItemKind::Statement(Statement::new(
                StatementKind::Expression(ExpressionStatement::new(call(
                    qualified_identifier(
                        &["math", "greet", "hello"],
                        &[Span::new(0, 4), Span::new(6, 11), Span::new(13, 18)],
                    ),
                    Vec::new(),
                    Span::new(0, 20),
                ))),
                Span::new(0, 21),
            )),
            Span::new(0, 21),
        )],
        Span::new(0, 21),
    );
    let source_file = SourceFile::new("main.ocelot", "math::greet::hello();");
    let mut symbol_table = create_symbol_table();
    symbol_table.add_module("main");
    let compilation_session = compilation_session();
    let module_environment = create_module_environment();
    let mut context = CompilationContext::default();
    resolve_module_items(
        &mut script,
        "main",
        &source_file,
        &mut context,
        &symbol_table,
        &module_environment,
        &compilation_session,
    )
    .unwrap();
    let error = finish_resolution(&context).unwrap_err();

    assert!(matches!(
        error.kind(),
        ocelot_base::error::ErrorKind::CompilationError(CompilationStage::Resolver)
    ));
    assert!(
        error
            .to_test_string()
            .contains("unknown module `math::greet`")
    );
}

#[test]
fn lowers_function_items_before_resolving_tests() {
    let mut script = CompilationUnit::new(
        vec![
            Item::new(
                ItemKind::Function(FunctionItem::new(
                    Identifier::new("helper", Span::new(4, 10)),
                    Vec::new(),
                    None,
                    None,
                    Vec::new(),
                    Span::new(0, 15),
                )),
                Span::new(0, 15),
            ),
            Item::new(
                ItemKind::Test(TestItem::new("works", Vec::new(), Span::new(16, 30))),
                Span::new(16, 30),
            ),
        ],
        Span::new(0, 30),
    );
    let source_file = SourceFile::new("main.ocelot", "fun helper() {} test \"works\" {}");
    let mut symbol_table = create_symbol_table();
    let compilation_session = compilation_session();
    let mut module_environment = create_module_environment();
    let mut context = CompilationContext::default();

    register_module_functions(
        &mut script,
        "main",
        &source_file,
        &mut context,
        &mut symbol_table,
        &mut module_environment,
        &compilation_session,
    )
    .unwrap();

    assert_eq!(script.items.len(), 1);
    assert!(matches!(script.items[0].kind, ItemKind::Test(_)));
    assert!(matches!(
        symbol_table
            .function_definition(symbol_table.resolve_function_exact("main::helper").unwrap())
            .unwrap()
            .kind,
        FunctionKind::UserDefined { .. }
    ));
}

#[test]
fn registers_effect_items_before_function_resolution() {
    let mut script = CompilationUnit::new(
        vec![Item::new(
            ItemKind::Effect(EffectItem::new(
                Identifier::new("exec", Span::new(7, 11)),
                Span::new(0, 12),
            )),
            Span::new(0, 12),
        )],
        Span::new(0, 12),
    );
    let source_file = SourceFile::new("main.ocelot", "effect exec;");
    let mut symbol_table = create_symbol_table();
    let mut context = CompilationContext::default();

    register_module_effects(&mut script, &source_file, &mut context, &mut symbol_table).unwrap();

    assert!(script.items.is_empty());
    assert!(symbol_table.resolve_effect("exec").is_some());
}

#[test]
fn propagates_explicit_can_effects_to_callers() {
    let mut script = CompilationUnit::new(
        vec![
            Item::new(
                ItemKind::Effect(EffectItem::new(
                    Identifier::new("exec", Span::new(7, 11)),
                    Span::new(0, 12),
                )),
                Span::new(0, 12),
            ),
            Item::new(
                ItemKind::Function(FunctionItem::new(
                    Identifier::new("child", Span::new(17, 22)),
                    Vec::new(),
                    Some(effect_clause("exec", Span::new(25, 33))),
                    None,
                    Vec::new(),
                    Span::new(13, 36),
                )),
                Span::new(13, 36),
            ),
            Item::new(
                ItemKind::Function(FunctionItem::new(
                    Identifier::new("parent", Span::new(41, 47)),
                    Vec::new(),
                    None,
                    None,
                    vec![Statement::new(
                        StatementKind::Expression(ExpressionStatement::new(call(
                            identifier("child", Span::new(53, 58)),
                            Vec::new(),
                            Span::new(53, 60),
                        ))),
                        Span::new(53, 61),
                    )],
                    Span::new(37, 63),
                )),
                Span::new(37, 63),
            ),
        ],
        Span::new(0, 63),
    );
    let source_file = SourceFile::new(
        "main.ocelot",
        "effect exec; fun child() can exec {} fun parent() { child(); }",
    );
    let mut symbol_table = create_symbol_table();
    let compilation_session = compilation_session();
    let mut module_environment = create_module_environment();
    let mut module_environments =
        HashMap::from([(source_file.path.clone(), create_module_environment())]);
    let mut context = CompilationContext::default();

    register_module_effects(&mut script, &source_file, &mut context, &mut symbol_table).unwrap();
    register_module_functions(
        &mut script,
        "main",
        &source_file,
        &mut context,
        &mut symbol_table,
        &mut module_environment,
        &compilation_session,
    )
    .unwrap();
    resolve_module_items(
        &mut script,
        "main",
        &source_file,
        &mut context,
        &symbol_table,
        module_environments
            .get_mut(&source_file.path)
            .expect("module environment should exist"),
        &compilation_session,
    )
    .unwrap();
    let resolved_functions = resolve_user_defined_function_definitions(
        &mut context,
        &symbol_table,
        &module_environments,
        &compilation_session,
    )
    .unwrap();
    let mut environment = ProgramEnvironment::from_symbol_table(&symbol_table);
    environment
        .apply_resolved_functions(resolved_functions)
        .unwrap();
    finish_resolution(&context).unwrap();

    let exec_effect = environment.resolve_effect("exec").unwrap();
    let parent = environment
        .function_definition(environment.resolve_function("main::parent").unwrap())
        .unwrap();

    assert!(parent.inferred_effects.contains(&exec_effect));
}

#[test]
fn reports_transitive_forbidden_effects() {
    let mut script = CompilationUnit::new(
        vec![
            Item::new(
                ItemKind::Effect(EffectItem::new(
                    Identifier::new("exec", Span::new(7, 11)),
                    Span::new(0, 12),
                )),
                Span::new(0, 12),
            ),
            Item::new(
                ItemKind::Function(FunctionItem::new(
                    Identifier::new("child", Span::new(17, 22)),
                    Vec::new(),
                    Some(effect_clause("exec", Span::new(25, 33))),
                    None,
                    Vec::new(),
                    Span::new(13, 36),
                )),
                Span::new(13, 36),
            ),
            Item::new(
                ItemKind::Function(FunctionItem::new(
                    Identifier::new("parent", Span::new(41, 47)),
                    Vec::new(),
                    None,
                    Some(effect_clause("exec", Span::new(50, 61))),
                    vec![Statement::new(
                        StatementKind::Expression(ExpressionStatement::new(call(
                            identifier("child", Span::new(65, 70)),
                            Vec::new(),
                            Span::new(65, 72),
                        ))),
                        Span::new(65, 73),
                    )],
                    Span::new(37, 75),
                )),
                Span::new(37, 75),
            ),
        ],
        Span::new(0, 75),
    );
    let source_file = SourceFile::new(
        "main.ocelot",
        "effect exec; fun child() can exec {} fun parent() cannot exec { child(); }",
    );
    let mut symbol_table = create_symbol_table();
    let compilation_session = compilation_session();
    let mut module_environment = create_module_environment();
    let mut module_environments =
        HashMap::from([(source_file.path.clone(), create_module_environment())]);
    let mut context = CompilationContext::default();

    register_module_effects(&mut script, &source_file, &mut context, &mut symbol_table).unwrap();
    register_module_functions(
        &mut script,
        "main",
        &source_file,
        &mut context,
        &mut symbol_table,
        &mut module_environment,
        &compilation_session,
    )
    .unwrap();
    resolve_module_items(
        &mut script,
        "main",
        &source_file,
        &mut context,
        &symbol_table,
        module_environments
            .get_mut(&source_file.path)
            .expect("module environment should exist"),
        &compilation_session,
    )
    .unwrap();
    let resolved_functions = resolve_user_defined_function_definitions(
        &mut context,
        &symbol_table,
        &module_environments,
        &compilation_session,
    )
    .unwrap();
    let mut environment = ProgramEnvironment::from_symbol_table(&symbol_table);
    environment
        .apply_resolved_functions(resolved_functions)
        .unwrap();

    let error = finish_resolution(&context).unwrap_err();

    assert!(matches!(
        error.kind(),
        ocelot_base::error::ErrorKind::CompilationError(CompilationStage::Resolver)
    ));
    assert!(
        error
            .to_test_string()
            .contains("effect error: function `main::parent` cannot perform effect `exec`")
    );
}

#[test]
fn reports_direct_builtin_effect_violations_at_the_call_site() {
    let mut script = CompilationUnit::new(
        vec![Item::new(
            ItemKind::Function(FunctionItem::new(
                Identifier::new("quiet", Span::new(4, 9)),
                Vec::new(),
                None,
                Some(effect_clause("write_stdout", Span::new(12, 32))),
                vec![Statement::new(
                    StatementKind::Expression(ExpressionStatement::new(call(
                        identifier("println", Span::new(35, 42)),
                        vec![string_literal("hello", Span::new(43, 50))],
                        Span::new(35, 51),
                    ))),
                    Span::new(35, 52),
                )],
                Span::new(0, 54),
            )),
            Span::new(0, 54),
        )],
        Span::new(0, 54),
    );
    let source_file = SourceFile::new(
        "main.ocelot",
        "fun quiet() cannot write_stdout { println(\"hello\"); }",
    );
    let mut symbol_table = create_symbol_table();
    let compilation_session = compilation_session();
    let mut module_environment = create_module_environment();
    let module_environments =
        HashMap::from([(source_file.path.clone(), create_module_environment())]);
    let mut context = CompilationContext::default();

    register_module_functions(
        &mut script,
        "main",
        &source_file,
        &mut context,
        &mut symbol_table,
        &mut module_environment,
        &compilation_session,
    )
    .unwrap();
    let resolved_functions = resolve_user_defined_function_definitions(
        &mut context,
        &symbol_table,
        &module_environments,
        &compilation_session,
    )
    .unwrap();
    let mut environment = ProgramEnvironment::from_symbol_table(&symbol_table);
    environment
        .apply_resolved_functions(resolved_functions)
        .unwrap();

    let error = finish_resolution(&context).unwrap_err();

    assert!(error.to_test_string().contains("println"));
}

#[test]
fn reports_unknown_effect_names_in_function_annotations() {
    let mut script = CompilationUnit::new(
        vec![Item::new(
            ItemKind::Function(FunctionItem::new(
                Identifier::new("quiet", Span::new(4, 9)),
                Vec::new(),
                Some(effect_clause("exec", Span::new(12, 20))),
                None,
                Vec::new(),
                Span::new(0, 23),
            )),
            Span::new(0, 23),
        )],
        Span::new(0, 23),
    );
    let source_file = SourceFile::new("main.ocelot", "fun quiet() can exec {}");
    let mut symbol_table = create_symbol_table();
    let compilation_session = compilation_session();
    let mut module_environment = create_module_environment();
    let mut context = CompilationContext::default();

    register_module_functions(
        &mut script,
        "main",
        &source_file,
        &mut context,
        &mut symbol_table,
        &mut module_environment,
        &compilation_session,
    )
    .unwrap();

    let error = finish_resolution(&context).unwrap_err();

    assert!(error.to_test_string().contains("unknown effect `exec`"));
}

#[test]
fn reports_duplicate_effect_declarations() {
    let mut script = CompilationUnit::new(
        vec![
            Item::new(
                ItemKind::Effect(EffectItem::new(
                    Identifier::new("exec", Span::new(7, 11)),
                    Span::new(0, 12),
                )),
                Span::new(0, 12),
            ),
            Item::new(
                ItemKind::Effect(EffectItem::new(
                    Identifier::new("exec", Span::new(20, 24)),
                    Span::new(13, 25),
                )),
                Span::new(13, 25),
            ),
        ],
        Span::new(0, 25),
    );
    let source_file = SourceFile::new("main.ocelot", "effect exec;\neffect exec;");
    let mut symbol_table = create_symbol_table();
    let mut context = CompilationContext::default();

    register_module_effects(&mut script, &source_file, &mut context, &mut symbol_table).unwrap();

    let error = finish_resolution(&context).unwrap_err();

    assert!(error.to_test_string().contains("duplicate effect `exec`"));
}
