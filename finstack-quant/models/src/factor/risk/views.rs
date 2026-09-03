//! Serializable factor-risk reporting views and matrix input adapters.

use super::{PositionRiskDecomposition, RiskBudgetResult};

/// Serializable Expected Shortfall contribution row.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PositionEsContributionView {
    /// Position identifier.
    pub position_id: String,
    /// Component Expected Shortfall allocated to the position.
    pub component_es: f64,
    /// Marginal Expected Shortfall, when available.
    pub marginal_es: Option<f64>,
    /// Fraction of total ES contributed by this position.
    pub pct_contribution: f64,
}

/// Serializable Expected Shortfall decomposition view.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ParametricEsDecompositionView {
    /// Total portfolio VaR.
    pub portfolio_var: f64,
    /// Total portfolio Expected Shortfall.
    pub portfolio_es: f64,
    /// Confidence level used for ES.
    pub confidence: f64,
    /// Number of positions in the decomposition.
    pub n_positions: usize,
    /// Per-position ES contributions.
    pub contributions: Vec<PositionEsContributionView>,
}

/// Serializable VaR contribution row.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PositionVarContributionView {
    /// Position identifier.
    pub position_id: String,
    /// Component VaR allocated to the position.
    pub component_var: f64,
    /// Marginal VaR, when available.
    pub marginal_var: Option<f64>,
    /// Fraction of total VaR contributed by this position.
    pub pct_contribution: f64,
    /// Incremental VaR, when available.
    pub incremental_var: Option<f64>,
}

/// Serializable VaR decomposition view.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ParametricVarDecompositionView {
    /// Total portfolio VaR.
    pub portfolio_var: f64,
    /// Total portfolio Expected Shortfall.
    pub portfolio_es: f64,
    /// Confidence level used for VaR.
    pub confidence: f64,
    /// Number of positions in the decomposition.
    pub n_positions: usize,
    /// Euler residual, when computed by the engine.
    pub euler_residual: Option<f64>,
    /// Per-position VaR contributions.
    pub contributions: Vec<PositionVarContributionView>,
}

/// Serializable risk-budget row.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PositionBudgetEntryView {
    /// Position identifier.
    pub position_id: String,
    /// Actual component VaR.
    pub actual_component_var: f64,
    /// Target component VaR.
    pub target_component_var: f64,
    /// Target share of portfolio VaR (`inf` when the portfolio VaR is zero
    /// but the target level is not).
    #[serde(with = "finstack_quant_core::wire::non_finite_f64")]
    pub target_pct: f64,
    /// Utilization ratio (negative for diversifiers, `±inf` for a non-zero
    /// component against a zero target).
    #[serde(with = "finstack_quant_core::wire::non_finite_f64")]
    pub utilization: f64,
    /// Over-budget amount.
    pub excess: f64,
    /// Whether utilization exceeds the configured threshold.
    pub breach: bool,
}

/// Serializable risk-budget result view.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RiskBudgetResultView {
    /// Portfolio VaR used for target scaling.
    pub portfolio_var: f64,
    /// Sum of over-budget amounts.
    pub total_overbudget: f64,
    /// Whether any position breached the utilization threshold.
    pub has_breach: bool,
    /// Utilization threshold used for breach classification.
    pub utilization_threshold: f64,
    /// Per-position budget rows.
    pub positions: Vec<PositionBudgetEntryView>,
}

/// Convert a full position risk decomposition into the serializable VaR view.
///
/// # Arguments
///
/// * `decomposition` - Position-level risk decomposition whose VaR
///   contributions, confidence, and portfolio totals are copied into the
///   reporting representation.
#[must_use]
pub fn parametric_var_decomposition_view(
    decomposition: &PositionRiskDecomposition,
) -> ParametricVarDecompositionView {
    let contributions = decomposition
        .var_contributions
        .iter()
        .map(|contribution| PositionVarContributionView {
            position_id: contribution.position_id.clone(),
            component_var: contribution.component_var,
            marginal_var: contribution.marginal_var,
            pct_contribution: contribution.relative_var,
            incremental_var: contribution.incremental_var,
        })
        .collect();
    ParametricVarDecompositionView {
        portfolio_var: decomposition.portfolio_var,
        portfolio_es: decomposition.portfolio_es,
        confidence: decomposition.confidence,
        n_positions: decomposition.n_positions,
        euler_residual: decomposition.euler_residual,
        contributions,
    }
}

/// Convert a full position risk decomposition into the serializable ES view.
///
/// # Arguments
///
/// * `decomposition` - Position-level risk decomposition whose Expected
///   Shortfall contributions, confidence, and portfolio totals are copied into
///   the reporting representation.
#[must_use]
pub fn parametric_es_decomposition_view(
    decomposition: &PositionRiskDecomposition,
) -> ParametricEsDecompositionView {
    let contributions = decomposition
        .es_contributions
        .iter()
        .map(|contribution| PositionEsContributionView {
            position_id: contribution.position_id.clone(),
            component_es: contribution.component_es,
            marginal_es: contribution.marginal_es,
            pct_contribution: contribution.relative_es,
        })
        .collect();
    ParametricEsDecompositionView {
        portfolio_var: decomposition.portfolio_var,
        portfolio_es: decomposition.portfolio_es,
        confidence: decomposition.confidence,
        n_positions: decomposition.n_positions,
        contributions,
    }
}

