//! Resource limits for persisted artifact loading.

/// Default maximum encoded artifact size: 64 MiB.
pub const DEFAULT_MAX_BYTES: usize = 64 * 1024 * 1024;
/// Default maximum number of artifacts in one load operation.
pub const DEFAULT_MAX_ARTIFACTS: usize = 100_000;
/// Default maximum number of portfolio positions in one artifact.
pub const DEFAULT_MAX_POSITIONS: usize = 1_000_000;
/// Default maximum JSON nesting depth accepted by contract loaders.
pub const DEFAULT_MAX_DEPTH: usize = 96;
/// Default maximum number of diagnostics retained in a validation report.
pub const DEFAULT_MAX_DIAGNOSTICS: usize = 256;

/// Resource limits applied while loading persisted artifacts.
///
/// The defaults protect ordinary callers from accidentally processing
/// pathologically large documents. Builder methods allow a loader to tighten or
/// relax an individual limit for a known workload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadLimits {
    /// Maximum encoded input size in bytes.
    pub max_bytes: usize,
    /// Maximum number of artifacts accepted in one load operation.
    pub max_artifacts: usize,
    /// Maximum number of portfolio positions accepted in one artifact.
    pub max_positions: usize,
    /// Maximum JSON nesting depth accepted by contract loaders.
    ///
    /// The default of 96 leaves headroom below `serde_json`'s recursion
    /// backstop.
    pub max_depth: usize,
    /// Maximum number of individual diagnostics retained in a report.
    ///
    /// Additional findings set [`ValidationReport::truncated`](super::ValidationReport::truncated).
    pub max_diagnostics: usize,
}

impl LoadLimits {
    /// Set the maximum encoded input size.
    ///
    /// # Arguments
    ///
    /// * `max_bytes` - Maximum number of encoded bytes accepted per input;
    ///   zero rejects every non-empty input.
    #[must_use]
    pub const fn with_max_bytes(mut self, max_bytes: usize) -> Self {
        self.max_bytes = max_bytes;
        self
    }

    /// Set the maximum artifact count.
    ///
    /// # Arguments
    ///
    /// * `max_artifacts` - Maximum number of artifacts accepted in one load
    ///   operation; zero permits only an empty collection.
    #[must_use]
    pub const fn with_max_artifacts(mut self, max_artifacts: usize) -> Self {
        self.max_artifacts = max_artifacts;
        self
    }

    /// Set the maximum portfolio-position count.
    ///
    /// # Arguments
    ///
    /// * `max_positions` - Maximum number of positions accepted in one
    ///   persisted portfolio; zero permits only an empty portfolio.
    #[must_use]
    pub const fn with_max_positions(mut self, max_positions: usize) -> Self {
        self.max_positions = max_positions;
        self
    }

    /// Set the maximum JSON nesting depth.
    ///
    /// # Arguments
    ///
    /// * `max_depth` - Maximum number of nested JSON containers accepted by a
    ///   loader; zero permits only scalar roots.
    #[must_use]
    pub const fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Set the maximum retained diagnostic count.
    ///
    /// # Arguments
    ///
    /// * `max_diagnostics` - Maximum number of findings retained individually;
    ///   further findings only mark the report as truncated.
    #[must_use]
    pub const fn with_max_diagnostics(mut self, max_diagnostics: usize) -> Self {
        self.max_diagnostics = max_diagnostics;
        self
    }
}

impl Default for LoadLimits {
    fn default() -> Self {
        Self {
            max_bytes: DEFAULT_MAX_BYTES,
            max_artifacts: DEFAULT_MAX_ARTIFACTS,
            max_positions: DEFAULT_MAX_POSITIONS,
            max_depth: DEFAULT_MAX_DEPTH,
            max_diagnostics: DEFAULT_MAX_DIAGNOSTICS,
        }
    }
}
