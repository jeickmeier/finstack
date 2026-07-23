//! Surface validators (volatility surfaces).
//!
//! Two families of checks are provided:
//!
//! - **Forward-aware validators** (`*_with_forwards`) — the correct
//!   no-arbitrage tests. Calendar monotonicity of total variance is evaluated
//!   at fixed *forward log-moneyness* `k = ln(K/F_T)` (Gatheral–Jacquier 2014,
//!   Lemma 2.1) and butterfly arbitrage is tested as *call-price convexity in
//!   strike* (equivalent to non-negative risk-neutral density,
//!   `∂²C/∂K² ≥ 0`). Prefer these whenever per-expiry forwards are available.
//! - **Legacy grid validators** (`validate_calendar_spread`,
//!   `validate_butterfly_spread`) — forward-free heuristics retained for
//!   callers with no forward information. The calendar check at fixed
//!   *absolute* strike is exact only under flat forwards (zero rates and
//!   dividends); the butterfly band on total-variance convexity in absolute
//!   strike is neither necessary nor sufficient for no-arbitrage and should
//!   be treated as a smoothness diagnostic only.

use crate::calibration::validation::ValidationConfig;
use finstack_quant_core::market_data::surfaces::VolSurface;
use finstack_quant_core::math::volatility::black_call;
use finstack_quant_core::{Error, Result};

/// Validate all volatility-surface constraints.
///
/// Forward-free variant; see the module docs for the flat-forward caveat on
/// the calendar check and the heuristic nature of the butterfly band. Prefer
/// [`validate_surface_with_forwards`] when per-expiry forwards are available.
///
/// # Arguments
///
/// * `surface` - Volatility surface whose expiry/strike grid and quoted
///   annualized volatilities are checked.
/// * `config` - Validation tolerances, volatility cap, and strict-versus-
///   lenient arbitrage policy.
pub fn validate_surface(surface: &VolSurface, config: &ValidationConfig) -> Result<()> {
    validate_calendar_spread(surface, config)?;
    validate_butterfly_spread(surface, config)?;
    validate_vol_bounds(surface, config)?;
    Ok(())
}

/// Validate all volatility-surface constraints using per-expiry forwards.
///
/// Runs the correct no-arbitrage tests: calendar monotonicity of total
/// variance at fixed forward log-moneyness, butterfly (density) via call-price
/// convexity in strike, and vol bounds.
///
/// # Arguments
///
/// * `surface` - Volatility surface whose expiry/strike grid is checked.
/// * `config` - Validation tolerances and strict-versus-lenient policy.
/// * `forwards` - Forward level `F(T_i)` for each expiry on the surface's
///   grid, in the same order as `surface.expiries()`. All must be positive.
///
/// # Errors
///
/// Returns an error on the first violated constraint (in strict mode), or if
/// `forwards` is malformed.
pub fn validate_surface_with_forwards(
    surface: &VolSurface,
    config: &ValidationConfig,
    forwards: &[f64],
) -> Result<()> {
    validate_forwards_input(surface, forwards)?;
    validate_calendar_spread_with_forwards(surface, config, forwards)?;
    validate_butterfly_call_convexity(surface, config, forwards)?;
    validate_vol_bounds(surface, config)?;
    Ok(())
}

/// Check that the `forwards` slice matches the surface's expiry grid.
fn validate_forwards_input(surface: &VolSurface, forwards: &[f64]) -> Result<()> {
    let n = surface.expiries().len();
    if forwards.len() != n {
        return Err(Error::Validation(format!(
            "Surface validation requires one forward per expiry: got {} forwards for {} expiries in {}",
            forwards.len(),
            n,
            surface.id().as_str()
        )));
    }
    if let Some(f) = forwards.iter().find(|f| !f.is_finite() || **f <= 0.0) {
        return Err(Error::Validation(format!(
            "Surface validation requires positive finite forwards; got {} in {}",
            f,
            surface.id().as_str()
        )));
    }
    Ok(())
}

