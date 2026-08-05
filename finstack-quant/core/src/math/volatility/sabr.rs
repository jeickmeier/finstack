//! SABR stochastic volatility model.
//!
//! Implements the SABR (Stochastic Alpha Beta Rho) model, the market standard
//! for swaption and cap/floor volatility smile modeling. Uses the Hagan et al.
//! (2002) analytical approximation for implied volatility.
//!
//! # Mathematical Foundation
//!
//! The SABR model describes the joint dynamics of a forward rate and its
//! stochastic volatility:
//!
//! ```text
//! dF = σ * F^β * dW₁
//! dσ = ν * σ * dW₂
//! E[dW₁ * dW₂] = ρ * dt
//!
//! where:
//!   F = forward rate
//!   σ = instantaneous volatility (alpha at t=0)
//!   β = CEV exponent (controls backbone; 0=normal, 1=lognormal)
//!   ν = vol-of-vol (controls smile curvature)
//!   ρ = correlation (controls skew direction)
//! ```
//!
//! # Parameters
//!
//! | Parameter | Symbol | Typical Range | Market Role |
//! |-----------|--------|---------------|-------------|
//! | Alpha (α) | `alpha` | 0.01–0.50 | ATM volatility level |
//! | Beta (β) | `beta` | 0.0–1.0 | Backbone/CEV exponent |
//! | Rho (ρ) | `rho` | (-1, 1) | Skew direction |
//! | Nu (ν) | `nu` | 0.01–1.50 | Smile curvature (vol-of-vol) |
//!
//! # Common Calibration Choices
//!
//! - **β = 0.5** (CMS market convention): Square-root dynamics
//! - **β = 0.0** (Normal SABR): Used for negative rate environments
//! - **β = 1.0** (Lognormal SABR): Traditional lognormal dynamics
//!
//! # Approximation Accuracy
//!
//! The Hagan approximation is accurate for:
//! - Options with expiry T < 10Y (shorter is better)
//! - Strikes not too far from ATM (within 2-3 standard deviations)
//! - Moderate vol-of-vol (ν < 1.5)
//!
//! For very long-dated options or deep OTM strikes, consider exact PDE solutions.
//!
//! # References
//!
//! - Hagan, P. S., Kumar, D., Lesniewski, A. S., & Woodward, D. E. (2002).
//!   "Managing Smile Risk." *Wilmott Magazine*, September 2002, 84-108.
//! - Obloj, J. (2008). "Fine-tune your smile: Correction to Hagan et al."
//!   *Wilmott Magazine*, May 2008.
//! - West, G. (2005). "Calibration of the SABR Model in Illiquid Markets."
//!   *Applied Mathematical Finance*, 12(4), 371-385.
//! - QuantLib SABR implementation: `ql/termstructures/volatility/sabr.cpp`

// SABR stochastic volatility model implementation.

/// SABR model parameters for a single expiry.
///
/// # Examples
///
/// ```rust
/// use finstack_quant_core::math::volatility::sabr::SabrParams;
///
/// // Typical USD swaption SABR parameters
/// let params = SabrParams::new(0.035, 0.5, -0.2, 0.4).unwrap();
/// let fwd = 0.05;
/// let strike = 0.05;
/// let expiry = 1.0;
/// let vol = params.implied_vol_lognormal(fwd, strike, expiry).expect("valid checked inputs");
/// assert!(vol > 0.0);
/// ```
#[derive(
    Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(try_from = "RawSabrParams")]
pub struct SabrParams {
    /// Alpha (α): initial volatility level.
    pub alpha: f64,
    /// Beta (β): CEV exponent, in [0, 1].
    pub beta: f64,
    /// Rho (ρ): correlation between forward and vol Brownian motions, in (-1, 1).
    pub rho: f64,
    /// Nu (ν): vol-of-vol, must be > 0.
    pub nu: f64,
    /// Shift for negative rate support. When set, the model uses `F+shift` and
    /// `K+shift` internally, keeping both arguments positive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shift: Option<f64>,
}

/// Raw deserialization state of [`SabrParams`].
///
/// Mirrors the serialized field layout exactly so the wire format is
/// unchanged; conversion runs [`SabrParams::new`] validation and rejects
/// unknown fields.
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct RawSabrParams {
    /// Initial volatility level.
    alpha: f64,
    /// CEV exponent.
    beta: f64,
    /// Forward-vol correlation.
    rho: f64,
    /// Vol-of-vol.
    nu: f64,
    /// Optional shift for negative rate support.
    #[serde(default)]
    shift: Option<f64>,
}

impl TryFrom<RawSabrParams> for SabrParams {
    type Error = crate::Error;

    fn try_from(raw: RawSabrParams) -> crate::Result<Self> {
        SabrParams::new_with_shift(raw.alpha, raw.beta, raw.rho, raw.nu, raw.shift)
    }
}

impl SabrParams {
    const LOGNORMAL_ATM_LOG_MONEYNESS_THRESHOLD: f64 = 1e-8;

    /// Alpha (α): initial volatility level.
    pub fn alpha(&self) -> f64 {
        self.alpha
    }
    /// Beta (β): CEV exponent, in [0, 1].
    pub fn beta(&self) -> f64 {
        self.beta
    }
    /// Rho (ρ): correlation between forward and vol Brownian motions.
    pub fn rho(&self) -> f64 {
        self.rho
    }
    /// Nu (ν): vol-of-vol.
    pub fn nu(&self) -> f64 {
        self.nu
    }

