//! Resolve already-fitted HW1F short-rate parameters for valuation.
//!
//! Pricing accepts either a complete explicit `(kappa, sigma)` override or a
//! complete pair written to the market by a prior calibration step. It never
//! reads a volatility surface, performs a fit, or substitutes model defaults.

use finstack_quant_core::market_data::context::MarketContext;
use finstack_quant_core::market_data::scalars::MarketScalar;
use finstack_quant_core::Result;
use finstack_quant_models::rates::hull_white::{
    capfloor_hw1f_scalar_keys, hw1f_scalar_keys, HullWhiteParams,
};

/// Source of the complete HW1F parameter pair used for pricing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hw1fParamSource {
    /// Both parameters came from instrument pricing overrides.
    Override,
    /// Both parameters came from the pre-calibrated market scalar store.
    MarketScalars,
}

impl Hw1fParamSource {
    /// Stable provenance label used by pricing diagnostics.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Override => "override",
            Self::MarketScalars => "market_scalars",
        }
    }
}

/// Parameter family that determines the market-scalar key convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hw1fParamFamily {
    /// Parameters fitted to a swaption volatility grid.
    Swaption,
    /// Parameters fitted to a cap/floor volatility strip.
    CapFloor,
}

impl Hw1fParamFamily {
    fn scalar_keys(self, curve_id: &str) -> (String, String) {
        match self {
            Self::Swaption => hw1f_scalar_keys(curve_id),
            Self::CapFloor => capfloor_hw1f_scalar_keys(curve_id),
        }
    }
}

/// Inputs needed to resolve a complete pre-fitted HW1F parameter pair.
pub struct Hw1fResolveRequest<'a> {
    /// Curve identifier used by the fitting step when storing scalars.
    pub curve_id: &'a str,
    /// Parameter family selecting the scalar key convention.
    pub family: Hw1fParamFamily,
    /// Optional model-configuration object containing `hw1f_kappa` and `hw1f_sigma`.
    pub overrides: Option<&'a serde_json::Value>,
    /// Instrument or pricing-path label included in validation errors.
    pub context: &'a str,
}

fn scalar_as_positive_f64(scalar: &MarketScalar) -> Option<f64> {
    let value = match scalar {
        MarketScalar::Unitless(value) => *value,
        MarketScalar::Price(money) => money.amount(),
    };
    (value.is_finite() && value > 0.0).then_some(value)
}

fn override_positive_f64(
    object: Option<&serde_json::Map<String, serde_json::Value>>,
    key: &str,
) -> Result<Option<f64>> {
    let Some(raw) = object.and_then(|value| value.get(key)) else {
        return Ok(None);
    };
    let value = raw.as_f64().ok_or_else(|| {
        finstack_quant_core::Error::Validation(format!(
            "{key} override must be a positive finite number"
        ))
    })?;
    if value.is_finite() && value > 0.0 {
        Ok(Some(value))
    } else {
        Err(finstack_quant_core::Error::Validation(format!(
            "{key} override must be positive and finite, got {value}"
        )))
    }
}

/// Resolve a complete HW1F parameter pair without fitting during pricing.
///
/// # Arguments
///
/// * `request` - Curve identity, scalar-key family, optional explicit overrides,
///   and diagnostic context.
/// * `market` - Pre-calibrated market that may contain the complete scalar pair.
///
/// # Errors
///
/// Returns a validation error for invalid, partial, or missing parameters.
pub fn resolve_hw1f_params(
    request: &Hw1fResolveRequest<'_>,
    market: &MarketContext,
) -> Result<(HullWhiteParams, Hw1fParamSource)> {
    let object = request.overrides.and_then(serde_json::Value::as_object);
    let override_kappa = override_positive_f64(object, "hw1f_kappa")?;
    let override_sigma = override_positive_f64(object, "hw1f_sigma")?;
    match (override_kappa, override_sigma) {
        (Some(kappa), Some(sigma)) => {
            return HullWhiteParams::new(kappa, sigma)
                .map(|params| (params, Hw1fParamSource::Override));
        }
        (None, None) => {}
        (kappa, sigma) => {
            return Err(finstack_quant_core::Error::Validation(format!(
                "{}: partial HW1F override (hw1f_kappa={kappa:?}, hw1f_sigma={sigma:?}); \
                 supply both positive finite parameters",
                request.context
            )));
        }
    }

    let (kappa_key, sigma_key) = request.family.scalar_keys(request.curve_id);
    let kappa = market
        .get_price(&kappa_key)
        .ok()
        .and_then(scalar_as_positive_f64);
    let sigma = market
        .get_price(&sigma_key)
        .ok()
        .and_then(scalar_as_positive_f64);
    match (kappa, sigma) {
        (Some(kappa), Some(sigma)) => HullWhiteParams::new(kappa, sigma)
            .map(|params| (params, Hw1fParamSource::MarketScalars)),
        (None, None) => Err(finstack_quant_core::Error::Validation(format!(
            "{}: missing HW1F parameters for curve '{}'; provide both hw1f_kappa and \
             hw1f_sigma overrides or pre-calibrate market scalars '{}' and '{}'",
            request.context, request.curve_id, kappa_key, sigma_key
        ))),
        (kappa, sigma) => Err(finstack_quant_core::Error::Validation(format!(
            "{}: partial calibrated HW1F scalars for curve '{}' \
             ({kappa_key}={kappa:?}, {sigma_key}={sigma:?}); both must be present",
            request.context, request.curve_id
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn request<'a>(overrides: Option<&'a serde_json::Value>) -> Hw1fResolveRequest<'a> {
        Hw1fResolveRequest {
            curve_id: "USD-OIS",
            family: Hw1fParamFamily::Swaption,
            overrides,
            context: "test",
        }
    }

    #[test]
    fn complete_override_is_used() {
        let overrides = json!({"hw1f_kappa": 0.05, "hw1f_sigma": 0.012});
        let (params, source) =
            resolve_hw1f_params(&request(Some(&overrides)), &MarketContext::new())
                .expect("complete override");
        assert_eq!(source, Hw1fParamSource::Override);
        assert_eq!(params, HullWhiteParams::new(0.05, 0.012).expect("valid"));
    }

    #[test]
    fn missing_parameters_are_rejected() {
        let error = resolve_hw1f_params(&request(None), &MarketContext::new())
            .expect_err("missing parameters");
        assert!(error.to_string().contains("missing HW1F parameters"));
    }

    #[test]
    fn partial_override_is_rejected() {
        let overrides = json!({"hw1f_kappa": 0.05});
        let error = resolve_hw1f_params(&request(Some(&overrides)), &MarketContext::new())
            .expect_err("partial override");
        assert!(error.to_string().contains("partial HW1F override"));
    }

    #[test]
    fn complete_market_pair_is_used() {
        let (kappa_key, sigma_key) = hw1f_scalar_keys("USD-OIS");
        let market = MarketContext::new()
            .insert_price(kappa_key, MarketScalar::Unitless(0.04))
            .insert_price(sigma_key, MarketScalar::Unitless(0.009));
        let (params, source) =
            resolve_hw1f_params(&request(None), &market).expect("complete market pair");
        assert_eq!(source, Hw1fParamSource::MarketScalars);
        assert_eq!(params, HullWhiteParams::new(0.04, 0.009).expect("valid"));
    }
}
