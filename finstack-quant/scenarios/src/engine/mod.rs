//! Deterministic scenario execution engine.
//!
//! The engine glues together adapters from this crate to compose multiple
//! [`ScenarioSpec`](crate::spec::ScenarioSpec) definitions and apply them to
//! a mutable [`ExecutionContext`]. Its responsibilities are:
//! - enforce a repeatable ordering of operations
//! - dispatch each `OperationSpec` variant to the appropriate adapter function
//!   via a centralized exhaustive `match`
//! - batch market bumps so the underlying `MarketContext` is cloned at most
//!   once per scenario application instead of once per operation
//! - collect reporting metadata about how many operations ran and any
//!   warnings produced during execution

mod effects;
mod hierarchy;
mod instrument_shocks;
mod types;

#[cfg(test)]
mod tests;

pub use types::{
    ApplicationEnvelope, ApplicationReport, ExecutionContext, RollForwardReport,
    ScenarioChangeManifest, ScenarioMarketTarget,
};

use crate::adapters::traits::ScenarioEffect;
use crate::error::Result;
use crate::spec::{OperationSpec, ScenarioSpec};
use crate::warning::Warning;
use effects::{flush_pending_bumps, process_effects};
use finstack_quant_core::market_data::bumps::MarketBump;
use finstack_quant_core::market_data::hierarchy::ResolutionMode;
use hierarchy::{expand_hierarchy_operations, ExpansionOutcome};

fn rounding_stamp(config: &finstack_quant_core::config::FinstackConfig) -> Option<String> {
    Some(config.rounding.mode.to_string())
}

/// Orchestrates the deterministic application of a [`ScenarioSpec`].
///
/// The engine is intentionally lightweight: it owns only an immutable
/// [`FinstackConfig`](finstack_quant_core::config::FinstackConfig) (used to stamp
/// the active rounding policy into reports) and can be cloned or reused
/// freely. All mutable inputs are supplied via [`ExecutionContext`].
#[derive(Debug, Default, Clone)]
pub struct ScenarioEngine {
    /// Active configuration; its rounding mode is stamped into
    /// [`ApplicationReport::rounding_context`].
    config: finstack_quant_core::config::FinstackConfig,
}

impl ScenarioEngine {
    /// Create a new scenario engine with the default [`FinstackConfig`](finstack_quant_core::config::FinstackConfig).
    ///
    /// # Examples
    /// ```rust
    /// use finstack_quant_scenarios::ScenarioEngine;
    ///
    /// let engine = ScenarioEngine::new();
    /// let other = ScenarioEngine::default();
    /// assert_eq!(format!("{:?}", engine), format!("{:?}", other));
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a scenario engine carrying the caller's active configuration.
    ///
    /// The configuration's rounding mode is stamped into
    /// [`ApplicationReport::rounding_context`] so reports reflect the policy
    /// actually in force rather than the library default.
    ///
    /// # Arguments
    ///
    /// * `config` - Active library configuration whose rounding mode is
    ///   recorded in every application report.
    #[must_use]
    pub fn with_config(config: finstack_quant_core::config::FinstackConfig) -> Self {
        Self { config }
    }

    pub(crate) fn compose_inner(&self, mut scenarios: Vec<ScenarioSpec>) -> ScenarioSpec {
        // Stable sort by priority (lower = higher priority)
        scenarios.sort_by_key(|s| s.priority);

        let composed_id = if scenarios.is_empty() {
            "composed".to_string()
        } else {
            scenarios
                .iter()
                .map(|scenario| scenario.id.as_str())
                .collect::<Vec<_>>()
                .join("+")
        };
        let composed_name = if scenarios.is_empty() {
            Some("Composed Scenario".to_string())
        } else {
            Some(
                scenarios
                    .iter()
                    .map(|scenario| scenario.name.as_deref().unwrap_or(scenario.id.as_str()))
                    .collect::<Vec<_>>()
                    .join(" + "),
            )
        };
        let mut all_operations = Vec::new();
        let resolution_mode = if scenarios.is_empty() {
            ResolutionMode::default()
        } else if scenarios
            .iter()
            .all(|scenario| scenario.resolution_mode == scenarios[0].resolution_mode)
        {
            scenarios[0].resolution_mode
        } else {
            ResolutionMode::Cumulative
        };

        for scenario in scenarios {
            all_operations.extend(scenario.operations);
        }

        ScenarioSpec {
            id: composed_id,
            name: composed_name,
            description: None,
            operations: all_operations,
            priority: 0,
            resolution_mode,
        }
    }

