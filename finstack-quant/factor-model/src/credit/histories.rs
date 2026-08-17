//! Rebuild covariance from embedded factor histories and historical-simulation
//! factor P&L.

use std::collections::BTreeMap;

use finstack_quant_core::Result;

use super::calibration::{
    assemble_factor_model_config, diff_sparse, factor_variances, CovarianceStrategy, PanelSpace,
    VolModelChoice,
};
use super::hierarchy::{CreditFactorModel, FactorHistories, FactorVolModel};
use crate::{FactorCovarianceMatrix, FactorId};

/// Rebuild [`FactorCovarianceMatrix`] from embedded factor histories.
///
/// Re-runs the same sample/EWMA variance estimator and correlation strategy
/// used at calibration. For [`VolModelChoice::Sample`] plus
/// [`CovarianceStrategy::FullSampleRepaired`] the result matches
/// `model.config.covariance` within a tight numerical tolerance.
///
/// # Arguments
///
/// * `model` - Calibrated artifact whose `factor_histories`,
///   `panel_frequency`, `use_returns_or_levels`, `vol_state`, hierarchy, and
///   issuer beta rows define the rebuild. Histories are dense **bp** series.
///   A [`PanelSpace::Levels`] artifact is first-differenced before vol/corr,
///   matching calibration.
/// * `strategy` - Correlation / covariance assembly rule. Pass the strategy
///   used at calibration to reproduce `config.covariance`.
///
/// # Errors
///
/// Returns [`finstack_quant_core::Error::Validation`] when histories are
/// missing, a factor series length disagrees with `dates`, the persisted
/// vol models are mixed or empty, or covariance assembly fails.
pub fn covariance_from_histories(
    model: &CreditFactorModel,
    strategy: CovarianceStrategy,
) -> Result<FactorCovarianceMatrix> {
    let histories = model.factor_histories.as_ref().ok_or_else(|| {
        finstack_quant_core::Error::Validation(
            "covariance_from_histories: CreditFactorModel.factor_histories is None".to_owned(),
        )
    })?;
    let n_dates = histories.dates.len();
    let mut factor_returns: BTreeMap<FactorId, Vec<Option<f64>>> = BTreeMap::new();
    for (fid, series) in &histories.values {
        if series.len() != n_dates {
            return Err(finstack_quant_core::Error::Validation(format!(
                "covariance_from_histories: factor {} has {} values, expected {n_dates}",
                fid.as_str(),
                series.len()
            )));
        }
        factor_returns.insert(fid.clone(), series.iter().copied().map(Some).collect());
    }

    let stat_returns = match model.use_returns_or_levels {
        PanelSpace::Returns => factor_returns,
        PanelSpace::Levels => factor_returns
            .iter()
            .map(|(fid, series)| (fid.clone(), diff_sparse(series)))
            .collect(),
    };

    let vol_model = vol_model_from_state(model)?;
    let factor_variances = factor_variances(
        &stat_returns,
        vol_model,
        model.panel_frequency.annualization_factor(),
    );
    let factor_id_order: Vec<FactorId> = model.config.covariance.factor_ids().to_vec();
    let (_corr, rebuilt) = assemble_factor_model_config(
        &factor_id_order,
        &factor_variances,
        &stat_returns,
        &model.hierarchy,
        &model.issuer_betas,
        strategy,
        model.panel_frequency.annualization_factor(),
    )?;
    Ok(rebuilt.covariance)
}

