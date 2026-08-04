use std::collections::BTreeMap;

use finstack_quant_core::types::IssuerId;
use finstack_quant_core::Result;

use super::assemble::{
    anchor_levels, assemble_factor_model_config, build_diagnostics, build_factor_histories,
    build_factor_id_order, build_vol_state,
};
use super::config::CreditCalibrationConfig;
use super::inputs::CreditCalibrationInputs;
use super::inventory::{apply_fold_up, build_bucket_inventory};
use super::panel::{build_working_panel, classify_mode};
use super::peel_fit::{run_peel, unit_betas};
use super::statistics::{
    adder_vols_from_history, assign_adder_vol, build_peer_proxy_index, factor_variances,
};
use super::validation::{validate_calibration_config, validate_calibration_inputs, validation_err};
use crate::credit::hierarchy::{
    CreditFactorModel, CreditFactorModelSchema, DateRange, IssuerBetaMode, IssuerBetaRow,
};

/// Deterministic calibrator that produces a [`CreditFactorModel`].
///
/// Construct with [`CreditCalibrator::new`], then run [`Self::calibrate`].
#[derive(Debug, Clone)]
pub struct CreditCalibrator {
    config: CreditCalibrationConfig,
}

impl CreditCalibrator {
    /// Wrap a configuration into a calibrator.
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration object controlling validation, rounding, or solver behavior
    #[must_use]
    pub fn new(config: CreditCalibrationConfig) -> Self {
        Self { config }
    }

