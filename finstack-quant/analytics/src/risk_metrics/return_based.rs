//! Return-based risk metrics: mean, volatility, Sharpe, Sortino, CAGR, and more.
//!
//! This module contains crate-internal building blocks for [`crate::Performance`].
//!
//! Most functions operate on `&[f64]` return slices and return scalar `f64`.
//! Annualization uses the caller-supplied factor (typically from
//! `PeriodKind::annualization_factor()`).

use crate::dates::{Date, DayCount, DayCountContext, HolidayCalendar};
use crate::math::stats::{mean, mean_var, variance};
use crate::math::summation::kahan_sum;

use super::tail_risk::cornish_fisher_var;

/// True when annualization is requested but `ann_factor` is not a positive finite
/// periods-per-year count (e.g. zero, negative, NaN, or infinity).
///
/// Shared analytics-wide guard; re-exported via [`crate::risk_metrics`] and used
/// from benchmark-relative metrics to avoid redefining the same check.
#[inline]
pub(crate) fn invalid_annualization_factor(annualize: bool, ann_factor: f64) -> bool {
    annualize && (!ann_factor.is_finite() || ann_factor <= 0.0)
}

/// Day-count convention for CAGR annualization over explicit calendar dates.
///
/// [`CagrDayCount::Act365_25`] is the default used by [`crate::Performance::cagr`].
/// [`CagrDayCount::DayCount`] wraps any core
/// [`finstack_quant_core::dates::DayCount`] (Act/365F, Act/Act, Bus/252, …).
/// `Bus252` requires a holiday calendar on the facade; missing calendar is
/// an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CagrDayCount {
    /// Actual calendar days divided by 365.25 (default).
    #[default]
    Act365_25,
    /// Any core day-count convention.
    DayCount(DayCount),
}

/// Basis used to annualize CAGR from either explicit dates or a periods-per-year factor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum CagrBasis {
    /// Annualize across an explicit calendar range using the chosen convention.
    Dates {
        /// Inclusive start date of the return span.
        start: Date,
        /// Inclusive end date of the return span.
        end: Date,
        /// Day-count convention used to convert the span to a year fraction.
        day_count: CagrDayCount,
    },
    /// Annualize from a periods-per-year factor such as 252 (daily) or 12 (monthly).
    #[cfg(test)]
    Factor(f64),
}

impl CagrBasis {
    /// Build a date-based CAGR basis using the default Act/365.25 convention.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn dates(start: Date, end: Date) -> Self {
        Self::Dates {
            start,
            end,
            day_count: CagrDayCount::default(),
        }
    }

    /// Build a date-based CAGR basis with an explicit day-count convention.
    #[must_use]
    pub(crate) fn dates_with(start: Date, end: Date, day_count: CagrDayCount) -> Self {
        Self::Dates {
            start,
            end,
            day_count,
        }
    }

    /// Build a factor-based CAGR basis from periods per year.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn factor(ann_factor: f64) -> Self {
        Self::Factor(ann_factor)
    }
}

