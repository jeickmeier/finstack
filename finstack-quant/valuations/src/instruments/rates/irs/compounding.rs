//! Compounding conventions for floating leg calculations in interest rate swaps.
//!
//! Defines how floating rate coupons are calculated based on the
//! underlying reference rate (LIBOR, SOFR, SONIA, etc.).
//!
//! # Implementation Notes
//!
//! ## Compounded-in-Arrears (Full Daily Compounding)
//!
//! For overnight-indexed swaps (OIS) with `CompoundedInArrears` compounding,
//! the implementation uses **full daily compounding** per ISDA 2021:
//!
//! ```text
//! Coupon = N × [∏(1 + r_i × dcf_i) - 1] + spread × accrual
//! ```
//!
//! where the product is taken over daily observations in the accrual period.
//!
//! ## Fast Path for Unseasoned Single-Curve OIS
//!
//! When all of the following conditions are met, the discount curve identity
//! is used as an optimization:
//!
//! - The contract uses `CompoundedInArrears { lookback_days: 0 }`
//! - Forward curve ID matches discount curve ID (single-curve)
//!
//! In this case:
//! ```text
//! ∏(1 + r_i × dcf_i) = DF(start) / DF(end)
//! ```
//!
//! This identity is exact and avoids iterating over daily observations.
//!
//! ## Lookback and Observation Shift
//!
//! Lookback and observation shift are distinct, mutually exclusive enum
//! variants:
//!
//! - **Lookback** (`CompoundedInArrears`): shifts observation dates backward
//!   while day-count weights remain on the original accrual dates.
//! - **Observation shift** (`CompoundedWithObservationShift`): shifts both
//!   observations and their day-count weights backward.
//!
//! Either non-zero convention disables the discount-identity fast path and
//! performs full daily compounding.
//!
//! ## Seasoned Swaps
//!
//! For seasoned swaps where `as_of` falls within an accrual period, historical
//! fixings are required for observation dates before `as_of`. Provide fixings
//! via `ScalarTimeSeries` with id `FIXING:{forward_curve_id}`.
//!
//! # References
//!
//! - **ISDA 2021 Definitions**: Compounded RFR conventions `docs/REFERENCES.md#isda-2021-definitions`
//! - **ARRC** (Alternative Reference Rates Committee): SOFR conventions `docs/REFERENCES.md#arrc-sofr-users-guide`
//! - **BoE** (Bank of England): SONIA conventions `docs/REFERENCES.md#boe-sonia-key-features`

/// Method for calculating floating leg coupon payments.
///
/// Different reference rates require different compounding conventions:
/// - **Term rates (SOFR 3M, EURIBOR, historical LIBOR)**: Simple interest
/// - **Overnight rates (SOFR, SONIA, €STR, TONA)**: Compounded in arrears
///
/// # Market Standards
///
/// ## Simple term-rate coupons
/// - **Formula**: `Coupon = Notional × (Forward_Rate + Spread) × DCF`
/// - **Use for**: current term-rate indices and legacy IBOR transactions
/// - **Standard**: ISDA 2021 Definitions; ISDA 2006 for legacy transactions
///
/// ## Compounded In Arrears (RFR-style)
/// - **Formula**: `Coupon = Notional × [∏(1 + r_i × dcf_i) - 1]`
/// - **Use for**: USD SOFR, GBP SONIA, EUR €STR, JPY TONA
/// - **Standard**: ISDA 2021 Definitions
/// - **Observation convention**: plain in-arrears for standard OIS presets;
///   lookback, observation shift, and cutoff are explicit contract variants
///
/// # Examples
///
/// ```
/// use finstack_quant_valuations::instruments::rates::irs::FloatingLegCompounding;
///
/// // LIBOR-style swap (simple compounding)
/// let simple = FloatingLegCompounding::Simple;
/// assert_eq!(simple, FloatingLegCompounding::default());
///
/// // SOFR OIS swap: plain compounded in arrears (no lookback)
/// let sofr = FloatingLegCompounding::CompoundedInArrears { lookback_days: 0 };
/// assert_eq!(sofr, FloatingLegCompounding::sofr());
///
/// // SONIA FRN-style leg with the BoE 5-day lookback (explicit, not the OIS preset)
/// let sonia_frn = FloatingLegCompounding::CompoundedInArrears { lookback_days: 5 };
/// assert_ne!(sonia_frn, FloatingLegCompounding::sonia());
/// ```
///
/// # References
///
/// - **ISDA 2021 Definitions**: Compounded RFR conventions `docs/REFERENCES.md#isda-2021-definitions`
/// - **ARRC** (Alternative Reference Rates Committee): SOFR conventions `docs/REFERENCES.md#arrc-sofr-users-guide`
/// - **BoE** (Bank of England): SONIA conventions `docs/REFERENCES.md#boe-sonia-key-features`
/// - **ECB**: €STR conventions `docs/REFERENCES.md#ecb-estr-methodology`
///
/// In the IRS instrument implementation, the RFR-style variant
/// (`CompoundedInArrears`) is also used to classify swaps as OIS for
/// discount-only float-leg pricing; see `InterestRateSwap::is_single_curve_ois` for details.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[non_exhaustive]
#[serde(rename_all = "snake_case")]
pub enum FloatingLegCompounding {
    /// Simple interest compounding (term-rate style).
    ///
    /// Coupon = Notional × (Forward_Rate + Spread) × Day_Count_Fraction
    ///
    /// Use for:
    /// - Current fixed-tenor term-rate indices
    /// - Legacy USD/EUR/GBP LIBOR swaps
    ///
    /// This is the generic vanilla-IRS default; the rate-index convention
    /// registry selects compounded RFR terms for overnight indices.
    Simple,