    /// Construct validated SABR parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `alpha <= 0`
    /// - `beta` not in `[0, 1]`
    /// - `rho` not in `(-1, 1)`
    /// - `nu <= 0`
    pub fn new(alpha: f64, beta: f64, rho: f64, nu: f64) -> crate::Result<Self> {
        Self::new_with_shift(alpha, beta, rho, nu, None)
    }

    /// Construct validated SABR parameters with an optional rate shift.
    ///
    /// This is the canonical constructor for deserialization and bindings so
    /// that shift validation cannot drift across host languages.
    ///
    /// `alpha` is the initial volatility level, `beta` is the CEV exponent,
    /// `rho` is the forward/volatility Brownian correlation, and `nu` is
    /// vol-of-vol. `shift`, when present, is an additive rate/price shift used
    /// by shifted-lognormal SABR conventions; it is stored unchanged and is
    /// not a volatility percentage.
    ///
    /// # Errors
    ///
    /// Returns an error if `alpha` or `nu` is non-finite or not strictly
    /// positive, `beta` is non-finite or outside `[0, 1]`, `rho` is non-finite
    /// or outside `(-1, 1)`, or an optional shift is non-finite.
    pub fn new_with_shift(
        alpha: f64,
        beta: f64,
        rho: f64,
        nu: f64,
        shift: Option<f64>,
    ) -> crate::Result<Self> {
        if alpha <= 0.0 || !alpha.is_finite() {
            return Err(crate::Error::Validation(format!(
                "SABR alpha must be positive, got {alpha}"
            )));
        }
        if !(0.0..=1.0).contains(&beta) || !beta.is_finite() {
            return Err(crate::Error::Validation(format!(
                "SABR beta must be in [0, 1], got {beta}"
            )));
        }
        if rho <= -1.0 || rho >= 1.0 || !rho.is_finite() {
            return Err(crate::Error::Validation(format!(
                "SABR rho must be in (-1, 1), got {rho}"
            )));
        }
        if nu <= 0.0 || !nu.is_finite() {
            return Err(crate::Error::Validation(format!(
                "SABR nu (vol-of-vol) must be positive, got {nu}"
            )));
        }
        if shift.is_some_and(|value| !value.is_finite()) {
            return Err(crate::Error::Validation(format!(
                "SABR shift must be finite, got {}",
                shift.unwrap_or_default()
            )));
        }
        Ok(Self {
            alpha,
            beta,
            rho,
            nu,
            shift,
        })
    }

    /// Return a copy of these parameters with the given shift applied.
    ///
    /// The shift is used to handle negative rate environments: when evaluating
    /// implied vol the model internally uses `F+shift` and `K+shift`, keeping
    /// both arguments positive. A typical value for EUR/JPY is `0.03` (300 bp).
    ///
    /// # Arguments
    ///
    /// * `shift` - Displacement shift applied to forward/strike for negative-rate SABR
    pub fn with_shift(self, shift: f64) -> Self {
        Self {
            shift: Some(shift),
            ..self
        }
    }

    /// Lognormal (Black-76) implied volatility using Hagan's approximation.
    ///
    /// This is the market-standard SABR approximation from Hagan et al. (2002).
    /// Returns the Black-76 implied volatility for a given forward, strike, and expiry.
    ///
    /// # Arguments
    ///
    /// * `f` - Forward rate
    /// * `k` - Strike rate
    /// * `t` - Time to expiry in years
    ///
    /// # Returns
    ///
    /// Black-76 implied volatility (lognormal). Returns `alpha` for the ATM case.
    ///
    /// Returns `f64::NAN` for degenerate inputs (non-positive forward/strike,
    /// non-positive expiry, or a χ(z) breakdown). Callers on pricing/risk paths
    /// must guard the result with `is_finite()`, or use the fallible
    /// [`implied_vol_lognormal`](Self::implied_vol_lognormal), since a
    /// silent NaN poisons Black-76 pricing and compensated summations downstream.
    ///
    /// # Special Cases
    ///
    /// - ATM (`f ≈ k`): Uses the simplified ATM formula for numerical stability
    /// - β = 0: Degenerates to normal SABR; lognormal vol is approximated
    /// - β = 1: Standard lognormal SABR formula
    fn implied_vol_lognormal_unchecked(&self, f: f64, k: f64, t: f64) -> f64 {
        let alpha = self.alpha;
        let beta = self.beta;
        let rho = self.rho;
        let nu = self.nu;

        // Apply shift for negative rate support
        let (f, k) = if let Some(s) = self.shift {
            (f + s, k + s)
        } else {
            (f, k)
        };

        // Guard: both forward and strike must be positive for lognormal model
        if f <= 0.0 || k <= 0.0 || t <= 0.0 {
            return f64::NAN;
        }

        // No special-case for ν → 0: the general Hagan expansion is continuous
        // in ν. z = (ν/α)(FK)^((1-β)/2) ln(F/K) → 0 and z/χ(z) → 1 (handled by
        // the small-z Taylor branch in `chi`), while the (1 + [...]T) correction
        // smoothly degenerates to the pure CEV correction. A hard ν threshold
        // with a corrections-free CEV formula introduced a vol discontinuity.

        let fk = f * k;
        let one_minus_beta = 1.0 - beta;
        let log_fk = (f / k).ln();

        // ATM case: use the stable formula once log-moneyness is tiny, including
        // low-rate environments where a purely relative |f-k| threshold becomes ineffective.
        if log_fk.abs() <= Self::LOGNORMAL_ATM_LOG_MONEYNESS_THRESHOLD {
            return self.atm_vol_lognormal(f, t);
        }

        // z = (ν/α) * (FK)^((1-β)/2) * ln(F/K)
        let fk_mid = fk.powf(one_minus_beta / 2.0);
        let z = (nu / alpha) * fk_mid * log_fk;

        // χ(z) = log[(√(1 - 2ρz + z²) + z - ρ) / (1 - ρ)]
        let chi_z = chi(z, rho).unwrap_or(f64::NAN);

        // Numerator: α
        let numerator = alpha;

        // Denominator: (FK)^((1-β)/2) * [1 + (1-β)²/24 * log²(F/K) + (1-β)⁴/1920 * log⁴(F/K)]
        let log_fk_sq = log_fk * log_fk;
        let omb2 = one_minus_beta * one_minus_beta;
        let denominator =
            fk_mid * (1.0 + omb2 / 24.0 * log_fk_sq + omb2 * omb2 / 1920.0 * log_fk_sq * log_fk_sq);

        // First-order correction factor
        // 1 + [ (1-β)²/24 * α² / (FK)^(1-β)
        //      + ¼ * ρβνα / (FK)^((1-β)/2)
        //      + (2-3ρ²)/24 * ν² ] * T
        let fk_omb = fk.powf(one_minus_beta);
        let correction = 1.0
            + (omb2 / 24.0 * alpha * alpha / fk_omb
                + 0.25 * rho * beta * nu * alpha / fk_mid
                + (2.0 - 3.0 * rho * rho) / 24.0 * nu * nu)
                * t;

        // σ_B(K) = (z / χ(z)) × (α / denominator) × correction
        let z_over_chi = if chi_z.abs() < 1e-14 {
            1.0 // L'Hôpital at z=0
        } else {
            z / chi_z
        };

        numerator / denominator * z_over_chi * correction
    }

