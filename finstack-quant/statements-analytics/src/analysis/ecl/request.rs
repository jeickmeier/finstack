//! Canonical request surfaces for simplified ECL staging and calculation.

use finstack_quant_core::{Error, Result};
use serde::{Deserialize, Serialize};

use super::engine::{compute_ecl_weighted, EclConfigBuilder, MacroScenario, WeightedEclResult};
use super::policy::default_staging_config;
use super::staging::{classify_stage, StageResult, StagingConfig};
use super::types::{Exposure, PdTermStructure, QualitativeFlags, RawPdCurve, Stage};

const CURRENT_RATING: &str = "current";
const ORIGINATION_RATING: &str = "origination";
const SCENARIO_RATING: &str = "scenario";

/// Inputs for the simplified IFRS 9 stage-classification workflow.
///
/// This request applies the canonical ECL policy defaults and constructs the
/// synthetic rating-based exposure used when callers supply current and
/// origination lifetime PDs directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EclStageRequest {
    /// Stable identifier used to label the classified exposure.
    pub exposure_id: String,
    /// Remaining contractual or behavioural maturity in years, used as the
    /// lifetime-PD comparison horizon subject to the canonical SICR cap.
    pub remaining_maturity_years: f64,
    /// Current lifetime probability of default as a decimal probability.
    pub current_pd: f64,
    /// Lifetime probability of default at initial recognition as a decimal
    /// probability.
    pub origination_pd: f64,
    /// Current days past due; `None` applies the performing-exposure default
    /// of zero days.
    #[serde(default)]
    pub days_past_due: Option<u32>,
    /// Optional absolute lifetime-PD increase that triggers Stage 2; `None`
    /// uses the default IFRS 9 staging-policy threshold.
    #[serde(default)]
    pub pd_delta_absolute: Option<f64>,
    /// Whether to apply the canonical Stage 2 days-past-due backstop; `None`
    /// enables the backstop.
    #[serde(default)]
    pub dpd_stage2_trigger: Option<bool>,
    /// Whether to apply the canonical Stage 3 days-past-due backstop; `None`
    /// enables the backstop.
    #[serde(default)]
    pub dpd_stage3_trigger: Option<bool>,
}

impl EclStageRequest {
    /// Return the request's days past due after applying the canonical default.
    #[must_use]
    pub fn resolved_days_past_due(&self) -> u32 {
        self.days_past_due.unwrap_or_default()
    }

    /// Classify the request under the canonical simplified staging policy.
    ///
    /// The simplified workflow intentionally disables relative-PD, rating,
    /// qualitative, and cure-state inputs that are absent from this request.
    /// Use [`classify_stage`] directly for the complete staging surface.
    ///
    /// # Returns
    ///
    /// The assigned IFRS 9 stage and its ordered trigger audit trail.
    ///
    /// # Errors
    ///
    /// Returns an error if the synthetic PD source cannot resolve the current
    /// or origination rating used during SICR comparison.
    pub fn classify(&self) -> Result<StageResult> {
        let defaults = default_staging_config();
        let config = StagingConfig {
            pd_delta_absolute: self.pd_delta_absolute.unwrap_or(defaults.pd_delta_absolute),
            pd_delta_relative: f64::INFINITY,
            rating_downgrade_notches: 0,
            rating_scale_labels: None,
            dpd_stage2_threshold: if self.dpd_stage2_trigger.unwrap_or(true) {
                defaults.dpd_stage2_threshold
            } else {
                u32::MAX
            },
            dpd_stage3_threshold: if self.dpd_stage3_trigger.unwrap_or(true) {
                defaults.dpd_stage3_threshold
            } else {
                u32::MAX
            },
            qualitative_triggers_enabled: false,
            stage3_qualitative_triggers_enabled: false,
            cure_periods_stage2_to_1: defaults.cure_periods_stage2_to_1,
            cure_periods_stage3_to_2: defaults.cure_periods_stage3_to_2,
        };
        let exposure = Exposure {
            id: self.exposure_id.clone(),
            segments: vec![],
            ead: 0.0,
            eir: 0.0,
            remaining_maturity_years: self.remaining_maturity_years,
            lgd: 0.0,
            days_past_due: self.resolved_days_past_due(),
            current_rating: Some(CURRENT_RATING.to_string()),
            origination_rating: Some(ORIGINATION_RATING.to_string()),
            qualitative_flags: QualitativeFlags::default(),
            consecutive_performing_periods: 0,
            previous_stage: None,
            ead_schedule: None,
            undrawn: 0.0,
            ccf: 0.75,
        };
        let pd_source = RequestPdSource {
            current_pd: self.current_pd,
            origination_pd: self.origination_pd,
        };

        classify_stage(&exposure, &pd_source, &config)
    }
}