/// Compound annual growth rate from a return series using the supplied basis.
///
/// Computes:
///
/// ```text
/// CAGR = (Π(1 + r_i))^(1/years) - 1
/// ```
///
/// where `years` comes either from an explicit date range or from a
/// periods-per-year factor, depending on `basis`.
///
/// # Arguments
///
/// * `returns`    - Slice of simple period returns.
/// * `basis`      - How to annualize the compounded return.
/// * `calendar`   - Holiday calendar required when `basis` uses
///   [`CagrDayCount::DayCount`] with [`DayCount::Bus252`]. Ignored for
///   Act/365.25 and for day-counts that do not need a calendar.
///
/// # Returns
///
/// Annualized growth rate as a decimal.
///
/// # Errors
///
/// Returns [`crate::error::InputError::Invalid`] when `returns` is empty, a
/// date basis has a non-positive span, or a factor basis uses a non-positive or
/// non-finite annualization factor. Propagates
/// [`crate::error::InputError::MissingCalendarForBus252`] when Bus/252 is
/// requested without a calendar.
pub(crate) fn cagr(
    returns: &[f64],
    basis: CagrBasis,
    calendar: Option<&dyn HolidayCalendar>,
) -> crate::Result<f64> {
    if returns.is_empty() {
        tracing::debug!(reason = "empty_returns", "invalid CAGR input");
        return Err(crate::error::InputError::Invalid.into());
    }

    match basis {
        CagrBasis::Dates {
            start,
            end,
            day_count,
        } => cagr_from_dates(returns, start, end, day_count, calendar),
        #[cfg(test)]
        CagrBasis::Factor(ann_factor) => cagr_from_factor(returns, ann_factor),
    }
}
fn cagr_from_dates(
    returns: &[f64],
    start: Date,
    end: Date,
    day_count: CagrDayCount,
    calendar: Option<&dyn HolidayCalendar>,
) -> crate::Result<f64> {
    let total = 1.0 + crate::returns::comp_total(returns);
    let years = annualized_years(start, end, day_count, calendar)?;
    if years <= 0.0 {
        tracing::debug!(
            ?start,
            ?end,
            ?day_count,
            reason = "non_positive_date_span",
            "invalid CAGR input"
        );
        return Err(crate::error::InputError::Invalid.into());
    }
    Ok(total.powf(1.0 / years) - 1.0)
}

#[cfg(test)]
fn cagr_from_factor(returns: &[f64], ann_factor: f64) -> crate::Result<f64> {
    if !ann_factor.is_finite() || ann_factor <= 0.0 {
        tracing::debug!(
            ann_factor,
            reason = "invalid_annualization_factor",
            "invalid CAGR input"
        );
        return Err(crate::error::InputError::Invalid.into());
    }
    let total = 1.0 + crate::returns::comp_total(returns);
    let years = returns.len() as f64 / ann_factor;
    if years > 0.0 {
        Ok(total.powf(1.0 / years) - 1.0)
    } else {
        Err(crate::error::InputError::Invalid.into())
    }
}

fn annualized_years(
    start: Date,
    end: Date,
    day_count: CagrDayCount,
    calendar: Option<&dyn HolidayCalendar>,
) -> crate::Result<f64> {
    match day_count {
        CagrDayCount::Act365_25 => {
            let days = (end - start).whole_days() as f64;
            Ok(if days <= 0.0 { 0.0 } else { days / 365.25 })
        }
        CagrDayCount::DayCount(convention) => {
            let ctx = DayCountContext {
                calendar,
                ..DayCountContext::default()
            };
            convention.year_fraction(start, end, ctx)
        }
    }
}

/// Mean return, optionally annualized.
///
/// Computes the **arithmetic** mean of `returns`. When `annualize` is `true`,
/// that mean is scaled by `ann_factor` (e.g., 252 for daily data):
///
/// ```text
/// μ_ann = μ_period × ann_factor
/// ```
///
/// This is **simple** annualization of the average **per-period** return, not a
/// compounded (geometric) annual return. For growth over time that compounds
/// period returns, use [`cagr`]. Volatility in this
/// module uses the usual root-time rule (`σ_ann = σ_period × √ann_factor`); mean
/// return uses **linear** scaling instead.
///
/// # Arguments
///
/// * `returns`    - Slice of period simple returns.
/// * `annualize`  - Whether to multiply the mean by `ann_factor`.
/// * `ann_factor` - Number of periods per year (e.g., 252 daily, 12 monthly).
///
/// # Returns
///
/// Arithmetic mean return, annualized if requested. Returns `0.0` for an
/// empty slice. When `annualize` is `true`, returns [`f64::NAN`] if `ann_factor`
/// is not finite or is `<= 0`.
#[must_use]
pub(crate) fn mean_return(returns: &[f64], annualize: bool, ann_factor: f64) -> f64 {
    if invalid_annualization_factor(annualize, ann_factor) {
        return f64::NAN;
    }
    let m = mean(returns);
    if annualize {
        m * ann_factor
    } else {
        m
    }
}

