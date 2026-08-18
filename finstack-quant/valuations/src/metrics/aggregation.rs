//! Cross-instrument metric aggregation semantics.

use super::MetricId;

/// Describes whether a metric can be scaled and summed across instruments.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum MetricAggregation {
    /// The metric is linear in position quantity and currency-convertible.
    Additive,
    /// The metric has instrument-specific meaning and must remain disaggregated.
    NonAdditive,
}

/// Return the aggregation policy for a metric identifier.
///
/// Structured metric keys such as `bucketed_dv01::USD-OIS::10y` inherit the
/// policy of their base identifier.
///
/// # Arguments
///
/// * `metric_id` - Metric identifier whose cross-instrument semantics are required.
#[must_use]
pub fn metric_aggregation(metric_id: &MetricId) -> MetricAggregation {
    let base = metric_id
        .as_str()
        .split_once("::")
        .map_or(metric_id.as_str(), |(base, _)| base);
    match base {
        "theta"
        | "dv01"
        | "cs01"
        | "delta"
        | "gamma"
        | "vega"
        | "volga"
        | "vanna"
        | "rho"
        | "pv01"
        | "ir01"
        | "fx01"
        | "inflation01"
        | "dividend01"
        | "ir_convexity"
        | "cs_gamma"
        | "inflation_convexity"
        | "cross_gamma_rates_credit"
        | "cross_gamma_rates_vol"
        | "cross_gamma_spot_vol"
        | "cross_gamma_spot_credit"
        | "cross_gamma_fx_vol"
        | "cross_gamma_fx_rates"
        | "cross_gamma_credit_vol"
        | "hazard_cs01"
        | "index_delta"
        | "fx_delta"
        | "foreign_rho"
        | "bucketed_dv01"
        | "bucketed_cs01"
        | "bucketed_vega"
        | "accrued_interest"
        | "pv_fixed"
        | "pv_float"
        | "pv_primary"
        | "pv_reference" => MetricAggregation::Additive,
        _ => MetricAggregation::NonAdditive,
    }
}

/// Return whether a metric may be scaled and summed across instruments.
///
/// # Arguments
///
/// * `metric_id` - Metric identifier to test.
#[must_use]
pub fn is_additive_metric(metric_id: &MetricId) -> bool {
    metric_aggregation(metric_id) == MetricAggregation::Additive
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bucketed_metric_inherits_base_policy() {
        assert!(is_additive_metric(&MetricId::custom(
            "bucketed_dv01::USD-OIS::10y"
        )));
        assert!(!is_additive_metric(&MetricId::Ytm));
    }
}
