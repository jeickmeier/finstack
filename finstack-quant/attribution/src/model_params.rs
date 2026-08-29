//! Model parameters extraction and modification for P&L attribution.
//!
//! Provides functionality to delegate model-specific parameter extraction to
//! instruments, create modified versions with different parameters, and measure
//! parameter shifts.

use finstack_quant_core::Result;
use finstack_quant_valuations::instruments::model_params::ModelParamsSnapshot;
use finstack_quant_valuations::instruments::Instrument;
use std::sync::Arc;

/// Extract model parameters from an instrument.
///
/// Delegates through the [`Instrument`] trait so each instrument owns its
/// model-parameter extraction behavior.
///
/// # Arguments
///
/// * `instrument` - Instrument to extract parameters from
///
/// # Returns
///
/// Snapshot of model parameters, or `ModelParamsSnapshot::None` if instrument
/// type doesn't have extractable parameters.
///
/// # Examples
///
/// ```
/// use finstack_quant_attribution::extract_model_params;
/// use finstack_quant_valuations::instruments::fixed_income::structured_credit::StructuredCredit;
/// use finstack_quant_valuations::instruments::model_params::ModelParamsSnapshot;
/// use finstack_quant_valuations::instruments::Instrument;
/// use std::sync::Arc;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let structured_credit = Arc::new(StructuredCredit::example())
///     as Arc<dyn Instrument>;
///
/// let params = extract_model_params(&structured_credit);
/// match params {
///     ModelParamsSnapshot::StructuredCredit { prepayment_spec, .. } => {
///         println!("Prepayment: {:?}", prepayment_spec);
///     }
///     _ => {}
/// }
/// # Ok(())
/// # }
/// ```
pub fn extract_model_params(instrument: &Arc<dyn Instrument>) -> ModelParamsSnapshot {
    instrument.model_params_snapshot()
}

/// Create a modified instrument with different model parameters.
///
/// Clones the instrument and replaces its model parameters with those from
/// the snapshot. Used for isolating model parameter P&L in attribution.
///
/// # Arguments
///
/// * `instrument` - Original instrument
/// * `params` - Model parameters to apply
///
/// # Returns
///
/// New instrument with modified parameters, or original if no params to modify.
///
/// # Errors
///
/// Returns error if instrument type doesn't match snapshot type.
///
/// # Examples
///
/// ```
/// // Extract T₀ parameters
/// use finstack_quant_attribution::{extract_model_params, with_model_params};
/// use finstack_quant_valuations::instruments::fixed_income::structured_credit::StructuredCredit;
/// use finstack_quant_valuations::instruments::Instrument;
/// use std::sync::Arc;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let instrument = Arc::new(StructuredCredit::example())
///     as Arc<dyn Instrument>;
///
/// let params_t0 = extract_model_params(&instrument);
///
/// // Create instrument with T₀ params for attribution
/// let instrument_t0_params = with_model_params(&instrument, &params_t0)?;
/// # let _ = instrument_t0_params;
/// # Ok(())
/// # }
/// ```
pub fn with_model_params(
    instrument: &Arc<dyn Instrument>,
    params: &ModelParamsSnapshot,
) -> Result<Arc<dyn Instrument>> {
    if matches!(params, ModelParamsSnapshot::None) {
        return Ok(Arc::clone(instrument));
    }

    instrument.with_model_params(params).map(Arc::from)
}

