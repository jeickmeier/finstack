//! Discount-curve construction, queries, and forward derivation.

use super::super::common::infer_discount_curve_day_count;
use super::super::forward_curve::ForwardCurve;
use super::{DiscountCurve, DiscountCurveBuilder, ValidationMode, DEFAULT_MIN_FORWARD_TENOR};
use crate::math::interp::{ExtrapolationPolicy, InterpStyle};
use crate::math::Compounding;
use crate::{
    dates::{Date, DayCount, DayCountContext},
    types::CurveId,
};

impl DiscountCurve {
    /// Construct a flat continuously-compounded discount curve.
    ///
    /// The curve uses the minimal two-knot representation
    /// `(0, 1)` and `(1, exp(-rate))`, log-linear interpolation, and
    /// flat-forward extrapolation. This preserves `DF(t) = exp(-rate * t)`
    /// for every non-negative maturity.
    ///
    /// # Errors
    ///
    /// Returns an error when `continuous_rate` is non-finite or its one-year
    /// discount factor cannot be represented as a finite positive value.
    pub fn flat(id: impl AsRef<str>, base_date: Date, continuous_rate: f64) -> crate::Result<Self> {
        if !continuous_rate.is_finite() {
            return Err(crate::Error::Validation(
                "DiscountCurve: flat continuous rate must be finite".to_string(),
            ));
        }
        let one_year_df = crate::math::Compounding::Continuous.df_from_rate(continuous_rate, 1.0);
        if !one_year_df.is_finite() || one_year_df <= 0.0 {
            return Err(crate::Error::Validation(format!(
                "DiscountCurve: flat continuous rate {continuous_rate} produces an invalid discount factor"
            )));
        }

        Self::builder(id.as_ref())
            .base_date(base_date)
            .knots([(0.0, 1.0), (1.0, one_year_df)])
            .interp(InterpStyle::LogLinear)
            .extrapolation(ExtrapolationPolicy::FlatForward)
            .validation(ValidationMode::Raw {
                allow_non_monotonic: continuous_rate < 0.0,
                forward_floor: None,
            })
            .build()
    }

    /// Unique identifier of the curve.
    #[inline]
    pub fn id(&self) -> &CurveId {
        &self.id
    }

    /// Base (valuation) date of the curve.
    #[inline]
    pub fn base_date(&self) -> Date {
        self.base
    }

    /// Day-count basis used for discount time mapping.
    #[inline]
    pub fn day_count(&self) -> DayCount {
        self.day_count
    }

    /// Interpolation style used by this curve.
    #[inline]
    pub fn interp_style(&self) -> InterpStyle {
        self.style
    }

    /// Extrapolation policy used by this curve.
    #[inline]
    pub fn extrapolation(&self) -> ExtrapolationPolicy {
        self.extrapolation
    }

    /// Exact typed conventions and quotes used to calibrate this curve.
    #[inline]
    pub fn rate_calibration(&self) -> Option<&super::super::RateCalibrationRecipe> {
        self.rate_calibration.as_ref()
    }

    /// OIS rate cut-off (business days) this curve was calibrated under, if any.
    ///
    /// Returns `None` for curves calibrated under a non-cut-off convention or
    /// hand-built curves with no calibration provenance.
    #[inline]
    pub fn calibration_ois_cutoff_days(&self) -> Option<i32> {
        self.calibration_ois_cutoff_days
    }

    /// Opaque FX policy stamp from curve construction, if any.
    ///
    /// Propagated onto `ResultsMeta.fx_policy_applied` for dependent instruments.
    #[inline]
    pub fn fx_policy(&self) -> Option<&str> {
        self.fx_policy.as_deref()
    }