    /// Normal (Bachelier) implied volatility using Hagan's approximation.
    ///
    /// Returns the normal/Bachelier implied volatility. This is useful for
    /// negative rate environments (EUR, CHF, JPY post-2014).
    ///
    /// # Arguments
    ///
    /// * `f` - Forward rate (may be negative)
    /// * `k` - Strike rate (may be negative)
    /// * `t` - Time to expiry in years
    ///
    /// # Returns
    ///
    /// Normal/Bachelier implied volatility, or `f64::NAN` for non-positive
    /// expiry, a χ(z) breakdown, or cross-zero inputs (`f·k ≤ 0` after any
    /// configured shift) with `beta > 0` — the CEV backbone is not
    /// shift-invariant, so such quotes require an explicit
    /// [`with_shift`](Self::with_shift). β = 0 (normal SABR) is shift-invariant
    /// and prices cross-zero quotes directly. Guard with `is_finite()` or use
    /// the checked [`implied_vol_normal`](Self::implied_vol_normal)
    /// on pricing paths.
    fn implied_vol_normal_unchecked(&self, f: f64, k: f64, t: f64) -> f64 {
        let alpha = self.alpha;
        let beta = self.beta;
        let rho = self.rho;
        let nu = self.nu;

        // Apply shift for negative rate support
        let (f, k) = if let Some(s) = self.shift {
            (f + s, k + s)
        } else {
            (f, k)
        };

        if t <= 0.0 {
            return f64::NAN;
        }
        if beta > 0.0 && (f <= 0.0 || k <= 0.0) {
            return f64::NAN;
        }

        // No special-case for ν → 0: as in the lognormal expansion, the general
        // formula is continuous in ν (z → 0, z/χ(z) → 1) and retains the full
        // (1 + [...]T) correction in the CEV limit.

        // ATM case
        if (f - k).abs() < 1e-12 * f.abs().max(1e-10) {
            return self.atm_vol_normal(f, t);
        }

        let fk = f * k;
        let one_minus_beta = 1.0 - beta;

        if fk <= 0.0 {
            // β = 0 (normal SABR) is shift-invariant: dF = σ dW₁ is unaffected
            // by translating F and K, so an internal shift recovers the correct
            // smile from the log-moneyness expansion.
            if beta == 0.0 {
                let shift_scale = (f - k).abs().max(f.abs()).max(k.abs()).max(1.0e-4);
                let shift = (-f.min(k)).max(0.0) + shift_scale;
                let shifted_f = f + shift;
                let shifted_k = k + shift;
                return self.implied_vol_normal_unchecked(shifted_f, shifted_k, t);
            }
            // β > 0: the CEV backbone F^β is NOT shift-invariant, so any
            // internal shift silently changes the model. Refuse (NaN here;
            // a descriptive error from `implied_vol_normal`) and require
            // an explicit, calibrated shift via `with_shift`.
            return f64::NAN;
        }

        let fk_mid = fk.powf(one_minus_beta / 2.0);
        let log_fk = (f / k).ln();

        // z = (ν/α) * (FK)^((1-β)/2) * ln(F/K)
        let z = (nu / alpha) * fk_mid * log_fk;
        let chi_z = chi(z, rho).unwrap_or(f64::NAN);

        let z_over_chi = if chi_z.abs() < 1e-14 { 1.0 } else { z / chi_z };

        // Normal vol (Hagan 2002, eq. 2.17b):
        //   σ_N = α·(FK)^(β/2) · [numerator series] / [denominator series]
        //         · (z/χ(z)) · [1 + correction·T]
        let fk_beta_half = fk.powf(beta / 2.0);

        let omb2 = one_minus_beta * one_minus_beta;
        let log_fk_sq = log_fk * log_fk;

        // Numerator series (β-independent):
        //   1 + (1/24)ln²(F/K) + (1/1920)ln⁴(F/K)
        let numer_series = 1.0 + log_fk_sq / 24.0 + log_fk_sq * log_fk_sq / 1920.0;
        // Denominator series:
        //   1 + ((1-β)²/24)ln²(F/K) + ((1-β)⁴/1920)ln⁴(F/K)
        let denom_series =
            1.0 + omb2 / 24.0 * log_fk_sq + omb2 * omb2 / 1920.0 * log_fk_sq * log_fk_sq;

        // First-order time correction. The leading term uses the normal-SABR
        // coefficient −β(2−β)/24 (eq. 2.17b), NOT the lognormal (1−β)²/24.
        let fk_omb = fk.powf(one_minus_beta);
        let correction = 1.0
            + (-beta * (2.0 - beta) / 24.0 * alpha * alpha / fk_omb
                + 0.25 * rho * beta * nu * alpha / fk_mid
                + (2.0 - 3.0 * rho * rho) / 24.0 * nu * nu)
                * t;

        alpha * fk_beta_half * numer_series / denom_series * z_over_chi * correction
    }