    /// Strict composition: returns an error at compose time when the
    /// concatenated operations would be rejected at apply time.
    ///
    /// Currently the only compose-time-detectable pathology is the presence of
    /// more than one [`OperationSpec::TimeRollForward`] across the composed
    /// scenarios. Production callers should prefer this method.
    ///
    /// # Errors
    ///
    /// Returns a validation error if composition contains multiple time-roll
    /// operations. Other conflicts remain in the composed spec and are
    /// validated when [`Self::apply`] is called.
    ///
    /// # Arguments
    ///
    /// * `scenarios` - Scenarios supplied by the caller for this operation
    pub fn try_compose(
        &self,
        scenarios: Vec<ScenarioSpec>,
    ) -> std::result::Result<ScenarioSpec, crate::error::Error> {
        let composed = self.compose_inner(scenarios);

        let time_roll_count = composed
            .operations
            .iter()
            .filter(|op| matches!(op, OperationSpec::TimeRollForward { .. }))
            .count();
        if time_roll_count > 1 {
            return Err(crate::error::Error::validation(format!(
                "Compose would produce {time_roll_count} TimeRollForward operations; only \
                 one is allowed per composed scenario. Merge the roll periods into a single \
                 `TimeRollForward` (preferred) or remove the duplicates before calling compose."
            )));
        }

        Ok(composed)
    }