/// Annualized mean and volatility from one Welford pass.
///
/// Equivalent to `(mean_return(returns, true, ann_factor),
/// volatility(returns, true, ann_factor))` but walks the slice once instead
/// of twice. Used by callers (e.g. Sharpe / M²) that need both.
///
/// # Arguments
///
/// * `returns`    - Slice of period simple returns.
/// * `ann_factor` - Number of periods per year (e.g., 252 daily, 12 monthly).
///
/// # Returns
///
/// `(annualized_mean, annualized_volatility)`. Returns `(NaN, NaN)` for the
/// same invalid annualization-factor cases as the individual functions.
#[must_use]
pub(crate) fn mean_vol_annualized(returns: &[f64], ann_factor: f64) -> (f64, f64) {
    if invalid_annualization_factor(true, ann_factor) {
        return (f64::NAN, f64::NAN);
    }
    let (m, var) = mean_var(returns);
    (m * ann_factor, var.sqrt() * ann_factor.sqrt())
}

/// Volatility (standard deviation of returns), optionally annualized.
///
/// Uses **sample** standard deviation (n-1 denominator), consistent with
/// Bloomberg, QuantLib, and the `OnlineStats::variance()` convention.
/// Annualizes by multiplying by `sqrt(ann_factor)` following the
/// square-root-of-time rule.
///
/// # Arguments
///
/// * `returns`    - Slice of period simple returns.
/// * `annualize`  - Whether to scale by `sqrt(ann_factor)`.
/// * `ann_factor` - Number of periods per year (e.g., 252 daily, 12 monthly).
///
/// # Returns
///
/// Sample standard deviation of `returns` (n-1 denominator), annualized if requested.
/// Returns `0.0` for an empty slice. When `annualize` is `true`, returns
/// [`f64::NAN`] if `ann_factor` is not finite or is `<= 0`.
#[must_use]
pub(crate) fn volatility(returns: &[f64], annualize: bool, ann_factor: f64) -> f64 {
    if invalid_annualization_factor(annualize, ann_factor) {
        return f64::NAN;
    }
    let v = variance(returns).sqrt();
    if annualize {
        v * ann_factor.sqrt()
    } else {
        v
    }
}
/// Sharpe ratio = annualized excess return / annualized volatility.
///
/// The annualized risk-free rate is geometrically decompounded to the
/// observation frequency before subtraction from the period arithmetic
/// mean, then the excess is scaled by `ann_factor`:
///
/// ```text
/// rf_period = (1 + rf_annual)^{1/N} − 1
/// excess_ann = (μ − rf_period) × N
/// Sharpe = excess_ann / σ_ann
/// ```
///
/// # Arguments
///
/// * `ann_return`     - Linearly annualized arithmetic mean (`μ × N`).
/// * `ann_vol`        - Annualized portfolio volatility.
/// * `risk_free_rate` - Annualized risk-free rate (e.g., `0.02` for 2%).
/// * `ann_factor`     - Periods per year `N` used to decompound
///   `risk_free_rate`. Pass `1.0` when `ann_return` and `risk_free_rate`
///   are already in the same (annual) units.
///
/// # Returns
///
/// The Sharpe ratio. When `ann_vol` is zero: returns `f64::INFINITY` if
/// excess return is positive, `f64::NEG_INFINITY` if negative, and `0.0`
/// if both are zero (matching [`sortino`] convention).
///
/// # References
///
/// - Sharpe (1966): see docs/REFERENCES.md#sharpe1966
#[must_use]
pub(crate) fn sharpe(ann_return: f64, ann_vol: f64, risk_free_rate: f64, ann_factor: f64) -> f64 {
    let excess = crate::returns::annualized_excess_return(ann_return, risk_free_rate, ann_factor);
    if ann_vol == 0.0 {
        return if excess > 0.0 {
            f64::INFINITY
        } else if excess < 0.0 {
            f64::NEG_INFINITY
        } else {
            0.0
        };
    }
    excess / ann_vol
}

