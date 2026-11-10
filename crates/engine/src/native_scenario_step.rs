use ocelot_base::shared_string::SharedString;

/// One finite embedded-session step to compile into a test driver.
pub enum NativeScenarioStep {
    /// Adds declarations and executes only newly submitted statements.
    Execute(SharedString),
    /// Evaluates exactly one expression without retaining its source.
    Eval(SharedString),
}
