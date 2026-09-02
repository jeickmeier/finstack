//! Shared transform parameter helpers.

use finstack_quant_core::{Error, Result};
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Parse a snake_case operation name through the enum's serde representation.
pub(crate) fn op_from_str<T: DeserializeOwned>(op: &str, kind: &str) -> Result<T> {
    serde_json::from_value(Value::String(op.to_owned()))
        .map_err(|_| Error::Validation(format!("unsupported {kind} transform op '{op}'")))
}

/// Numerical tolerance used for zero-denominator checks.
pub(crate) const ZERO_TOLERANCE: f64 = 1e-12;

/// Φ⁻¹(0.75) — the third-quartile standard-normal quantile.
///
/// Scaling a median-absolute-deviation by `MAD / PHI_INV_075` (equivalently
/// multiplying by [`MAD_NORMAL_CONSISTENCY`]) makes the MAD a consistent
/// estimator of σ for normally distributed data.
///
/// # References
///
/// - Rousseeuw, P. J., & Croux, C. (1993). "Alternatives to the Median Absolute
///   Deviation." *Journal of the American Statistical Association*, 88(424),
///   1273-1283.
///
/// Value verified against `NormalDist().inv_cdf(0.75)`; it is the exact
/// reciprocal of [`MAD_NORMAL_CONSISTENCY`], and the two MUST stay reciprocal —
/// they previously drifted apart by 1.5e-6, silently biasing `robust_zscore`.
pub(crate) const PHI_INV_075: f64 = 0.674_489_750_196_081_7;

/// 1 / Φ⁻¹(0.75) — the MAD-to-σ normal consistency factor.
///
/// See [`PHI_INV_075`] for the citation and the reciprocity invariant.
pub(crate) const MAD_NORMAL_CONSISTENCY: f64 = 1.482_602_218_505_602;

pub(crate) fn finite(value: Option<f64>) -> Option<f64> {
    value.filter(|inner| inner.is_finite())
}

pub(crate) fn validate_lengths(primary: usize, others: &[(&str, usize)]) -> Result<()> {
    for (name, len) in others {
        if *len != primary {
            return Err(Error::Validation(format!(
                "panel transform length mismatch: values has length {primary}, {name} has length {len}"
            )));
        }
    }
    Ok(())
}

pub(crate) fn usize_param(params: Option<&Value>, key: &str, default: usize) -> Result<usize> {
    match params.and_then(|value| value.get(key)) {
        Some(value) => {
            let raw = value.as_u64().ok_or_else(|| {
                Error::Validation(format!(
                    "panel transform parameter '{key}' must be an integer"
                ))
            })?;
            if raw == 0 {
                return Err(Error::Validation(format!(
                    "panel transform parameter '{key}' must be positive"
                )));
            }
            usize::try_from(raw).map_err(|_| {
                Error::Validation(format!("panel transform parameter '{key}' is too large"))
            })
        }
        None => Ok(default),
    }
}

pub(crate) fn required_f64_param(params: Option<&Value>, key: &str) -> Result<f64> {
    params
        .and_then(|value| value.get(key))
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
        .ok_or_else(|| {
            Error::Validation(format!("panel transform parameter '{key}' must be finite"))
        })
}

pub(crate) fn f64_param(params: Option<&Value>, key: &str, default: f64) -> Result<f64> {
    match params.and_then(|value| value.get(key)) {
        Some(value) => value
            .as_f64()
            .filter(|inner| inner.is_finite())
            .ok_or_else(|| {
                Error::Validation(format!("panel transform parameter '{key}' must be finite"))
            }),
        None => Ok(default),
    }
}

pub(crate) fn bool_param(params: Option<&Value>, key: &str, default: bool) -> Result<bool> {
    match params.and_then(|value| value.get(key)) {
        Some(value) => value.as_bool().ok_or_else(|| {
            Error::Validation(format!(
                "panel transform parameter '{key}' must be a boolean"
            ))
        }),
        None => Ok(default),
    }
}

pub(crate) fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    Some(values.iter().sum::<f64>() / values.len() as f64)
}

/// Return the Type-7 continuous quantile of an ascending, total-ordered slice.
pub(crate) fn quantile_cont(sorted: &[f64], probability: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    if sorted.len() == 1 {
        return Some(sorted[0]);
    }
    let pos = probability * (sorted.len() - 1) as f64;
    let lower_idx = pos.floor() as usize;
    let upper_idx = pos.ceil() as usize;
    let weight = pos - lower_idx as f64;
    let lower = sorted[lower_idx];
    let upper = sorted[upper_idx];
    Some(lower + weight * (upper - lower))
}

pub(crate) fn sample_std(values: &[f64]) -> Option<f64> {
    if values.len() < 2 {
        return None;
    }
    let mean = mean(values)?;
    let variance = values
        .iter()
        .map(|value| {
            let centered = *value - mean;
            centered * centered
        })
        .sum::<f64>()
        / (values.len() - 1) as f64;
    Some(variance.sqrt())
}

pub(crate) fn population_std(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let mean = mean(values)?;
    let variance = values
        .iter()
        .map(|value| {
            let centered = *value - mean;
            centered * centered
        })
        .sum::<f64>()
        / values.len() as f64;
    Some(variance.sqrt())
}

#[cfg(test)]
mod tests {
    use super::{MAD_NORMAL_CONSISTENCY, PHI_INV_075};

    /// The two normal-consistency constants are reciprocals of one another and
    /// must stay that way. They previously drifted: `cross_sectional.rs` carried
    /// 0.674_490_759_476_595_2 (relative error 1.5e-6 against the true
    /// Φ⁻¹(0.75)) while `advanced.rs` carried an exact reciprocal, so the same
    /// statistic was scaled two different ways depending on which transform you
    /// called — and a golden test pinned the wrong value, freezing the defect.
    #[test]
    fn normal_consistency_constants_are_exact_reciprocals() {
        let product = PHI_INV_075 * MAD_NORMAL_CONSISTENCY;
        assert!(
            (product - 1.0).abs() < 1e-15,
            "PHI_INV_075 * MAD_NORMAL_CONSISTENCY must be 1.0, got {product}"
        );
    }

    /// Pin Φ⁻¹(0.75) against its published value so a future edit cannot
    /// reintroduce the 1.5e-6 drift.
    #[test]
    fn phi_inv_075_matches_the_standard_normal_third_quartile() {
        // scipy.stats.norm.ppf(0.75) / Python statistics.NormalDist().inv_cdf(0.75)
        assert!(
            (PHI_INV_075 - 0.674_489_750_196_081_7).abs() < 1e-16,
            "PHI_INV_075 drifted from the published Φ⁻¹(0.75)"
        );
    }
}
