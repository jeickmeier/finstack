use super::MetricId;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::OnceLock;

/// Logical grouping of standard metrics for discovery and display.
///
/// Each standard metric belongs to exactly one group. Use
/// [`MetricGroup::metrics()`] to list members and
/// [`MetricGroup::ALL`] to iterate all groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MetricGroup {
    /// Static pricing outputs: prices, yields, spreads, durations, implied
    /// levels, convexity, embedded option value.
    Pricing,
    /// Time-driven P&L: theta decomposition, carry components, financing,
    /// dollar-roll carry.
    Carry,
    /// First-order bump sensitivities to market curves: DV01, PV01,
    /// bucketed DV01, rho, and other rates-focused "01" metrics.
    Sensitivity,
    /// Options-style Greeks and all second-order / higher-order
    /// sensitivities: delta, gamma, vega, cross-gammas, variance vega.
    Greeks,
    /// CDS/credit analytics and credit-specific sensitivities: CS01,
    /// bucketed CS01, par spread, risky PV01/annuity, spread DV01,
    /// correlation01, default metrics, recovery.
    Credit,
    /// Rates instrument decomposition: IRS legs, annuities, par rates,
    /// basis swap, TRS, deposit/calibration intermediates.
    Rates,
    /// FX instrument pricing and analytics: spot rates, amounts, FX
    /// sensitivities (FX01, FX delta, FX vega).
    Fx,
    /// Equity/basket/ETF pricing, equity-derivative analytics, and
    /// variance swap pricing outputs.
    Equity,
    /// Securitization pool and tranche analytics: WAL, WAM, CPR, CDR,
    /// prepayment/severity sensitivities, ABS/CLO/CMBS/RMBS specifics.
    StructuredCredit,
    /// PE fund metrics, DCF valuation, repo analytics,
    /// inflation-linked bond metrics, VaR.
    Alternatives,
}

impl MetricGroup {
    /// All groups in display order.
    pub const ALL: &'static [MetricGroup] = &[
        MetricGroup::Pricing,
        MetricGroup::Carry,
        MetricGroup::Sensitivity,
        MetricGroup::Greeks,
        MetricGroup::Credit,
        MetricGroup::Rates,
        MetricGroup::Fx,
        MetricGroup::Equity,
        MetricGroup::StructuredCredit,
        MetricGroup::Alternatives,
    ];

    /// Human-readable group name.
    pub const fn display_name(&self) -> &'static str {
        match self {
            MetricGroup::Pricing => "Pricing",
            MetricGroup::Carry => "Carry",
            MetricGroup::Sensitivity => "Sensitivity",
            MetricGroup::Greeks => "Greeks",
            MetricGroup::Credit => "Credit",
            MetricGroup::Rates => "Rates",
            MetricGroup::Fx => "FX",
            MetricGroup::Equity => "Equity",
            MetricGroup::StructuredCredit => "Structured Credit",
            MetricGroup::Alternatives => "Alternatives",
        }
    }

    /// Standard metrics belonging to this group.
    pub fn metrics(&self) -> &'static [MetricId] {
        let (start, end) = self.metric_range();
        &MetricId::ALL_STANDARD[start..end]
    }

    /// All groups with their metrics, for iteration.
    pub fn all_with_metrics() -> &'static [(MetricGroup, &'static [MetricId])] {
        static DATA: OnceLock<Vec<(MetricGroup, &'static [MetricId])>> = OnceLock::new();
        DATA.get_or_init(|| MetricGroup::ALL.iter().map(|g| (*g, g.metrics())).collect())
    }

    fn metric_range(&self) -> (usize, usize) {
        match self {
            MetricGroup::Pricing => (0, 29),
            MetricGroup::Carry => (29, 40),
            MetricGroup::Sensitivity => (40, 59),
            MetricGroup::Greeks => (59, 83),
            MetricGroup::Credit => (83, 101),
            MetricGroup::Rates => (101, 129),
            MetricGroup::Fx => (129, 136),
            MetricGroup::Equity => (136, 154),
            MetricGroup::StructuredCredit => (154, 183),
            MetricGroup::Alternatives => (183, 209),
        }
    }
}

impl fmt::Display for MetricGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}
