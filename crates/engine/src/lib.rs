//! High-level language pipeline orchestration for `ocelot`.

pub mod build_native;
pub mod builtin_module;
pub mod captured_process;
pub mod compile_program;
pub mod core_module;
pub mod discovered_test;
pub mod embedded_engine;
pub mod engine;
pub mod engine_command;
pub mod engine_worker;
pub mod failed_test_result;
pub mod module_name_from_path;
pub mod native_build_request;
pub mod native_scenario_step;
pub mod native_source;
pub mod native_toolchain;
pub mod native_translation_unit;
pub mod passed_test_result;
pub mod prepare_eval_input;
pub mod tcc_toolchain;
pub mod test_filter;
pub mod test_run_summary;
