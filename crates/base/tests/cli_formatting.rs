use expect_test::expect;
use ocelot_base::assertion_error::AssertionError;
use ocelot_base::cli::format_cli_error;
use ocelot_base::compilation_stage::CompilationStage;
use ocelot_base::error::OcelotError;
use ocelot_base::source_file::SourceFile;
use ocelot_base::span::Span;

/* 📖 # Why keep CLI formatting tests in their own file?
These snapshots include `#[track_caller]` locations from the test call sites.
Keeping them out of `src/cli.rs` avoids pointless snapshot churn whenever the CLI
implementation changes but the formatting behavior does not.
*/
#[test]
fn format_cli_error_renders_headline_and_cause_chain() {
    let error = OcelotError::message("failed to verify")
        .with_source(OcelotError::message("missing reference output"));

    expect!([r#"
        ━━ verification failed
        × error failed to verify
          at crates/base/tests/cli_formatting.rs:16:17
        caused by: missing reference output
             at crates/base/tests/cli_formatting.rs:17:22

        ━━ cause chain
          • missing reference output
            at crates/base/tests/cli_formatting.rs:17:22
    "#])
    .assert_eq(&ocelot_base::unansi(&format_cli_error(
        "verification failed",
        &error,
    )));
}

#[test]
fn format_cli_error_skips_cause_chain_for_multiline_cause() {
    let error = OcelotError::message("failed to load recipe")
        .with_source(OcelotError::message("line one\nline two"));

    expect!([r#"
        ━━ recipe failed
        × error failed to load recipe
          at crates/base/tests/cli_formatting.rs:38:17
        caused by:
           line one
           line two
             at crates/base/tests/cli_formatting.rs:39:22
    "#])
    .assert_eq(&ocelot_base::unansi(&format_cli_error(
        "recipe failed",
        &error,
    )));
}

#[test]
fn format_cli_error_returns_only_rendered_diagnostics_for_compilation_errors() {
    let error = OcelotError::compilation_error(CompilationStage::Parser).with_source(
        OcelotError::message("error: invalid syntax\n  --> test.ocelot:1:1\n"),
    );

    expect!([r#"
        error: invalid syntax
          --> test.ocelot:1:1
    "#])
    .assert_eq(&ocelot_base::unansi(&format_cli_error(
        "operation failed",
        &error,
    )));
}

#[test]
fn format_cli_error_returns_only_rendered_assertion_errors() {
    let source_file = SourceFile::new("examples/tests.ocelot", "assert_eq(\"a\", \"b\");");
    let error = OcelotError::assertion_error(AssertionError::new(
        &source_file,
        Span::new(0, source_file.source().len() - 1),
        "assert_eq values differ",
        "\"a\"",
        "\"b\"",
    ));

    expect!([r#"
        error: assert_eq values differ
          ╭▸ examples/tests.ocelot:1:1
          │
        1 │ assert_eq("a", "b");
          ╰╴━━━━━━━━━━━━━━━━━━━ assertion failed here

        expected: "a"
        actual:   "b""#])
    .assert_eq(&ocelot_base::unansi(&format_cli_error(
        "operation failed",
        &error,
    )));
}