/// Validate no calendar-spread arbitrage at fixed **forward log-moneyness**.
///
/// The no-arbitrage condition is `∂_T w(k, T) ≥ 0` at fixed
/// `k = ln(K/F_T)` (Gatheral & Jacquier 2014, Lemma 2.1) — *not* at fixed
/// absolute strike, which differs whenever rates/dividends give the forward a
/// term structure. For each adjacent expiry pair the grid strikes of the later
/// expiry are mapped to the earlier expiry at equal `k`; probes falling
/// outside the quoted strike range are skipped (extrapolated vols would
/// contaminate the test).
///
/// # Arguments
///
/// * `surface` - Volatility surface under test.
/// * `config` - Arbitrage-check toggle, tolerance, and strict/lenient policy.
/// * `forwards` - `F(T_i)` per expiry, aligned with `surface.expiries()`.
///
/// # Errors
///
/// In strict mode (`lenient_arbitrage = false`), returns an error listing the
/// violating `(k, T)` points. Lenient mode logs warnings and returns `Ok`.
pub fn validate_calendar_spread_with_forwards(
    surface: &VolSurface,
    config: &ValidationConfig,
    forwards: &[f64],
) -> Result<()> {
    if !config.check_arbitrage {
        return Ok(());
    }
    validate_forwards_input(surface, forwards)?;

    let strikes = surface.strikes();
    let expiries = surface.expiries();
    let (Some(&k_min), Some(&k_max)) = (strikes.first(), strikes.last()) else {
        return Ok(());
    };

    // (k, T_prev, T_next, w_prev, w_next)
    let mut violations: Vec<(f64, f64, f64, f64, f64)> = Vec::new();

    for i in 1..expiries.len() {
        let (t1, f1) = (expiries[i - 1], forwards[i - 1]);
        let (t2, f2) = (expiries[i], forwards[i]);
        for &strike in strikes {
            let k = (strike / f2).ln();
            // Same forward log-moneyness at the earlier expiry.
            let strike_prev = f1 * k.exp();
            if strike_prev < k_min || strike_prev > k_max {
                continue; // avoid testing against extrapolated vols
            }
            let v1 = surface.value_checked(t1, strike_prev)?;
            let v2 = surface.value_checked(t2, strike)?;
            let w1 = v1 * v1 * t1;
            let w2 = v2 * v2 * t2;
            if w2 < w1 - config.tolerance {
                violations.push((k, t1, t2, w1, w2));
                if config.lenient_arbitrage {
                    tracing::warn!(
                        "Calendar spread arbitrage at fixed forward-moneyness k={:.4}: \
                         w({:.4}y)={:.6} > w({:.4}y)={:.6} in {}",
                        k,
                        t1,
                        w1,
                        t2,
                        w2,
                        surface.id().as_str()
                    );
                }
            }
        }
    }

    if !violations.is_empty() && !config.lenient_arbitrage {
        let details: Vec<String> = violations
            .iter()
            .take(5)
            .map(|(k, t1, t2, w1, w2)| {
                format!("k={k:.4}: w({t1:.4}y)={w1:.6} > w({t2:.4}y)={w2:.6}")
            })
            .collect();
        let suffix = if violations.len() > 5 {
            format!(" (and {} more)", violations.len() - 5)
        } else {
            String::new()
        };
        return Err(Error::Validation(format!(
            "Calendar spread arbitrage (fixed forward-moneyness) at {} point(s) in {}: [{}]{}. \
             Total variance must be non-decreasing in expiry at fixed k = ln(K/F_T).",
            violations.len(),
            surface.id().as_str(),
            details.join("; "),
            suffix
        )));
    }

    Ok(())
}

