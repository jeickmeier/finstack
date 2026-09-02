//! Backward-looking time-series panel transforms.

mod advanced;

use crate::types::{
    finite, op_from_str, required_f64_param, sample_std, usize_param, validate_lengths,
    ZERO_TOLERANCE,
};
use advanced::{drawdown, exponential_decay_weights, rolling_advanced, AdvancedRollingOp};
use finstack_quant_core::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::str::FromStr;

/// Supported backward-looking time-series transform operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum TimeSeriesOp {
    /// Simple return `v_t / v_{t-periods} - 1`; `None` near zero prior values.
    Returns,
    /// Log return `ln(v_t / v_{t-periods})`; `None` when the ratio is not positive.
    LogReturns,
    /// Difference `v_t - v_{t-periods}`.
    Diff,
    /// Value shifted forward by `periods`.
    Lag,
    /// Mean over the trailing window of finite observations.
    RollingMean,
    /// Sum over the trailing window of finite observations.
    RollingSum,
    /// Sample (Bessel-corrected) standard deviation; needs ≥ 2 finite points.
    RollingStd,
    /// Minimum over the trailing window.
    RollingMin,
    /// Maximum over the trailing window.
    RollingMax,
    /// Z-score of the current value against the trailing sample mean/std.
    RollingZscore,
    /// Percentile rank of the current value against the trailing window.
    RollingRank,
    /// Quantile over the trailing window (`quantile`, default `0.5`).
    RollingQuantile,
    /// Fisher G1 skewness over the trailing window; needs ≥ 3 finite points.
    RollingSkew,
    /// Fisher G2 excess kurtosis over the trailing window; needs ≥ 4 finite points.
    RollingKurtosis,
    /// Linear trend slope over the trailing window; needs ≥ 2 finite points.
    RollingSlope,
    /// Period Sharpe `(mean - risk_free) / sample_std` over the trailing window.
    ///
    /// This is a research period feature, not the annualized `analytics` Sharpe.
    /// Optional JSON `risk_free` defaults to `0.0` in the same units as the
    /// return series. No annualization.
    RollingSharpe,
    /// Clamp the current value to trailing quantile bounds.
    RollingWinsorize,
    /// Drawdown `value / running_peak - 1` for a positive level series.
    Drawdown,
    /// Replace outliers with the trailing median (Hampel filter).
    HampelFilter,
    /// Current row's normalized exponential-decay weight (`half_life` required).
    ExponentialDecayWeights,
    /// Exponentially weighted mean of a return series (`span` is pandas, not RiskMetrics `lambda`).
    EwmaMean,
    /// Exponentially weighted volatility of a return series (`span` required).
    EwmaVol,
    /// Return z-score against shared EWMA mean/variance (`span` required).
    EwmaZscore,
}

impl FromStr for TimeSeriesOp {
    type Err = Error;

    fn from_str(op: &str) -> Result<Self> {
        op_from_str(op, "time-series")
    }
}

/// Transform a value column per entity, ordered by a sortable key.
///
/// `order` is compared lexicographically within each entity. Use ISO-8601 date
/// strings or another sortable key format when passing temporal labels.
/// `window`, `periods`, `half_life`, and EWMA `span` count finite observations
/// (pandas `skipna`); missing rows do not advance decay. `drawdown` expects a
/// level series. `rolling_sharpe` is a period feature, not the `analytics`
/// Sharpe; optional JSON `risk_free` defaults to `0.0` in the same units as
/// the return series.
///
/// # Arguments
///
/// * `values` - Row-aligned observations to transform; missing and non-finite
///   values are handled by the selected time-series operation.
/// * `entity` - Row-aligned entity identifiers; each entity is transformed
///   independently.
/// * `order` - Row-aligned sortable keys that define chronological order within
///   an entity, typically ISO-8601 date strings.
/// * `op` - Canonical snake-case operation name, such as `"rolling_mean"` or
///   `"returns"`.
/// * `params` - Optional operation-specific JSON parameters; omitted keys use
///   the operation's documented defaults. `rolling_sharpe` accepts `risk_free`
///   (default `0.0`, same units as the return series).
///
/// # Errors
///
/// Returns a validation error when input lengths differ, `op` is unsupported,
/// or operation parameters are malformed.
pub fn transform_timeseries(
    values: &[Option<f64>],
    entity: &[String],
    order: &[String],
    op: &str,
    params: Option<&Value>,
) -> Result<Vec<Option<f64>>> {
    transform_timeseries_with_op(values, entity, order, TimeSeriesOp::from_str(op)?, params)
}

