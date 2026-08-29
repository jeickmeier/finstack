//! Term-loan-specific pricing overrides (covenants and schedule adjustments).

use finstack_quant_core::dates::Date;
use finstack_quant_core::money::Money;

/// Term loan specific overrides for covenants and schedule adjustments.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "json-schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct TermLoanOverrides {
    /// Additional margin step-ups by date (bp)
    #[serde(with = "finstack_quant_core::wire::dated_i32_values")]
    #[cfg_attr(
        feature = "json-schema",
        schemars(with = "Vec<(finstack_quant_core::wire::DateWire, i32)>")
    )]
    pub margin_add_bp_by_date: Vec<(Date, i32)>,
    /// Force PIK toggles by date
    #[serde(with = "finstack_quant_core::wire::dated_bool_values")]
    #[cfg_attr(
        feature = "json-schema",
        schemars(with = "Vec<(finstack_quant_core::wire::DateWire, bool)>")
    )]
    pub pik_toggle_by_date: Vec<(Date, bool)>,
    /// Extra cash sweeps by date
    #[serde(with = "finstack_quant_core::wire::dated_money_values")]
    #[cfg_attr(
        feature = "json-schema",
        schemars(with = "Vec<(finstack_quant_core::wire::DateWire, Money)>")
    )]
    pub extra_cash_sweeps: Vec<(Date, Money)>,
    /// Draw stop date (earliest date after which draws are blocked)
    #[serde(default, with = "finstack_quant_core::wire::optional_date")]
    #[cfg_attr(
        feature = "json-schema",
        schemars(with = "Option<finstack_quant_core::wire::DateWire>")
    )]
    pub draw_stop_date: Option<Date>,
}