    /// Number of knot points in the curve.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.knots.len()
    }

    /// Returns `true` if the curve has no knot points.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.knots.is_empty()
    }

    /// Continuously-compounded zero rate.
    ///
    /// Formula: `r_cc = -ln(DF) / t`
    #[must_use]
    #[inline]
    pub fn zero(&self, t: f64) -> f64 {
        if t == 0.0 {
            return 0.0;
        }
        -self.df(t).ln() / t
    }

    /// Annually-compounded zero rate (bond equivalent yield convention).
    ///
    /// This is the rate quoted for most bonds and is commonly used by
    /// Bloomberg for displaying zero rates.
    ///
    /// Formula: `r_annual = DF^(-1/t) - 1`
    ///
    /// # Example
    ///
    /// ```
    /// use finstack_quant_core::market_data::term_structures::DiscountCurve;
    /// use finstack_quant_core::dates::Date;
    /// use time::Month;
    ///
    /// let curve = DiscountCurve::builder("USD-OIS")
    ///     .base_date(Date::from_calendar_date(2025, Month::January, 1).expect("Valid date"))
    ///     .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80)])
    ///     .build()
    ///     .expect("DiscountCurve should build");
    ///
    /// // At 1Y, DF = 0.95, so annual rate = 0.95^(-1) - 1 ≈ 5.26%
    /// let annual_rate = curve.zero_annual(1.0);
    /// assert!((annual_rate - 0.0526).abs() < 0.001);
    /// ```
    #[inline]
    pub fn zero_annual(&self, t: f64) -> f64 {
        if t == 0.0 {
            return 0.0;
        }
        self.df(t).powf(-1.0 / t) - 1.0
    }

    /// Periodically-compounded zero rate with `n` compounding periods per year.
    ///
    /// Common values for `n`:
    /// - 1: Annual (same as `zero_annual`)
    /// - 2: Semi-annual (US Treasury convention)
    /// - 4: Quarterly
    /// - 12: Monthly
    ///
    /// Formula: `r_periodic = n * (DF^(-1/(n*t)) - 1)`
    ///
    /// # Example
    ///
    /// ```
    /// use finstack_quant_core::market_data::term_structures::DiscountCurve;
    /// use finstack_quant_core::dates::Date;
    /// use time::Month;
    ///
    /// let curve = DiscountCurve::builder("USD-OIS")
    ///     .base_date(Date::from_calendar_date(2025, Month::January, 1).expect("Valid date"))
    ///     .knots([(0.0, 1.0), (1.0, 0.95), (5.0, 0.80)])
    ///     .build()
    ///     .expect("DiscountCurve should build");
    ///
    /// // Semi-annual compounded rate at 1Y
    /// let semi_annual_rate = curve.zero_periodic(1.0, 2);
    /// // Annual rate should equal periodic with n=1
    /// let annual_via_periodic = curve.zero_periodic(1.0, 1);
    /// assert!((curve.zero_annual(1.0) - annual_via_periodic).abs() < 1e-12);
    /// ```
    #[inline]
    pub fn zero_periodic(&self, t: f64, n: u32) -> f64 {
        if t == 0.0 || n == 0 {
            return 0.0;
        }
        let n_f = n as f64;
        n_f * (self.df(t).powf(-1.0 / (n_f * t)) - 1.0)
    }

    /// Simple interest (money market) zero rate.
    ///
    /// Returns the simple interest rate (no compounding) implied by the discount factor.
    /// This is the standard convention for money market instruments with tenors under 1 year,
    /// including deposits, CDs, T-bills, and short-term rate fixings.
    ///
    /// # Compounding Convention
    ///
    /// **Simple interest means NO compounding.** Interest accrues linearly:
    /// - Future Value = Principal × (1 + rate × time)
    /// - This differs from annually compounded rates which compound once per year
    ///
    /// # Formula
    ///
    /// ```text
    /// r_simple = (1/DF - 1) / t
    /// ```
    ///
    /// Derived from the simple interest present value formula: `DF(t) = 1 / (1 + r × t)`
    ///
    /// # Market Standards
    ///
    /// Simple interest is the market convention for:
    /// - **USD**: SOFR, Fed Funds, T-bills, CDs, deposits (< 1Y tenor)
    /// - **EUR**: €STR, Euribor fixings
    /// - **GBP**: SONIA
    /// - **Most markets**: Interbank deposits, repo rates
    ///
    /// **Day count**: Typically paired with ACT/360 (USD, EUR) or ACT/365F (GBP).
    ///
    /// # Bloomberg Equivalent
    ///
    /// This matches Bloomberg's simple interest zero rate output when compounding
    /// is set to "Simple" in curve display screens (e.g., SWPM, SWCV).
    ///
    /// # Comparison with Other Rate Conventions
    ///
    /// For a given discount factor at time t:
    /// - `zero()` returns continuously compounded rate: `r_cc = -ln(DF) / t`
    /// - `zero_annual()` returns annually compounded: `r_annual = DF^(-1/t) - 1`
    /// - `zero_simple()` returns simple interest: `r_simple = (1/DF - 1) / t`
    ///
    /// For positive rates and t > 0: `r_simple > r_annual > r_cc`
    ///
    /// # Example
    ///
    /// ```
    /// use finstack_quant_core::market_data::term_structures::DiscountCurve;
    /// use finstack_quant_core::dates::Date;
    /// use time::Month;
    ///
    /// let curve = DiscountCurve::builder("USD-OIS")
    ///     .base_date(Date::from_calendar_date(2025, Month::January, 1).expect("Valid date"))
    ///     .knots([(0.0, 1.0), (0.25, 0.99), (1.0, 0.95)])
    ///     .build()
    ///     .expect("DiscountCurve should build");
    ///
    /// // At 3M (0.25Y), DF = 0.99, so simple rate = (1/0.99 - 1) / 0.25 ≈ 4.04%
    /// let simple_rate = curve.zero_simple(0.25);
    /// assert!((simple_rate - 0.0404).abs() < 0.001);
    /// ```
    #[inline]
    pub fn zero_simple(&self, t: f64) -> f64 {
        if t == 0.0 {
            return 0.0;
        }
        (1.0 / self.df(t) - 1.0) / t
    }

    /// Continuously-compounded forward rate between `t1` and `t2`.
    ///
    /// The forward rate `f(t1, t2)` satisfies `DF(t2) = DF(t1) · exp(-f · (t2 − t1))`,
    /// so equivalently
    ///
    /// ```text
    /// f(t1, t2) = -ln(DF(t2) / DF(t1)) / (t2 - t1).
    /// ```
    ///
    /// This is the form evaluated here. The algebraically equivalent
    /// zero-rate form `(z2·t2 − z1·t1) / (t2 − t1)` (with `z·t =
    /// -ln(DF)`) round-trips each endpoint through an extra division
    /// and multiplication — two wasted ulps — and costs two `ln`
    /// evaluations instead of one. The current form avoids both and
    /// matches the canonical identity to ~1 ulp even at sub-
    /// millisecond tenors.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - `t1` or `t2` is non-finite
    /// - `t2 <= t1`
    /// - `(t2 − t1) < min_forward_tenor` (configurable, default ~30 seconds) to avoid
    ///   numerical precision issues from catastrophic cancellation
    /// - either `DF(t1)` or `DF(t2)` is non-positive (pathological curve)
    ///
    /// # Configuring Minimum Tenor
    ///
    /// The minimum forward tenor can be customized when building the curve:
    /// ```ignore
    /// use finstack_quant_core::market_data::term_structures::DiscountCurve;
    /// # use time::macros::date;
    /// # fn main() -> finstack_quant_core::Result<()> {
    /// let curve = DiscountCurve::builder("USD")
    ///     .base_date(date!(2025-01-01))
    ///     .knots([(0.0, 1.0), (1.0, 0.95)])
    ///     .min_forward_tenor(1e-8)  // Allow very short tenors
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Arguments
    ///
    /// * `t1` - Start year-fraction of the forward or rate interval being queried
    /// * `t2` - End year-fraction of the forward or rate interval being queried
    #[inline]
    #[must_use = "computed forward rate should not be discarded"]
    pub fn forward(&self, t1: f64, t2: f64) -> crate::Result<f64> {
        if !t1.is_finite() || !t2.is_finite() || t2 <= t1 {
            return Err(crate::error::InputError::Invalid.into());
        }
        if (t2 - t1) < self.min_forward_tenor {
            return Err(crate::error::InputError::Invalid.into());
        }
        let df1 = self.df(t1);
        let df2 = self.df(t2);
        if !(df1.is_finite() && df1 > 0.0 && df2.is_finite() && df2 > 0.0) {
            return Err(crate::error::InputError::Invalid.into());
        }
        Ok(-(df2 / df1).ln() / (t2 - t1))
    }

    /// Get the minimum forward tenor configured for this curve.
    #[inline]
    pub fn min_forward_tenor(&self) -> f64 {
        self.min_forward_tenor
    }

    /// Whether validation permits increasing discount factors.
    #[inline]
    pub fn allows_non_monotonic(&self) -> bool {
        self.allow_non_monotonic
    }

    /// Minimum implied forward rate accepted by validation, if configured.
    #[inline]
    pub fn min_forward_rate(&self) -> Option<f64> {
        self.min_forward_rate
    }

    /// Fallible: discount factor on a specific date `date` using explicit day-count `day_count`.
    ///
    /// # Errors
    ///
    /// Propagates a failure from `day_count.signed_year_fraction` for the curve base
    /// date and `date`.
    #[inline]
    #[must_use = "computed discount factor should not be discarded"]
    pub fn df_on_date(&self, date: Date, day_count: crate::dates::DayCount) -> crate::Result<f64> {
        let t = if date == self.base {
            0.0
        } else {
            day_count.signed_year_fraction(self.base, date, DayCountContext::default())?
        };
        Ok(self.df(t))
    }

    /// Fallible: discount factor on a specific date `date` using the curve's day-count.
    ///
    /// # Errors
    ///
    /// Propagates a failure while computing the curve day-count fraction from
    /// the base date to `date`.
    #[inline]
    #[must_use = "computed discount factor should not be discarded"]
    pub fn df_on_date_curve(&self, date: Date) -> crate::Result<f64> {
        let t = self.year_fraction_to(date)?;
        Ok(self.df(t))
    }

    /// Fallible: discount factor from `from` to `to` using the curve's day-count.
    ///
    /// This is the canonical helper for the common "relative DF" pattern:
    /// `DF(from→to) = DF(0→to) / DF(0→from)`.
    ///
    /// Works for both forward and backward date order. Returns `1.0` when
    /// `from == to`.
    ///
    /// # Errors
    ///
    /// Propagates failures while computing either date's curve year fraction,
    /// and returns `Error::Validation` when an evaluated discount factor is
    /// non-finite or non-positive.
    #[inline]
    #[must_use = "computed discount factor should not be discarded"]
    pub fn df_between_dates(&self, from: Date, to: Date) -> crate::Result<f64> {
        if from == to {
            return Ok(1.0);
        }

        let df_from = self.df_on_date_curve(from)?;
        if !df_from.is_finite() || df_from <= 0.0 {
            return Err(crate::Error::Validation(format!(
                "Invalid discount factor on 'from' date ({from}): {df_from}"
            )));
        }

        let df_to = self.df_on_date_curve(to)?;
        if !df_to.is_finite() || df_to <= 0.0 {
            return Err(crate::Error::Validation(format!(
                "Invalid discount factor on 'to' date ({to}): {df_to}"
            )));
        }
        Ok(df_to / df_from)
    }

    /// Returns the zero rate for a given date with specified compounding convention.
    ///
    /// This is the unified method for obtaining zero rates under any compounding convention.
    /// It replaces the individual `zero_on_date`, `zero_annual_on_date`, `zero_periodic_on_date`,
    /// and `zero_simple_on_date` methods.
    ///
    /// # Arguments
    /// * `date` - Target date for the zero rate
    /// * `compounding` - Compounding convention (Continuous, Annual, Periodic(n), Simple)
    ///
    /// # Mathematical Formulas
    ///
    /// For a discount factor `df` and time `t`:
    ///
    /// | Compounding | Formula | Use Case |
    /// |-------------|---------|----------|
    /// | Continuous | r = -ln(df) / t | Internal calculations, curve building |
    /// | Annual | r = df^(-1/t) - 1 | Bond markets (UK, Europe) |
    /// | Periodic(n) | r = n × (df^(-1/(n×t)) - 1) | US Treasuries (n=2), corporates |
    /// | Simple | r = (1/df - 1) / t | Money market (< 1Y) |
    ///
    /// # Examples
    ///
    /// ```rust
    /// use finstack_quant_core::market_data::term_structures::DiscountCurve;
    /// use finstack_quant_core::math::Compounding;
    /// use finstack_quant_core::dates::Date;
    /// use time::Month;
    ///
    /// let anchor = Date::from_calendar_date(2024, Month::January, 2).unwrap();
    /// // Build a flat 5% curve (df at 1Y = exp(-0.05 * 1) ≈ 0.9512)
    /// let curve = DiscountCurve::builder("USD-OIS")
    ///     .base_date(anchor)
    ///     .knots([(0.0, 1.0), (1.0, (-0.05_f64).exp())])
    ///     .build()?;
    /// let target = Date::from_calendar_date(2025, Month::January, 2).unwrap();
    ///
    /// // Continuous rate (default for internal calculations)
    /// let r_cont = curve.zero_rate_on_date(target, Compounding::Continuous)?;
    ///
    /// // Annual rate (for European bonds)
    /// let r_ann = curve.zero_rate_on_date(target, Compounding::Annual)?;
    ///
    /// // Semi-annual rate (for US Treasuries)
    /// let r_semi = curve.zero_rate_on_date(target, Compounding::SEMI_ANNUAL)?;
    ///
    /// // Simple rate (for money market)
    /// let r_simple = curve.zero_rate_on_date(target, Compounding::Simple)?;
    /// # Ok::<(), finstack_quant_core::Error>(())
    /// ```
    ///
    /// # Errors
    /// Returns an error if the date is before the anchor.
    #[inline]
    #[must_use = "computed zero rate should not be discarded"]
    pub fn zero_rate_on_date(
        &self,
        date: Date,
        compounding: crate::math::Compounding,
    ) -> crate::Result<f64> {
        let t = self.year_fraction_to(date)?;
        Ok(self.zero_rate(t, compounding))
    }

    /// Returns the zero rate for a given year fraction with specified compounding.
    ///
    /// This is the unified method for obtaining zero rates under any compounding convention.
    ///
    /// # Arguments
    /// * `t` - Year fraction from the anchor date
    /// * `compounding` - Compounding convention (Continuous, Annual, Periodic(n), Simple)
    ///
    /// # Edge Cases
    /// - For t = 0, all compounding conventions return 0.0 (instantaneous rate is undefined)
    #[inline]
    #[must_use]
    pub fn zero_rate(&self, t: f64, compounding: crate::math::Compounding) -> f64 {
        match compounding {
            Compounding::Continuous => self.zero(t),
            Compounding::Annual => self.zero_annual(t),
            Compounding::Periodic(n) => self.zero_periodic(t, n.get()),
            Compounding::Simple => self.zero_simple(t),
        }
    }

    /// Helper: compute year fraction from base date to target date using curve's day-count.
    #[inline]
    fn year_fraction_to(&self, date: Date) -> crate::Result<f64> {
        super::super::common::year_fraction_to(self.base, date, self.day_count)
    }

    /// Discount factor at time `t` (helper calling the underlying interpolator).
    #[must_use]
    #[inline]
    pub fn df(&self, t: f64) -> f64 {
        self.interp.interp(t)
    }

    /// Raw knot times (t) in **years** passed at construction.
    #[inline]
    pub fn knots(&self) -> &[f64] {
        &self.knots
    }

    /// Raw discount factors corresponding to each knot.
    #[inline]
    pub fn dfs(&self) -> &[f64] {
        &self.dfs
    }

    /// Builder entry-point.
    ///
    /// Takes the curve identifier as a required argument because every curve
    /// is uniquely keyed by its `CurveId`, and the remaining parameters
    /// (`base`, `day_count`, interpolation, etc.) all have sensible defaults.
    /// This makes `DiscountCurve::builder("USD-OIS")` both concise and
    /// self-documenting.
    ///
    /// **Design note:** This `Type::builder(id)` pattern is used consistently
    /// across all `finstack-quant-core` term structures (discount, forward, hazard,
    /// inflation, price, vol-index, vol-surface, base-correlation). Instrument
    /// types in `finstack-quant-valuations` use a different convention —
    /// `Type::builder()` with no args — because instruments have many
    /// required fields where named setters are more practical than positional
    /// arguments. See the `FinancialBuilder` derive macro docs for the full
    /// rationale.
    ///
    /// **Note:** Monotonic discount factor validation is enabled by default to ensure
    /// no-arbitrage conditions. Use [`DiscountCurveBuilder::validation`] with
    /// [`ValidationMode::Raw`] if you need to disable this validation (not
    /// recommended for production use).
    ///
    /// **Defaults:** The builder infers a market day-count from the curve ID when
    /// possible (for example `USD-OIS -> Act360`, `GBP-SONIA -> Act365F`). Synthetic
    /// IDs without a market hint fall back to `Act365F`. Interpolation defaults to
    /// MonotoneConvex with FlatForward extrapolation.
    ///
    /// **Build-vs-query basis trap:** the day-count basis is used both to convert
    /// dated pillars to year fractions at build time and to convert query dates
    /// back at lookup time. Because inference is substring-based, *renaming* the
    /// curve ID (e.g. `USD-SOFR` → `OIS-1`) can silently change the inferred
    /// basis and shift every pillar time by ~1.4% (Act/360 vs Act/365F). When the
    /// basis matters, set [`DiscountCurveBuilder::day_count`] explicitly instead
    /// of relying on inference; each inference is logged at `debug` level.
    ///
    /// **Negative rates:** the default [`ValidationMode::MarketStandard`] enforces
    /// monotonic discount factors with a -50bp implied-forward floor. For deeply
    /// negative-rate markets (CHF, JPY, EUR historical), pass
    /// [`ValidationMode::NegativeRateFriendly`] (or `Raw`) via
    /// [`DiscountCurveBuilder::validation`]. All interpolation styles —
    /// including the default MonotoneConvex — support increasing-DF
    /// (negative-rate) inputs; MonotoneConvex auto-detects negative discrete
    /// forwards and skips its Hagan-West positivity amelioration so negative
    /// rates interpolate faithfully.
    #[must_use]
    pub fn builder(id: impl Into<CurveId>) -> DiscountCurveBuilder {
        let id: CurveId = id.into();
        let day_count = infer_discount_curve_day_count(id.as_str());
        DiscountCurveBuilder {
            id,
            base: None,
            day_count,
            points: Vec::new(),
            style: InterpStyle::MonotoneConvex,
            extrapolation: ExtrapolationPolicy::FlatForward,
            min_forward_rate: None,     // No floor by default
            allow_non_monotonic: false, // Strict validation by default
            min_forward_tenor: DEFAULT_MIN_FORWARD_TENOR, // Default ~30 seconds
            rate_calibration: None,
            calibration_ois_cutoff_days: None,
            fx_policy: None,
        }
    }

    /// Create a builder pre-populated with this curve's data but a new ID.
    pub fn to_builder_with_id(&self, new_id: impl Into<CurveId>) -> DiscountCurveBuilder {
        self.metadata_builder(new_id)
            .knots(self.knots.iter().copied().zip(self.dfs.iter().copied()))
    }

    /// Rebuild this curve with replacement knots while preserving all metadata.
    ///
    /// This retains interpolation, extrapolation, validation policy, calibration
    /// provenance, minimum forward tenor, and FX policy.
    ///
    /// # Errors
    ///
    /// Returns an error when replacement knots violate the preserved curve
    /// validation, interpolation, or forward-rate constraints.
    pub fn rebuild_with_knots<I>(&self, knots: I) -> crate::Result<Self>
    where
        I: IntoIterator<Item = (f64, f64)>,
    {
        self.metadata_builder(self.id.clone()).knots(knots).build()
    }

    /// Builder pre-populated with this curve's full metadata but **no** knots.
    /// Shared by all rebuild-style operations (bumps, rolls) so that no
    /// metadata field (day-count, interpolation, extrapolation, calibration
    /// settings, fx_policy, non-monotonic settings) is dropped.
    pub(super) fn metadata_builder(&self, new_id: impl Into<CurveId>) -> DiscountCurveBuilder {
        DiscountCurve::builder(new_id)
            .base_date(self.base)
            .day_count(self.day_count)
            .interp(self.style)
            .extrapolation(self.extrapolation)
            .min_forward_tenor(self.min_forward_tenor)
            .rate_calibration_opt(self.rate_calibration.clone())
            .calibration_ois_cutoff_days_opt(self.calibration_ois_cutoff_days)
            .fx_policy_opt(self.fx_policy.clone())
            .apply_non_monotonic_settings(self.allow_non_monotonic, self.min_forward_rate)
    }

    /// Create a forward curve from this discount curve.
    ///
    /// For single-curve bootstrapping, this creates a fixed-tenor simple-rate
    /// forward curve using:
    /// `F(t, t+tau) = (DF(t) / DF(t+tau) - 1) / tau`.
    ///
    /// # Arguments
    ///
    /// * `forward_id` - Identifier for the resulting forward curve
    /// * `tenor_years` - Tenor of the forward rate in years
    /// * `interp_style` - Optional interpolation style; defaults to `Linear` if `None`
    ///
    /// # Errors
    ///
    /// Returns `Error::Validation` when `tenor_years` is non-finite or not
    /// strictly positive, `InputError::TooFewPoints` when the discount curve
    /// has fewer than two knots, or an error when the derived forward curve
    /// fails validation.
    pub fn to_forward_curve(
        &self,
        forward_id: impl Into<CurveId>,
        tenor_years: f64,
        interp_style: Option<InterpStyle>,
    ) -> crate::Result<ForwardCurve> {
        if !tenor_years.is_finite() || tenor_years <= 0.0 {
            return Err(crate::Error::Validation(format!(
                "forward tenor must be finite and positive, got {tenor_years}"
            )));
        }

        // Monotone-convex is a discount-factor interpolation strategy and must
        // not be applied to already-derived forward-rate ordinates.
        let style = interp_style.unwrap_or(InterpStyle::Linear);

        // Calculate forward rates at each knot point
        let mut forward_rates = Vec::with_capacity(self.knots.len());

        // Ensure we have enough points
        if self.knots.len() < 2 {
            return Err(crate::error::InputError::TooFewPoints.into());
        }

        for &t in self.knots.iter() {
            let df_start = self.df(t);
            let df_end = self.df(t + tenor_years);
            if !df_start.is_finite() || !df_end.is_finite() || df_start <= 0.0 || df_end <= 0.0 {
                return Err(crate::Error::Validation(format!(
                    "cannot derive forward at t={t}: invalid discount factors \
                     DF(t)={df_start}, DF(t+tenor)={df_end}"
                )));
            }
            let forward_rate = (df_start / df_end - 1.0) / tenor_years;
            if !forward_rate.is_finite() {
                return Err(crate::Error::Validation(format!(
                    "derived non-finite forward rate at t={t}"
                )));
            }
            forward_rates.push((t, forward_rate));
        }

        // Build forward curve with the specified interpolation style
        ForwardCurve::builder(forward_id, tenor_years)
            .base_date(self.base)
            .day_count(self.day_count)
            .knots(forward_rates)
            .interp(style)
            .build()
    }
}