/// Compute a prepayment parameter shift between two snapshots.
///
/// Returns the shift in **basis points** of CPR. Pairs directly with the
/// `Prepayment01` metric, which is `$ per 1bp` of CPR:
/// `model_params_pnl ≈ Prepayment01 × measure_prepayment_shift(t0, t1)`.
///
/// # PSA terminal-CPR proxy
///
/// PSA multiplier changes are converted linearly at the **terminal** rate
/// (100% PSA ≈ 6% CPR after the 30-month seasoning ramp), i.e.
/// `Δmult × 600bp`. This proxy is exact only for fully-seasoned (≥ 30 month)
/// collateral; on-ramp collateral's effective CPR is `age/30 × 6% × mult`,
/// so the proxy overstates the shift by up to 2.5× for new collateral.
/// Collateral age is not part of [`ModelParamsSnapshot`], so no
/// seasoning-aware conversion is possible here. A `(Psa, None)` /
/// `(None, Psa)` curve pair treats the `None` side as multiplier 0 (with a
/// `tracing::warn!`) instead of silently reporting a 0bp shift.
///
/// # Arguments
///
/// * `snapshot_t0` - Parameters at T₀
/// * `snapshot_t1` - Parameters at T₁
///
fn prepayment_shift(
    snapshot_t0: &ModelParamsSnapshot,
    snapshot_t1: &ModelParamsSnapshot,
) -> Option<f64> {
    match (snapshot_t0, snapshot_t1) {
        (
            ModelParamsSnapshot::StructuredCredit {
                prepayment_spec: prep_t0,
                ..
            },
            ModelParamsSnapshot::StructuredCredit {
                prepayment_spec: prep_t1,
                ..
            },
        ) => {
            use finstack_quant_cashflows::builder::specs::PrepaymentCurve;

            match (&prep_t0.curve, &prep_t1.curve) {
                (
                    Some(PrepaymentCurve::Psa {
                        speed_multiplier: mult_t0,
                    }),
                    Some(PrepaymentCurve::Psa {
                        speed_multiplier: mult_t1,
                    }),
                ) => {
                    // PSA multiplier change converted via the TERMINAL CPR:
                    // 100% PSA ≈ 6% CPR after the 30-month seasoning ramp, so
                    // Δmultiplier × 600bp. This linear conversion is a
                    // terminal-CPR proxy valid for fully-seasoned (≥ 30
                    // month) collateral; for collateral still on the ramp
                    // (age < 30m) the effective CPR is age/30 × 6% × mult and
                    // this proxy overstates the shift by up to 2.5×.
                    // Collateral age is not available from the snapshots, so
                    // the proxy is documented rather than seasoning-adjusted.
                    Some((mult_t1 - mult_t0) * 600.0) // Convert to basis points
                }
                (None, None)
                | (Some(PrepaymentCurve::Constant), Some(PrepaymentCurve::Constant)) => {
                    // Direct CPR difference in basis points
                    Some((prep_t1.cpr - prep_t0.cpr) * 10000.0)
                }
                // A (PSA, None) pair used to fall through to `None` and be
                // silently zeroed by the caller. The None side
                // is treated as PSA multiplier 0 (zero prepayment baseline;
                // its `cpr` field is ignored, matching the PSA branch which
                // also ignores `cpr`) so the Some side's shift is measured.
                (
                    Some(PrepaymentCurve::Psa {
                        speed_multiplier: mult_t0,
                    }),
                    None,
                ) => {
                    tracing::warn!(
                        mult_t0,
                        "prepayment shift: T1 snapshot has no prepayment curve; \
                         treated as PSA multiplier 0 (terminal-CPR proxy)"
                    );
                    Some((0.0 - mult_t0) * 600.0)
                }
                (
                    None,
                    Some(PrepaymentCurve::Psa {
                        speed_multiplier: mult_t1,
                    }),
                ) => {
                    tracing::warn!(
                        mult_t1,
                        "prepayment shift: T0 snapshot has no prepayment curve; \
                         treated as PSA multiplier 0 (terminal-CPR proxy)"
                    );
                    Some(mult_t1 * 600.0)
                }
                _ => None, // Mixed or unsupported model types
            }
        }
        _ => None,
    }
}

fn measure_or_zero(shift: Option<f64>, what: &str) -> f64 {
    shift.unwrap_or_else(|| {
        tracing::warn!("Model parameter {what} shift defaulted to zero");
        0.0
    })
}

/// Measure prepayment parameter shift between two snapshots.
///
/// Returns the shift in **basis points** of CPR (0.0 if not applicable),
/// pairing directly with the `$ per 1bp` `Prepayment01` metric.
///
/// # Arguments
///
/// * `snapshot_t0` - Opening model-parameter snapshot containing the
///   prepayment specification, if applicable.
/// * `snapshot_t1` - Closing model-parameter snapshot whose prepayment terms
///   are compared with `snapshot_t0`.
pub fn measure_prepayment_shift(
    snapshot_t0: &ModelParamsSnapshot,
    snapshot_t1: &ModelParamsSnapshot,
) -> f64 {
    measure_or_zero(prepayment_shift(snapshot_t0, snapshot_t1), "prepayment")
}

/// Compute a default rate parameter shift between two snapshots.
///
/// Returns the shift in **basis points** of CDR. Pairs directly with the
/// `Default01` metric (`$ per 1bp` of CDR).
fn default_shift(
    snapshot_t0: &ModelParamsSnapshot,
    snapshot_t1: &ModelParamsSnapshot,
) -> Option<f64> {
    match (snapshot_t0, snapshot_t1) {
        (
            ModelParamsSnapshot::StructuredCredit {
                default_spec: def_t0,
                ..
            },
            ModelParamsSnapshot::StructuredCredit {
                default_spec: def_t1,
                ..
            },
        ) => {
            // CDR difference in basis points (works for both constant and SDA curves)
            Some((def_t1.cdr - def_t0.cdr) * 10000.0)
        }
        _ => None,
    }
}