/// Downside deviation: semi-standard deviation below a minimum acceptable return.
///
/// Computes the root-mean-square of returns falling below `mar`, using
/// the full series length as the denominator (population convention),
/// consistent with Sortino & van der Meer (1991):
///
/// ```text
/// DD = sqrt( (1/n) × Σ min(r_i − MAR, 0)² )
/// ```
///
/// # Arguments
///
/// * `returns`    - Slice of period simple returns.
/// * `mar`        - Minimum acceptable return (threshold). Use `0.0` for
///   the standard Sortino definition.
/// * `annualize`  - Whether to scale by `sqrt(ann_factor)`.
/// * `ann_factor` - Number of periods per year.
///
/// # Returns
///
/// The downside deviation (non-negative). Returns `0.0` for an empty
/// slice or when no returns fall below `mar`. When `annualize` is `true`,
/// returns [`f64::NAN`] if `ann_factor` is not finite or is `<= 0`.
///
/// # References
///
/// - Sortino & van der Meer (1991): see docs/REFERENCES.md#sortinoVanDerMeer1991
#[must_use]
pub(crate) fn downside_deviation(
    returns: &[f64],
    mar: f64,
    annualize: bool,
    ann_factor: f64,
) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    if invalid_annualization_factor(annualize, ann_factor) {
        return f64::NAN;
    }
    let downside_sq = kahan_sum(returns.iter().filter(|&&r| r < mar).map(|&r| {
        let d = r - mar;
        d * d
    }));
    let dd = (downside_sq / returns.len() as f64).sqrt();
    if annualize {
        dd * ann_factor.sqrt()
    } else {
        dd
    }
}
/// Sortino ratio: penalises only downside volatility.
///
/// Unlike the Sharpe ratio, the Sortino ratio uses the **downside deviation**
/// (semi-standard deviation of negative returns) as the risk denominator,
/// leaving upside volatility unrewarded:
///
/// ```text
/// Sortino = (annualized mean return) / (annualized downside deviation)
/// ```
///
/// Downside deviation is computed over the full return series (denominator
/// is `n`, not the number of negative observations), consistent with the
/// Sortino & van der Meer (1991) definition.
///
/// # Arguments
///
/// * `returns` - Slice of period simple returns.
/// * `annualize` - Whether to annualize both numerator and denominator.
/// * `ann_factor` - Number of periods per year.
/// * `mar` - Minimum acceptable return per period in decimal form.
///
/// # Returns
///
/// The Sortino ratio. Returns `±∞` when the mean is nonzero but there
/// are no negative returns (zero downside risk), and `0.0` when the
/// mean is zero or the downside deviation is zero. When `annualize` is
/// `true`, returns [`f64::NAN`] if `ann_factor` is not finite or is `<= 0`.
///
/// # References
///
/// - Sortino & van der Meer (1991): see docs/REFERENCES.md#sortinoVanDerMeer1991
#[must_use]
pub(crate) fn sortino(returns: &[f64], annualize: bool, ann_factor: f64, mar: f64) -> f64 {
    if invalid_annualization_factor(annualize, ann_factor) {
        return f64::NAN;
    }
    let excess_mean = mean(returns) - mar;
    let dd = downside_deviation(returns, mar, false, ann_factor);
    if dd == 0.0 {
        return if excess_mean > 0.0 {
            f64::INFINITY
        } else if excess_mean < 0.0 {
            f64::NEG_INFINITY
        } else {
            0.0
        };
    }
    if annualize {
        (excess_mean * ann_factor) / (dd * ann_factor.sqrt())
    } else {
        excess_mean / dd
    }
}
/// Geometric mean return per period.
///
/// The compound-average return: the constant per-period return that
/// would produce the same terminal wealth as the actual series.
///
/// ```text
/// geo_mean = (Π(1 + r_i))^(1/n) − 1
/// ```
///
/// Computed in log-space with Kahan summation for numerical stability.
/// Returns [`f64::NEG_INFINITY`] if any return is `<= -1.0`, which
/// represents a full wipeout (or worse) and avoids the upward bias that
/// a positive clamp would introduce near total loss.
///
/// # Arguments
///
/// * `returns` - Slice of period simple returns.
///
/// # Returns
///
/// The geometric mean return. Returns [`f64::NAN`] for an empty slice.
#[must_use]
pub(crate) fn geometric_mean(returns: &[f64]) -> f64 {
    if returns.is_empty() {
        return f64::NAN;
    }
    let mut saw_total_wipeout = false;
    for &r in returns {
        if r < -1.0 {
            return f64::NEG_INFINITY;
        }
        if (r + 1.0).abs() < f64::EPSILON {
            saw_total_wipeout = true;
        }
    }
    if saw_total_wipeout {
        return -1.0;
    }
    let n = returns.len() as f64;
    let log_sum = kahan_sum(returns.iter().map(|&r| (1.0 + r).ln()));
    (log_sum / n).exp() - 1.0
}