/// Period factor P&L `s · F_t` in currency units.
///
/// `histories.values` are treated as **period factor moves in bp** (the
/// Returns-space calibration series). Each date `t` contributes
/// `Σ_i sensitivities[i] * F_{factor_ids[i]}[t]`. A one-factor unit
/// sensitivity therefore reproduces that factor's return series.
///
/// # Arguments
///
/// * `histories` - Dense aligned factor histories in bp of spread move.
/// * `factor_ids` - Factors participating in the dot product, in the same
///   order as `sensitivities`. Each id must exist in `histories.values`.
/// * `sensitivities` - Position exposures in P&L per bp (`β × CS01` for
///   credit). Must have the same length as `factor_ids`.
///
/// # Errors
///
/// Returns [`finstack_quant_core::Error::Validation`] when the sensitivity
/// vector length disagrees with `factor_ids`, a requested factor is missing,
/// or a series length disagrees with `histories.dates`.
pub fn historical_factor_pnl(
    histories: &FactorHistories,
    factor_ids: &[FactorId],
    sensitivities: &[f64],
) -> Result<Vec<f64>> {
    if factor_ids.len() != sensitivities.len() {
        return Err(finstack_quant_core::Error::Validation(format!(
            "historical_factor_pnl: factor_ids len {} != sensitivities len {}",
            factor_ids.len(),
            sensitivities.len()
        )));
    }
    let n = histories.dates.len();
    let mut series_refs: Vec<&[f64]> = Vec::with_capacity(factor_ids.len());
    for fid in factor_ids {
        let series = histories.values.get(fid).ok_or_else(|| {
            finstack_quant_core::Error::Validation(format!(
                "historical_factor_pnl: factor {} is not in histories",
                fid.as_str()
            ))
        })?;
        if series.len() != n {
            return Err(finstack_quant_core::Error::Validation(format!(
                "historical_factor_pnl: factor {} has {} values, expected {n}",
                fid.as_str(),
                series.len()
            )));
        }
        series_refs.push(series.as_slice());
    }

    let mut pnl = vec![0.0; n];
    for (t, slot) in pnl.iter_mut().enumerate() {
        let mut acc = 0.0;
        for (series, s) in series_refs.iter().zip(sensitivities.iter()) {
            acc += s * series[t];
        }
        *slot = acc;
    }
    Ok(pnl)
}

