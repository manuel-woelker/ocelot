use ocelot_base::error::OcelotError;
use ocelot_base::shared_string::SharedString;

/// Generated C and a possible later frontend error in an embedded scenario.
pub struct NativeTranslationUnit {
    /// Standalone C99 source.
    pub source: SharedString,
    /// Language error reported only after the valid prefix completes successfully.
    pub deferred_error: Option<OcelotError>,
    /// Number of successfully prepared scenario steps.
    pub step_count: usize,
}