    /// Lognormal SABR implied volatility with checked error semantics.
    ///
    /// # Errors
    ///
    /// Returns [`InputError::Invalid`](crate::error::InputError::Invalid) when
    /// the underlying expansion yields a non-finite volatility.
    pub fn implied_vol_lognormal(&self, f: f64, k: f64, t: f64) -> crate::Result<f64> {
        let v = self.implied_vol_lognormal_unchecked(f, k, t);
        if v.is_finite() {
            Ok(v)
        } else {
            Err(crate::error::InputError::Invalid.into())
        }
    }

    /// Normal SABR implied volatility with checked error semantics.
    ///
    /// # Errors
    ///
    /// - Returns a [`Validation`](crate::Error::Validation) error when
    ///   `f·k ≤ 0` (after any configured shift) with `beta > 0`: the CEV
    ///   backbone is not shift-invariant, so cross-zero quotes require an
    ///   explicit shift via [`with_shift`](Self::with_shift).
    /// - Returns [`InputError::Invalid`](crate::error::InputError::Invalid)
    ///   when the underlying expansion yields a non-finite volatility.
    ///
    /// # Arguments
    ///
    /// * `f` - Objective or payoff closure evaluated by the solver or Monte Carlo engine
    /// * `k` - K supplied by the caller for this operation
    /// * `t` - Year-fraction time from the curve or surface base date to the query point
    pub fn implied_vol_normal(&self, f: f64, k: f64, t: f64) -> crate::Result<f64> {
        let shift = self.shift.unwrap_or(0.0);
        let (sf, sk) = (f + shift, k + shift);
        if sf * sk <= 0.0 && self.beta > 0.0 && (sf - sk).abs() > 1e-12 * sf.abs().max(1e-10) {
            return Err(crate::Error::Validation(format!(
                "SABR implied_vol_normal: forward*strike <= 0 (F={sf}, K={sk} after shift) with \
                 beta = {} > 0. The CEV backbone is not shift-invariant; configure an explicit \
                 shift via SabrParams::with_shift for negative/cross-zero rates.",
                self.beta
            )));
        }
        let v = self.implied_vol_normal_unchecked(f, k, t);
        if v.is_finite() {
            Ok(v)
        } else {
            Err(crate::error::InputError::Invalid.into())
        }
    }

    /// ATM lognormal volatility (simplified formula when F ≈ K).
    ///
    /// ```text
    /// σ_ATM = α / F^(1-β) * [1 + ((1-β)²/24 * α²/F^(2(1-β)) + ¼ρβνα/F^(1-β) + (2-3ρ²)/24 * ν²) * T]
    /// ```
    fn atm_vol_lognormal(&self, f: f64, t: f64) -> f64 {
        let alpha = self.alpha;
        let beta = self.beta;
        let rho = self.rho;
        let nu = self.nu;

        let omb = 1.0 - beta;
        let f_safe = f.max(1e-10);
        let f_omb = f_safe.powf(omb);

        let base = alpha / f_omb;

        let correction = 1.0
            + (omb * omb / 24.0 * alpha * alpha / (f_omb * f_omb)
                + 0.25 * rho * beta * nu * alpha / f_omb
                + (2.0 - 3.0 * rho * rho) / 24.0 * nu * nu)
                * t;

        base * correction
    }

    /// ATM normal volatility (simplified formula when F ≈ K).
    fn atm_vol_normal(&self, f: f64, t: f64) -> f64 {
        let alpha = self.alpha;
        let beta = self.beta;
        let rho = self.rho;
        let nu = self.nu;

        let f_abs = f.abs().max(1e-10);
        let omb = 1.0 - beta;
        let f_beta = f_abs.powf(beta);
        let f_omb = f_abs.powf(omb);

        let base = alpha * f_beta;

        // Normal-SABR leading coefficient is −β(2−β)/24 (Hagan 2.17b at F=K),
        // NOT the lognormal (1−β)²/24. The two series cancel at ATM.
        let correction = 1.0
            + (-beta * (2.0 - beta) / 24.0 * alpha * alpha / (f_omb * f_omb)
                + 0.25 * rho * beta * nu * alpha / f_omb
                + (2.0 - 3.0 * rho * rho) / 24.0 * nu * nu)
                * t;

        base * correction
    }
}