/// Transform a value column per entity with a typed operation.
///
/// `order` is compared lexicographically within each entity. Use ISO-8601 date
/// strings or another sortable key format when passing temporal labels.
/// Windows and EWMA spans count finite observations (pandas `skipna`).
///
/// # Arguments
///
/// * `values` - Row-aligned observations to transform; output preserves this
///   row order after processing each entity chronologically.
/// * `entity` - Row-aligned entity identifiers; length must equal `values`.
/// * `order` - Row-aligned sortable keys that establish order within each
///   entity; length must equal `values`.
/// * `op` - Typed time-series operation that determines the transform and
///   accepted parameter keys.
/// * `params` - Optional operation-specific JSON parameters; omitted keys use
///   the operation's documented defaults.
///
/// # Errors
///
/// Returns a validation error when input lengths differ or operation parameters
/// are malformed.
pub fn transform_timeseries_with_op(
    values: &[Option<f64>],
    entity: &[String],
    order: &[String],
    op: TimeSeriesOp,
    params: Option<&Value>,
) -> Result<Vec<Option<f64>>> {
    validate_lengths(
        values.len(),
        &[("entity", entity.len()), ("order", order.len())],
    )?;
    let mut output = vec![None; values.len()];
    let indices = crate::index::sorted_indices(entity, order);
    crate::index::try_for_each_entity(entity, &indices, |entity_indices| {
        transform_entity(values, entity_indices, op, params, &mut output)
    })?;
    Ok(output)
}

fn transform_entity(
    values: &[Option<f64>],
    indices: &[usize],
    op: TimeSeriesOp,
    params: Option<&Value>,
    output: &mut [Option<f64>],
) -> Result<()> {
    match op {
        TimeSeriesOp::Returns => shifted_ratio(values, indices, params, output, false),
        TimeSeriesOp::LogReturns => shifted_ratio(values, indices, params, output, true),
        TimeSeriesOp::Diff => diff(values, indices, params, output),
        TimeSeriesOp::Lag => lag(values, indices, params, output),
        TimeSeriesOp::RollingMean => rolling(values, indices, params, output, RollingOp::Mean),
        TimeSeriesOp::RollingSum => rolling(values, indices, params, output, RollingOp::Sum),
        TimeSeriesOp::RollingStd => rolling(values, indices, params, output, RollingOp::Std),
        TimeSeriesOp::RollingMin => rolling(values, indices, params, output, RollingOp::Min),
        TimeSeriesOp::RollingMax => rolling(values, indices, params, output, RollingOp::Max),
        TimeSeriesOp::RollingZscore => rolling(values, indices, params, output, RollingOp::Zscore),
        TimeSeriesOp::RollingRank => {
            rolling_advanced(values, indices, params, output, AdvancedRollingOp::Rank)
        }
        TimeSeriesOp::RollingQuantile => {
            rolling_advanced(values, indices, params, output, AdvancedRollingOp::Quantile)
        }
        TimeSeriesOp::RollingSkew => {
            rolling_advanced(values, indices, params, output, AdvancedRollingOp::Skew)
        }
        TimeSeriesOp::RollingKurtosis => {
            rolling_advanced(values, indices, params, output, AdvancedRollingOp::Kurtosis)
        }
        TimeSeriesOp::RollingSlope => {
            rolling_advanced(values, indices, params, output, AdvancedRollingOp::Slope)
        }
        TimeSeriesOp::RollingSharpe => {
            rolling_advanced(values, indices, params, output, AdvancedRollingOp::Sharpe)
        }
        TimeSeriesOp::RollingWinsorize => rolling_advanced(
            values,
            indices,
            params,
            output,
            AdvancedRollingOp::Winsorize,
        ),
        TimeSeriesOp::Drawdown => drawdown(values, indices, output),
        TimeSeriesOp::HampelFilter => {
            rolling_advanced(values, indices, params, output, AdvancedRollingOp::Hampel)
        }
        TimeSeriesOp::ExponentialDecayWeights => {
            exponential_decay_weights(values, indices, params, output)
        }
        TimeSeriesOp::EwmaMean => ewma_mean(values, indices, params, output),
        TimeSeriesOp::EwmaVol => ewma_vol(values, indices, params, output),
        TimeSeriesOp::EwmaZscore => ewma_zscore(values, indices, params, output),
    }
}

fn shifted_ratio(
    values: &[Option<f64>],
    indices: &[usize],
    params: Option<&Value>,
    output: &mut [Option<f64>],
    log_return: bool,
) -> Result<()> {
    let periods = usize_param(params, "periods", 1)?;
    for (pos, &idx) in indices.iter().enumerate() {
        if pos < periods {
            output[idx] = None;
            continue;
        }
        let current = finite(values[idx]);
        let previous = finite(values[indices[pos - periods]]);
        output[idx] = match (current, previous) {
            (Some(current), Some(previous)) if previous.abs() > ZERO_TOLERANCE => {
                let ratio = current / previous;
                if log_return {
                    if ratio > 0.0 {
                        Some(ratio.ln())
                    } else {
                        None
                    }
                } else {
                    Some(ratio - 1.0)
                }
            }
            _ => None,
        };
    }
    Ok(())
}

fn lag(
    values: &[Option<f64>],
    indices: &[usize],
    params: Option<&Value>,
    output: &mut [Option<f64>],
) -> Result<()> {
    let periods = usize_param(params, "periods", 1)?;
    for (pos, &idx) in indices.iter().enumerate() {
        output[idx] = if pos < periods {
            None
        } else {
            finite(values[indices[pos - periods]])
        };
    }
    Ok(())
}