    /// Compounded in arrears (overnight RFR rates).
    ///
    /// Coupon = Notional × [∏(1 + r_i × dcf_i) - 1] where the product
    /// is taken over daily observations in the accrual period.
    ///
    /// Use for:
    /// - USD SOFR (Secured Overnight Financing Rate)
    /// - GBP SONIA (Sterling Overnight Index Average)
    /// - EUR €STR (Euro Short-Term Rate)
    /// - JPY TONA (Tokyo Overnight Average Rate)
    ///
    /// # Fields
    ///
    /// - `lookback_days`: Business days by which observation dates move
    ///   backward while accrual day-count weights remain unshifted.
    ///
    /// Standard cleared OIS presets use zero lookback. Non-zero lookbacks are
    /// explicit contractual variants, commonly used for RFR-linked notes.
    CompoundedInArrears {
        /// Number of business days to shift observation dates back from the accrual
        /// period (lookback).  Typically 2–5 days depending on market convention.
        ///
        /// The observation dates are shifted while the day-count-fraction (DCF)
        /// weights remain anchored to the **original** accrual period dates.
        /// This is consistent with "lookback without observation shift" as
        /// described in the ISDA 2021 Definitions and ARRC SOFR conventions.
        #[cfg_attr(feature = "json-schema", schemars(range(min = 0, max = 31)))]
        lookback_days: i32,
    },

    /// Compounded in arrears with true ISDA 2021 observation shift.
    ///
    /// Unlike `CompoundedInArrears` (lookback semantics), this variant shifts
    /// **both** the observation dates AND the day-count-fraction (DCF) weights.
    /// This matches ISDA 2021 Definitions Section 4.5(c).
    ///
    /// ```text
    /// Lookback:           DCF(d, d+1)           × rate(d - shift, d+1 - shift)
    /// Observation Shift:  DCF(d - shift, d+1 - shift) × rate(d - shift, d+1 - shift)
    /// ```
    CompoundedWithObservationShift {
        /// Number of business days to shift both observation dates and DCF weights.
        #[cfg_attr(feature = "json-schema", schemars(range(min = 0, max = 31)))]
        shift_days: i32,
    },

    /// Compounded in arrears with a rate cut-off near the period end.
    ///
    /// This freezes the last observed overnight rate for the final `cutoff_days`
    /// business days of the accrual period. Bloomberg SWPM labels this
    /// convention as "Rate Cut-Off Days".
    CompoundedWithRateCutoff {
        /// Number of business days before period end to freeze the overnight rate.
        #[cfg_attr(feature = "json-schema", schemars(range(min = 0, max = 31)))]
        cutoff_days: i32,
    },
}

impl Default for FloatingLegCompounding {
    /// Default to simple compounding (LIBOR-style, most conservative).
    fn default() -> Self {
        Self::Simple
    }
}

/// Market-standard compounding presets for common RFR **swaps** (cleared OIS).
///
/// Cleared OIS swaps compound the overnight rate plain in-arrears over the
/// accrual period, with only a payment delay — no observation lookback or
/// shift. The ARRC 2-business-day and BoE 5-business-day lookbacks are *FRN
/// coupon* conventions, not OIS swap conventions; using them on OIS both
/// introduces a small basis (sub-bp to ~1bp) and disables the exact `1/DF`
/// telescoping fast path. FRN-style legs should use
/// [`FloatingLegCompounding::CompoundedInArrears`] with an explicit lookback or
/// the `*_observation_shift` presets.
///
/// # Day count is NOT part of these presets
///
/// A preset only sets the compounding method; the leg's `day_count` must be
/// configured to the index's own basis separately (ISDA 2021 Definitions):
/// ACT/360 for SOFR / EFFR / €STR / SARON, but **ACT/365F for SONIA and
/// TONA**. Pairing [`Self::sonia`] or [`Self::tona`] with an ACT/360 leg
/// misstates every accrual by ~365/360 (≈ 1.4% of the coupon). The
/// currency-level `SwapConvention` templates carry the correct pairing;
/// hand-built legs must set it explicitly.
impl FloatingLegCompounding {
    /// USD SOFR OIS convention (plain compounded in arrears, payment delay only).
    pub fn sofr() -> Self {
        Self::CompoundedInArrears { lookback_days: 0 }
    }