/// Inputs for probability-weighted ECL from cumulative-PD schedules.
///
/// The request owns the complete simplified calculation policy: omitted
/// configuration values come from the canonical IFRS 9 ECL defaults, scenario
/// schedules are anchored at `(0, 0)`, and the supplied maturity determines the
/// 12-month or lifetime horizon according to [`Stage`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EclRequest {
    /// Stable identifier copied into the calculation result.
    pub exposure_id: String,
    /// Exposure at default in the exposure's base currency.
    pub ead: f64,
    /// Loss given default as a decimal probability in `[0, 1]`.
    pub lgd: f64,
    /// Annual effective interest rate as a decimal used for discounting.
    pub eir: f64,
    /// Remaining contractual or behavioural maturity in years.
    pub remaining_maturity_years: f64,
    /// Assigned IFRS 9 stage controlling the 12-month or lifetime horizon.
    pub stage: Stage,
    /// Probability-weighted scenarios represented as `(weight, schedule)`;
    /// each schedule contains `(time_years, cumulative_pd)` knots.
    pub scenarios: Vec<(f64, Vec<(f64, f64)>)>,
    /// Optional ECL integration-bucket width in years; `None` uses the
    /// canonical IFRS 9 policy default.
    #[serde(default)]
    pub bucket_width_years: Option<f64>,
    /// Optional exposure-at-default profile represented as
    /// `(time_years, ead)` knots.
    #[serde(default)]
    pub ead_schedule: Option<Vec<(f64, f64)>>,
    /// Optional Stage 3 expected-recovery horizon in years; `None` uses the
    /// canonical IFRS 9 ECL default.
    #[serde(default)]
    pub stage3_time_to_recovery_years: Option<f64>,
}

impl EclRequest {
    /// Compute probability-weighted ECL under the canonical request policy.
    ///
    /// # Returns
    ///
    /// The weighted ECL, assigned stage, exposure identifier, and individual
    /// scenario results.
    ///
    /// # Errors
    ///
    /// Returns an error when exposure inputs, configuration, scenario weights,
    /// or cumulative-PD schedules violate their canonical invariants.
    pub fn compute(&self) -> Result<WeightedEclResult> {
        if self.scenarios.is_empty() {
            return Err(Error::Validation(
                "At least one scenario is required for weighted ECL".to_string(),
            ));
        }

        let scenarios = self
            .scenarios
            .iter()
            .enumerate()
            .map(|(index, (weight, _))| MacroScenario {
                id: format!("scenario_{index}"),
                weight: *weight,
                lgd_override: None,
            })
            .collect::<Vec<_>>();
        let mut config = EclConfigBuilder::new().scenarios(scenarios.clone());
        if let Some(years) = self.bucket_width_years {
            config = config.bucket_width(years);
        }
        if let Some(years) = self.stage3_time_to_recovery_years {
            config = config.stage3_time_to_recovery(years);
        }
        let config = config.build()?;

        let exposure = Exposure {
            id: self.exposure_id.clone(),
            segments: vec![],
            ead: self.ead,
            eir: self.eir,
            remaining_maturity_years: self.remaining_maturity_years,
            lgd: self.lgd,
            days_past_due: 0,
            current_rating: Some(SCENARIO_RATING.to_string()),
            origination_rating: Some(SCENARIO_RATING.to_string()),
            qualitative_flags: QualitativeFlags::default(),
            consecutive_performing_periods: 0,
            previous_stage: None,
            ead_schedule: self.ead_schedule.clone(),
            undrawn: 0.0,
            ccf: 0.75,
        };
        let curves = self
            .scenarios
            .iter()
            .map(|(_, schedule)| {
                RawPdCurve::new(SCENARIO_RATING, RawPdCurve::anchor_knots(schedule.clone()))
            })
            .collect::<Result<Vec<_>>>()?;
        let pd_sources = scenarios
            .iter()
            .zip(curves.iter())
            .map(|(scenario, curve)| (scenario, curve as &dyn PdTermStructure))
            .collect::<Vec<_>>();

        compute_ecl_weighted(&exposure, self.stage, &pd_sources, &config)
    }
}

struct RequestPdSource {
    current_pd: f64,
    origination_pd: f64,
}

