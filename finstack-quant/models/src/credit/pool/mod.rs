//! Product-independent stochastic models for structured-credit collateral pools.
//!
//! This module owns default, prepayment, and correlation engines and their
//! serializable specifications. Deal construction, calibration presets,
//! waterfalls, and tranche pricing remain in `finstack-quant-valuations`.

pub mod correlation;
pub mod default;
pub mod prepayment;

pub use correlation::CorrelationStructure;
pub use default::{
    MacroCreditFactors, PerNameCopulaDefault, PoolGranularity, StochasticDefault,
    StochasticDefaultSpec,
};
pub use prepayment::{RichardRollPrepay, StochasticPrepaySpec, StochasticPrepayment};

fn clamped_cdr_to_mdr(cdr: f64) -> f64 {
    finstack_quant_cashflows::builder::cdr_to_mdr(cdr.clamp(0.0, 1.0)).unwrap_or(f64::NAN)
}

fn clamped_cpr_to_smm(cpr: f64) -> f64 {
    finstack_quant_cashflows::builder::cpr_to_smm(cpr.clamp(0.0, 1.0)).unwrap_or(f64::NAN)
}