fn diff(
    values: &[Option<f64>],
    indices: &[usize],
    params: Option<&Value>,
    output: &mut [Option<f64>],
) -> Result<()> {
    let periods = usize_param(params, "periods", 1)?;
    for (pos, &idx) in indices.iter().enumerate() {
        if pos < periods {
            output[idx] = None;
            continue;
        }
        output[idx] = match (finite(values[idx]), finite(values[indices[pos - periods]])) {
            (Some(current), Some(previous)) => Some(current - previous),
            _ => None,
        };
    }
    Ok(())
}

#[derive(Clone, Copy)]
enum RollingOp {
    Mean,
    Sum,
    Std,
    Min,
    Max,
    Zscore,
}

fn rolling(
    values: &[Option<f64>],
    indices: &[usize],
    params: Option<&Value>,
    output: &mut [Option<f64>],
    op: RollingOp,
) -> Result<()> {
    let window = usize_param(params, "window", 1)?;
    let min_periods = usize_param(params, "min_periods", window)?;
    let required = match op {
        RollingOp::Std | RollingOp::Zscore => min_periods.max(2),
        _ => min_periods,
    };
    crate::index::try_for_each_trailing_window(indices, window, |idx, window_indices| {
        let finite_values = window_indices
            .iter()
            .filter_map(|window_idx| finite(values[*window_idx]))
            .collect::<Vec<_>>();
        if finite_values.len() < required {
            output[idx] = None;
            return Ok(());
        }
        output[idx] = match op {
            RollingOp::Mean => Some(finite_values.iter().sum::<f64>() / finite_values.len() as f64),
            RollingOp::Sum => Some(finite_values.iter().sum()),
            RollingOp::Std => sample_std(&finite_values),
            RollingOp::Min => finite_values.into_iter().reduce(f64::min),
            RollingOp::Max => finite_values.into_iter().reduce(f64::max),
            RollingOp::Zscore => {
                let current = finite(values[idx]);
                let mean = finite_values.iter().sum::<f64>() / finite_values.len() as f64;
                let std = sample_std(&finite_values);
                match (current, std) {
                    (Some(current), Some(std)) if std > ZERO_TOLERANCE => {
                        Some((current - mean) / std)
                    }
                    (Some(_), Some(_)) => Some(0.0),
                    _ => None,
                }
            }
        };
        Ok(())
    })
}

fn ewma_alpha(params: Option<&Value>) -> Result<f64> {
    let span = required_f64_param(params, "span")?;
    if span <= 0.0 {
        return Err(Error::Validation(
            "panel transform parameter 'span' must be positive".to_string(),
        ));
    }
    Ok(2.0 / (span + 1.0))
}

/// Shared pandas `adjust=False` EWMA mean/variance after one finite return.
#[derive(Clone, Copy)]
struct EwmaState {
    mean: f64,
    variance: f64,
}

impl EwmaState {
    fn first(value: f64) -> Self {
        Self {
            mean: value,
            variance: 0.0,
        }
    }

    fn update(self, value: f64, alpha: f64) -> Self {
        let diff = value - self.mean;
        Self {
            mean: self.mean + alpha * diff,
            variance: (1.0 - alpha) * (self.variance + alpha * diff * diff),
        }
    }

    fn vol(self) -> Option<f64> {
        if self.variance > ZERO_TOLERANCE {
            Some(self.variance.sqrt())
        } else {
            None
        }
    }

    fn zscore(self, value: f64) -> f64 {
        match self.vol() {
            Some(vol) => (value - self.mean) / vol,
            None => 0.0,
        }
    }
}

fn ewma_step(state: Option<EwmaState>, value: f64, alpha: f64) -> EwmaState {
    match state {
        Some(prev) => prev.update(value, alpha),
        None => EwmaState::first(value),
    }
}

fn ewma_mean(
    values: &[Option<f64>],
    indices: &[usize],
    params: Option<&Value>,
    output: &mut [Option<f64>],
) -> Result<()> {
    let alpha = ewma_alpha(params)?;
    let mut state = None;
    for &idx in indices {
        output[idx] = match finite(values[idx]) {
            Some(value) => {
                state = Some(ewma_step(state, value, alpha));
                state.map(|next| next.mean)
            }
            None => None,
        };
    }
    Ok(())
}

fn ewma_vol(
    values: &[Option<f64>],
    indices: &[usize],
    params: Option<&Value>,
    output: &mut [Option<f64>],
) -> Result<()> {
    let alpha = ewma_alpha(params)?;
    let mut state = None;
    for &idx in indices {
        output[idx] = match finite(values[idx]) {
            Some(value) => {
                state = Some(ewma_step(state, value, alpha));
                state.and_then(EwmaState::vol)
            }
            None => None,
        };
    }
    Ok(())
}

fn ewma_zscore(
    values: &[Option<f64>],
    indices: &[usize],
    params: Option<&Value>,
    output: &mut [Option<f64>],
) -> Result<()> {
    let alpha = ewma_alpha(params)?;
    let mut state = None;
    for &idx in indices {
        output[idx] = match finite(values[idx]) {
            Some(value) => {
                state = Some(ewma_step(state, value, alpha));
                state.map(|next| next.zscore(value))
            }
            None => None,
        };
    }
    Ok(())
}
