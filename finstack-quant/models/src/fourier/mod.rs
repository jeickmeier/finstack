//! Fourier pricing engines.

pub mod characteristic_function;
pub mod cos;

pub use cos::{
    bs_cos_price, merton_jump_cos_price, vg_cos_price, BlackScholesCosParams, CosConfig, CosPricer,
    MertonJumpCosParams, VarianceGammaCosParams,
};

/// Product-independent Fourier pricing failure.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("Fourier model failure: {message}")]
pub struct FourierError {
    /// Diagnostic describing the invalid input or numerical failure.
    pub message: String,
}

impl FourierError {
    pub(crate) fn model_failure(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl From<FourierError> for finstack_quant_core::Error {
    fn from(error: FourierError) -> Self {
        Self::Calibration {
            message: error.message,
            category: "fourier".to_string(),
        }
    }
}