/// χ(z) function used in the Hagan SABR approximation.
///
/// ```text
/// χ(z) = log[(√(1 - 2ρz + z²) + z - ρ) / (1 - ρ)]
/// ```
///
/// Uses a Taylor expansion for small z to avoid cancellation.
#[inline]
fn chi(z: f64, rho: f64) -> crate::Result<f64> {
    if z.abs() < 1e-10 {
        // Taylor expansion to O(z²): χ(z) ≈ z + ρz²/2. Higher-order terms are
        // negligible at the |z| < 1e-10 cutover (z³ ≲ 1e-30).
        return Ok(z * (1.0 + 0.5 * rho * z));
    }

    let discriminant = 1.0 - 2.0 * rho * z + z * z;
    if discriminant < 0.0 {
        return Err(crate::Error::Validation(format!(
            "SABR chi: negative discriminant {discriminant:.6} for z={z:.6}, rho={rho:.6}"
        )));
    }

    let sqrt_disc = discriminant.sqrt();
    let numerator = sqrt_disc + z - rho;
    let denominator = 1.0 - rho;

    if numerator <= 0.0 || denominator <= 0.0 {
        return Err(crate::Error::Validation(format!(
            "SABR chi: non-positive log argument (num={numerator:.6}, den={denominator:.6})"
        )));
    }

    Ok((numerator / denominator).ln())
}

// Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sabr_params_validation() {
        assert!(SabrParams::new(0.03, 0.5, -0.2, 0.4).is_ok());
        assert!(SabrParams::new(-0.01, 0.5, -0.2, 0.4).is_err()); // alpha <= 0
        assert!(SabrParams::new(0.03, 1.5, -0.2, 0.4).is_err()); // beta > 1
        assert!(SabrParams::new(0.03, 0.5, -1.0, 0.4).is_err()); // rho = -1
        assert!(SabrParams::new(0.03, 0.5, 1.0, 0.4).is_err()); // rho = 1
        assert!(SabrParams::new(0.03, 0.5, -0.2, 0.0).is_err()); // nu = 0
    }

    #[test]
    fn sabr_atm_vol_is_positive() {
        let params = SabrParams::new(0.035, 0.5, -0.2, 0.4).expect("valid params");
        let fwd = 0.05;
        let vol = params
            .implied_vol_lognormal(fwd, fwd, 1.0)
            .expect("valid checked inputs");
        assert!(vol > 0.0, "ATM vol should be positive: {vol}");
    }

    #[test]
    fn sabr_try_implied_vol_errors_on_degenerate_inputs() {
        let params = SabrParams::new(0.035, 0.5, -0.2, 0.4).expect("valid params");
        // Valid inputs return Ok and match the infallible path.
        let ok = params
            .implied_vol_lognormal(0.05, 0.06, 1.0)
            .expect("finite vol");
        assert!(
            (ok - params
                .implied_vol_lognormal(0.05, 0.06, 1.0)
                .expect("valid checked inputs"))
            .abs()
                < 1e-12
        );

        // Degenerate inputs (non-positive forward/strike/expiry) yield NaN in the
        // infallible path and an error in the fallible one.
        assert!(params.implied_vol_lognormal(-0.05, 0.06, 1.0).is_err());
        assert!(params.implied_vol_lognormal(0.05, 0.06, 0.0).is_err());
        assert!(params.implied_vol_normal(0.05, 0.06, 0.0).is_err());
        // Cross-zero with beta > 0 now errors (CEV backbone is not
        // shift-invariant); use with_shift for negative-rate quotes.
        assert!(params.implied_vol_normal(0.01, -0.01, 1.0).is_err());
    }

    #[test]
    fn sabr_vol_smile_shape() {
        let params = SabrParams::new(0.035, 0.5, -0.25, 0.45).expect("valid params");
        let fwd = 0.05;
        let t = 1.0;

        let vol_otm_put = params
            .implied_vol_lognormal(fwd, 0.03, t)
            .expect("valid checked inputs");
        let vol_atm = params
            .implied_vol_lognormal(fwd, fwd, t)
            .expect("valid checked inputs");
        let vol_otm_call = params
            .implied_vol_lognormal(fwd, 0.07, t)
            .expect("valid checked inputs");

        // With negative rho, we expect left-skew: OTM put vol > ATM vol
        assert!(
            vol_otm_put > vol_atm,
            "Expected left skew: vol(K=3%) = {vol_otm_put:.4} should be > vol(ATM) = {vol_atm:.4}"
        );
        // Smile: far OTM on both sides should be higher than ATM
        assert!(vol_otm_put > 0.0);
        assert!(vol_atm > 0.0);
        assert!(vol_otm_call > 0.0);
    }

    #[test]
    fn sabr_beta_zero_normal_sabr() {
        // β=0 is the normal SABR model
        let params = SabrParams::new(0.005, 0.0, -0.3, 0.3).expect("valid params");
        let fwd = 0.03;
        let vol = params
            .implied_vol_lognormal(fwd, fwd, 1.0)
            .expect("valid checked inputs");
        assert!(vol > 0.0, "Normal SABR ATM vol should be positive: {vol}");
    }

    #[test]
    fn sabr_beta_one_lognormal() {
        // β=1 is the standard lognormal SABR
        let params = SabrParams::new(0.2, 1.0, -0.15, 0.3).expect("valid params");
        let fwd = 0.05;
        let vol = params
            .implied_vol_lognormal(fwd, fwd, 1.0)
            .expect("valid checked inputs");
        // With β=1, α=0.2, ATM vol should be close to α=0.2
        assert!(
            (vol - 0.2).abs() < 0.05,
            "Lognormal SABR ATM vol should be near alpha: {vol:.4}"
        );
    }

    #[test]
    fn sabr_normal_vol_positive() {
        let params = SabrParams::new(0.035, 0.5, -0.2, 0.4).expect("valid params");
        let fwd = 0.05;
        let vol = params
            .implied_vol_normal(fwd, fwd, 1.0)
            .expect("valid checked inputs");
        assert!(vol > 0.0, "Normal vol should be positive: {vol}");
    }

    #[test]
    fn normal_sabr_requires_positive_shifted_levels_when_beta_is_positive() {
        let cev = SabrParams::new(0.035, 0.5, -0.2, 0.4).unwrap();
        assert!(cev.implied_vol_normal(-0.01, -0.01, 1.0).is_err());
        assert!(cev.implied_vol_normal(0.01, 0.0, 1.0).is_err());

        let shifted_to_zero = cev.with_shift(0.01);
        assert!(shifted_to_zero
            .implied_vol_normal(-0.01, -0.01, 1.0)
            .is_err());

        let normal = SabrParams::new(0.005, 0.0, -0.2, 0.4).unwrap();
        assert!(normal
            .implied_vol_normal(-0.01, -0.02, 1.0)
            .unwrap()
            .is_finite());
    }

    /// Independent textbook implementation of Hagan (2002) eq. 2.17b, used as a
    /// reference to lock in [`SabrParams::implied_vol_normal`].
    fn hagan_normal_vol_reference(
        alpha: f64,
        beta: f64,
        rho: f64,
        nu: f64,
        f: f64,
        k: f64,
        t: f64,
    ) -> f64 {
        let fk = f * k;
        let omb = 1.0 - beta;
        let log_fk = (f / k).ln();
        let z = (nu / alpha) * fk.powf(omb / 2.0) * log_fk;
        let chi = (((1.0 - 2.0 * rho * z + z * z).sqrt() + z - rho) / (1.0 - rho)).ln();
        let z_over_chi = if z.abs() < 1e-12 { 1.0 } else { z / chi };
        let l2 = log_fk * log_fk;
        let num = 1.0 + l2 / 24.0 + l2 * l2 / 1920.0;
        let den = 1.0 + omb * omb / 24.0 * l2 + omb.powi(4) / 1920.0 * l2 * l2;
        let corr = 1.0
            + (-beta * (2.0 - beta) / 24.0 * alpha * alpha / fk.powf(omb)
                + 0.25 * rho * beta * nu * alpha / fk.powf(omb / 2.0)
                + (2.0 - 3.0 * rho * rho) / 24.0 * nu * nu)
                * t;
        alpha * fk.powf(beta / 2.0) * num / den * z_over_chi * corr
    }

    #[test]
    fn sabr_normal_vol_matches_hagan_2_17b() {
        // Grid over β and OTM/ITM strikes, where the numerator series and the
        // −β(2−β)/24 correction term both contribute, vs the canonical formula.
        let f = 0.03;
        let t = 1.5;
        for &(alpha, beta, rho, nu) in &[
            (0.01, 0.0, -0.3, 0.4),
            (0.02, 0.5, -0.2, 0.3),
            (0.20, 1.0, -0.15, 0.35),
        ] {
            let p = SabrParams::new(alpha, beta, rho, nu).expect("valid params");
            for &k in &[0.018_f64, 0.024, 0.036, 0.045] {
                let got = p.implied_vol_normal(f, k, t).expect("valid checked inputs");
                let want = hagan_normal_vol_reference(alpha, beta, rho, nu, f, k, t);
                assert!(
                    (got - want).abs() <= 1e-9 * want.abs() + 1e-13,
                    "β={beta} k={k}: got {got:.12}, want {want:.12}"
                );
            }
        }
    }

    #[test]
    fn sabr_normal_atm_beta_zero_has_no_alpha_squared_term() {
        // At β=0 the normal-SABR time correction has NO α²/(FK)^(1-β) term
        // (−β(2−β)/24 = 0). The pre-fix code used −(1−β)²/24 = −1/24, adding a
        // spurious term that biased β=0 (negative-rate) ATM normal vols.
        let alpha = 0.01;
        let nu = 0.3;
        let rho = 0.0;
        let f = 0.03;
        let t = 2.0;
        let p = SabrParams::new(alpha, 0.0, rho, nu).expect("valid params");
        let got = p.atm_vol_normal(f, t);
        // Expected: α·[1 + (2−3ρ²)/24·ν²·t]  (ρ=0 ⇒ ρβ term gone, β=0 ⇒ α² gone).
        let want = alpha * (1.0 + (2.0 - 3.0 * rho * rho) / 24.0 * nu * nu * t);
        assert!(
            (got - want).abs() <= 1e-12,
            "got {got:.12}, want {want:.12}"
        );
    }

    #[test]
    fn sabr_cross_zero_normal_vol_is_not_midpoint_atm_shortcut() {
        let params = SabrParams::new(0.01, 0.0, -0.2, 0.4).expect("valid params");
        let fwd = -0.01;
        let strike = 0.02;
        let t = 1.0;

        let vol = params
            .implied_vol_normal(fwd, strike, t)
            .expect("valid checked inputs");
        let midpoint_atm = params.atm_vol_normal(0.5 * (fwd + strike), t);

        assert!(
            vol.is_finite() && vol > 0.0,
            "cross-zero normal vol should stay finite"
        );
        assert!(
            (vol - midpoint_atm).abs() > 1e-12,
            "cross-zero handling should not collapse to midpoint ATM vol"
        );
    }

    #[test]
    fn sabr_symmetry_at_atm() {
        // Vol at ATM should be continuous regardless of approach direction
        let params = SabrParams::new(0.035, 0.5, -0.2, 0.4).expect("valid params");
        let fwd = 0.05;
        let t = 1.0;

        let vol_exact = params
            .implied_vol_lognormal(fwd, fwd, t)
            .expect("valid checked inputs");
        let vol_near_above = params
            .implied_vol_lognormal(fwd, fwd + 1e-8, t)
            .expect("valid checked inputs");
        let vol_near_below = params
            .implied_vol_lognormal(fwd, fwd - 1e-8, t)
            .expect("valid checked inputs");

        assert!(
            (vol_exact - vol_near_above).abs() < 1e-4,
            "Vol should be continuous at ATM: exact={vol_exact:.6}, above={vol_near_above:.6}"
        );
        assert!(
            (vol_exact - vol_near_below).abs() < 1e-4,
            "Vol should be continuous at ATM: exact={vol_exact:.6}, below={vol_near_below:.6}"
        );
    }

    #[test]
    fn sabr_low_rate_near_atm_is_stable() {
        let params = SabrParams::new(0.01, 0.5, -0.2, 0.4).expect("valid params");
        let fwd = 0.0005;
        let t = 1.0;

        let vol_exact = params
            .implied_vol_lognormal(fwd, fwd, t)
            .expect("valid checked inputs");
        let vol_near_above = params
            .implied_vol_lognormal(fwd, fwd + 1e-12, t)
            .expect("valid checked inputs");
        let vol_near_below = params
            .implied_vol_lognormal(fwd, fwd - 1e-12, t)
            .expect("valid checked inputs");

        assert!(vol_exact.is_finite() && vol_exact > 0.0);
        assert_eq!(
            vol_exact, vol_near_above,
            "Low-rate ATM continuity failed above ATM: exact={vol_exact:.6}, above={vol_near_above:.6}"
        );
        assert_eq!(
            vol_exact, vol_near_below,
            "Low-rate ATM continuity failed below ATM: exact={vol_exact:.6}, below={vol_near_below:.6}"
        );
    }

    #[test]
    fn chi_function_small_z() {
        // For small z, χ(z) ≈ z
        let result = chi(1e-12, 0.0).expect("chi should succeed for small z");
        assert!((result - 1e-12).abs() < 1e-20);
    }

    #[test]
    fn chi_function_zero_rho() {
        // For ρ=0, χ(z) = ln(√(1+z²) + z) = arcsinh(z)
        let z = 0.5;
        let result = chi(z, 0.0).expect("chi should succeed for rho=0");
        let expected = z.asinh();
        assert!(
            (result - expected).abs() < 1e-10,
            "χ(z, ρ=0) should equal arcsinh(z): got {result}, expected {expected}"
        );
    }

    #[test]
    fn sabr_invalid_inputs_return_nan() {
        let params = SabrParams::new(0.035, 0.5, -0.2, 0.4).expect("valid params");
        assert!(params.implied_vol_lognormal(-0.01, 0.05, 1.0).is_err());
        assert!(params.implied_vol_lognormal(0.05, -0.01, 1.0).is_err());
        assert!(params.implied_vol_lognormal(0.05, 0.05, 0.0).is_err());
    }

    #[test]
    fn sabr_cross_zero_beta_positive_refuses_silent_shift() {
        // β > 0: the CEV backbone is not shift-invariant, so cross-zero
        // quotes must NOT be priced with a silently invented internal shift.
        let params = SabrParams::new(0.01, 0.5, -0.2, 0.4).expect("valid params");
        let fwd = -0.01;
        let strike = 0.02;
        let t = 1.0;

        assert!(
            params.implied_vol_normal(fwd, strike, t).is_err(),
            "cross-zero with beta > 0 must return NaN"
        );
        let err = params
            .implied_vol_normal(fwd, strike, t)
            .expect_err("cross-zero with beta > 0 must error");
        assert!(
            err.to_string().contains("with_shift"),
            "error should direct users to with_shift: {err}"
        );

        // With an explicit shift the same quote prices fine.
        let shifted = params.with_shift(0.03);
        let vol = shifted
            .implied_vol_normal(fwd, strike, t)
            .expect("shifted SABR should price cross-zero quotes");
        assert!(vol.is_finite() && vol > 0.0);
    }

    #[test]
    fn sabr_cross_zero_beta_zero_is_shift_invariant() {
        // β = 0 (normal SABR) is shift-invariant: cross-zero quotes price
        // directly and the result is insensitive to the internal shift choice.
        let params = SabrParams::new(0.01, 0.0, -0.2, 0.4).expect("valid params");
        let fwd = -0.01;
        let strike = 0.02;
        let t = 1.0;

        let vol = params
            .implied_vol_normal(fwd, strike, t)
            .expect("valid checked inputs");
        assert!(
            vol.is_finite() && vol > 0.0,
            "beta = 0 cross-zero quote should price: {vol}"
        );

        // Shift invariance of the exact normal SABR model: explicit shifts of
        // different sizes agree closely (the log-moneyness expansion is only
        // asymptotically shift-invariant, hence the modest tolerance).
        let vol_s1 = params
            .with_shift(0.05)
            .implied_vol_normal(fwd, strike, t)
            .expect("valid checked inputs");
        let vol_s2 = params
            .with_shift(0.10)
            .implied_vol_normal(fwd, strike, t)
            .expect("valid checked inputs");
        assert!(
            (vol_s1 - vol_s2).abs() / vol_s1 < 5e-2,
            "normal SABR should be (approximately) shift-invariant: \
             s=0.05 → {vol_s1:.6}, s=0.10 → {vol_s2:.6}"
        );
    }

    #[test]
    fn sabr_vol_is_continuous_across_tiny_nu() {
        // The old ν < 1e-10 CEV fallback dropped the (1 + [...]T) correction
        // terms, creating a vol jump across the threshold. The general path
        // now degenerates smoothly.
        let f = 0.05;
        let t = 2.0;
        for &beta in &[0.0, 0.5, 1.0] {
            for &k in &[0.04, 0.05, 0.06] {
                let below = SabrParams {
                    alpha: 0.03,
                    beta,
                    rho: -0.2,
                    nu: 1e-10 - 1e-12,
                    shift: None,
                };
                let above = SabrParams {
                    alpha: 0.03,
                    beta,
                    rho: -0.2,
                    nu: 1e-10 + 1e-12,
                    shift: None,
                };
                // Tolerance: the χ(z) evaluation carries ~1e-16/z relative
                // cancellation noise for z ≈ 1e-10, i.e. ~1e-7 on the vol.
                // The pre-fix discontinuity (dropped correction terms) was
                // orders of magnitude larger (~1e-3 relative).
                let v_below = below
                    .implied_vol_lognormal(f, k, t)
                    .expect("valid checked inputs");
                let v_above = above
                    .implied_vol_lognormal(f, k, t)
                    .expect("valid checked inputs");
                assert!(
                    (v_below - v_above).abs() < 1e-7,
                    "lognormal vol discontinuity at ν=1e-10 (β={beta}, K={k}): \
                     below={v_below:.12}, above={v_above:.12}"
                );

                let n_below = below
                    .implied_vol_normal(f, k, t)
                    .expect("valid checked inputs");
                let n_above = above
                    .implied_vol_normal(f, k, t)
                    .expect("valid checked inputs");
                assert!(
                    (n_below - n_above).abs() < 1e-7,
                    "normal vol discontinuity at ν=1e-10 (β={beta}, K={k}): \
                     below={n_below:.12}, above={n_above:.12}"
                );
            }
        }
    }

    #[test]
    fn sabr_tiny_nu_includes_hagan_time_correction() {
        // In the ν → 0 CEV limit the lognormal vol must retain the
        // (1 + (1−β)²/24 · α²/(FK)^(1−β) · T) correction (Hagan 2002), which
        // the old fallback dropped.
        let alpha = 0.3;
        let beta = 0.5;
        let f: f64 = 0.05;
        let t = 5.0;
        let p = SabrParams {
            alpha,
            beta,
            rho: 0.0,
            nu: 1e-12,
            shift: None,
        };
        let got = p
            .implied_vol_lognormal(f, f, t)
            .expect("valid checked inputs");
        let omb = 1.0 - beta;
        let f_omb = f.powf(omb);
        let want = alpha / f_omb * (1.0 + omb * omb / 24.0 * alpha * alpha / (f_omb * f_omb) * t);
        assert!(
            (got - want).abs() < 1e-9,
            "CEV limit must keep the time correction: got {got:.10}, want {want:.10}"
        );
    }

    #[test]
    fn sabr_params_serde_validates_on_deserialize() {
        // Valid JSON round-trips, including the optional shift.
        let p = SabrParams::new(0.035, 0.5, -0.2, 0.4)
            .expect("valid")
            .with_shift(0.03);
        let json = serde_json::to_string(&p).expect("serialize");
        let back: SabrParams = serde_json::from_str(&json).expect("round-trip");
        assert_eq!(p, back);

        // Shift omitted on the wire when None.
        let p_no_shift = SabrParams::new(0.035, 0.5, -0.2, 0.4).expect("valid");
        let json_no_shift = serde_json::to_string(&p_no_shift).expect("serialize");
        assert!(!json_no_shift.contains("shift"));
        let back2: SabrParams = serde_json::from_str(&json_no_shift).expect("round-trip");
        assert_eq!(p_no_shift, back2);

        // Out-of-range rho rejected.
        let bad = r#"{"alpha":0.035,"beta":0.5,"rho":1.5,"nu":0.4}"#;
        assert!(serde_json::from_str::<SabrParams>(bad).is_err());

        // Unknown field rejected.
        let unknown = r#"{"alpha":0.035,"beta":0.5,"rho":-0.2,"nu":0.4,"extra":1.0}"#;
        assert!(serde_json::from_str::<SabrParams>(unknown).is_err());
    }

    #[test]
    fn test_sabr_params_with_shift() {
        let p = SabrParams::new(0.035, 0.5, -0.2, 0.4).expect("valid params");
        assert!(p.shift.is_none());

        let shifted = p.with_shift(0.03);
        assert_eq!(shifted.shift, Some(0.03));
    }

    #[test]
    fn test_shifted_sabr_implied_vol_lognormal() {
        // Shifted SABR: evaluate with F+shift, K+shift
        let p = SabrParams::new(0.035, 0.5, -0.2, 0.4)
            .expect("valid params")
            .with_shift(0.03);
        let f = -0.005; // negative forward
        let k = 0.01;
        let t = 1.0;
        let vol = p
            .implied_vol_lognormal(f, k, t)
            .expect("valid checked inputs");
        assert!(
            vol.is_finite() && vol > 0.0,
            "shifted SABR should handle negative rates"
        );
    }
}