fn vol_model_from_state(model: &CreditFactorModel) -> Result<VolModelChoice> {
    let mut inferred: Option<VolModelChoice> = None;
    for vol in model.vol_state.factors.values() {
        let choice = match vol {
            FactorVolModel::Sample { .. } => VolModelChoice::Sample,
            FactorVolModel::Ewma { lambda, .. } => VolModelChoice::Ewma { lambda: *lambda },
        };
        match inferred {
            None => inferred = Some(choice),
            Some(prev) if prev == choice => {}
            Some(prev) => {
                return Err(finstack_quant_core::Error::Validation(format!(
                    "covariance_from_histories: mixed vol models in vol_state ({prev:?} vs {choice:?})"
                )));
            }
        }
    }
    inferred.ok_or_else(|| {
        finstack_quant_core::Error::Validation(
            "covariance_from_histories: vol_state.factors is empty".to_owned(),
        )
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use finstack_quant_core::dates::create_date;
    use finstack_quant_core::types::IssuerId;
    use time::Month;

    use super::{covariance_from_histories, historical_factor_pnl};
    use crate::credit::calibration::{
        BucketSizeThresholds, BucketWeighting, CovarianceStrategy, CreditCalibrationConfig,
        CreditCalibrationInputs, CreditCalibrator, GenericFactorSeries, HistoryPanel,
        IssuerTagPanel, PanelFrequency, PanelSpace, VolModelChoice,
    };
    use crate::credit::hierarchy::{
        CreditHierarchySpec, GenericFactorSpec, HierarchyDimension, IssuerBetaPolicy, IssuerTags,
    };
    use crate::matching::CREDIT_GENERIC_FACTOR_ID;
    use crate::FactorId;

    fn monthly_dates(n: usize) -> Vec<finstack_quant_core::dates::Date> {
        let months = [
            Month::January,
            Month::February,
            Month::March,
            Month::April,
            Month::May,
            Month::June,
            Month::July,
            Month::August,
            Month::September,
            Month::October,
            Month::November,
            Month::December,
        ];
        (0..n)
            .map(|i| {
                create_date(2020 + i32::try_from(i / 12).unwrap(), months[i % 12], 28).unwrap()
            })
            .collect()
    }

    fn calibrate_sample_full(
        n: usize,
        space: PanelSpace,
    ) -> crate::credit::hierarchy::CreditFactorModel {
        let dates = monthly_dates(n);
        let generic: Vec<f64> = (0..n)
            .map(|i| 0.0010 + 0.0001 * (i as f64 * 0.4).sin())
            .collect();
        let mut tags = BTreeMap::new();
        let mut spreads = BTreeMap::new();
        let mut as_of_spreads = BTreeMap::new();
        for (id, base) in [("A", 0.010), ("B", 0.012)] {
            let issuer = IssuerId::new(id);
            let series: Vec<f64> = (0..n)
                .map(|i| base + 0.0005 * (i as f64 * 1.1).sin())
                .collect();
            tags.insert(
                issuer.clone(),
                IssuerTags(BTreeMap::from([("rating".to_string(), "IG".to_string())])),
            );
            spreads.insert(issuer.clone(), series.iter().map(|v| Some(*v)).collect());
            as_of_spreads.insert(issuer, series[n - 1]);
        }
        let config = CreditCalibrationConfig {
            policy: IssuerBetaPolicy::GloballyOff,
            hierarchy: CreditHierarchySpec {
                levels: vec![HierarchyDimension::Rating],
            },
            min_bucket_size_per_level: BucketSizeThresholds { per_level: vec![1] },
            vol_model: VolModelChoice::Sample,
            covariance_strategy: CovarianceStrategy::FullSampleRepaired,
            beta_shrinkage: crate::credit::calibration::BetaShrinkage::None,
            use_returns_or_levels: space,
            panel_frequency: PanelFrequency::Monthly,
            bucket_weighting: BucketWeighting::Equal,
        };
        CreditCalibrator::new(config)
            .calibrate(CreditCalibrationInputs {
                history_panel: HistoryPanel {
                    dates: dates.clone(),
                    spreads,
                },
                issuer_tags: IssuerTagPanel { tags },
                generic_factor: GenericFactorSeries {
                    spec: GenericFactorSpec {
                        name: "CDX IG".into(),
                        series_id: "cdx.ig".into(),
                    },
                    values: generic,
                },
                as_of: dates[n - 1],
                as_of_spreads,
                idiosyncratic_overrides: BTreeMap::new(),
                spread_durations: BTreeMap::new(),
            })
            .expect("calibration succeeds")
    }

    #[test]
    fn covariance_from_histories_matches_sample_full_repaired() {
        let model = calibrate_sample_full(24, PanelSpace::Returns);
        let rebuilt = covariance_from_histories(&model, CovarianceStrategy::FullSampleRepaired)
            .expect("rebuild succeeds");
        let original = &model.config.covariance;
        assert_eq!(rebuilt.factor_ids(), original.factor_ids());
        assert_eq!(rebuilt.as_slice().len(), original.as_slice().len());
        for (a, b) in rebuilt.as_slice().iter().zip(original.as_slice()) {
            assert!(
                (a - b).abs() <= 1e-10 * a.abs().max(b.abs()).max(1.0),
                "rebuilt covariance {a} must match calibrated {b}"
            );
        }
    }

    #[test]
    fn historical_factor_pnl_unit_sensitivity_equals_return_series() {
        let model = calibrate_sample_full(18, PanelSpace::Returns);
        let histories = model.factor_histories.as_ref().expect("histories present");
        let generic = FactorId::new(CREDIT_GENERIC_FACTOR_ID);
        let series = histories
            .values
            .get(&generic)
            .expect("generic history")
            .clone();
        let pnl = historical_factor_pnl(histories, &[generic], &[1.0]).expect("pnl");
        assert_eq!(pnl, series);
    }

    #[test]
    fn factor_histories_are_dense_bp_without_zero_fill() {
        let model = calibrate_sample_full(12, PanelSpace::Levels);
        let histories = model.factor_histories.as_ref().expect("histories present");
        let generic = histories
            .values
            .get(&FactorId::new(CREDIT_GENERIC_FACTOR_ID))
            .expect("generic history");
        assert_eq!(generic.len(), histories.dates.len());
        // Generic input was ~10 bp after conversion, never a missing date.
        assert!(
            generic.iter().all(|v| (v - 10.0).abs() < 2.0),
            "generic history must be the converted bp series, not 0-filled holes: {generic:?}"
        );
        assert!(
            generic.iter().all(|v| v.abs() > 1.0),
            "complete panel must not 0-fill generic history: {generic:?}"
        );
    }
}
