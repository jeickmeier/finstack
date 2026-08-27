//! Portfolio reporting adapters for credit factor-risk decompositions.

use std::collections::{BTreeMap, BTreeSet};

use crate::types::PositionId;
use finstack_quant_models::factor::credit::hierarchy::{CreditFactorModel, HierarchyDimension};
use finstack_quant_models::factor::matching::CREDIT_GENERIC_FACTOR_ID;
use finstack_quant_models::factor::risk::RiskDecomposition;
use finstack_quant_models::factor::RiskMeasure;

/// Aggregated credit risk grouped by hierarchy level.
#[derive(Debug, Clone, PartialEq)]
pub struct CreditVolReport {
    /// Total risk under the selected measure.
    pub total: f64,
    /// Risk measure used by the underlying decomposition.
    pub measure: RiskMeasure,
    /// Contribution from the generic credit factor.
    pub generic: f64,
    /// Per-hierarchy-level rollups.
    pub by_level: Vec<LevelVolContribution>,
    /// Portfolio idiosyncratic contribution.
    pub idiosyncratic_total: f64,
    /// Optional position-level breakdown.
    pub by_position_optional: Option<Vec<PositionVolContribution>>,
}

/// Aggregated risk contribution for one hierarchy level.
#[derive(Debug, Clone, PartialEq)]
pub struct LevelVolContribution {
    /// Human-readable hierarchy level name.
    pub level_name: String,
    /// Total contribution across the level's buckets.
    pub total: f64,
    /// Contributions keyed by canonical bucket path.
    pub by_bucket: BTreeMap<String, f64>,
}

/// Position-level credit risk breakdown.
#[derive(Debug, Clone, PartialEq)]
pub struct PositionVolContribution {
    /// Portfolio position identifier.
    pub position_id: PositionId,
    /// Systematic factor contribution.
    pub factor_total: f64,
    /// Idiosyncratic contribution.
    pub idiosyncratic: f64,
    /// Sum of systematic and idiosyncratic contributions.
    pub total: f64,
}

/// Build a portfolio credit report from a models-owned risk decomposition.
///
/// # Arguments
///
/// * `decomposition` - Factor and residual contributions to aggregate.
/// * `model` - Credit factor model whose hierarchy supplies level names.
/// * `by_position` - Whether to include position-level contribution rows.
#[must_use]
pub fn build_credit_vol_report(
    decomposition: &RiskDecomposition,
    model: &CreditFactorModel,
    by_position: bool,
) -> CreditVolReport {
    let mut by_level: Vec<LevelVolContribution> = model
        .hierarchy
        .levels
        .iter()
        .map(|level| LevelVolContribution {
            level_name: match level {
                HierarchyDimension::Rating => "Rating".to_owned(),
                HierarchyDimension::Region => "Region".to_owned(),
                HierarchyDimension::Sector => "Sector".to_owned(),
                HierarchyDimension::Custom(name) => name.clone(),
                _ => "Unknown".to_owned(),
            },
            total: 0.0,
            by_bucket: BTreeMap::new(),
        })
        .collect();

    let mut generic = 0.0;
    for contribution in &decomposition.factor_contributions {
        let id = contribution.factor_id.as_str();
        if id == CREDIT_GENERIC_FACTOR_ID {
            generic += contribution.absolute_risk;
            continue;
        }
        let Some(rest) = id.strip_prefix("credit::level") else {
            continue;
        };
        let mut parts = rest.splitn(3, "::");
        let (Some(level), Some(_dimension), Some(bucket)) =
            (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let Ok(level) = level.parse::<usize>() else {
            continue;
        };
        let Some(output) = by_level.get_mut(level) else {
            continue;
        };
        output.total += contribution.absolute_risk;
        *output.by_bucket.entry(bucket.to_owned()).or_insert(0.0) += contribution.absolute_risk;
    }

    let residual_variance: f64 = decomposition
        .position_residual_contributions
        .iter()
        .map(|contribution| contribution.residual_variance)
        .sum();
    let residual_scale = if residual_variance > 0.0 {
        decomposition.residual_risk / residual_variance
    } else {
        0.0
    };

    let by_position_optional = by_position.then(|| {
        let mut factor_by_position: BTreeMap<String, f64> = BTreeMap::new();
        for contribution in &decomposition.position_factor_contributions {
            *factor_by_position
                .entry(contribution.position_id.clone())
                .or_insert(0.0) += contribution.risk_contribution;
        }
        let mut residual_by_position: BTreeMap<String, f64> = BTreeMap::new();
        for contribution in &decomposition.position_residual_contributions {
            *residual_by_position
                .entry(contribution.position_id.clone())
                .or_insert(0.0) += contribution.residual_variance * residual_scale;
        }
        let keys: BTreeSet<String> = factor_by_position
            .keys()
            .chain(residual_by_position.keys())
            .cloned()
            .collect();
        keys.into_iter()
            .map(|id| {
                let factor_total = factor_by_position.get(&id).copied().unwrap_or(0.0);
                let idiosyncratic = residual_by_position.get(&id).copied().unwrap_or(0.0);
                PositionVolContribution {
                    position_id: PositionId::new(id),
                    factor_total,
                    idiosyncratic,
                    total: factor_total + idiosyncratic,
                }
            })
            .collect()
    });

    CreditVolReport {
        total: decomposition.total_risk,
        measure: decomposition.measure,
        generic,
        by_level,
        idiosyncratic_total: decomposition.residual_risk,
        by_position_optional,
    }
}
