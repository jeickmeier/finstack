/// Quote input for the bond quote engine.
///
/// All spreads are expressed in **decimal** (`0.01 = 100bp`).
#[derive(
    Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum BondQuoteInput {
    /// Clean price quoted as percentage of par (e.g., 99.5 = 99.5% of par).
    CleanPricePct(f64),
    /// Dirty price in currency units.
    DirtyPriceCurrency(f64),
    /// Yield to maturity (decimal).
    Ytm(f64),
    /// Yield to worst (decimal).
    ///
    /// For non-callable bonds this is equivalent to [`BondQuoteInput::Ytm`].
    /// For callable bonds, prefer [`BondQuoteInput::Oas`] when exercise-aware
    /// pricing is required — YTW inversion via this variant uses maturity flows
    /// (consistent with `Bond::base_value`'s `quoted_ytw` path).
    Ytw(f64),
    /// Z-spread over the discount curve (decimal).
    ZSpread(f64),
    /// Discount margin for FRNs (decimal).
    DiscountMargin(f64),
    /// Option-adjusted spread (decimal).
    Oas(f64),
    /// Asset swap market spread (decimal).
    AswMarket(f64),
    /// I-spread (decimal).
    ISpread(f64),
}

/// Full quote set produced by the quote engine.
///
/// - Prices are returned both in currency and as % of par.
/// - All spreads are decimal (`0.01 = 100bp`).
#[derive(Debug, Clone)]
pub struct BondQuoteSet {
    /// Clean price in currency.
    pub clean_price_currency: f64,
    /// Clean price as percentage of par (quote convention).
    pub clean_price_pct: f64,
    /// Dirty price in currency.
    pub dirty_price_currency: f64,
    /// Yield to maturity (decimal), if applicable.
    pub ytm: Option<f64>,
    /// Yield to worst (decimal), if applicable.
    pub ytw: Option<f64>,
    /// Z-spread over discount curve (decimal), if applicable.
    pub z_spread: Option<f64>,
    /// Discount margin for FRNs (decimal), if applicable.
    pub discount_margin: Option<f64>,
    /// Option-adjusted spread (decimal), if applicable.
    pub oas: Option<f64>,
    /// Asset swap par spread (decimal), if applicable.
    pub asw_par: Option<f64>,
    /// Asset swap market spread (decimal), if applicable.
    pub asw_market: Option<f64>,
    /// I-spread (decimal), if applicable.
    pub i_spread: Option<f64>,
}

/// Yield Compounding enumeration.
///
/// Defines how yield-to-maturity is compounded when calculating present values.
/// Different markets and instrument types use different conventions.
///
/// # Market Standard Conventions
///
/// | Convention | Use Case | Formula |
/// |------------|----------|---------|
/// | `Street` | Most secondary market trading | `(1 + y/f)^(-f*t)` |
/// | `TreasuryActual` | US Treasury new issues with stubs | Simple interest for first period |
/// | `Simple` | Money market instruments | `1/(1 + y*t)` |
/// | `Continuous` | Theoretical/academic | `exp(-y*t)` |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YieldCompounding {
    /// Simple interest: `DF = 1 / (1 + y * t)`
    ///
    /// Used for money market instruments and short-dated securities.
    Simple,

    /// Annual compounding: `DF = (1 + y)^(-t)`
    Annual,

    /// Periodic compounding with explicit periods per year: `DF = (1 + y/m)^(-m*t)`
    Periodic(u32),

    /// Continuous compounding: `DF = exp(-y * t)`
    ///
    /// Used in theoretical models and some derivative pricing.
    Continuous,

    /// Street convention: periodic compounding aligned with bond's coupon frequency.
    ///
    /// This is the standard convention for secondary market bond trading.
    /// Formula: `DF = (1 + y/f)^(-f*t)` where `f` is coupon frequency.
    Street,

    /// ISDA/Treasury actual convention with simple interest for odd first period.
    ///
    /// Uses simple interest `1/(1 + y*t)` for the first (potentially irregular) period,
    /// then switches to periodic compounding for subsequent periods. This matches
    /// the official SEC/Treasury methodology for new issue pricing with stub periods.
    ///
    /// # When to Use
    ///
    /// - US Treasury new issues with short first coupons
    /// - Regulatory yield calculations requiring ISDA compliance
    /// - Benchmarking against official Bloomberg/Reuters Treasury yields
    ///
    /// # Typical Difference
    ///
    /// The difference vs `Street` convention is typically < 0.5 basis points for
    /// seasoned bonds, but can be 1-2 basis points for new issues with significant stubs.
    ///
    /// # Limitation
    ///
    /// Stub period detection is **time-based**, using `t < 1/frequency` as the criterion.
    /// This works correctly for standard bonds but may misclassify stubs on bonds with
    /// irregular first coupons that don't align with the standard frequency (e.g., a
    /// long-first stub spanning 8 months on a semi-annual bond).
    TreasuryActual,
}