/// Omega ratio: probability-weighted gain-to-loss ratio above a threshold.
///
/// ```text
/// Ω(L) = Σ max(r_i − L, 0) / Σ max(L − r_i, 0)
/// ```
///
/// Unlike the Sharpe ratio (which uses only mean and variance), the Omega
/// ratio incorporates the full return distribution.
///
/// # Arguments
///
/// * `returns`   - Slice of period simple returns.
/// * `threshold` - Return threshold (typically `0.0`).
///
/// # Returns
///
/// The Omega ratio. Returns `f64::INFINITY` if gains exist but no losses,
/// `1.0` if all returns equal the threshold (neutral outcome per
/// Keating-Shadwick), and [`f64::NAN`] for an empty slice.
///
/// # References
///
/// - Keating & Shadwick (2002): see docs/REFERENCES.md#keatingShadwick2002
#[must_use]
pub(crate) fn omega_ratio(returns: &[f64], threshold: f64) -> f64 {
    if returns.is_empty() {
        return f64::NAN;
    }
    let mut gains = 0.0_f64;
    let mut losses = 0.0_f64;
    for &r in returns {
        if r > threshold {
            gains += r - threshold;
        } else {
            losses += threshold - r;
        }
    }
    if losses == 0.0 {
        return if gains > 0.0 { f64::INFINITY } else { 1.0 };
    }
    gains / losses
}

/// Gain-to-pain ratio: total return divided by total losses.
///
/// ```text
/// GtP = Σ r_i / Σ |r_i| for r_i < 0
/// ```
///
/// Popular among CTA and systematic macro managers as a simple
/// measure of return efficiency relative to the pain of drawdowns.
///
/// # Arguments
///
/// * `returns` - Slice of period simple returns.
///
/// # Returns
///
/// The gain-to-pain ratio. Returns `f64::INFINITY` when total return is
/// positive but there are no losses, and [`f64::NAN`] for an empty slice.
///
/// # References
///
/// - Schwager (2012): see docs/REFERENCES.md#schwager2012
#[must_use]
pub(crate) fn gain_to_pain(returns: &[f64]) -> f64 {
    if returns.is_empty() {
        return f64::NAN;
    }
    let total: f64 = kahan_sum(returns.iter().copied());
    let abs_losses: f64 = kahan_sum(returns.iter().filter(|&&r| r < 0.0).map(|&r| r.abs()));
    if abs_losses == 0.0 {
        return if total > 0.0 { f64::INFINITY } else { 0.0 };
    }
    total / abs_losses
}