    /// Run the full calibration pipeline.
    ///
    /// # Errors
    ///
    /// Returns [`finstack_quant_core::Error::Validation`] when:
    /// - the calibration config fails validation (e.g. an out-of-range
    ///   [`VolModelChoice::Ewma`][super::super::calibration::VolModelChoice] `lambda`),
    /// - the inputs are structurally malformed (length mismatches, missing
    ///   `as_of` in the date grid, missing tags),
    /// - the assembled [`CreditFactorModel::validate`] check fails.
    ///
    /// # Arguments
    ///
    /// * `inputs` - Inputs supplied by the caller for this operation
    pub fn calibrate(&self, inputs: CreditCalibrationInputs) -> Result<CreditFactorModel> {
        validate_calibration_config(&self.config)?;
        validate_calibration_inputs(&inputs)?;

        // -- Structural validation of inputs. -------------------------------
        let dates = &inputs.history_panel.dates;
        if dates.is_empty() {
            return Err(validation_err(
                "CreditCalibrator: history_panel.dates is empty",
            ));
        }
        if inputs.generic_factor.values.len() != dates.len() {
            return Err(validation_err(format!(
                "CreditCalibrator: generic_factor.values length {} != dates length {}",
                inputs.generic_factor.values.len(),
                dates.len()
            )));
        }
        for (issuer, series) in &inputs.history_panel.spreads {
            if series.len() != dates.len() {
                return Err(validation_err(format!(
                    "CreditCalibrator: spread series for issuer {:?} has length {}, expected {}",
                    issuer.as_str(),
                    series.len(),
                    dates.len()
                )));
            }
        }

        let asof_idx = dates
            .iter()
            .position(|d| *d == inputs.as_of)
            .ok_or_else(|| {
                validation_err(format!(
                    "CreditCalibrator: as_of {:?} not present in history_panel.dates",
                    inputs.as_of
                ))
            })?;

        // -- 1. Mode classification. ----------------------------------------
        let mut modes: BTreeMap<IssuerId, IssuerBetaMode> = BTreeMap::new();
        for issuer in inputs.history_panel.spreads.keys() {
            let mode = classify_mode(
                &self.config.policy,
                issuer,
                &inputs.history_panel.spreads,
                &self.config.use_returns_or_levels,
            );
            modes.insert(issuer.clone(), mode);
        }

        // -- 2. Returns or levels. ------------------------------------------
        let panel = build_working_panel(
            &self.config.use_returns_or_levels,
            dates,
            &inputs.history_panel.spreads,
            &inputs.generic_factor.values,
        );

        // -- 3. Bucket inventory + fold-up. ---------------------------------
        let inventory =
            build_bucket_inventory(&self.config.hierarchy, &inputs.issuer_tags.tags, &modes)?;
        let (folded, fold_ups) = apply_fold_up(&inventory, &self.config.min_bucket_size_per_level);
        let bucket_sizes_per_level = inventory.bucket_sizes_per_level.clone();
        let tag_taxonomy = inventory.tag_taxonomy.clone();

        // -- 4 + 5. PC peel + per-level peel. -------------------------------
        let peel_outcome = run_peel(
            &self.config,
            &panel,
            &modes,
            &inventory.bucket_paths,
            &folded,
        );

        // -- 6. Adder series → idiosyncratic vol. ---------------------------
        // Compute from-history vols for every issuer with enough residual
        // observations, regardless of mode: under `GloballyOff` (the default
        // policy) all issuers are `BucketOnly`, and restricting this step to
        // `IssuerBeta` issuers would leave every idiosyncratic vol at the
        // hard-coded 0.0 fallback — silently zeroing issuer-specific risk.
        let from_history_vols = adder_vols_from_history(
            &peel_outcome.adder_series,
            self.config.vol_model,
            self.config.annualization_factor,
        );
        // Build per-level peer proxy index: level_k → bucket_path → [vols].
        let peer_proxy_index = build_peer_proxy_index(
            &from_history_vols,
            &inventory.bucket_paths,
            self.config.hierarchy.levels.len(),
        );

        // -- 7. Anchor levels at as_of. -------------------------------------
        let generic_at_asof = inputs.generic_factor.values[asof_idx];
        let anchor = anchor_levels(
            &self.config.hierarchy,
            &inputs.as_of_spreads,
            &inputs.issuer_tags.tags,
            generic_at_asof,
            &peel_outcome.betas,
            &folded,
        )?;

        // -- 8. Per-factor variance forecast (sample or EWMA). --------------
        let factor_variances = factor_variances(
            &peel_outcome.factor_returns,
            self.config.vol_model,
            self.config.annualization_factor,
        );

        // -- Build issuer beta rows. ----------------------------------------
        let mut issuer_betas: Vec<IssuerBetaRow> = Vec::new();
        for issuer_id in inputs.history_panel.spreads.keys() {
            // Every issuer in `spreads` was classified in step 1 above, so this
            // lookup is by-construction `Some(_)`. Fall back to BucketOnly to
            // avoid `.expect()` (clippy::expect_used is `#[deny]` in this crate).
            let mode = modes
                .get(issuer_id)
                .copied()
                .unwrap_or(IssuerBetaMode::BucketOnly);
            let tags = inputs
                .issuer_tags
                .tags
                .get(issuer_id)
                .cloned()
                .unwrap_or_default();
            let betas = peel_outcome
                .betas
                .get(issuer_id)
                .cloned()
                .unwrap_or_else(|| unit_betas(self.config.hierarchy.levels.len()));
            let adder_at_anchor = anchor.adder.get(issuer_id).copied().unwrap_or(0.0);
            let (adder_vol, adder_vol_source) = assign_adder_vol(
                issuer_id,
                &from_history_vols,
                &peer_proxy_index,
                &inventory.bucket_paths,
                &inputs.idiosyncratic_overrides,
                self.config.hierarchy.levels.len(),
            );
            let fit_quality = peel_outcome.fit_quality.get(issuer_id).cloned();
            issuer_betas.push(IssuerBetaRow {
                issuer_id: issuer_id.clone(),
                tags,
                mode,
                betas,
                adder_at_anchor,
                adder_vol_annualized: adder_vol,
                adder_vol_source,
                fit_quality,
            });
        }
        // BTreeMap iteration is already sorted by issuer_id, but be defensive.
        issuer_betas.sort_by(|a, b| a.issuer_id.as_str().cmp(b.issuer_id.as_str()));

        // -- 9. Correlation matrix and covariance assembly. -----------------
        let factor_id_order = build_factor_id_order(&peel_outcome.factor_returns);

        // -- 10. Assemble FactorModelConfig. --------------------------------
        let (static_correlation, config) = assemble_factor_model_config(
            &factor_id_order,
            &factor_variances,
            &peel_outcome.factor_returns,
            &self.config.hierarchy,
            &issuer_betas,
            self.config.covariance_strategy,
            self.config.annualization_factor,
        )?;

        // -- 11. Diagnostics. -----------------------------------------------
        let diagnostics = build_diagnostics(
            &modes,
            bucket_sizes_per_level,
            fold_ups,
            &peel_outcome.fit_quality,
            tag_taxonomy,
        );

        // -- 12. Bundle artifact + final validate(). ------------------------
        let calibration_window = DateRange {
            start: *dates
                .first()
                .ok_or_else(|| validation_err("dates non-empty checked above"))?,
            end: *dates
                .last()
                .ok_or_else(|| validation_err("dates non-empty checked above"))?,
        };

        let factor_histories = Some(build_factor_histories(
            dates,
            &self.config.use_returns_or_levels,
            &peel_outcome.factor_returns,
        ));
        let vol_state = build_vol_state(&factor_variances, &issuer_betas, self.config.vol_model);

        let model = CreditFactorModel {
            schema: CreditFactorModelSchema::CURRENT,
            as_of: inputs.as_of,
            calibration_window,
            policy: self.config.policy.clone(),
            generic_factor: inputs.generic_factor.spec.clone(),
            hierarchy: self.config.hierarchy.clone(),
            config,
            issuer_betas,
            anchor_state: anchor.levels,
            static_correlation,
            vol_state,
            factor_histories,
            diagnostics,
        };

        model.validate()?;
        Ok(model)
    }
}