impl PdTermStructure for RequestPdSource {
    fn cumulative_pd(&self, rating: &str, _t: f64) -> Result<f64> {
        match rating {
            CURRENT_RATING => Ok(self.current_pd),
            ORIGINATION_RATING => Ok(self.origination_pd),
            other => Err(Error::Validation(format!(
                "EclStageRequest: unknown synthetic rating '{other}'"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::StagingTrigger;

    fn stage_request() -> EclStageRequest {
        EclStageRequest {
            exposure_id: "loan".to_string(),
            remaining_maturity_years: 5.0,
            current_pd: 0.02,
            origination_pd: 0.015,
            days_past_due: None,
            pd_delta_absolute: None,
            dpd_stage2_trigger: None,
            dpd_stage3_trigger: None,
        }
    }

    fn ecl_request() -> EclRequest {
        EclRequest {
            exposure_id: "loan".to_string(),
            ead: 100.0,
            lgd: 0.4,
            eir: 0.0,
            remaining_maturity_years: 2.0,
            stage: Stage::Stage2,
            scenarios: vec![(1.0, vec![(1.0, 0.1), (2.0, 0.2)])],
            bucket_width_years: None,
            ead_schedule: None,
            stage3_time_to_recovery_years: None,
        }
    }

    #[test]
    fn stage_request_applies_default_and_explicit_backstops() {
        let default_request = stage_request();
        assert_eq!(default_request.resolved_days_past_due(), 0);
        assert_eq!(default_request.classify().unwrap().stage, Stage::Stage1);

        let explicit_default = EclStageRequest {
            days_past_due: Some(30),
            ..stage_request()
        };
        let result = explicit_default.classify().unwrap();
        assert_eq!(result.stage, Stage::Stage2);
        assert!(matches!(
            result.triggers.first(),
            Some(StagingTrigger::DpdStage2 {
                dpd: 30,
                threshold: 30
            })
        ));

        let disabled = EclStageRequest {
            days_past_due: Some(91),
            dpd_stage2_trigger: Some(false),
            dpd_stage3_trigger: Some(false),
            ..stage_request()
        };
        assert_eq!(disabled.classify().unwrap().stage, Stage::Stage1);
    }

    #[test]
    fn stage_request_uses_default_or_explicit_pd_threshold() {
        let default_result = EclStageRequest {
            current_pd: 0.026,
            ..stage_request()
        }
        .classify()
        .unwrap();
        assert_eq!(default_result.stage, Stage::Stage2);

        let explicit_result = EclStageRequest {
            current_pd: 0.026,
            pd_delta_absolute: Some(0.02),
            ..stage_request()
        }
        .classify()
        .unwrap();
        assert_eq!(explicit_result.stage, Stage::Stage1);
    }

    #[test]
    fn ecl_request_preserves_weights_defaults_and_lifetime_horizon() {
        let default = ecl_request().compute().unwrap();
        assert!((default.ecl - 8.0).abs() < 1e-12);
        assert_eq!(default.scenario_breakdown.len(), 1);
        assert_eq!(default.scenario_breakdown[0].1, 1.0);
        assert_eq!(default.scenario_breakdown[0].2.buckets.len(), 8);

        let weighted = EclRequest {
            scenarios: vec![
                (0.25, vec![(1.0, 0.1), (2.0, 0.2)]),
                (0.75, vec![(1.0, 0.2), (2.0, 0.4)]),
            ],
            ..ecl_request()
        }
        .compute()
        .unwrap();
        assert!((weighted.ecl - 14.0).abs() < 1e-12);
        assert_eq!(weighted.scenario_breakdown.len(), 2);

        let stage1 = EclRequest {
            stage: Stage::Stage1,
            ..ecl_request()
        }
        .compute()
        .unwrap();
        assert_eq!(stage1.scenario_breakdown[0].2.buckets.len(), 4);
    }

    #[test]
    fn ecl_request_rejects_missing_and_invalid_scenarios() {
        let missing = EclRequest {
            scenarios: vec![],
            ..ecl_request()
        }
        .compute()
        .unwrap_err();
        assert_eq!(
            missing.to_string(),
            "Validation error: At least one scenario is required for weighted ECL"
        );

        let invalid_weights = EclRequest {
            scenarios: vec![(0.4, vec![(1.0, 0.1)])],
            ..ecl_request()
        }
        .compute()
        .unwrap_err();
        assert_eq!(
            invalid_weights.to_string(),
            "Validation error: Scenario weights must sum to 1.0, got 0.400000"
        );

        let missing_curve = EclRequest {
            scenarios: vec![(1.0, vec![])],
            ..ecl_request()
        }
        .compute()
        .unwrap_err();
        assert!(missing_curve
            .to_string()
            .contains("At least two data points are required"));
    }

    #[test]
    fn request_serde_rejects_unknown_fields() {
        let error = serde_json::from_value::<EclRequest>(serde_json::json!({
            "exposure_id": "loan",
            "ead": 100.0,
            "lgd": 0.4,
            "eir": 0.0,
            "remaining_maturity_years": 2.0,
            "stage": "stage2",
            "scenarios": [[1.0, [[1.0, 0.1]]]],
            "bucket_width_years": null,
            "ead_schedule": null,
            "stage3_time_to_recovery_years": null,
            "obsolete": true
        }))
        .unwrap_err();
        assert!(error.to_string().contains("unknown field `obsolete`"));

        let parsed = serde_json::from_value::<EclRequest>(serde_json::json!({
            "exposure_id": "loan",
            "ead": 100.0,
            "lgd": 0.4,
            "eir": 0.0,
            "remaining_maturity_years": 2.0,
            "stage": "stage2",
            "scenarios": [[1.0, [[1.0, 0.1]]]]
        }))
        .unwrap();
        assert!(parsed.bucket_width_years.is_none());
        assert!(parsed.ead_schedule.is_none());
        assert!(parsed.stage3_time_to_recovery_years.is_none());
    }
}