/// Validate no butterfly arbitrage via **call-price convexity in strike**.
///
/// Non-negative risk-neutral density is equivalent to `∂²C/∂K² ≥ 0`
/// (Breeden–Litzenberger); discounting scales prices by a positive constant
/// per expiry, so convexity of the *undiscounted* Black-76 call in strike is
/// an exact test. This replaces the total-variance-convexity band, which is
/// neither necessary nor sufficient. Call monotonicity (`∂C/∂K ≤ 0`,
/// non-negative vertical spreads) is checked as well.
///
/// # Arguments
///
/// * `surface` - Volatility surface under test.
/// * `config` - Arbitrage-check toggle, tolerance, and strict/lenient policy.
///   `config.tolerance` is applied *relative to the forward* (prices scale
///   with `F`).
/// * `forwards` - `F(T_i)` per expiry, aligned with `surface.expiries()`.
///
/// # Errors
///
/// In strict mode, returns an error listing the violating `(K, T)` points.
pub fn validate_butterfly_call_convexity(
    surface: &VolSurface,
    config: &ValidationConfig,
    forwards: &[f64],
) -> Result<()> {
    if !config.check_arbitrage {
        return Ok(());
    }
    validate_forwards_input(surface, forwards)?;

    let strikes = surface.strikes();
    let expiries = surface.expiries();
    if strikes.len() < 3 {
        return Ok(());
    }

    // (expiry, strike, call_mid, call_interp)
    let mut violations: Vec<(f64, f64, f64, f64)> = Vec::new();

    for (i, &expiry) in expiries.iter().enumerate() {
        let fwd = forwards[i];
        let price_tol = config.tolerance.max(1e-12) * fwd;
        let calls: Vec<f64> = strikes
            .iter()
            .map(|&k| {
                Ok(black_call(
                    fwd,
                    k,
                    surface.value_checked(expiry, k)?,
                    expiry,
                ))
            })
            .collect::<Result<_>>()?;

        for j in 1..strikes.len() - 1 {
            let (k1, k2, k3) = (strikes[j - 1], strikes[j], strikes[j + 1]);
            let (c1, c2, c3) = (calls[j - 1], calls[j], calls[j + 1]);

            // Vertical spread: calls must be non-increasing in strike.
            if c2 > c1 + price_tol || c3 > c2 + price_tol {
                violations.push((expiry, k2, c2, c1.min(c2)));
                continue;
            }

            // Butterfly: C(K2) must lie on or below the chord through
            // (K1, C1) and (K3, C3).
            let lambda = (k2 - k1) / (k3 - k1);
            let c_interp = c1 + lambda * (c3 - c1);
            if c2 > c_interp + price_tol {
                violations.push((expiry, k2, c2, c_interp));
                if config.lenient_arbitrage {
                    tracing::warn!(
                        "Butterfly arbitrage (negative density) at T={:.4}, K={:.4} in {}: \
                         call={:.6e} above chord={:.6e}",
                        expiry,
                        k2,
                        surface.id().as_str(),
                        c2,
                        c_interp
                    );
                }
            }
        }
    }

    if !violations.is_empty() && !config.lenient_arbitrage {
        let details: Vec<String> = violations
            .iter()
            .take(5)
            .map(|(t, k, c, ci)| format!("T={t:.4}y, K={k:.4} (call={c:.6e} > chord={ci:.6e})"))
            .collect();
        let suffix = if violations.len() > 5 {
            format!(" (and {} more)", violations.len() - 5)
        } else {
            String::new()
        };
        return Err(Error::Validation(format!(
            "Butterfly arbitrage (negative implied density) at {} point(s) in {}: [{}]{}. \
             Undiscounted call prices must be convex and non-increasing in strike \
             (Breeden–Litzenberger).",
            violations.len(),
            surface.id().as_str(),
            details.join("; "),
            suffix
        )));
    }

    Ok(())
}

