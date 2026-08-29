//! Host-language attribute payload for [`ExecuteError`].
//!
//! Python and WASM attach these fields; they do not rebuild the structured
//! error from `category` / `stage` / diagnostics pieces.

use super::engine::{ExecuteError, ExecutionStage};
use super::errors::EnvelopeError;

/// Stable Py/WASM exception attributes for a calibration execution failure.
///
/// Field names match the host contract: `kind`, `stage`, `step_id`,
/// `solver_diagnostics`, and `details`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostExecuteError {
    /// Exception / `Error` message shown to the caller.
    pub message: String,
    /// Programmatic error category attached as `kind`.
    pub kind: String,
    /// Pipeline stage identifier (`ingestion`, `solver`, …).
    pub stage: ExecutionStage,
    /// Failing calibration step identifier, when the failure is step-scoped.
    pub step_id: Option<String>,
    /// JSON object for fit-acceptance diagnostics, when present.
    pub solver_diagnostics: Option<String>,
    /// Pretty-printed [`super::engine::ExecutionErrorDetails`] JSON.
    pub details: String,
}

impl ExecuteError {
    /// Flatten this error into the Py/WASM attribute payload.
    ///
    /// `kind` is the execution category. `details` is the existing
    /// [`ExecuteError::to_json`] document. Solver diagnostics are pre-serialized
    /// so bindings only attach strings.
    #[must_use]
    pub fn host_error(&self) -> HostExecuteError {
        let details = self.details();
        let solver_diagnostics = match details.solver_diagnostics.as_ref() {
            Some(diagnostics) => match serde_json::to_string(diagnostics) {
                Ok(json) => Some(json),
                Err(error) => {
                    return Self::envelope(
                        details.stage,
                        EnvelopeError::JsonSerialize {
                            target: "ExecutionSolverDiagnostics".to_string(),
                            message: error.to_string(),
                        },
                    )
                    .host_error();
                }
            },
            None => None,
        };
        HostExecuteError {
            message: details.cause,
            kind: details.category,
            stage: details.stage,
            step_id: details.step_id,
            solver_diagnostics,
            details: self.to_json(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::engine::{ExecuteError, ExecutionStage};

    #[test]
    fn host_error_uses_kind_not_category_and_pre_serializes_diagnostics() {
        let error = ExecuteError::envelope(
            ExecutionStage::Solver,
            EnvelopeError::SolverNotConverged {
                step_id: "hazard".to_string(),
                max_residual: 2e-6,
                tolerance: 1e-6,
                iterations: 12,
                worst_quote_id: Some("CDS-5Y".to_string()),
                worst_quote_residual: Some(2e-6),
            },
        );
        let host = error.host_error();
        assert_eq!(host.kind, "solver_not_converged");
        assert_eq!(host.stage.as_str(), "solver");
        assert_eq!(host.step_id.as_deref(), Some("hazard"));
        let diagnostics: serde_json::Value = serde_json::from_str(
            host.solver_diagnostics
                .as_deref()
                .expect("solver diagnostics present"),
        )
        .expect("diagnostics JSON");
        assert_eq!(diagnostics["worst_quote_id"], "CDS-5Y");
        assert_eq!(diagnostics["iterations"], 12);
        let details: serde_json::Value = serde_json::from_str(&host.details).expect("details JSON");
        assert_eq!(details["category"], host.kind);
        assert_eq!(details["stage"], host.stage.as_str());
    }
}
