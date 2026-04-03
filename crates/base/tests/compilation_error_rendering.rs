use expect_test::expect;
use ocelot_base::compilation_stage::CompilationStage;
use ocelot_base::error::OcelotError;

/* 📖 # Why keep compilation error rendering tests in their own file?
These assertions are also track_caller-sensitive, but they exercise the newer compilation error
kind. Keeping them separate avoids renumbering the older generic error rendering snapshots when
compilation-specific cases are added later.
*/
#[test]
fn compilation_error_formats_stage_name() {
    let error = OcelotError::compilation_error(CompilationStage::Parser);

    expect!([r#"
        × error parser compilation error
          at crates/base/tests/compilation_error_rendering.rs:12:17
    "#])
    .assert_eq(&error.to_test_string());
}

#[test]
fn compilation_error_can_wrap_another_error() {
    let error = OcelotError::compilation_error(CompilationStage::Resolver)
        .with_source(OcelotError::message("unresolved import"));

    expect!([r#"
        × error resolver compilation error
          at crates/base/tests/compilation_error_rendering.rs:23:17
        caused by: unresolved import
             at crates/base/tests/compilation_error_rendering.rs:24:22
    "#])
    .assert_eq(&error.to_test_string());
}