/// Validate no calendar spread arbitrage (forward-free variant).
///
/// Checks that total variance (sigma^2 T) is monotonically increasing with expiry,
/// ensuring that longer-dated options are not cheaper than shorter-dated ones.
///
/// **Caveat**: monotonicity is evaluated at fixed *absolute* strike, which
/// equals the true condition (fixed forward log-moneyness, Gatheral–Jacquier
/// 2014 Lemma 2.1) only under flat forwards. With a rates/dividend term
/// structure prefer [`validate_calendar_spread_with_forwards`].
///
/// # Arguments
///
/// * `surface` - Volatility surface tested along every strike across increasing
///   expiry nodes.
/// * `config` - Arbitrage-check toggle, numerical tolerance, and policy that
///   turns violations into errors or warnings.
pub fn validate_calendar_spread(surface: &VolSurface, config: &ValidationConfig) -> Result<()> {
    if !config.check_arbitrage {
        return Ok(());
    }

    // Total variance (σ²T) must be monotonically increasing with time to prevent calendar arbitrage.
    // This is a fundamental no-arbitrage condition: longer-dated options must have at least
    // as much total variance as shorter-dated options at the same strike.
    let strikes = surface.strikes();
    let expiries = surface.expiries();
    let mut violations: Vec<(f64, f64, f64, f64)> = Vec::new(); // (strike, expiry, actual, expected)

    for strike in strikes {
        let mut prev_total_var = 0.0;
        let mut prev_expiry = 0.0_f64;

        for &expiry in expiries {
            let vol = surface.value_checked(expiry, *strike)?;
            let total_var = vol * vol * expiry; // σ²T

            // Check monotonicity of total variance
            if total_var < prev_total_var - config.tolerance {
                violations.push((*strike, expiry, total_var, prev_total_var));

                if config.lenient_arbitrage {
                    tracing::warn!(
                            "Calendar spread arbitrage detected: total variance {:.6} < {:.6} at K={}, T={:.4} (prev T={:.4}) in {}. \
                            Consider using SVI or monotone convex fitting for arbitrage-free surfaces.",
                            total_var,
                            prev_total_var,
                            strike,
                            expiry,
                            prev_expiry,
                            surface.id().as_str()
                        );
                }
            }

            prev_total_var = total_var;
            prev_expiry = expiry;
        }
    }

    // In strict mode (default), fail on any calendar arbitrage violations
    if !violations.is_empty() && !config.lenient_arbitrage {
        let details: Vec<String> = violations
            .iter()
            .take(5)
            .map(|(k, t, actual, expected)| {
                format!(
                    "K={:.2}, T={:.4}y (var={:.6} < {:.6})",
                    k, t, actual, expected
                )
            })
            .collect();
        let suffix = if violations.len() > 5 {
            format!(" (and {} more)", violations.len() - 5)
        } else {
            String::new()
        };
        return Err(Error::Validation(format!(
            "Calendar spread arbitrage detected at {} point(s) in {}: [{}]{}. \
                Total variance must be monotonically increasing in expiry. \
                Consider using SVI or monotone convex fitting for arbitrage-free surfaces.",
            violations.len(),
            surface.id().as_str(),
            details.join("; "),
            suffix
        )));
    }

    Ok(())
}