/// Modified Sharpe ratio: excess return divided by Cornish-Fisher VaR.
///
/// Replaces the standard deviation in the Sharpe denominator with the
/// Cornish-Fisher adjusted VaR, accounting for skewness and kurtosis.
/// Excess return uses the same geometric rf decompounding as [`sharpe`]:
///
/// ```text
/// Modified Sharpe = ((μ − rf_period) × N) / |CF-VaR|
/// ```
///
/// # Arguments
///
/// * `returns`        - Slice of period simple returns.
/// * `risk_free_rate` - Annualized risk-free rate in decimal form.
/// * `confidence`     - VaR confidence level (e.g., `0.95`).
/// * `ann_factor`     - Number of periods per year used to decompound
///   `risk_free_rate` and to annualize the arithmetic mean.
///
/// # Returns
///
/// The Modified Sharpe ratio. Returns `0.0` for empty slices and
/// [`f64::NAN`] when the Cornish-Fisher VaR is unexpectedly non-negative.
///
/// # References
///
/// - Gregoriou & Gueyie (2003): see docs/REFERENCES.md#gregoriou2003
#[must_use]
pub(crate) fn modified_sharpe(
    returns: &[f64],
    risk_free_rate: f64,
    confidence: f64,
    ann_factor: f64,
) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }
    let excess_return = crate::returns::annualized_excess_return(
        mean_return(returns, true, ann_factor),
        risk_free_rate,
        ann_factor,
    );
    let cf_var = cornish_fisher_var(returns, confidence, Some(ann_factor));
    if cf_var >= 0.0 {
        return f64::NAN;
    }
    excess_return / cf_var.abs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::Month;
    use crate::math::stats::{mean, variance};

    fn jan1(year: i32) -> crate::dates::Date {
        crate::dates::Date::from_calendar_date(year, Month::January, 1).expect("valid date")
    }

    #[test]
    fn cagr_basic() {
        let r = [0.10];
        let c = cagr(&r, CagrBasis::dates(jan1(2024), jan1(2025)), None).expect("valid CAGR");
        assert!((c - 0.10).abs() < 0.01);
    }

    #[test]
    fn cagr_with_act_365_fixed() {
        let r = [0.10];
        let c = cagr(
            &r,
            CagrBasis::dates_with(
                jan1(2024),
                jan1(2025),
                CagrDayCount::DayCount(DayCount::Act365F),
            ),
            None,
        )
        .expect("valid CAGR");
        assert!((c - 0.09971358593414137).abs() < 1.0e-12);
    }

    #[test]
    fn cagr_default_convention_is_act_365_25() {
        let r = [0.10];
        let c_default =
            cagr(&r, CagrBasis::dates(jan1(2024), jan1(2025)), None).expect("valid CAGR");
        let c_fixed = cagr(
            &r,
            CagrBasis::dates_with(
                jan1(2024),
                jan1(2025),
                CagrDayCount::DayCount(DayCount::Act365F),
            ),
            None,
        )
        .expect("valid CAGR");
        assert!(c_default > c_fixed);
        assert!((c_default - 0.09978518245839707).abs() < 1.0e-12);
    }

    #[test]
    fn cagr_bus252_without_calendar_is_err() {
        let r = [0.10];
        let err = cagr(
            &r,
            CagrBasis::dates_with(
                jan1(2024),
                jan1(2025),
                CagrDayCount::DayCount(DayCount::Bus252),
            ),
            None,
        )
        .expect_err("Bus252 requires a calendar");
        let msg = err.to_string();
        assert!(
            msg.to_lowercase().contains("calendar") || msg.to_lowercase().contains("bus"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn mean_return_volatility_nan_when_annualized_with_invalid_factor() {
        let r = [0.01_f64, 0.02];
        assert!(mean_return(&r, true, 0.0).is_nan());
        assert!(mean_return(&r, true, -1.0).is_nan());
        assert!(mean_return(&r, true, f64::NAN).is_nan());
        assert!(volatility(&r, true, 0.0).is_nan());
        assert!(volatility(&r, true, f64::INFINITY).is_nan());
    }

    #[test]
    fn downside_deviation_and_sortino_nan_when_annualized_with_invalid_factor() {
        let r = [0.01_f64, -0.02, 0.03];
        assert!(downside_deviation(&r, 0.0, true, 0.0).is_nan());
        assert!(sortino(&r, true, 0.0, 0.0).is_nan());
    }

    #[test]
    fn cagr_factor_basis_rejects_bad_ann_factor() {
        assert!(cagr(&[0.01, 0.02], CagrBasis::factor(0.0), None).is_err());
        assert!(cagr(&[0.01, 0.02], CagrBasis::factor(-1.0), None).is_err());
        assert!(cagr(&[0.01, 0.02], CagrBasis::factor(f64::NAN), None).is_err());
    }

    #[test]
    fn cagr_factor_basis_accepts_single_period() {
        assert!(
            (cagr(&[0.10], CagrBasis::factor(1.0), None).expect("valid CAGR") - 0.10).abs()
                < 1.0e-12
        );
    }

    #[test]
    fn cagr_date_basis_rejects_non_positive_spans() {
        let returns = [0.10];

        assert!(cagr(&returns, CagrBasis::dates(jan1(2024), jan1(2024)), None).is_err());
        assert!(cagr(&returns, CagrBasis::dates(jan1(2025), jan1(2024)), None).is_err());
    }

    #[test]
    fn mean_return_annualized_scales_linearly_not_compounded() {
        let r = [0.01, 0.02, 0.03];
        let m_ann = mean_return(&r, true, 252.0);
        let mean_p = mean(&r);
        assert!((m_ann - mean_p * 252.0).abs() < 1e-10);
        let cagr_ann = cagr(&r, CagrBasis::factor(252.0), None).expect("valid CAGR");
        assert!(
            cagr_ann.is_finite() && (m_ann - cagr_ann).abs() > 1e-6,
            "arithmetic annualized mean should differ from compounded cagr"
        );
    }

    #[test]
    fn sharpe_basic() {
        assert!((sharpe(0.10, 0.15, 0.0, 1.0) - 0.6666).abs() < 0.01);
        assert_eq!(sharpe(0.10, 0.0, 0.0, 1.0), f64::INFINITY);
        assert_eq!(sharpe(-0.05, 0.0, 0.0, 1.0), f64::NEG_INFINITY);
        assert_eq!(sharpe(0.02, 0.0, 0.02, 1.0), 0.0);
    }

    #[test]
    fn sharpe_with_risk_free_rate() {
        assert!((sharpe(0.10, 0.15, 0.02, 1.0) - 0.5333).abs() < 0.01);
    }

    #[test]
    fn sharpe_daily_excess_uses_geometric_rf() {
        let mu = 0.0004_f64;
        let ann_factor = 252.0;
        let rf_annual = 0.02;
        let ann_return = mu * ann_factor;
        let ann_vol = 0.15;
        let rf_period = 1.02_f64.powf(1.0 / ann_factor) - 1.0;
        let expected_excess = (mu - rf_period) * ann_factor;
        let linear_excess = ann_return - rf_annual;
        assert!(
            (expected_excess - linear_excess).abs() > 1e-6,
            "geometric and linear rf subtraction must differ at daily frequency"
        );
        let s = sharpe(ann_return, ann_vol, rf_annual, ann_factor);
        assert!((s * ann_vol - expected_excess).abs() < 1e-12);
        assert!((s * ann_vol - linear_excess).abs() > 1e-6);
    }

    #[test]
    fn sortino_positive_returns() {
        let r = [0.01, 0.02, 0.03, -0.005, 0.01];
        let s = sortino(&r, false, 252.0, 0.0);
        assert!(s > 0.0);
    }

    #[test]
    fn downside_deviation_hand_calc() {
        let r = [0.01, -0.02, 0.03, -0.01, 0.005];
        let dd = downside_deviation(&r, 0.0, false, 252.0);
        assert!((dd - 0.01).abs() < 1e-14);
    }

    #[test]
    fn downside_deviation_annualized() {
        let r = [0.01, -0.02, 0.03, -0.01, 0.005];
        let dd_raw = downside_deviation(&r, 0.0, false, 252.0);
        let dd_ann = downside_deviation(&r, 0.0, true, 252.0);
        assert!((dd_ann - dd_raw * 252.0_f64.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn downside_deviation_all_positive() {
        let dd = downside_deviation(&[0.01, 0.02, 0.03], 0.0, false, 252.0);
        assert_eq!(dd, 0.0);
    }

    #[test]
    fn downside_deviation_empty() {
        assert_eq!(downside_deviation(&[], 0.0, false, 252.0), 0.0);
    }

    #[test]
    fn downside_deviation_with_mar() {
        let r = [0.01, 0.02, 0.03, 0.005];
        let dd = downside_deviation(&r, 0.02, false, 252.0);
        let expected = (0.000325_f64 / 4.0).sqrt();
        assert!((dd - expected).abs() < 1e-14);
    }

    #[test]
    fn sortino_consistent_with_downside_deviation() {
        let r = [0.01, 0.02, 0.03, -0.005, 0.01];
        let m = mean(&r);
        let dd = downside_deviation(&r, 0.0, false, 252.0);
        let s = sortino(&r, false, 252.0, 0.0);
        assert!((s - m / dd).abs() < 1e-12);
    }

    #[test]
    fn sortino_respects_mar_in_numerator_and_denominator() {
        let r = [0.01, 0.02, 0.03, 0.04];
        let mar = 0.02;
        let expected = (mean(&r) - mar) / downside_deviation(&r, mar, false, 252.0);
        let actual = sortino(&r, false, 252.0, mar);
        assert!((actual - expected).abs() < 1e-12);
    }

    #[test]
    fn geometric_mean_constant() {
        let gm = geometric_mean(&[0.05, 0.05, 0.05]);
        assert!((gm - 0.05).abs() < 1e-12);
    }

    #[test]
    fn geometric_mean_volatility_drag_exact() {
        let gm = geometric_mean(&[0.10, -0.10]);
        let expected = 0.99_f64.sqrt() - 1.0;
        assert!((gm - expected).abs() < 1e-12);
    }

    #[test]
    fn geometric_mean_empty() {
        assert!(geometric_mean(&[]).is_nan());
    }

    #[test]
    fn geometric_mean_total_wipeout_returns_minus_one() {
        assert_eq!(geometric_mean(&[0.10, -1.0]), -1.0);
        assert_eq!(geometric_mean(&[-1.5]), f64::NEG_INFINITY);
    }

    #[test]
    fn geometric_mean_less_than_arithmetic() {
        let r = [0.05, 0.10, -0.03, 0.08];
        let gm = geometric_mean(&r);
        let am = mean(&r);
        assert!(gm < am);
    }

    #[test]
    fn omega_ratio_hand_calc() {
        let r = [0.05, -0.02, 0.03, -0.01, 0.04];
        let omega = omega_ratio(&r, 0.0);
        assert!((omega - 4.0).abs() < 1e-12);
    }

    #[test]
    fn omega_ratio_no_losses() {
        assert_eq!(omega_ratio(&[0.01, 0.02, 0.03], 0.0), f64::INFINITY);
    }

    #[test]
    fn omega_ratio_empty() {
        assert!(omega_ratio(&[], 0.0).is_nan());
    }

    #[test]
    fn gain_to_pain_hand_calc() {
        let r = [0.05, -0.02, 0.03, -0.01, 0.04];
        let gtp = gain_to_pain(&r);
        assert!((gtp - 3.0).abs() < 1e-12);
    }

    #[test]
    fn gain_to_pain_no_losses() {
        assert_eq!(gain_to_pain(&[0.01, 0.02]), f64::INFINITY);
    }

    #[test]
    fn gain_to_pain_empty() {
        assert!(gain_to_pain(&[]).is_nan());
    }

    #[test]
    fn modified_sharpe_is_finite_when_cf_var_is_a_loss() {
        let r = [-0.06, -0.03, -0.02, 0.01, 0.02, 0.025, 0.03, 0.04];
        let ms = modified_sharpe(&r, 0.02, 0.95, 252.0);
        assert!(ms.is_finite());
    }

    #[test]
    fn modified_sharpe_empty() {
        assert_eq!(modified_sharpe(&[], 0.02, 0.95, 252.0), 0.0);
    }

    #[test]
    fn modified_sharpe_positive_cf_var_returns_nan() {
        let r = [0.03; 12];
        let ms = modified_sharpe(&r, 0.0, 0.95, 12.0);
        assert!(ms.is_nan());
    }

    #[test]
    fn cagr_empty_is_err() {
        assert!(cagr(&[], CagrBasis::factor(252.0), None).is_err());
    }

    #[test]
    fn parametric_var_scales_mean_and_vol_by_horizon() {
        let returns = [0.01, -0.02, 0.03, -0.01, 0.02, -0.005];
        let ann_factor = 12.0;
        let m = mean(&returns);
        let vol = variance(&returns).sqrt();
        let z = crate::math::special_functions::standard_normal_inv_cdf(0.05);
        let expected = m * ann_factor + z * vol * ann_factor.sqrt();
        let actual = crate::risk_metrics::parametric_var(&returns, 0.95, Some(ann_factor));
        assert!((actual - expected).abs() < 1e-14, "{actual} vs {expected}");
    }
}