    /// Apply a scenario specification to the execution context.
    ///
    /// Operations are applied in this order:
    /// 0. Time roll-forward, if present
    /// 1. Market data (FX, equities, vol surfaces, curves, base correlation) — all
    ///    [`MarketBump`] effects accumulated during this phase are applied to the
    ///    context in a single batched market bump call.
    /// 2. Rate bindings update (if configured)
    /// 3. Statement forecast adjustments
    /// 4. Statement re-evaluation
    ///
    /// If a [`crate::spec::OperationSpec::TimeRollForward`] sets
    /// `apply_shocks = false`, the engine returns immediately after phase 0 and
    /// does not apply the remaining operations in `spec`.
    ///
    /// # Atomicity
    ///
    /// Application is **not atomic**: operations mutate `ctx.market` (and the
    /// statement model) in place as they execute. If a later operation fails —
    /// for example a curve id that does not exist in the market — the engine
    /// returns `Err` with all earlier operations already applied and no
    /// rollback. Callers that need all-or-nothing semantics should apply the
    /// scenario to a clone of the market context and swap it in on success
    /// (the Python and WASM bindings do exactly this by operating on
    /// deserialized copies).
    ///
    /// # Errors
    ///
    /// Returns validation errors for an invalid spec, unsupported operation
    /// data, missing market objects, or hierarchy-targeted operations without
    /// an attached hierarchy. Because execution is not atomic, an error can
    /// follow successful mutation by earlier operations.
    ///
    /// # Arguments
    ///
    /// * `spec` - Spec supplied by the caller for this operation
    /// * `ctx` - Ctx supplied by the caller for this operation
    #[tracing::instrument(skip_all, fields(scenario_id = %spec.id))]
    pub fn apply(
        &self,
        spec: &ScenarioSpec,
        ctx: &mut ExecutionContext,
    ) -> Result<ApplicationReport> {
        // Validate up-front so malformed specs cannot reach adapters. FFI
        // bindings (Python, WASM) deserialize JSON straight into a spec and
        // call this entry point without their own validation pass.
        spec.validate()?;

        let mut applied = 0;
        let mut warnings: Vec<Warning> = Vec::new();
        let initial_as_of = ctx.as_of;
        let mut changes = ScenarioChangeManifest::default();

        let user_operations = spec.operations.len();

        // Phase -1: Expand hierarchy-targeted operations to direct operations.
        // Errors fast if the spec contains hierarchy ops but no hierarchy is
        // attached to the market context. Hierarchy targets that resolve to
        // zero curves emit a `Warning::HierarchyNoMatch` so the caller can
        // detect the unintended no-op.
        let ExpansionOutcome {
            operations: expanded_ops,
            warnings: expansion_warnings,
        } = expand_hierarchy_operations(&spec.operations, ctx.market, spec.resolution_mode)?;
        let expanded_operations = expanded_ops.len();
        warnings.extend(expansion_warnings);

        // Phase 0: Time Roll Forward (`spec.validate()` already enforced the
        // at-most-one invariant; no need to re-count here.)
        let mut time_roll: Option<RollForwardReport> = None;
        for op in expanded_ops.iter() {
            if let OperationSpec::TimeRollForward {
                period,
                apply_shocks,
                roll_mode,
            } = op
            {
                let _span = tracing::info_span!("phase_0_time_roll", period = %period).entered();
                let roll_report =
                    crate::adapters::time_roll::apply_time_roll_forward(ctx, period, *roll_mode)?;
                applied += 1;

                // Valuation failures during the roll must not vanish: surface
                // each as a structured warning so callers that only inspect
                // the ApplicationReport still see them.
                for (instrument_id, reason) in &roll_report.failed_instruments {
                    warnings.push(Warning::TimeRollInstrumentFailed {
                        instrument_id: instrument_id.clone(),
                        reason: reason.clone(),
                    });
                }

                let stop_after_roll = !*apply_shocks;
                time_roll = Some(roll_report);
                changes.as_of_changed = ctx.as_of != initial_as_of;
                changes.all_dirty |= changes.as_of_changed;

                if stop_after_roll {
                    return Ok(ApplicationReport {
                        operations_applied: applied,
                        user_operations,
                        expanded_operations,
                        changes,
                        warnings,
                        rounding_context: rounding_stamp(&self.config),
                        time_roll,
                    });
                }
            }
        }

        let has_rate_bindings = ctx.rate_bindings.is_some();
        let mut deferred_stmts = Vec::new();
        let mut pending_bumps: Vec<MarketBump> = Vec::new();

        // Phase 1: Generate effects and split into market bumps (intra-op
        // batched), curve replacements, instrument shocks, and deferred
        // statement ops. Bumps from the previous iteration are flushed before
        // generating effects for the next op so adapters always observe a
        // fully-applied prior-op market state — this preserves the sequential
        // semantics that downstream cross-curve calibrations depend on.
        {
            let _span = tracing::info_span!("phase_1_market", ops = expanded_operations).entered();
            for op in expanded_ops.iter() {
                if let OperationSpec::TimeRollForward { .. } = op {
                    continue; // handled in Phase 0
                }

                // Apply any bumps queued by the previous iteration so the
                // adapter's `ctx.market` reads reflect everything done so far.
                flush_pending_bumps(&mut pending_bumps, ctx.market)?;

                process_effects(
                    op,
                    ctx,
                    &mut pending_bumps,
                    &mut deferred_stmts,
                    &mut warnings,
                    &mut applied,
                    &mut changes,
                )?;
            }

            // Flush any remaining bumps before moving on to statements.
            flush_pending_bumps(&mut pending_bumps, ctx.market)?;
        }

        // Phase 2: Rate bindings update (from context configuration).
        //
        // The map key is authoritative for routing; mismatched binding.node_id
        // is a hard error so the caller fixes the binding upstream rather than
        // discovering a silent rewrite later.
        if let Some(bindings) = &ctx.rate_bindings {
            let _span = tracing::info_span!("phase_2_rate_bindings").entered();
            for (node_id, binding) in bindings {
                if binding.node_id != *node_id {
                    return Err(crate::error::Error::Validation(format!(
                        "Rate binding node_id mismatch: map key '{node_id}' does not equal \
                         binding.node_id '{}'. The map key is authoritative for routing; \
                         rebuild the binding with node_id set to the map key.",
                        binding.node_id
                    )));
                }

                let Some(model) = ctx.model.as_deref_mut() else {
                    return Err(crate::error::Error::missing_statement_model("rate binding"));
                };
                match crate::adapters::statements::update_rate_from_binding(
                    binding,
                    model,
                    ctx.market,
                    ctx.calendar,
                ) {
                    Ok(true) => {}
                    Ok(false) => warnings.push(Warning::RateBindingNoForecastValues {
                        node_id: node_id.as_str().to_string(),
                        curve_id: binding.curve_id.as_str().to_string(),
                    }),
                    Err(e) => warnings.push(Warning::RateBindingFailed {
                        node_id: node_id.as_str().to_string(),
                        curve_id: binding.curve_id.as_str().to_string(),
                        reason: e.to_string(),
                    }),
                }
            }
        }

        // Phase 3: Statement Operations (Deferred)
        let mut applied_stmt_ops = 0usize;
        {
            let _span = tracing::info_span!("phase_3_statements").entered();
            for effect in deferred_stmts {
                match effect {
                    ScenarioEffect::RateBinding { binding } => {
                        if let Some(rb) = &mut ctx.rate_bindings {
                            rb.insert(binding.node_id.clone(), binding.clone());
                        }
                        let Some(model) = ctx.model.as_deref_mut() else {
                            return Err(crate::error::Error::missing_statement_model(
                                "rate binding",
                            ));
                        };
                        match crate::adapters::statements::update_rate_from_binding(
                            &binding,
                            model,
                            ctx.market,
                            ctx.calendar,
                        ) {
                            Ok(true) => {
                                applied += 1;
                                applied_stmt_ops += 1;
                            }
                            Ok(false) => {
                                applied += 1;
                                applied_stmt_ops += 1;
                                warnings.push(Warning::RateBindingNoForecastValues {
                                    node_id: binding.node_id.as_str().to_string(),
                                    curve_id: binding.curve_id.as_str().to_string(),
                                });
                            }
                            Err(e) => warnings.push(Warning::RateBindingFailed {
                                node_id: binding.node_id.as_str().to_string(),
                                curve_id: binding.curve_id.as_str().to_string(),
                                reason: e.to_string(),
                            }),
                        }
                    }
                    ScenarioEffect::StmtForecastPercent { node_id, pct } => {
                        let Some(model) = ctx.model.as_deref_mut() else {
                            return Err(crate::error::Error::missing_statement_model(
                                "statement forecast percent",
                            ));
                        };
                        match crate::adapters::statements::apply_forecast_percent(
                            model,
                            node_id.as_str(),
                            pct,
                        ) {
                            Ok(true) => {
                                applied += 1;
                                applied_stmt_ops += 1;
                            }
                            Ok(false) => warnings.push(Warning::StatementNodeNoValues {
                                node_id: node_id.as_str().to_string(),
                                op: "forecast_percent".to_string(),
                            }),
                            Err(e) => warnings.push(Warning::StatementOpFailed {
                                node_id: node_id.as_str().to_string(),
                                op: "forecast_percent".to_string(),
                                reason: e.to_string(),
                            }),
                        }
                    }
                    ScenarioEffect::StmtForecastAssign { node_id, value } => {
                        let Some(model) = ctx.model.as_deref_mut() else {
                            return Err(crate::error::Error::missing_statement_model(
                                "statement forecast assign",
                            ));
                        };
                        match crate::adapters::statements::apply_forecast_assign(
                            model,
                            node_id.as_str(),
                            value,
                            None,
                        ) {
                            Ok(true) => {
                                applied += 1;
                                applied_stmt_ops += 1;
                            }
                            Ok(false) => warnings.push(Warning::StatementNodeNoValues {
                                node_id: node_id.as_str().to_string(),
                                op: "forecast_assign".to_string(),
                            }),
                            Err(e) => warnings.push(Warning::StatementOpFailed {
                                node_id: node_id.as_str().to_string(),
                                op: "forecast_assign".to_string(),
                                reason: e.to_string(),
                            }),
                        }
                    }
                    _ => {}
                }
            }
        }

        // Phase 4: Re-evaluate statements only if statement work was performed.
        if applied_stmt_ops > 0 || has_rate_bindings {
            let _span = tracing::info_span!("phase_4_reevaluate").entered();
            let Some(model) = ctx.model.as_deref_mut() else {
                return Err(crate::error::Error::missing_statement_model(
                    "statement re-evaluation",
                ));
            };
            match crate::adapters::statements::reevaluate_model(model) {
                Ok(eval_warnings) => warnings.extend(eval_warnings),
                Err(e) => warnings.push(Warning::ModelReevaluationFailed {
                    reason: e.to_string(),
                }),
            }
        }

        changes.as_of_changed = ctx.as_of != initial_as_of;
        changes.all_dirty |= changes.as_of_changed;

        Ok(ApplicationReport {
            operations_applied: applied,
            user_operations,
            expanded_operations,
            changes,
            warnings,
            rounding_context: rounding_stamp(&self.config),
            time_roll,
        })
    }
}