/// Validate no butterfly arbitrage (forward-free heuristic).
///
/// Checks that total variance (sigma^2 T) stays within a convexity band in
/// absolute strike.
///
/// **Caveat**: this is a smoothness diagnostic, not a density test —
/// total-variance convexity in absolute strike is neither necessary nor
/// sufficient for non-negative implied density. When per-expiry forwards are
/// available prefer [`validate_butterfly_call_convexity`], which tests the
/// exact Breeden–Litzenberger condition (call-price convexity in strike).
///
/// # Arguments
///
/// * `surface` - Volatility surface tested along every expiry across ordered
///   strike nodes.
/// * `config` - Arbitrage-check toggle and allowed convexity-ratio band used
///   to classify butterfly violations.
pub fn validate_butterfly_spread(surface: &VolSurface, config: &ValidationConfig) -> Result<()> {
    if !config.check_arbitrage {
        return Ok(());
    }

    // Check convexity of total variance in strike dimension.
    // Proper butterfly arbitrage check requires that total variance (σ²T) is convex in strike,
    // which prevents risk-free arbitrage via butterfly spreads.
    let strikes = surface.strikes();
    let expiries = surface.expiries();

    if strikes.len() < 3 {
        return Ok(()); // Need at least 3 strikes to check
    }

    let mut violations: Vec<(f64, f64, f64, f64, f64)> = Vec::new(); // (expiry, strike, actual, interp, ratio)

    for &expiry in expiries {
        for i in 1..strikes.len() - 1 {
            let k1 = strikes[i - 1];
            let k2 = strikes[i];
            let k3 = strikes[i + 1];

            let v1 = surface.value_checked(expiry, k1)?;
            let v2 = surface.value_checked(expiry, k2)?;
            let v3 = surface.value_checked(expiry, k3)?;

            // Convert to total variance for proper arbitrage check
            let w1 = v1 * v1 * expiry;
            let w2 = v2 * v2 * expiry;
            let w3 = v3 * v3 * expiry;

            // Check convexity of total variance: w2 should be ≤ linear interpolation
            let weight = (k2 - k1) / (k3 - k1);
            let w2_interpolated = w1 + weight * (w3 - w1);

            let ratio = if w2_interpolated.abs() > 1e-12 {
                w2 / w2_interpolated
            } else {
                1.0
            };

            if w2 > w2_interpolated * config.butterfly_upper_ratio
                || w2 < w2_interpolated * config.butterfly_lower_ratio
            {
                violations.push((expiry, k2, w2, w2_interpolated, ratio));

                if config.lenient_arbitrage {
                    tracing::warn!(
                        "Potential butterfly arbitrage at T={:.2}, K={:.2} in {}: \
                            total_var={:.6} vs interpolated={:.6} (ratio {:.2}). \
                            Consider SVI or monotone convex fitting.",
                        expiry,
                        k2,
                        surface.id().as_str(),
                        w2,
                        w2_interpolated,
                        ratio
                    );
                }
            }
        }
    }

    // In strict mode (default), fail on any butterfly arbitrage violations
    if !violations.is_empty() && !config.lenient_arbitrage {
        let details: Vec<String> = violations
            .iter()
            .take(5)
            .map(|(t, k, actual, interp, ratio)| {
                format!(
                    "T={:.2}y, K={:.2} (var={:.6} vs interp={:.6}, ratio={:.2})",
                    t, k, actual, interp, ratio
                )
            })
            .collect();
        let suffix = if violations.len() > 5 {
            format!(" (and {} more)", violations.len() - 5)
        } else {
            String::new()
        };
        return Err(Error::Validation(format!(
            "Butterfly spread arbitrage detected at {} point(s) in {}: [{}]{}. \
                Total variance must be convex in strike. \
                Consider using SVI or monotone convex fitting for arbitrage-free surfaces.",
            violations.len(),
            surface.id().as_str(),
            details.join("; "),
            suffix
        )));
    }

    Ok(())
}

/// Validate volatility bounds.
///
/// Ensures volatility is positive and within reasonable financial limits.
///
/// # Arguments
///
/// * `surface` - Volatility surface whose annualized values are inspected.
/// * `config` - Validation configuration supplying the maximum permissible
///   annualized volatility and other surface policy controls.
pub fn validate_vol_bounds(surface: &VolSurface, config: &ValidationConfig) -> Result<()> {
    let strikes = surface.strikes();
    let expiries = surface.expiries();

    for &expiry in expiries {
        for strike in strikes {
            let vol = surface.value_checked(expiry, *strike)?;

            // Volatility should be positive
            if vol <= 0.0 {
                return Err(Error::Validation(format!(
                    "Non-positive volatility {:.2}% at T={}, K={} in {}",
                    vol * 100.0,
                    expiry,
                    strike,
                    surface.id().as_str()
                )));
            }

            // Cap at reasonable maximum (500% vol)
            if vol > config.max_volatility {
                return Err(Error::Validation(format!(
                    "Unreasonably high volatility {:.2}% at T={}, K={} in {} (limit: {:.2}%)",
                    vol * 100.0,
                    expiry,
                    strike,
                    surface.id().as_str(),
                    config.max_volatility * 100.0
                )));
            }
        }
    }

    Ok(())
}
