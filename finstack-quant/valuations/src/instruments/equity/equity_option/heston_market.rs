//! Valuation-side resolution of closed-form Heston parameters.

use crate::instruments::common_impl::helpers::get_unitless_scalar_strict;
use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_models::closed_form::heston::HestonPricingParams;

/// Resolve all required Heston scalars from a valuation market context.
pub(crate) fn heston_params_from_market_strict(
    market: &MarketContext,
    rate: f64,
    dividend_yield: f64,
) -> finstack_quant_core::Result<HestonPricingParams> {
    HestonPricingParams::new(
        rate,
        dividend_yield,
        get_unitless_scalar_strict(market, "HESTON_KAPPA", "Heston")?,
        get_unitless_scalar_strict(market, "HESTON_THETA", "Heston")?,
        get_unitless_scalar_strict(market, "HESTON_SIGMA_V", "Heston")?,
        get_unitless_scalar_strict(market, "HESTON_RHO", "Heston")?,
        get_unitless_scalar_strict(market, "HESTON_V0", "Heston")?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use finstack_quant_core::market_data::scalars::MarketScalar;

    #[test]
    fn strict_resolution_rejects_missing_heston_scalars() {
        let error = heston_params_from_market_strict(&MarketContext::new(), 0.03, 0.01)
            .expect_err("missing Heston scalars must fail");
        assert!(error.to_string().contains("HESTON_KAPPA"));
    }

    #[test]
    fn strict_resolution_names_a_partially_missing_scalar_set() {
        let market = MarketContext::new()
            .insert_price("HESTON_KAPPA", MarketScalar::Unitless(1.5))
            .insert_price("HESTON_THETA", MarketScalar::Unitless(0.06))
            .insert_price("HESTON_SIGMA_V", MarketScalar::Unitless(0.4))
            .insert_price("HESTON_RHO", MarketScalar::Unitless(-0.5));
        let error = heston_params_from_market_strict(&market, 0.0, 0.0)
            .expect_err("missing HESTON_V0 must fail");
        assert!(error.to_string().contains("HESTON_V0"));
    }

    #[test]
    fn strict_resolution_preserves_complete_market_parameters() {
        let market = MarketContext::new()
            .insert_price("HESTON_KAPPA", MarketScalar::Unitless(1.5))
            .insert_price("HESTON_THETA", MarketScalar::Unitless(0.06))
            .insert_price("HESTON_SIGMA_V", MarketScalar::Unitless(0.4))
            .insert_price("HESTON_RHO", MarketScalar::Unitless(-0.5))
            .insert_price("HESTON_V0", MarketScalar::Unitless(0.05));
        let params = heston_params_from_market_strict(&market, 0.03, 0.01)
            .expect("complete Heston inputs must resolve");
        assert_eq!(params.kappa, 1.5);
        assert_eq!(params.theta, 0.06);
        assert_eq!(params.sigma_v, 0.4);
        assert_eq!(params.rho, -0.5);
        assert_eq!(params.v0, 0.05);
    }
}