/// Measure default rate parameter shift between two snapshots.
///
/// Returns the shift in **basis points** of CDR (0.0 if not applicable),
/// pairing directly with the `$ per 1bp` `Default01` metric.
///
/// # Arguments
///
/// * `snapshot_t0` - Opening model-parameter snapshot containing the default
///   specification, if applicable.
/// * `snapshot_t1` - Closing model-parameter snapshot whose default terms are
///   compared with `snapshot_t0`.
pub fn measure_default_shift(
    snapshot_t0: &ModelParamsSnapshot,
    snapshot_t1: &ModelParamsSnapshot,
) -> f64 {
    measure_or_zero(default_shift(snapshot_t0, snapshot_t1), "default")
}

/// Compute a recovery rate parameter shift between two snapshots.
///
/// Returns the shift in **percentage points** (not basis points). Pairs
/// directly with the `Recovery01` metric (`$ per 1%` recovery move).
fn recovery_shift(
    snapshot_t0: &ModelParamsSnapshot,
    snapshot_t1: &ModelParamsSnapshot,
) -> Option<f64> {
    match (snapshot_t0, snapshot_t1) {
        (
            ModelParamsSnapshot::StructuredCredit {
                recovery_spec: rec_t0,
                ..
            },
            ModelParamsSnapshot::StructuredCredit {
                recovery_spec: rec_t1,
                ..
            },
        ) => {
            // Direct recovery rate difference in percentage points
            Some((rec_t1.rate - rec_t0.rate) * 100.0)
        }
        _ => None,
    }
}

/// Measure recovery rate parameter shift between two snapshots.
///
/// Returns the shift in **percentage points** (0.0 if not applicable),
/// pairing directly with the `$ per 1%` `Recovery01` metric.
///
/// # Arguments
///
/// * `snapshot_t0` - Opening model-parameter snapshot containing the recovery
///   specification, if applicable.
/// * `snapshot_t1` - Closing model-parameter snapshot whose recovery terms are
///   compared with `snapshot_t0`.
pub fn measure_recovery_shift(
    snapshot_t0: &ModelParamsSnapshot,
    snapshot_t1: &ModelParamsSnapshot,
) -> f64 {
    measure_or_zero(recovery_shift(snapshot_t0, snapshot_t1), "recovery")
}