    /// USD Fed Funds / EFFR-style overnight convention (no lookback).
    ///
    /// Bloomberg `FEDL01 Index` OIS conventions typically do **not** apply the SOFR-style
    /// observation lookback. We model that as `lookback_days = 0`.
    pub fn fedfunds() -> Self {
        Self::CompoundedInArrears { lookback_days: 0 }
    }

    /// GBP SONIA OIS convention (plain compounded in arrears).
    ///
    /// The BoE 5-business-day lookback applies to SONIA FRNs, not OIS swaps.
    pub fn sonia() -> Self {
        Self::CompoundedInArrears { lookback_days: 0 }
    }

    /// EUR €STR OIS convention (plain compounded in arrears).
    pub fn estr() -> Self {
        Self::CompoundedInArrears { lookback_days: 0 }
    }

    /// JPY TONA OIS convention (plain compounded in arrears).
    pub fn tona() -> Self {
        Self::CompoundedInArrears { lookback_days: 0 }
    }

    /// CHF SARON OIS convention (plain compounded in arrears).
    pub fn saron() -> Self {
        Self::CompoundedInArrears { lookback_days: 0 }
    }

    /// USD SOFR with ISDA 2021 observation shift (2-day shift).
    pub fn sofr_observation_shift() -> Self {
        Self::CompoundedWithObservationShift { shift_days: 2 }
    }

    /// GBP SONIA with ISDA 2021 observation shift (5-day shift).
    pub fn sonia_observation_shift() -> Self {
        Self::CompoundedWithObservationShift { shift_days: 5 }
    }

    /// Compounded RFR with an end-of-period rate cut-off.
    pub fn rate_cutoff(cutoff_days: i32) -> Self {
        Self::CompoundedWithRateCutoff { cutoff_days }
    }
}

impl std::fmt::Display for FloatingLegCompounding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FloatingLegCompounding::Simple => write!(f, "simple"),
            FloatingLegCompounding::CompoundedInArrears { .. } => {
                write!(f, "compounded_in_arrears")
            }
            FloatingLegCompounding::CompoundedWithObservationShift { .. } => {
                write!(f, "compounded_observation_shift")
            }
            FloatingLegCompounding::CompoundedWithRateCutoff { .. } => {
                write!(f, "compounded_rate_cutoff")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_simple() {
        assert_eq!(
            FloatingLegCompounding::default(),
            FloatingLegCompounding::Simple
        );
    }

    #[test]
    fn test_market_presets() {
        // Cleared OIS compounds plain in-arrears (payment delay only); the
        // ARRC 2bd / BoE 5bd lookbacks are FRN conventions, not OIS.
        for preset in [
            FloatingLegCompounding::sofr(),
            FloatingLegCompounding::sonia(),
            FloatingLegCompounding::estr(),
            FloatingLegCompounding::tona(),
            FloatingLegCompounding::saron(),
            FloatingLegCompounding::fedfunds(),
        ] {
            assert_eq!(
                preset,
                FloatingLegCompounding::CompoundedInArrears { lookback_days: 0 }
            );
        }
    }

    #[test]
    fn test_serde_roundtrip() {
        let methods = vec![
            FloatingLegCompounding::Simple,
            FloatingLegCompounding::sofr(),
            FloatingLegCompounding::sonia(),
        ];

        for method in methods {
            let json =
                serde_json::to_string(&method).expect("Serialization should succeed in test");
            let deserialized: FloatingLegCompounding =
                serde_json::from_str(&json).expect("Deserialization should succeed in test");
            assert_eq!(method, deserialized);
        }
    }

    #[test]
    fn rate_cutoff_roundtrips() {
        let method = FloatingLegCompounding::CompoundedWithRateCutoff { cutoff_days: 1 };
        let json = serde_json::to_string(&method).expect("serialize rate cutoff");
        let deserialized: FloatingLegCompounding =
            serde_json::from_str(&json).expect("deserialize rate cutoff");

        assert_eq!(deserialized, method);
    }
}
