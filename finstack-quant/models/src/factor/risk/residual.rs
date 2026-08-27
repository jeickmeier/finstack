//! Residual-risk overlays for additive factor decompositions.

use super::math::{normal_pdf, normal_quantile};
use super::types::{PositionResidualContribution, RiskDecomposition};
use crate::factor::RiskMeasure;

/// Add position residual variances to an existing factor decomposition.
///
/// Factor and position-factor contributions are rescaled so the resulting
/// decomposition remains Euler-additive under the selected risk measure.
///
/// # Arguments
///
/// * `decomposition` - Factor decomposition to update in place.
/// * `residual_contributions` - Per-position residual variance contributions.
///
/// # Errors
///
/// Returns a validation error when the decomposition uses a risk measure that
/// cannot be inverted to variance.
pub fn apply_residual_contributions(
    decomposition: &mut RiskDecomposition,
    residual_contributions: Vec<PositionResidualContribution>,
) -> finstack_quant_core::Result<()> {
    let residual_variance: f64 = residual_contributions
        .iter()
        .map(|contribution| contribution.residual_variance)
        .sum();
    if residual_variance <= 0.0 {
        decomposition
            .position_residual_contributions
            .extend(residual_contributions);
        return Ok(());
    }

    let systematic_variance =
        variance_from_measure(decomposition.measure, decomposition.total_risk)?;
    let combined_variance = systematic_variance + residual_variance;
    let (combined_total, combined_component_scale) =
        risk_total_and_component_scale(decomposition.measure, combined_variance)?;
    let (_, systematic_component_scale) =
        risk_total_and_component_scale(decomposition.measure, systematic_variance)?;
    let factor_rescale = if systematic_component_scale.abs() > 0.0 {
        combined_component_scale / systematic_component_scale
    } else {
        0.0
    };

    for contribution in &mut decomposition.factor_contributions {
        contribution.absolute_risk *= factor_rescale;
        contribution.marginal_risk *= factor_rescale;
        contribution.relative_risk = if combined_total.abs() > 0.0 {
            contribution.absolute_risk / combined_total
        } else {
            0.0
        };
    }
    for contribution in &mut decomposition.position_factor_contributions {
        contribution.risk_contribution *= factor_rescale;
    }

    decomposition.total_risk = combined_total;
    decomposition.residual_risk = residual_variance * combined_component_scale;
    decomposition
        .position_residual_contributions
        .extend(residual_contributions);
    Ok(())
}

fn variance_from_measure(
    measure: RiskMeasure,
    total_risk: f64,
) -> finstack_quant_core::Result<f64> {
    let variance = match measure {
        RiskMeasure::Variance => total_risk.max(0.0),
        RiskMeasure::Volatility => total_risk * total_risk,
        RiskMeasure::VaR { confidence } => {
            let z = normal_quantile(confidence);
            if z > 0.0 {
                (total_risk / -z).powi(2)
            } else {
                0.0
            }
        }
        RiskMeasure::ExpectedShortfall { confidence } => {
            let z = normal_quantile(confidence);
            let es_multiplier = normal_pdf(z) / (1.0 - confidence);
            if es_multiplier > 0.0 {
                (total_risk / -es_multiplier).powi(2)
            } else {
                0.0
            }
        }
    };
    Ok(variance)
}

fn risk_total_and_component_scale(
    measure: RiskMeasure,
    variance: f64,
) -> finstack_quant_core::Result<(f64, f64)> {
    let variance = variance.max(0.0);
    let sigma = variance.sqrt();
    let scaled = match measure {
        RiskMeasure::Variance => (variance, 1.0),
        RiskMeasure::Volatility => {
            if sigma > 0.0 {
                (sigma, sigma.recip())
            } else {
                (0.0, 0.0)
            }
        }
        RiskMeasure::VaR { confidence } => {
            let z = normal_quantile(confidence);
            if sigma > 0.0 {
                (-sigma * z, -z * sigma.recip())
            } else {
                (0.0, 0.0)
            }
        }
        RiskMeasure::ExpectedShortfall { confidence } => {
            let z = normal_quantile(confidence);
            let es_multiplier = normal_pdf(z) / (1.0 - confidence);
            if sigma > 0.0 {
                (-sigma * es_multiplier, -es_multiplier * sigma.recip())
            } else {
                (0.0, 0.0)
            }
        }
    };
    Ok(scaled)
}