/// Compute a conversion ratio shift between two snapshots.
///
/// Returns shift in percentage points for use with Conversion01 metric.
fn conversion_shift(
    snapshot_t0: &ModelParamsSnapshot,
    snapshot_t1: &ModelParamsSnapshot,
) -> Option<f64> {
    match (snapshot_t0, snapshot_t1) {
        (
            ModelParamsSnapshot::Convertible {
                conversion_spec: conv_t0,
            },
            ModelParamsSnapshot::Convertible {
                conversion_spec: conv_t1,
            },
        ) => {
            match (conv_t0.ratio, conv_t1.ratio) {
                (Some(ratio_t0), Some(ratio_t1)) if ratio_t0 != 0.0 => {
                    // Conversion ratio change as percentage
                    Some(((ratio_t1 - ratio_t0) / ratio_t0) * 100.0)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// Measure conversion ratio shift between two snapshots.
///
/// Returns shift in percentage points, or 0.0 if not applicable.
///
/// # Arguments
///
/// * `snapshot_t0` - Opening convertible model-parameter snapshot.
/// * `snapshot_t1` - Closing convertible model-parameter snapshot whose
///   conversion ratio is compared with `snapshot_t0`.
pub fn measure_conversion_shift(
    snapshot_t0: &ModelParamsSnapshot,
    snapshot_t1: &ModelParamsSnapshot,
) -> f64 {
    measure_or_zero(conversion_shift(snapshot_t0, snapshot_t1), "conversion")
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_quant_cashflows::builder::{
        DefaultModelSpec, PrepaymentModelSpec, RecoveryModelSpec,
    };
    use finstack_quant_valuations::instruments::fixed_income::convertible::{
        AntiDilutionPolicy, ConversionPolicy, ConversionSpec, DividendAdjustment,
    };

    #[test]
    fn test_measure_prepayment_shift_psa() {
        let params_t0 = ModelParamsSnapshot::StructuredCredit {
            prepayment_spec: PrepaymentModelSpec::psa(1.0),
            default_spec: DefaultModelSpec::constant_cdr(0.02),
            recovery_spec: RecoveryModelSpec::with_lag(0.60, 12),
        };

        let params_t1 = ModelParamsSnapshot::StructuredCredit {
            prepayment_spec: PrepaymentModelSpec::psa(1.5),
            default_spec: DefaultModelSpec::constant_cdr(0.02),
            recovery_spec: RecoveryModelSpec::with_lag(0.60, 12),
        };

        let shift = measure_prepayment_shift(&params_t0, &params_t1);
        // PSA increased by 0.5, which is 0.5 * 600bp = 300bp
        assert_eq!(shift, 300.0);
    }

    /// A (PSA, None) prepayment-curve pair used to fall through
    /// the match to `None` and be silently reported as a 0bp shift. The None
    /// side is treated as PSA multiplier 0 (zero baseline), so the Some side's
    /// shift is measured rather than dropped.
    #[test]
    fn test_measure_prepayment_shift_psa_none_pair_uses_zero_baseline() {
        let psa_side = ModelParamsSnapshot::StructuredCredit {
            prepayment_spec: PrepaymentModelSpec::psa(1.0),
            default_spec: DefaultModelSpec::constant_cdr(0.02),
            recovery_spec: RecoveryModelSpec::with_lag(0.60, 12),
        };
        // `constant_cpr` carries `curve: None`.
        let none_side = ModelParamsSnapshot::StructuredCredit {
            prepayment_spec: PrepaymentModelSpec::constant_cpr(0.0),
            default_spec: DefaultModelSpec::constant_cdr(0.02),
            recovery_spec: RecoveryModelSpec::with_lag(0.60, 12),
        };

        // PSA 1.0 → (none ≡ 0): shift = (0 − 1.0) × 600bp = −600bp.
        assert_eq!(measure_prepayment_shift(&psa_side, &none_side), -600.0);
        // And symmetrically for the (None, PSA) direction.
        assert_eq!(measure_prepayment_shift(&none_side, &psa_side), 600.0);
    }

    #[test]
    fn test_measure_shift_defaults_to_zero_for_snapshot_type_mismatch() {
        let structured = ModelParamsSnapshot::StructuredCredit {
            prepayment_spec: PrepaymentModelSpec::psa(1.0),
            default_spec: DefaultModelSpec::constant_cdr(0.02),
            recovery_spec: RecoveryModelSpec::with_lag(0.60, 12),
        };
        let convertible = ModelParamsSnapshot::Convertible {
            conversion_spec: ConversionSpec {
                ratio: Some(20.0),
                price: None,
                policy: ConversionPolicy::Voluntary,
                anti_dilution: AntiDilutionPolicy::None,
                dividend_adjustment: DividendAdjustment::None,
                dilution_events: Vec::new(),
            },
        };

        assert_eq!(measure_prepayment_shift(&structured, &convertible), 0.0);
        assert_eq!(measure_default_shift(&structured, &convertible), 0.0);
        assert_eq!(measure_recovery_shift(&structured, &convertible), 0.0);
        assert_eq!(measure_conversion_shift(&structured, &convertible), 0.0);
    }

    #[test]
    fn test_measure_default_shift_cdr() {
        let params_t0 = ModelParamsSnapshot::StructuredCredit {
            prepayment_spec: PrepaymentModelSpec::psa(1.0),
            default_spec: DefaultModelSpec::constant_cdr(0.02),
            recovery_spec: RecoveryModelSpec::with_lag(0.60, 12),
        };

        let params_t1 = ModelParamsSnapshot::StructuredCredit {
            prepayment_spec: PrepaymentModelSpec::psa(1.0),
            default_spec: DefaultModelSpec::constant_cdr(0.03),
            recovery_spec: RecoveryModelSpec::with_lag(0.60, 12),
        };

        let shift = measure_default_shift(&params_t0, &params_t1);
        // CDR increased by 1% = 100bp
        assert!((shift - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_measure_recovery_shift() {
        let params_t0 = ModelParamsSnapshot::StructuredCredit {
            prepayment_spec: PrepaymentModelSpec::psa(1.0),
            default_spec: DefaultModelSpec::constant_cdr(0.02),
            recovery_spec: RecoveryModelSpec::with_lag(0.60, 12),
        };

        let params_t1 = ModelParamsSnapshot::StructuredCredit {
            prepayment_spec: PrepaymentModelSpec::psa(1.0),
            default_spec: DefaultModelSpec::constant_cdr(0.02),
            recovery_spec: RecoveryModelSpec::with_lag(0.65, 12),
        };

        let shift = measure_recovery_shift(&params_t0, &params_t1);
        // Recovery rate increased from 60% to 65% (5 percentage points)
        assert!((shift - 5.0).abs() < 0.01);
    }
}
