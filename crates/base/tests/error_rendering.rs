use expect_test::expect;
use ocelot_base::compilation_stage::CompilationStage;
use ocelot_base::error::OcelotError;
use ocelot_base::logging::{info_span, init_logging};
use ocelot_base::result::OcelotResult;

/* 📖 # Why keep error rendering tests in a separate file?
These assertions verify track_caller locations and rendered source lines, so the expected file and
line numbers are part of the behavior under test. Keeping them in a dedicated file avoids snapshot
churn when error.rs changes for unrelated implementation reasons.
*/
#[test]
fn err_macro_formats_error_with_caller_location() {
    let error = ocelot_base::err!("test {}", 123);

    expect!([r#"
        × error test 123
          at crates/base/tests/error_rendering.rs:14:17
    "#])
    .assert_eq(&error.to_test_string());
}

#[test]
fn bail_macro_formats_error_with_caller_location() {
    let error = (|| -> OcelotResult<()> {
        ocelot_base::bail!("test {}", 123);
    })()
    .unwrap_err();

    expect!([r#"
        × error test 123
          at crates/base/tests/error_rendering.rs:26:9
    "#])
    .assert_eq(&error.to_test_string());
}

#[test]
fn chained_error_formats_cause_and_locations() {
    let error = OcelotError::message("failed to read file")
        .with_source(OcelotError::message("missing file"));

    expect!([r#"
        × error failed to read file
          at crates/base/tests/error_rendering.rs:39:17
        caused by: missing file
             at crates/base/tests/error_rendering.rs:40:22
    "#])
    .assert_eq(&error.to_test_string());
}

#[test]
fn std_source_error_formats_cause_and_locations() {
    let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "missing config");
    let error = OcelotError::message("cannot initialize").with_std_source(io_error);

    expect!([r#"
        × error cannot initialize
          at crates/base/tests/error_rendering.rs:54:17
        caused by: missing config
             at crates/base/tests/error_rendering.rs:54:59
    "#])
    .assert_eq(&error.to_test_string());
}

#[test]
fn multiline_cause_renders_as_indented_block() {
    let error = OcelotError::message("failed to load recipe")
        .with_source(OcelotError::message("line one\nline two"));

    expect!([r#"
        × error failed to load recipe
          at crates/base/tests/error_rendering.rs:67:17
        caused by:
           line one
           line two
             at crates/base/tests/error_rendering.rs:68:22
    "#])
    .assert_eq(&error.to_test_string());
}

#[test]
fn span_trace_renders_as_structured_frames() {
    init_logging();
    let span = info_span!("error_test_span");
    let _guard = span.enter();

    let error = OcelotError::message("failed inside span");

    expect!([r#"
        × error failed inside span
          at crates/base/tests/error_rendering.rs:87:17
          span trace:
            0: error_rendering::error_test_span
               at crates/base/tests/error_rendering.rs:84
    "#])
    .assert_eq(&error.to_test_string());
}

#[test]
fn chained_error_only_renders_root_cause_span_trace() {
    init_logging();

    let outer_span = info_span!("outer_error_span");
    let outer_guard = outer_span.enter();
    let error = {
        let inner_span = info_span!("inner_error_span");
        let _inner_guard = inner_span.enter();
        OcelotError::message("outer failure").with_source(OcelotError::message("root cause"))
    };
    drop(outer_guard);

    let rendered = error.to_test_string();

    assert!(rendered.contains("outer failure"));
    assert!(rendered.contains("caused by: root cause"));
    assert!(rendered.contains("inner_error_span"));
    assert!(rendered.contains("outer_error_span"));
    assert_eq!(rendered.matches("span trace:").count(), 1);
}

#[test]
fn compilation_error_formats_stage_name() {
    let error = OcelotError::compilation_error(CompilationStage::Parser);

    expect!([r#"
        × error parser compilation error
          at crates/base/tests/error_rendering.rs:123:17
    "#])
    .assert_eq(&error.to_test_string());
}

#[test]
fn compilation_error_can_wrap_another_error() {
    let error = OcelotError::compilation_error(CompilationStage::Resolver)
        .with_source(OcelotError::message("unresolved import"));

    expect!([r#"
        × error resolver compilation error
          at crates/base/tests/error_rendering.rs:134:17
        caused by: unresolved import
             at crates/base/tests/error_rendering.rs:135:22
    "#])
    .assert_eq(&error.to_test_string());
}