/// Convert a risk-budget result into the serializable reporting view.
///
/// # Arguments
///
/// * `result` - Per-position risk-budget allocation result to expose.
/// * `portfolio_var` - Signed portfolio VaR in reporting-currency amount
///   units; its absolute magnitude scales each target percentage.
/// * `utilization_threshold` - Dimensionless utilization ratio above which a
///   position is flagged as breaching its risk budget.
#[must_use]
pub fn risk_budget_result_view(
    result: &RiskBudgetResult,
    portfolio_var: f64,
    utilization_threshold: f64,
) -> RiskBudgetResultView {
    let portfolio_var_magnitude = portfolio_var.abs();
    let positions = result
        .positions
        .iter()
        .map(|entry| {
            let target_pct = if portfolio_var_magnitude > 1e-15 {
                entry.target_component_var / portfolio_var_magnitude
            } else if entry.target_component_var.abs() > 1e-15 {
                f64::INFINITY
            } else {
                0.0
            };
            PositionBudgetEntryView {
                position_id: entry.position_id.clone(),
                actual_component_var: entry.actual_component_var,
                target_component_var: entry.target_component_var,
                target_pct,
                utilization: entry.utilization,
                excess: entry.excess,
                breach: entry.utilization > utilization_threshold,
            }
        })
        .collect();
    RiskBudgetResultView {
        portfolio_var,
        total_overbudget: result.total_overbudget,
        has_breach: result.has_breach,
        utilization_threshold,
        positions,
    }
}

/// Flatten a row-major nested matrix after validating squareness against `n`.
///
/// # Errors
///
/// Returns [`finstack_quant_core::Error::Validation`] when the matrix has the
/// wrong number of rows or any row has the wrong number of columns.
///
/// # Arguments
///
/// * `matrix` - Row-major nested vector with one inner vector per row.
/// * `n` - Expected square dimension.
/// * `label` - Caller-provided label included in validation messages.
pub fn flatten_square_matrix(
    matrix: Vec<Vec<f64>>,
    n: usize,
    label: &str,
) -> finstack_quant_core::Result<Vec<f64>> {
    if matrix.len() != n {
        return Err(finstack_quant_core::Error::Validation(format!(
            "{label} must have {n} rows, got {}",
            matrix.len()
        )));
    }
    let mut flat = Vec::with_capacity(n * n);
    for (index, row) in matrix.into_iter().enumerate() {
        if row.len() != n {
            return Err(finstack_quant_core::Error::Validation(format!(
                "{label} row {index} must have {n} columns, got {}",
                row.len()
            )));
        }
        flat.extend(row);
    }
    Ok(flat)
}

/// Flatten per-position scenario P&Ls into a scenario-major buffer.
///
/// # Errors
///
/// Returns [`finstack_quant_core::Error::Validation`] when the number of rows
/// does not equal `n_positions` or rows have inconsistent scenario counts.
///
/// # Arguments
///
/// * `position_pnls` - Position-major P&L matrix with one row per position.
/// * `n_positions` - Expected number of position rows.
pub fn flatten_position_pnls(
    position_pnls: Vec<Vec<f64>>,
    n_positions: usize,
) -> finstack_quant_core::Result<(Vec<f64>, usize)> {
    if position_pnls.len() != n_positions {
        return Err(finstack_quant_core::Error::Validation(format!(
            "position_pnls must have {n_positions} rows, got {}",
            position_pnls.len()
        )));
    }
    if n_positions == 0 {
        return Ok((Vec::new(), 0));
    }
    let n_scenarios = position_pnls[0].len();
    for (index, row) in position_pnls.iter().enumerate() {
        if row.len() != n_scenarios {
            return Err(finstack_quant_core::Error::Validation(format!(
                "position_pnls row {index} has {} scenarios, expected {n_scenarios}",
                row.len()
            )));
        }
    }
    let mut flat = Vec::with_capacity(n_scenarios * n_positions);
    for scenario in 0..n_scenarios {
        for row in &position_pnls {
            flat.push(row[scenario]);
        }
    }
    Ok((flat, n_scenarios))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_entry_view_serializes_non_finite_fields() {
        let view = PositionBudgetEntryView {
            position_id: "A".to_string(),
            actual_component_var: 1.0,
            target_component_var: 1.0,
            target_pct: f64::INFINITY,
            utilization: f64::NEG_INFINITY,
            excess: 0.0,
            breach: false,
        };
        let json = serde_json::to_string(&view).expect("serialize");
        assert!(json.contains("\"target_pct\":\"inf\""));
        assert!(json.contains("\"utilization\":\"-inf\""));
    }

    #[test]
    fn flatten_square_matrix_validates_shape() {
        let flat = flatten_square_matrix(vec![vec![1.0, 2.0], vec![3.0, 4.0]], 2, "cov")
            .expect("valid matrix");
        assert_eq!(flat, vec![1.0, 2.0, 3.0, 4.0]);
        assert!(flatten_square_matrix(vec![vec![1.0, 2.0]], 2, "cov").is_err());
        assert!(
            flatten_square_matrix(vec![vec![1.0, 2.0, 3.0], vec![1.0, 2.0]], 2, "cov").is_err()
        );
    }
}
