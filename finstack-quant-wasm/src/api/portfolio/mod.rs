//! WASM bindings for the `finstack-quant-portfolio` crate.
//!
//! Exposes portfolio spec parsing, validation, and result extraction for
//! JavaScript/TypeScript consumption.
//!
//! # Return conventions
//!
//! Two return shapes are used, and the name says which:
//!
//! - **Computation results** — valuations, aggregations, attributions,
//!   optimizations, decompositions and P&L — return **plain structured
//!   JavaScript objects** built with
//!   [`crate::utils::to_js_value`]. Read properties directly; call
//!   `JSON.stringify` when a canonical JSON string is needed (for example to
//!   chain into an entry point that still takes a `*Json` argument).
//! - **Wire / spec / validator surfaces** — anything whose purpose is to echo a
//!   canonical document for re-ingest — return a **JSON string** and their
//!   names end in `Json` (`Portfolio.toJson`, `parsePortfolioSpecJson`,
//!   `buildPortfolioFromSpecJson`, `Portfolio.validateMaterialization`).
//!
//! # Stability tiers
//!
//! The exports below fall into three stability tiers. Treat the tier as a
//! contract about how disruptive future changes are likely to be.
//!
//! **Stable** — golden-tested, signatures preserved across releases:
//! - `Portfolio` (typed handle: `fromSpec`, `toJson`, `id`, `asOf`,
//!   `baseCurrency`, `numPositions`)
//! - `parsePortfolioSpecJson`, `buildPortfolioFromSpecJson`
//! - `valuePortfolio`, `valuePortfolioBuilt`,
//!   `aggregateFullCashflows`, `aggregateFullCashflowsBuilt`,
//!   `applyScenarioAndRevalue`, `applyScenarioAndRevalueBuilt`,
//!   `scenarioPnl`, `scenarioPnlBuilt`
//! - `aggregateMetrics`, `portfolioResultTotalValue`,
//!   `portfolioResultGetMetric`
//! - `replayPortfolio`
//!
//! **Stable, JSON-shape may evolve** — function names stable, but the
//! returned / accepted JSON payload structure may grow additive
//! (non-breaking) fields between releases:
//! - `optimizePortfolio`
//!   (`PortfolioOptimizationSpec` / `PortfolioOptimizationResult` JSON)
//! - `parametricVarDecomposition`, `parametricEsDecomposition`,
//!   `historicalVarDecomposition`, `evaluateRiskBudget`
//!
//! **Experimental** — calibration parameters or signatures still under
//! review:
//! - `lvarBangia` — exogenous spread-cost model; calibration defaults live in
//!   the Rust crate's `liquidity` module.
//! - `almgrenChrissImpact` — `delta` is fixed at 0.5; the underlying
//!   `optimal_trajectory` accepts only `delta = 1` (linear impact).
//! - `kyleLambda`, `rollEffectiveSpread`, `amihudIlliquidity`,
//!   `daysToLiquidate`, `liquidityTier` — small free functions; may be
//!   re-grouped or renamed.
//!
//! For repeated calls against the same portfolio (scenario sweeps,
//! interactive dashboards), prefer the `*Built` variants which take a
//! `Portfolio` handle and skip the per-call `from_spec` rebuild.

use std::sync::Arc;

use crate::api::core::market_data::JsDiscountCurve;
use crate::utils::{to_js_err, to_js_value};
use wasm_bindgen::prelude::*;

pub mod materialization;
pub mod sensitivity;

/// Handle to a built [`finstack_quant_portfolio::Portfolio`] that can be reused
/// across WASM calls without re-parsing and rebuilding from the spec.
///
/// `Portfolio::from_spec` parses positions, builds indices, and validates
/// invariants; for pipelines that call both `valuePortfolio` and
/// `aggregateFullCashflows` on the same portfolio, holding this handle
/// avoids paying that cost twice.
#[wasm_bindgen(js_name = Portfolio)]
pub struct JsPortfolio {
    pub(crate) inner: Arc<finstack_quant_portfolio::Portfolio>,
}

#[wasm_bindgen(js_class = Portfolio)]
impl JsPortfolio {
    /// Build from a JSON-serialised `PortfolioSpec`.
    /// @param spec_json - Canonical portfolio specification JSON defining positions, quantities, and base currency.
    ///
    /// # Errors
    ///
    /// Throws a JavaScript exception if `specJson` is malformed or does not match
    /// the portfolio schema, a position has an invalid quantity or instrument
    /// specification, or portfolio validation finds duplicate identifiers or an
    /// unknown entity reference.
    #[wasm_bindgen(js_name = fromSpec)]
    pub fn from_spec(spec_json: &str) -> Result<JsPortfolio, JsValue> {
        let spec: finstack_quant_portfolio::portfolio::PortfolioSpec =
            serde_json::from_str(spec_json).map_err(to_js_err)?;
        let portfolio = finstack_quant_portfolio::Portfolio::from_spec(spec).map_err(to_js_err)?;
        Ok(Self {
            inner: Arc::new(portfolio),
        })
    }

    /// Portfolio identifier.
    #[wasm_bindgen(getter)]
    pub fn id(&self) -> String {
        self.inner.id.clone()
    }

    /// Valuation date (ISO 8601).
    #[wasm_bindgen(getter, js_name = asOf)]
    pub fn as_of(&self) -> String {
        self.inner.as_of.to_string()
    }

    /// Base currency code.
    ///
    /// Renamed from the historical `baseCcy` to the full-word camelCase
    /// convention used everywhere else (matches Python `base_currency`).
    #[wasm_bindgen(getter, js_name = baseCurrency)]
    pub fn base_currency(&self) -> String {
        self.inner.base_currency.to_string()
    }

    /// Number of positions in the portfolio.
    #[wasm_bindgen(js_name = numPositions)]
    pub fn num_positions(&self) -> usize {
        self.inner.positions().len()
    }

    /// Serialise the canonical spec back to JSON.
    ///
    /// # Errors
    ///
    /// Throws a JavaScript exception if the canonical portfolio specification
    /// cannot be serialized to JSON.
    #[wasm_bindgen(js_name = toJson)]
    pub fn to_json(&self) -> Result<String, JsValue> {
        let spec = self.inner.to_spec();
        serde_json::to_string(&spec).map_err(to_js_err)
    }
}

/// Parse and validate a portfolio specification from JSON.
///
/// Wire/validator surface: returns the re-serialized canonical JSON **string**,
/// suitable for storage or re-ingest by `Portfolio.fromSpec`.
/// @param json_str - Canonical JSON string to validate and re-serialize.
///
/// # Errors
///
/// Throws a JavaScript exception if `jsonStr` is malformed or does not match the
/// `PortfolioSpec` schema, or if the canonical form cannot be serialized.
#[wasm_bindgen(js_name = parsePortfolioSpecJson)]
pub fn parse_portfolio_spec_json(json_str: &str) -> Result<String, JsValue> {
    let spec: finstack_quant_portfolio::portfolio::PortfolioSpec =
        serde_json::from_str(json_str).map_err(to_js_err)?;

    serde_json::to_string(&spec).map_err(to_js_err)
}

/// Compute a single-period Brinson-Fachler attribution from sector JSON.
///
/// Accepts a JSON array of `SectorPeriod` objects and returns a structured
/// `BrinsonPeriodResult` object.
/// @param sectors_json - Sector-classification JSON.
///
/// # Errors
///
/// Throws a JavaScript exception if `sectorsJson` is malformed, contains no
/// sectors or a non-finite weight or return, portfolio or benchmark weights do
/// not sum to one, or the result cannot be converted to a JavaScript value.
#[wasm_bindgen(js_name = brinsonFachler)]
pub fn brinson_fachler(sectors_json: &str) -> Result<JsValue, JsValue> {
    let sectors: Vec<finstack_quant_portfolio::SectorPeriod> =
        serde_json::from_str(sectors_json).map_err(to_js_err)?;
    let result = finstack_quant_portfolio::brinson_fachler(&sectors).map_err(to_js_err)?;
    to_js_value(&result)
}

/// Compute Carino-linked multi-period Brinson attribution from period JSON.
///
/// Accepts a JSON array of periods, where each period is an array of
/// `SectorPeriod` objects, and returns a structured `CarinoLinkedAttribution`
/// object.
/// @param periods_json - Chronological period-result JSON array.
///
/// # Errors
///
/// Throws a JavaScript exception if `periodsJson` is malformed, any period fails
/// Brinson validation, the sequence is empty or changes sector ordering, a
/// period return is non-finite or at most `-1`, or the result cannot be
/// converted to a JavaScript value.
#[wasm_bindgen(js_name = carinoLink)]
pub fn carino_link(periods_json: &str) -> Result<JsValue, JsValue> {
    let periods: Vec<Vec<finstack_quant_portfolio::SectorPeriod>> =
        serde_json::from_str(periods_json).map_err(to_js_err)?;
    let result =
        finstack_quant_portfolio::carino_link_from_sector_periods(&periods).map_err(to_js_err)?;
    to_js_value(&result)
}

/// Compute a single-period Campisi fixed-income attribution from JSON.
///
/// Decomposes both sides into carry / treasury / spread / selection and
/// splits the active return into allocation plus four active component
/// effects (Campisi 2000). Returns a structured `FiAttributionResult` object;
/// `JSON.stringify` it to chain into `campisiCarinoLink` or
/// `campisiReconciliationCheck`.
///
/// Every snapshot must use the quote-reproducing `z_spread` basis:
/// `spread_duration` is the canonical Z-spread duration, and `spread` plus
/// `delta_spread` are the matching Z-spread level and move. OAS, G-spread, or
/// discount-margin values are incompatible. The direct JSON shape contains
/// numeric values but no metric IDs, so this binding cannot detect mislabeled
/// spread provenance.
///
/// Throws when JSON is malformed or canonical Rust validation rejects empty
/// sides, non-finite values, invalid weights or period length, or a sector
/// present on either side has `|net sector weight| <= 1e-6 * gross absolute
/// sector weight`. Spread-basis provenance cannot be validated from numeric
/// JSON alone.
/// @param portfolio_json - Canonical JSON array of `FiPositionSnapshot` objects describing the portfolio side on the quote-reproducing Z-spread basis; weights must sum to 1.
/// @param benchmark_json - Canonical JSON array of `FiPositionSnapshot` objects describing the benchmark side on the quote-reproducing Z-spread basis; weights must sum to 1.
/// @param config_json - Canonical JSON `FiAttributionConfig`; `period_years` is its only field, is required (no default), and unknown keys are rejected.
///
/// # Errors
///
/// Throws a JavaScript exception if any JSON input is malformed; either side is
/// empty; a value is non-finite; weights do not sum to one; `periodYears` is not
/// finite and positive; a sector has a zero or near-zero net weight relative to
/// gross weight; or the result cannot be converted to a JavaScript value.
#[wasm_bindgen(js_name = campisiAttribution)]
pub fn campisi_attribution(
    portfolio_json: &str,
    benchmark_json: &str,
    config_json: &str,
) -> Result<JsValue, JsValue> {
    let portfolio: Vec<finstack_quant_portfolio::FiPositionSnapshot> =
        serde_json::from_str(portfolio_json).map_err(to_js_err)?;
    let benchmark: Vec<finstack_quant_portfolio::FiPositionSnapshot> =
        serde_json::from_str(benchmark_json).map_err(to_js_err)?;
    let config: finstack_quant_portfolio::FiAttributionConfig =
        serde_json::from_str(config_json).map_err(to_js_err)?;
    let result = finstack_quant_portfolio::campisi_attribution(&portfolio, &benchmark, &config)
        .map_err(to_js_err)?;
    to_js_value(&result)
}

/// Carino-link already-computed single-period Campisi results.
///
/// Binds Rust `campisi_carino_link`. Each period carries its own
/// already-applied `period_years`, so periods of *different* lengths (e.g.
/// act/365 calendar months) link correctly here; prefer this entry point
/// whenever the periods are not all the same length. Returns a structured
/// `FiCarinoLinkedResult` object.
///
/// Throws if no periods are supplied, sector ordering differs, a consumed
/// top-level return/effect, per-sector linked effect, or sector `total_active`
/// is non-finite, `active_return` disagrees with the portfolio-minus-benchmark
/// return, a sector `total_active` disagrees with its five effects, sector
/// effects do not reconcile to their declared top-level totals, the five
/// totals do not reconcile to `active_return` within the overflow-safe
/// scaled-L1 tolerance, a reconciliation residual is non-finite, or a return
/// is outside the Carino domain.
/// @param periods_json - Canonical JSON array of `FiAttributionResult` objects in chronological order, as returned by `campisiAttribution`.
///
/// # Errors
///
/// Throws a JavaScript exception if `periodsJson` is malformed, the sequence is
/// empty or changes sector ordering, a consumed value or reconciliation is
/// non-finite or inconsistent, a return is at most `-1`, or the linked result
/// cannot be converted to a JavaScript value.
#[wasm_bindgen(js_name = campisiCarinoLink)]
pub fn campisi_carino_link(periods_json: &str) -> Result<JsValue, JsValue> {
    let periods: Vec<finstack_quant_portfolio::FiAttributionResult> =
        serde_json::from_str(periods_json).map_err(to_js_err)?;
    let result = finstack_quant_portfolio::campisi_carino_link(&periods).map_err(to_js_err)?;
    to_js_value(&result)
}

/// Compute per-period Campisi attributions from snapshots and Carino-link them.
///
/// Binds Rust `campisi_carino_link_from_snapshots`. One shared config — hence
/// one shared `period_years` — is applied to every period, so this entry point
/// is only correct for equal-length periods; use `campisiCarinoLink` for
/// unequal periods. Returns a structured `FiCarinoLinkedResult` object.
/// @param periods_json - Canonical JSON array of `FiPeriodInput` objects, each holding `portfolio` and `benchmark` arrays of `FiPositionSnapshot`.
/// @param config_json - Canonical JSON `FiAttributionConfig` applied to every period; `period_years` is its only field and is required (no default).
///
/// # Errors
///
/// Throws a JavaScript exception if either JSON input is malformed, any period
/// fails Campisi attribution validation, the computed periods fail Carino
/// linking validation, or the result cannot be converted to a JavaScript value.
#[wasm_bindgen(js_name = campisiCarinoLinkFromSnapshots)]
pub fn campisi_carino_link_from_snapshots(
    periods_json: &str,
    config_json: &str,
) -> Result<JsValue, JsValue> {
    let periods: Vec<finstack_quant_portfolio::FiPeriodInput> =
        serde_json::from_str(periods_json).map_err(to_js_err)?;
    let config: finstack_quant_portfolio::FiAttributionConfig =
        serde_json::from_str(config_json).map_err(to_js_err)?;
    let result = finstack_quant_portfolio::campisi_carino_link_from_snapshots(&periods, &config)
        .map_err(to_js_err)?;
    to_js_value(&result)
}

/// Reconcile the five Campisi effect totals against the active return.
///
/// Binds the Rust method `FiAttributionResult::reconciliation_check`. The
/// decomposition reconciles by construction (selection is the residual), so
/// this is a floating-point sanity gate rather than a model check; without it
/// callers must re-sum the five totals by hand. Returns a structured
/// `FiReconciliationReport` object with `total_residual`, `is_reconciled` and
/// `tolerance`.
/// @param result_json - Canonical JSON `FiAttributionResult` as returned by `campisiAttribution` (`JSON.stringify` its structured result); unknown fields are rejected.
/// @param tolerance - Absolute reconciliation tolerance in return units; `1e-10` suits return-space values.
///
/// # Errors
///
/// Throws a JavaScript exception if `resultJson` is malformed or does not match
/// `FiAttributionResult`, or if the reconciliation report cannot be converted to
/// a JavaScript value.
#[wasm_bindgen(js_name = campisiReconciliationCheck)]
pub fn campisi_reconciliation_check(result_json: &str, tolerance: f64) -> Result<JsValue, JsValue> {
    let result: finstack_quant_portfolio::FiAttributionResult =
        serde_json::from_str(result_json).map_err(to_js_err)?;
    to_js_value(&result.reconciliation_check(tolerance))
}

/// Build a duration-cell base-return table from a reference universe.
///
/// Binds Rust `cell_returns_from_reference` (Dynkin, Hyman & Vankudre 1998,
/// Appendix B): buckets `referenceJson` into fixed-width duration cells and
/// averages each cell's member total returns, interpolating interior gaps
/// and flat-extrapolating leading/trailing gaps. Returns a structured
/// `DurationCellTable` object; `JSON.stringify` it to chain into
/// `excessReturns`.
/// @param referenceJson - Canonical JSON array of `ReferenceReturn` objects (`duration`, `total_return`, both decimals with duration in years); must be non-empty.
/// @param baseLabel - Label identifying the resulting curve (e.g. `"UST"`), carried through to the output's `base_label` for policy visibility.
/// @param configJson - Canonical JSON `CellConfig`; `width` is its only field (cell width in years, finite and positive) and is required, with no default.
///
/// # Errors
///
/// Throws a JavaScript exception if either JSON input is malformed, the
/// reference universe is empty or contains an invalid duration or return, the
/// cell width is not finite and positive, labels collide, the grid exceeds its
/// safety bound, or the result cannot be converted to a JavaScript value.
#[wasm_bindgen(js_name = cellReturnsFromReference)]
pub fn cell_returns_from_reference(
    reference_json: &str,
    base_label: &str,
    config_json: &str,
) -> Result<JsValue, JsValue> {
    let reference: Vec<finstack_quant_portfolio::ReferenceReturn> =
        serde_json::from_str(reference_json).map_err(to_js_err)?;
    let config: finstack_quant_portfolio::CellConfig =
        serde_json::from_str(config_json).map_err(to_js_err)?;
    let table =
        finstack_quant_portfolio::cell_returns_from_reference(&reference, base_label, &config)
            .map_err(to_js_err)?;
    to_js_value(&table)
}

/// Build a duration-cell base-return table from start/end discount curves.
///
/// Binds Rust `cell_returns_from_curves`: each cell's base return is the
/// holding-period return of a hypothetical zero-coupon position bought at
/// the cell midpoint off `start` and revalued off `end` after
/// `horizonYears` have elapsed. Every resulting cell is observed, unlike the
/// reference-universe path in `cellReturnsFromReference`. Returns a structured
/// `DurationCellTable` object; `JSON.stringify` it to chain into
/// `excessReturns`.
/// @param start - Discount curve observed at the start of the holding period.
/// @param end - Discount curve observed `horizonYears` later, at period end.
/// @param horizonYears - Length of the holding period, in years; must be finite and positive.
/// @param maxDuration - Upper bound of the duration grid, in years; must be finite and strictly greater than `horizonYears`.
/// @param baseLabel - Label identifying the base curve (e.g. `"UST"`, `"USD-SOFR"`), stamped into the result purely for policy visibility.
/// @param configJson - Canonical JSON `CellConfig`; `width` is its only field and is required, with no default.
///
/// # Errors
///
/// Throws a JavaScript exception if `configJson` is malformed; the width,
/// horizon, or maximum duration is invalid; a cell matures within the holding
/// period; the grid is too large or has duplicate labels; a required discount
/// factor is not finite and positive; or the result cannot be converted to a
/// JavaScript value.
#[wasm_bindgen(js_name = cellReturnsFromCurves)]
pub fn cell_returns_from_curves(
    start: &JsDiscountCurve,
    end: &JsDiscountCurve,
    horizon_years: f64,
    max_duration: f64,
    base_label: &str,
    config_json: &str,
) -> Result<JsValue, JsValue> {
    let config: finstack_quant_portfolio::CellConfig =
        serde_json::from_str(config_json).map_err(to_js_err)?;
    let table = finstack_quant_portfolio::cell_returns_from_curves(
        &start.inner,
        &end.inner,
        horizon_years,
        max_duration,
        base_label,
        &config,
    )
    .map_err(to_js_err)?;
    to_js_value(&table)
}

/// Compute duration-matched credit excess returns against a base-return table.
///
/// Binds Rust `excess_returns` (Dynkin, Hyman & Vankudre 1998, Appendix B):
/// each position's `duration` is matched to its duration cell in
/// `tableJson` and the position's excess return is `total_return -
/// cell.base_return`, the credit-specific component of performance isolated
/// from the general level/shape move of the base curve. Returns a structured
/// `ExcessReturnResult` object with per-position and portfolio-level totals.
///
/// @param positionsJson - Canonical JSON array of `ExcessReturnPosition` objects (`id`, `weight`, `duration`, `total_return`); weights must sum to 1.
/// @param tableJson - Canonical JSON `DurationCellTable`; `JSON.stringify` the structured table returned by `cellReturnsFromReference` or `cellReturnsFromCurves`.
///
/// # Errors
///
/// Throws a JavaScript exception if either JSON input is malformed, the cell
/// table is invalid, a position is invalid or falls in no cell, position
/// weights do not sum to one, or the result cannot be converted to a JavaScript
/// value.
#[wasm_bindgen(js_name = excessReturns)]
pub fn excess_returns(positions_json: &str, table_json: &str) -> Result<JsValue, JsValue> {
    let positions: Vec<finstack_quant_portfolio::ExcessReturnPosition> =
        serde_json::from_str(positions_json).map_err(to_js_err)?;
    let table: finstack_quant_portfolio::DurationCellTable =
        serde_json::from_str(table_json).map_err(to_js_err)?;
    let result = finstack_quant_portfolio::excess_returns(&positions, &table).map_err(to_js_err)?;
    to_js_value(&result)
}

/// Compute a single-period hierarchical duration-cell x sector grid attribution.
///
/// Binds Rust `grid_attribution` (Dynkin, Hyman & Vankudre 1998, Appendix A):
/// decomposes active return into a per-cell curve (positioning) effect, a
/// within-cell sector allocation effect, and a security-selection residual
/// per (cell, sector). Returns a structured `GridAttributionResult` object
/// (`JSON.stringify` it to chain into `gridCarinoLink`) whose
/// `total_curve`, `total_sector` and `total_selection` sum to
/// `active_return` to floating-point precision for well-conditioned inputs;
/// among accepted inputs, the reconciliation residual grows the closer any
/// bucket's net weight sits to the near-zero-net-weight rejection boundary
/// (see the Rust module docs for measured magnitudes).
/// @param portfolioJson - Canonical JSON array of `GridPosition` objects (`cell`, `sector`, `weight`, `total_return`) for the portfolio side; weights must sum to 1.
/// @param benchmarkJson - Canonical JSON array of `GridPosition` objects for the benchmark side; same weight-sum requirement.
///
/// # Errors
///
/// Throws a JavaScript exception if either JSON input is malformed, a weight or
/// return is non-finite, either side's weights do not sum to one, a cell or
/// cell-sector bucket has a zero or near-zero net weight relative to gross
/// weight, or the result cannot be converted to a JavaScript value.
#[wasm_bindgen(js_name = gridAttribution)]
pub fn grid_attribution(portfolio_json: &str, benchmark_json: &str) -> Result<JsValue, JsValue> {
    let portfolio: Vec<finstack_quant_portfolio::GridPosition> =
        serde_json::from_str(portfolio_json).map_err(to_js_err)?;
    let benchmark: Vec<finstack_quant_portfolio::GridPosition> =
        serde_json::from_str(benchmark_json).map_err(to_js_err)?;
    let result =
        finstack_quant_portfolio::grid_attribution(&portfolio, &benchmark).map_err(to_js_err)?;
    to_js_value(&result)
}

/// Carino-link multi-period hierarchical grid attribution results.
///
/// Binds Rust `grid_carino_link` (Carino 1999): applies Carino smoothing to
/// a chronological sequence of single-period `gridAttribution` results so
/// the three top-level effects (`linked_curve`, `linked_sector`,
/// `linked_selection`) sum exactly to the geometrically compounded active
/// return. Only the three top-level effects are linked; per-cell /
/// per-(cell, sector) multi-period linking is out of scope. Returns a structured
/// `GridCarinoLinkedResult` object.
/// @param periodsJson - Canonical JSON array of `GridAttributionResult` objects, in chronological order; `JSON.stringify` the structured results returned by `gridAttribution`.
///
/// # Errors
///
/// Throws a JavaScript exception if `periodsJson` is malformed, the sequence is
/// empty, a consumed value is non-finite or inconsistent, a return is at most
/// `-1`, or the linked result cannot be converted to a JavaScript value.
#[wasm_bindgen(js_name = gridCarinoLink)]
pub fn grid_carino_link(periods_json: &str) -> Result<JsValue, JsValue> {
    let periods: Vec<finstack_quant_portfolio::GridAttributionResult> =
        serde_json::from_str(periods_json).map_err(to_js_err)?;
    let result = finstack_quant_portfolio::grid_carino_link(&periods).map_err(to_js_err)?;
    to_js_value(&result)
}

/// Compute Jeet-Partani (2023) factor-Brinson unified attribution.
///
/// Binds Rust `factor_brinson_attribution`: generalizes classical
/// Brinson-Fachler allocation/selection to continuous factor exposures by
/// replacing the sector partition with a factor-exposure matrix and a
/// caller-supplied benchmark factor-return vector. Returns a structured
/// `FactorBrinsonResult` object with `allocation`, `selection`, and their
/// per-factor / per-asset breakdowns.
/// @param inputJson - Canonical JSON `FactorBrinsonInput` with `asset_ids`, `asset_returns`, `exposures` (row-major n_assets x n_factors), `factor_names`, `portfolio_weights` and `benchmark_weights`; each weight vector must sum to 1.
/// @param factorReturns - Caller-supplied benchmark factor returns `f_b` as a `number[]` or `Float64Array`, length `input.factor_names`; the `Float64Array` returned by `analytics.constrainedLeastSquares` can be passed directly.
///
/// # Errors
///
/// Throws a JavaScript exception if `inputJson` is malformed; the asset or
/// factor sets are empty; dimensions disagree; a value is non-finite; either
/// weight vector does not sum to one; benchmark factor completeness is outside
/// tolerance; or the result cannot be converted to a JavaScript value.
#[wasm_bindgen(js_name = factorBrinsonAttribution)]
pub fn factor_brinson_attribution(
    input_json: &str,
    factor_returns: Vec<f64>,
) -> Result<JsValue, JsValue> {
    let input: finstack_quant_portfolio::FactorBrinsonInput =
        serde_json::from_str(input_json).map_err(to_js_err)?;
    let result = finstack_quant_portfolio::factor_brinson_attribution(&input, &factor_returns)
        .map_err(to_js_err)?;
    to_js_value(&result)
}

/// Compute a Modified-Dietz TWRR sub-period return from period JSON.
/// @param period_json - Single-period result JSON.
///
/// # Errors
///
/// Throws a JavaScript exception if `periodJson` is malformed, does not match
/// the expected period schema, or the return is undefined (non-positive
/// adjusted denominator, out-of-range cashflow weight, non-finite inputs).
#[wasm_bindgen(js_name = twrrModifiedDietz)]
pub fn twrr_modified_dietz(period_json: &str) -> Result<f64, JsValue> {
    let period: finstack_quant_portfolio::TwrrPeriod =
        serde_json::from_str(period_json).map_err(to_js_err)?;
    finstack_quant_portfolio::twrr_modified_dietz(&period).map_err(to_js_err)
}

/// Geometrically link TWRR sub-period returns from returns JSON.
/// @param returns_json - Numeric return-series JSON.
/// @param horizon_years - Return-linking horizon measured in years for annualization.
///
/// # Errors
///
/// Throws a JavaScript exception if `returnsJson` is malformed, the return
/// series is invalid (non-finite sub-period return, non-positive compounded
/// growth factor), or the linked result cannot be converted to a JavaScript
/// value.
#[wasm_bindgen(js_name = twrrLinked)]
pub fn twrr_linked(returns_json: &str, horizon_years: f64) -> Result<JsValue, JsValue> {
    let returns: Vec<f64> = serde_json::from_str(returns_json).map_err(to_js_err)?;
    let result =
        finstack_quant_portfolio::twrr_linked(&returns, horizon_years).map_err(to_js_err)?;
    to_js_value(&result)
}

/// Compute money-weighted return via XIRR from dated cashflow JSON.
/// @param cashflows_json - Dated cashflow JSON.
///
/// # Errors
///
/// Throws a JavaScript exception if `cashflowsJson` is malformed, contains an
/// invalid date or insufficient cash flows for XIRR, or the numerical root
/// cannot be found.
#[wasm_bindgen(js_name = mwrXirr)]
pub fn mwr_xirr(cashflows_json: &str) -> Result<f64, JsValue> {
    let cashflows: Vec<finstack_quant_portfolio::DatedCashflow> =
        serde_json::from_str(cashflows_json).map_err(to_js_err)?;
    finstack_quant_portfolio::mwr_xirr_from_cashflows(&cashflows).map_err(to_js_err)
}

/// Build a runtime portfolio from a JSON spec, validate, and round-trip.
///
/// Wire/validator surface: deserializes the spec, constructs the portfolio with
/// live instruments, validates structural invariants, then re-serializes the
/// canonical JSON **string** for confirmation or re-ingest.
/// @param spec_json - Canonical portfolio specification JSON defining positions, quantities, and base currency.
///
/// # Errors
///
/// Throws a JavaScript exception if `specJson` is malformed or violates the
/// portfolio schema, a position has an invalid quantity or instrument
/// specification, portfolio validation fails, or the round-trip form cannot be
/// serialized.
#[wasm_bindgen(js_name = buildPortfolioFromSpecJson)]
pub fn build_portfolio_from_spec_json(spec_json: &str) -> Result<String, JsValue> {
    let spec: finstack_quant_portfolio::portfolio::PortfolioSpec =
        serde_json::from_str(spec_json).map_err(to_js_err)?;

    let portfolio = finstack_quant_portfolio::Portfolio::from_spec(spec).map_err(to_js_err)?;

    let round_tripped = portfolio.to_spec();
    serde_json::to_string(&round_tripped).map_err(to_js_err)
}

/// Extract the total portfolio value from a JSON result.
/// @param result_json - Result JSON produced by a prior call.
///
/// # Errors
///
/// Throws a JavaScript exception if `resultJson` is malformed or does not match
/// the `PortfolioResult` schema.
#[wasm_bindgen(js_name = portfolioResultTotalValue)]
pub fn portfolio_result_total_value(result_json: &str) -> Result<f64, JsValue> {
    let result: finstack_quant_portfolio::results::PortfolioResult =
        serde_json::from_str(result_json).map_err(to_js_err)?;

    Ok(result.total_value().amount())
}

/// Extract a specific metric from a portfolio result JSON.
///
/// Returns `undefined` (via `Option`) if the metric was not produced.
/// @param result_json - Result JSON produced by a prior call.
/// @param metric_id - Stable metric identifier used to select the required domain object.
///
/// # Errors
///
/// Throws a JavaScript exception if `resultJson` is malformed or does not match
/// the `PortfolioResult` schema. An absent `metricId` returns `undefined`.
#[wasm_bindgen(js_name = portfolioResultGetMetric)]
pub fn portfolio_result_get_metric(
    result_json: &str,
    metric_id: &str,
) -> Result<Option<f64>, JsValue> {
    let result: finstack_quant_portfolio::results::PortfolioResult =
        serde_json::from_str(result_json).map_err(to_js_err)?;

    Ok(result.get_metric(metric_id))
}

/// Aggregate portfolio metrics from a valuation JSON.
/// @param valuation_json - Portfolio or instrument valuation JSON.
/// @param base_currency - ISO-4217 base currency in which aggregate portfolio values are reported.
/// @param market_json - Canonical market-context JSON supplying curves, quotes, and FX data.
/// @param as_of - ISO-8601 valuation date used to resolve date-dependent market data.
///
/// # Errors
///
/// Throws a JavaScript exception if either JSON input is malformed,
/// `baseCurrency` or `asOf` is invalid, valuation currency or date metadata is
/// inconsistent, a required FX conversion is unavailable or invalid, or the
/// metrics cannot be converted to a JavaScript value.
#[wasm_bindgen(js_name = aggregateMetrics)]
pub fn aggregate_metrics(
    valuation_json: &str,
    base_currency: &str,
    market_json: &str,
    as_of: &str,
) -> Result<JsValue, JsValue> {
    let valuation: finstack_quant_portfolio::valuation::PortfolioValuation =
        serde_json::from_str(valuation_json).map_err(to_js_err)?;
    let ccy: finstack_quant_core::currency::Currency = base_currency.parse().map_err(to_js_err)?;
    let market: finstack_quant_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let format = time::format_description::well_known::Iso8601::DEFAULT;
    let date = time::Date::parse(as_of, &format).map_err(to_js_err)?;
    let metrics =
        finstack_quant_portfolio::metrics::aggregate_metrics(&valuation, ccy, &market, date)
            .map_err(to_js_err)?;
    to_js_value(&metrics)
}

/// Value a portfolio from its spec and market context.
/// @param spec_json - Canonical portfolio specification JSON defining positions, quantities, and base currency.
/// @param market_json - Canonical market-context JSON supplying curves, quotes, and FX data.
/// @param strict_risk - Optional; when omitted or `undefined`, defaults to
///   `true` (fail closed on unavailable requested risk metrics), matching
///   Rust `PortfolioValuationOptions`. Pass `false` only for an intentional
///   PV-preserving fallback.
/// @param metrics - Optional exact risk-metric ids to compute. Omit for the
///   standard set; an empty array performs PV-only valuation. Names are
///   validated strictly against the standard `MetricId` set — an unknown
///   name throws. Mirrors the Python `metrics=` keyword.
///
/// # Errors
///
/// Throws a JavaScript exception if the portfolio or market JSON is malformed,
/// a requested metric name is unknown, portfolio construction or valuation
/// fails, strict risk calculation cannot produce a requested metric, a
/// required FX conversion is unavailable, or the valuation cannot be
/// converted to a JavaScript value.
#[wasm_bindgen(js_name = valuePortfolio)]
pub fn value_portfolio(
    spec_json: &str,
    market_json: &str,
    strict_risk: Option<bool>,
    metrics: Option<Vec<String>>,
) -> Result<JsValue, JsValue> {
    let portfolio = JsPortfolio::from_spec(spec_json)?;
    value_portfolio_built(&portfolio, market_json, strict_risk, metrics)
}

/// Aggregate the full classified cashflow ladder for a portfolio.
/// @param spec_json - Canonical portfolio specification JSON defining positions, quantities, and base currency.
/// @param market_json - Canonical market-context JSON supplying curves, quotes, and FX data.
/// @param allow_partial - Optional; when omitted or `undefined`, defaults to
///   `false` (fail closed if any position fails schedule construction).
///   Pass `true` to keep a partial ladder with issues on the result.
///
/// # Errors
///
/// Throws a JavaScript exception if the portfolio or market JSON is malformed,
/// portfolio construction fails, any position fails schedule construction
/// while `allowPartial` is not `true`, monetary cash-flow aggregation
/// overflows, or the aggregate cannot be converted to a JavaScript value.
#[wasm_bindgen(js_name = aggregateFullCashflows)]
pub fn aggregate_full_cashflows(
    spec_json: &str,
    market_json: &str,
    allow_partial: Option<bool>,
) -> Result<JsValue, JsValue> {
    let portfolio = JsPortfolio::from_spec(spec_json)?;
    aggregate_full_cashflows_built(&portfolio, market_json, allow_partial)
}

/// Aggregate the full classified cashflow ladder for an already-built
/// [`JsPortfolio`] handle.
///
/// Skips the per-call `PortfolioSpec` parse + `Portfolio::from_spec` rebuild.
/// For batched or chained workflows (repeated cashflow builds across market
/// scenarios on the same portfolio), this is the cheap path.
/// @param portfolio - Built portfolio object whose positions and weights are used by the calculation.
/// @param market_json - Canonical market-context JSON supplying curves, quotes, and FX data.
/// @param allow_partial - Optional; when omitted or `undefined`, defaults to
///   `false` (fail closed if any position fails schedule construction).
///   Pass `true` to keep a partial ladder with issues on the result.
///
/// # Errors
///
/// Throws a JavaScript exception if `marketJson` is malformed, any position
/// fails schedule construction while `allowPartial` is not `true`, monetary
/// cash-flow aggregation overflows, or the aggregate cannot be converted to
/// a JavaScript value.
#[wasm_bindgen(js_name = aggregateFullCashflowsBuilt)]
pub fn aggregate_full_cashflows_built(
    portfolio: &JsPortfolio,
    market_json: &str,
    allow_partial: Option<bool>,
) -> Result<JsValue, JsValue> {
    let market: finstack_quant_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let options = finstack_quant_portfolio::cashflows::CashflowAggregationOptions {
        allow_partial: allow_partial.unwrap_or(false),
    };
    let cashflows = finstack_quant_portfolio::cashflows::aggregate_full_cashflows(
        &portfolio.inner,
        &market,
        &options,
    )
    .map_err(to_js_err)?;
    to_js_value(&cashflows)
}

/// Value an already-built [`JsPortfolio`] handle. Skips the per-call
/// `PortfolioSpec` parse + `Portfolio::from_spec` rebuild that
/// [`value_portfolio`] performs; use this when sweeping market scenarios
/// against a fixed portfolio.
/// @param portfolio - Built portfolio object whose positions and weights are used by the calculation.
/// @param market_json - Canonical market-context JSON supplying curves, quotes, and FX data.
/// @param strict_risk - Optional; when omitted or `undefined`, defaults to
///   `true` (fail closed on unavailable requested risk metrics), matching
///   Rust `PortfolioValuationOptions`. Pass `false` only for an intentional
///   PV-preserving fallback.
/// @param metrics - Optional exact risk-metric ids to compute. Omit for the
///   standard set; an empty array performs PV-only valuation. Names are
///   validated strictly against the standard `MetricId` set — an unknown
///   name throws instead of silently degrading to PV-only valuation.
///   Mirrors the Python `metrics=` keyword.
///
/// # Errors
///
/// Throws a JavaScript exception if `marketJson` is malformed, a requested
/// metric name is unknown, portfolio valuation fails, strict risk calculation
/// cannot produce a requested metric, a required FX conversion is unavailable,
/// or the valuation cannot be converted to a JavaScript value.
#[wasm_bindgen(js_name = valuePortfolioBuilt)]
pub fn value_portfolio_built(
    portfolio: &JsPortfolio,
    market_json: &str,
    strict_risk: Option<bool>,
    metrics: Option<Vec<String>>,
) -> Result<JsValue, JsValue> {
    let market: finstack_quant_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let config = finstack_quant_core::config::FinstackConfig::default();
    // Strict parsing via the canonical portfolio-crate helper (shared with
    // the Python binding): an unknown metric name throws instead of silently
    // degrading to PV-only valuation.
    let options = finstack_quant_portfolio::valuation::PortfolioValuationOptions {
        strict_risk: strict_risk.unwrap_or(true),
        metrics: finstack_quant_portfolio::valuation::RequestedMetrics::try_from_metric_names(
            metrics,
        )
        .map_err(to_js_err)?,
    };
    let valuation = finstack_quant_portfolio::valuation::value_portfolio(
        &portfolio.inner,
        &market,
        &config,
        &options,
    )
    .map_err(to_js_err)?;
    to_js_value(&valuation)
}

/// Apply a scenario to an already-built [`JsPortfolio`] handle and revalue.
/// Returns a JS object with structured `valuation` and `report` values.
/// @param portfolio - Built portfolio object whose positions and weights are used by the calculation.
/// @param scenario_json - Scenario specification JSON.
/// @param market_json - Canonical market-context JSON supplying curves, quotes, and FX data.
///
/// # Errors
///
/// Throws a JavaScript exception if the scenario or market JSON is malformed,
/// scenario application or portfolio revaluation fails, or the structured
/// result cannot be converted to a JavaScript value.
#[wasm_bindgen(js_name = applyScenarioAndRevalueBuilt)]
pub fn apply_scenario_and_revalue_built(
    portfolio: &JsPortfolio,
    scenario_json: &str,
    market_json: &str,
) -> Result<JsValue, JsValue> {
    let scenario: finstack_quant_scenarios::ScenarioSpec =
        serde_json::from_str(scenario_json).map_err(to_js_err)?;
    let market: finstack_quant_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let config = finstack_quant_core::config::FinstackConfig::default();
    let out = finstack_quant_portfolio::scenarios::apply_and_revalue_view(
        &portfolio.inner,
        &scenario,
        &market,
        &config,
    )
    .map_err(to_js_err)?;
    to_js_value(&out)
}

/// Apply a scenario to a portfolio and revalue.
///
/// Returns a JS object with structured `valuation` and `report` values.
/// @param spec_json - Canonical portfolio specification JSON defining positions, quantities, and base currency.
/// @param scenario_json - Scenario specification JSON.
/// @param market_json - Canonical market-context JSON supplying curves, quotes, and FX data.
///
/// # Errors
///
/// Throws a JavaScript exception if the portfolio, scenario, or market JSON is
/// malformed; portfolio construction, scenario application, or revaluation
/// fails; or the structured result cannot be converted to a JavaScript value.
#[wasm_bindgen(js_name = applyScenarioAndRevalue)]
pub fn apply_scenario_and_revalue(
    spec_json: &str,
    scenario_json: &str,
    market_json: &str,
) -> Result<JsValue, JsValue> {
    let portfolio = JsPortfolio::from_spec(spec_json)?;
    apply_scenario_and_revalue_built(&portfolio, scenario_json, market_json)
}

/// Compute the profit and loss attributable to a scenario for an already-built
/// [`JsPortfolio`] handle.
///
/// Values the portfolio against the unshocked market and against the
/// scenario-shocked market, and returns a JS object with structured `pnl`
/// (base-currency `total` plus `by_position`) and `report` values. Positions
/// added or removed by the scenario are zero-filled against the missing side,
/// so the drill-down always sums to the total.
/// @param portfolio - Built portfolio object whose positions and weights are used by the calculation.
/// @param scenario_json - Canonical JSON payload representing the scenario whose profit-and-loss impact is measured.
/// @param market_json - Canonical market-context JSON supplying the unshocked curves, quotes, and FX data used for the base leg.
///
/// # Errors
///
/// Throws a JavaScript exception if the scenario or market JSON is malformed,
/// scenario application or either valuation fails, valuation currencies are
/// inconsistent, or the structured result cannot be converted to JavaScript.
#[wasm_bindgen(js_name = scenarioPnlBuilt)]
pub fn scenario_pnl_built(
    portfolio: &JsPortfolio,
    scenario_json: &str,
    market_json: &str,
) -> Result<JsValue, JsValue> {
    let scenario: finstack_quant_scenarios::ScenarioSpec =
        serde_json::from_str(scenario_json).map_err(to_js_err)?;
    let market: finstack_quant_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let config = finstack_quant_core::config::FinstackConfig::default();
    let out = finstack_quant_portfolio::scenarios::scenario_pnl_view(
        &portfolio.inner,
        &scenario,
        &market,
        &config,
    )
    .map_err(to_js_err)?;
    to_js_value(&out)
}

/// Compute the profit and loss attributable to a scenario.
///
/// Values the portfolio against the unshocked market and against the
/// scenario-shocked market, and returns a JS object with structured `pnl`
/// (base-currency `total` plus `by_position`) and `report` values.
/// @param spec_json - Canonical portfolio specification JSON defining positions, quantities, and base currency.
/// @param scenario_json - Canonical JSON payload representing the scenario whose profit-and-loss impact is measured.
/// @param market_json - Canonical market-context JSON supplying the unshocked curves, quotes, and FX data used for the base leg.
///
/// # Errors
///
/// Throws a JavaScript exception if the portfolio, scenario, or market JSON is
/// malformed; portfolio construction, scenario application, or either
/// valuation fails; valuation currencies are inconsistent; or the structured
/// result cannot be converted to JavaScript.
#[wasm_bindgen(js_name = scenarioPnl)]
pub fn scenario_pnl(
    spec_json: &str,
    scenario_json: &str,
    market_json: &str,
) -> Result<JsValue, JsValue> {
    let portfolio = JsPortfolio::from_spec(spec_json)?;
    scenario_pnl_built(&portfolio, scenario_json, market_json)
}

/// Optimize portfolio weights using the LP-based optimizer.
///
/// Accepts a `PortfolioOptimizationSpec` JSON (portfolio + objective +
/// constraints + options) and a `MarketContext` JSON, and returns a structured
/// `PortfolioOptimizationResult` object.
/// @param spec_json - Canonical portfolio specification JSON defining positions, quantities, and base currency.
/// @param market_json - Canonical market-context JSON supplying curves, quotes, and FX data.
///
/// # Errors
///
/// Throws a JavaScript exception if either JSON input is malformed, the
/// portfolio, objective, constraints, weighting, or missing-metric policy is
/// invalid, a required market-dependent valuation fails, the solver cannot
/// produce a result, or the result cannot be converted to a JavaScript value.
#[wasm_bindgen(js_name = optimizePortfolio)]
pub fn optimize_portfolio(spec_json: &str, market_json: &str) -> Result<JsValue, JsValue> {
    let spec: finstack_quant_portfolio::optimization::PortfolioOptimizationSpec =
        serde_json::from_str(spec_json).map_err(to_js_err)?;
    let market: finstack_quant_core::market_data::context::MarketContext =
        serde_json::from_str(market_json).map_err(to_js_err)?;
    let config = finstack_quant_core::config::FinstackConfig::default();
    let result =
        finstack_quant_portfolio::optimization::optimize_from_spec(&spec, &market, &config)
            .map_err(to_js_err)?;
    to_js_value(&result)
}

/// Replay a portfolio through dated market snapshots.
///
/// Accepts a portfolio spec, an array of dated market snapshots, and a
/// replay configuration. Returns a structured `ReplayResult` object.
/// @param spec_json - Canonical portfolio specification JSON defining positions, quantities, and base currency.
/// @param snapshots_json - Market-snapshot JSON array.
/// @param config_json - Configuration JSON for this call.
///
/// # Errors
///
/// Throws a JavaScript exception if any JSON input is malformed; the portfolio,
/// replay configuration, or snapshot dates and ordering are invalid; valuation,
/// attribution, or currency conversion fails; best-effort replay retains no
/// step; or the result cannot be converted to a JavaScript value.
#[wasm_bindgen(js_name = replayPortfolio)]
pub fn replay_portfolio(
    spec_json: &str,
    snapshots_json: &str,
    config_json: &str,
) -> Result<JsValue, JsValue> {
    let spec: finstack_quant_portfolio::portfolio::PortfolioSpec =
        serde_json::from_str(spec_json).map_err(to_js_err)?;
    let portfolio = finstack_quant_portfolio::Portfolio::from_spec(spec).map_err(to_js_err)?;
    let config: finstack_quant_portfolio::replay::ReplayConfig =
        serde_json::from_str(config_json).map_err(to_js_err)?;
    let timeline =
        finstack_quant_portfolio::replay::ReplayTimeline::from_json_snapshots(snapshots_json)
            .map_err(to_js_err)?;
    let finstack_config = finstack_quant_core::config::FinstackConfig::default();
    let result = finstack_quant_portfolio::replay::replay_portfolio(
        &portfolio,
        &timeline,
        &config,
        &finstack_config,
    )
    .map_err(to_js_err)?;
    to_js_value(&result)
}

// Position-level VaR / ES decomposition and risk budgeting

/// Decompose portfolio VaR into position contributions via parametric Euler
/// allocation. Inputs mirror the Python binding's signature.
///
/// `covariance_json` must deserialize to an `n x n` row-major nested array.
/// @param position_ids_json - JSON array of position identifiers.
/// @param weights_json - Position or asset weight-vector JSON.
/// @param covariance_json - Covariance-matrix JSON.
/// @param confidence - Tail confidence as a decimal probability, such as 0.95 for 95%.
/// @param compute_incremental - Optional; when `true`, also computes
///   incremental VaR (one full repricing per position). Defaults to `false`,
///   mirroring the Python `compute_incremental=` keyword.
///
/// # Errors
///
/// Throws a JavaScript exception if any JSON input is malformed; identifier,
/// weight, or covariance dimensions disagree; the covariance matrix is not
/// finite, symmetric, and positive semidefinite; `confidence` is not finite and
/// in `(0.5, 1)`; or the result cannot be converted to a JavaScript value.
#[wasm_bindgen(js_name = parametricVarDecomposition)]
pub fn parametric_var_decomposition(
    position_ids_json: &str,
    weights_json: &str,
    covariance_json: &str,
    confidence: f64,
    compute_incremental: Option<bool>,
) -> Result<JsValue, JsValue> {
    use finstack_quant_portfolio::factor_model::{
        parametric_var_decomposition_view, DecompositionConfig, ParametricPositionDecomposer,
    };
    use finstack_quant_portfolio::types::PositionId;

    let ids: Vec<String> = serde_json::from_str(position_ids_json).map_err(to_js_err)?;
    let weights: Vec<f64> = serde_json::from_str(weights_json).map_err(to_js_err)?;
    let covariance: Vec<Vec<f64>> = serde_json::from_str(covariance_json).map_err(to_js_err)?;
    let n = weights.len();
    let cov_flat = flatten_square_matrix(covariance, n, "covariance")?;
    let ids: Vec<PositionId> = ids.into_iter().map(PositionId::new).collect();

    let mut config = DecompositionConfig::parametric(confidence);
    if compute_incremental.unwrap_or(false) {
        config = config.with_incremental();
    }

    let decomposer = ParametricPositionDecomposer;
    let result = decomposer
        .decompose_positions(&weights, &cov_flat, &ids, &config)
        .map_err(to_js_err)?;
    let out = parametric_var_decomposition_view(&result);
    to_js_value(&out)
}

/// Decompose portfolio Expected Shortfall into position contributions via
/// parametric Euler allocation.
///
/// Returns an ES-shaped structured object mirroring the Python
/// ``parametric_es_decomposition`` return value: a top-level
/// ``{portfolio_var, portfolio_es, confidence, n_positions, contributions}``
/// object whose ``contributions`` entries are
/// ``{position_id, component_es, marginal_es, pct_contribution}``.
/// @param position_ids_json - JSON array of position identifiers.
/// @param weights_json - Position or asset weight-vector JSON.
/// @param covariance_json - Covariance-matrix JSON.
/// @param confidence - Tail confidence as a decimal probability, such as 0.95 for 95%.
///
/// # Errors
///
/// Throws a JavaScript exception if any JSON input is malformed; identifier,
/// weight, or covariance dimensions disagree; the covariance matrix is not
/// finite, symmetric, and positive semidefinite; `confidence` is not finite and
/// in `(0.5, 1)`; or the result cannot be converted to a JavaScript value.
#[wasm_bindgen(js_name = parametricEsDecomposition)]
pub fn parametric_es_decomposition(
    position_ids_json: &str,
    weights_json: &str,
    covariance_json: &str,
    confidence: f64,
) -> Result<JsValue, JsValue> {
    use finstack_quant_portfolio::factor_model::{
        parametric_es_decomposition_view, DecompositionConfig, ParametricPositionDecomposer,
    };
    use finstack_quant_portfolio::types::PositionId;

    let ids: Vec<String> = serde_json::from_str(position_ids_json).map_err(to_js_err)?;
    let weights: Vec<f64> = serde_json::from_str(weights_json).map_err(to_js_err)?;
    let covariance: Vec<Vec<f64>> = serde_json::from_str(covariance_json).map_err(to_js_err)?;
    let n = weights.len();
    let cov_flat = flatten_square_matrix(covariance, n, "covariance")?;
    let ids: Vec<PositionId> = ids.into_iter().map(PositionId::new).collect();

    let config = DecompositionConfig::parametric(confidence);

    let decomposition = ParametricPositionDecomposer
        .decompose_positions(&weights, &cov_flat, &ids, &config)
        .map_err(to_js_err)?;

    let out = parametric_es_decomposition_view(&decomposition);
    to_js_value(&out)
}

/// Decompose portfolio VaR/ES from per-position scenario P&Ls via historical
/// simulation.
///
/// `position_pnls_json` is a nested array shaped `[n_positions][n_scenarios]`.
/// @param position_ids_json - JSON array of position identifiers.
/// @param position_pnls_json - Per-position P&L JSON.
/// @param confidence - Tail confidence as a decimal probability, such as 0.95 for 95%.
///
/// # Errors
///
/// Throws a JavaScript exception if either JSON input is malformed, position or
/// scenario dimensions disagree, `confidence` is not finite and in `(0.5, 1)`,
/// too few scenarios resolve the requested tail, a P-and-L value is non-finite,
/// or the result cannot be converted to a JavaScript value.
#[wasm_bindgen(js_name = historicalVarDecomposition)]
pub fn historical_var_decomposition(
    position_ids_json: &str,
    position_pnls_json: &str,
    confidence: f64,
) -> Result<JsValue, JsValue> {
    use finstack_quant_portfolio::factor_model::{
        flatten_position_pnls, parametric_var_decomposition_view, DecompositionConfig,
        HistoricalPositionDecomposer,
    };
    use finstack_quant_portfolio::types::PositionId;

    let ids: Vec<String> = serde_json::from_str(position_ids_json).map_err(to_js_err)?;
    let position_pnls: Vec<Vec<f64>> =
        serde_json::from_str(position_pnls_json).map_err(to_js_err)?;
    let n = ids.len();
    let ids: Vec<PositionId> = ids.into_iter().map(PositionId::new).collect();
    let config = DecompositionConfig::historical(confidence);

    let (flat, n_scenarios) = flatten_position_pnls(position_pnls, n).map_err(to_js_err)?;
    let result = HistoricalPositionDecomposer
        .decompose_from_pnls(&flat, &ids, n_scenarios, &config)
        .map_err(to_js_err)?;
    let out = parametric_var_decomposition_view(&result);
    to_js_value(&out)
}

/// Evaluate a per-position risk budget against actual component VaRs.
///
/// Validation (array-length agreement, duplicate position-id rejection) and
/// the default `utilizationThreshold` live in the canonical Rust
/// `evaluate_risk_budget_arrays` / `DEFAULT_UTILIZATION_THRESHOLD` path
/// shared with the Python binding.
/// @param position_ids_json - JSON array of position identifiers.
/// @param actual_var_json - Actual component-VaR JSON.
/// @param target_var_pct_json - Target VaR-share JSON.
/// @param portfolio_var - Total portfolio VaR used to convert risk-budget shares into absolute amounts.
/// @param utilization_threshold - Optional actual-to-target risk ratio that
///   flags a budget breach; omit for the Rust default of 1.2.
///
/// # Errors
///
/// Throws a JavaScript exception if any JSON input is malformed, actual or
/// target arrays do not match the identifier count, a position id is
/// duplicated, non-empty target shares do not sum to one within tolerance,
/// nonzero component risk is paired with zero `portfolioVar`, or the result
/// cannot be converted to a JavaScript value.
#[wasm_bindgen(js_name = evaluateRiskBudget)]
pub fn evaluate_risk_budget(
    position_ids_json: &str,
    actual_var_json: &str,
    target_var_pct_json: &str,
    portfolio_var: f64,
    utilization_threshold: Option<f64>,
) -> Result<JsValue, JsValue> {
    use finstack_quant_portfolio::factor_model::{
        evaluate_risk_budget_arrays, risk_budget_result_view, DEFAULT_UTILIZATION_THRESHOLD,
    };

    let ids: Vec<String> = serde_json::from_str(position_ids_json).map_err(to_js_err)?;
    let actual_var: Vec<f64> = serde_json::from_str(actual_var_json).map_err(to_js_err)?;
    let target_var_pct: Vec<f64> = serde_json::from_str(target_var_pct_json).map_err(to_js_err)?;
    let threshold = utilization_threshold.unwrap_or(DEFAULT_UTILIZATION_THRESHOLD);

    let result =
        evaluate_risk_budget_arrays(ids, &actual_var, &target_var_pct, portfolio_var, threshold)
            .map_err(to_js_err)?;
    let out = risk_budget_result_view(&result, portfolio_var, threshold);
    to_js_value(&out)
}

/// Forward to the shared `finstack_quant_portfolio::factor_model::flatten_square_matrix`
/// and remap the validation error to a `JsValue` so the same matrix-validation
/// diagnostics surface from both the WASM and Python bindings.
fn flatten_square_matrix(
    matrix: Vec<Vec<f64>>,
    n: usize,
    label: &str,
) -> Result<Vec<f64>, JsValue> {
    finstack_quant_portfolio::factor_model::flatten_square_matrix(matrix, n, label)
        .map_err(to_js_err)
}

// Liquidity: spread estimators, tiering, LVaR, market impact

/// Effective bid-ask spread via Roll (1984). Returns `undefined` when the
/// serial covariance is non-negative (Roll assumption violated) or inputs too short.
/// @param returns_json - Numeric return-series JSON.
///
/// # Errors
///
/// Throws a JavaScript exception if `returnsJson` is malformed or does not
/// contain a numeric array. Invalid estimator inputs return `undefined`.
#[wasm_bindgen(js_name = rollEffectiveSpread)]
pub fn roll_effective_spread(returns_json: &str) -> Result<Option<f64>, JsValue> {
    let returns: Vec<f64> = serde_json::from_str(returns_json).map_err(to_js_err)?;
    Ok(finstack_quant_portfolio::liquidity::roll_effective_spread(
        &returns,
    ))
}

/// Amihud (2002) illiquidity ratio from returns and volumes.
/// @param returns_json - Numeric return-series JSON.
/// @param volumes_json - Volume-series JSON.
///
/// # Errors
///
/// Throws a JavaScript exception if either JSON input is malformed or does not
/// contain a numeric array. Invalid estimator inputs return `undefined`.
#[wasm_bindgen(js_name = amihudIlliquidity)]
pub fn amihud_illiquidity(returns_json: &str, volumes_json: &str) -> Result<Option<f64>, JsValue> {
    let returns: Vec<f64> = serde_json::from_str(returns_json).map_err(to_js_err)?;
    let volumes: Vec<f64> = serde_json::from_str(volumes_json).map_err(to_js_err)?;
    Ok(finstack_quant_portfolio::liquidity::amihud_illiquidity(
        &returns, &volumes,
    ))
}

/// Trading days required to liquidate at the given participation rate.
///
/// Share-space contract (matches the Rust `days_to_liquidate` signature):
/// both quantity and ADV are counts of shares/contracts, not currency
/// notionals. Mixing a notional with a share-count ADV silently mis-scales
/// the result by the share price.
/// @param position_quantity - Number of shares/contracts to liquidate (absolute value used).
/// @param adv - Average daily traded volume in shares/contracts.
/// @param participation_rate - Maximum fraction of average daily volume used for execution.
#[wasm_bindgen(js_name = daysToLiquidate)]
pub fn days_to_liquidate(position_quantity: f64, adv: f64, participation_rate: f64) -> f64 {
    finstack_quant_portfolio::liquidity::days_to_liquidate(
        position_quantity,
        adv,
        participation_rate,
    )
}

/// Classify a position into a liquidity tier from its days-to-liquidate.
///
/// Uses the default `[1, 5, 20, 60]` trading-day thresholds. Returns one of
/// `"tier1" .. "tier5"`.
/// @param days_to_liquidate - Trading days to liquidate; classified with default thresholds `[1, 5, 20, 60]`.
#[wasm_bindgen(js_name = liquidityTier)]
pub fn liquidity_tier(days_to_liquidate: f64) -> String {
    use finstack_quant_portfolio::liquidity::classify_tier;
    let config = finstack_quant_portfolio::liquidity::LiquidityConfig::default();
    classify_tier(days_to_liquidate, &config.tier_thresholds)
        .as_binding_str()
        .to_string()
}

/// Liquidity-adjusted VaR following Bangia, Diebold, Schuermann & Stroughair (1999).
/// Loss sign convention: `var` and `lvar` are non-positive. Returns a structured
/// object with `var`, `spread_cost`, `lvar` and `lvar_ratio`, matching the
/// Python binding's dict.
/// @param var - Base market value-at-risk before adding the liquidity adjustment.
/// @param spread_mean - Mean bid-ask spread in the quote units required by the liquidity model.
/// @param spread_vol - Volatility of the bid-ask spread in the liquidity model's units.
/// @param confidence - Tail confidence as a decimal probability strictly inside (0.5, 1), such as 0.95 for 95%.
/// @param position_value - Current position market value in the relevant currency units.
///
/// # Errors
///
/// Throws a JavaScript exception if `var` is non-finite or positive; either
/// spread input is non-finite or negative; `confidence` is outside `(0.5, 1)`;
/// `positionValue` is non-finite; or the result cannot be converted to a
/// JavaScript value.
#[wasm_bindgen(js_name = lvarBangia)]
pub fn lvar_bangia(
    var: f64,
    spread_mean: f64,
    spread_vol: f64,
    confidence: f64,
    position_value: f64,
) -> Result<JsValue, JsValue> {
    let result = finstack_quant_portfolio::liquidity::lvar_bangia_scalar(
        var,
        spread_mean,
        spread_vol,
        confidence,
        position_value,
    )
    .map_err(to_js_err)?;
    to_js_value(&result)
}

/// Almgren-Chriss (2001) market impact decomposition for a uniform execution.
///
/// Returns a structured object with `permanent_impact`, `temporary_impact`,
/// `total_impact`, `expected_cost_bp` and `execution_risk`, matching the
/// Python binding's dict (both delegate to the canonical Rust
/// `AlmgrenChrissImpactView`).
/// @param position_size - Trade size in shares or notional units for the execution calculation.
/// @param avg_daily_volume - Average daily trading volume in the same units as the position size.
/// @param volatility - Daily return volatility expressed as a decimal, such as 0.02 for 2% (per the Rust `almgren_chriss_uniform_impact` contract; do not pass annualized vol).
/// @param execution_horizon_days - Planned execution horizon measured in trading days.
/// @param permanent_impact_coef - Permanent market-impact coefficient in the execution-cost model.
/// @param temporary_impact_coef - Temporary market-impact coefficient in the execution-cost model.
/// @param reference_price - Optional reference price used to express execution impact in monetary units.
///
/// # Errors
///
/// Throws a JavaScript exception if `positionSize` is non-finite; volume,
/// volatility, or horizon is not finite and positive; an impact coefficient is
/// outside its valid range; `referencePrice` is present but not finite and
/// positive; impact calculation fails; or the result cannot be converted to a
/// JavaScript value.
#[wasm_bindgen(js_name = almgrenChrissImpact)]
pub fn almgren_chriss_impact(
    position_size: f64,
    avg_daily_volume: f64,
    volatility: f64,
    execution_horizon_days: f64,
    permanent_impact_coef: f64,
    temporary_impact_coef: f64,
    reference_price: Option<f64>,
) -> Result<JsValue, JsValue> {
    let out = finstack_quant_portfolio::liquidity::almgren_chriss_uniform_impact(
        position_size,
        avg_daily_volume,
        volatility,
        execution_horizon_days,
        permanent_impact_coef,
        temporary_impact_coef,
        reference_price,
    )
    .map_err(to_js_err)?;
    to_js_value(&finstack_quant_portfolio::liquidity::almgren_chriss_impact_view(&out))
}

/// Kyle (1985) linear price impact lambda estimated from observed volumes
/// and returns via the Amihud-ratio proxy. Returns `undefined` on invalid inputs.
/// @param volumes_json - Volume-series JSON.
/// @param returns_json - Numeric return-series JSON.
/// @param reference_price - Positive price per share or contract used to convert the return-space ratio into price-space lambda.
///
/// # Errors
///
/// Throws a JavaScript exception if either JSON input is malformed or does not
/// contain a numeric array. Invalid estimator inputs, including a non-positive
/// or non-finite `referencePrice`, return `undefined`.
#[wasm_bindgen(js_name = kyleLambda)]
pub fn kyle_lambda(
    volumes_json: &str,
    returns_json: &str,
    reference_price: f64,
) -> Result<Option<f64>, JsValue> {
    let volumes: Vec<f64> = serde_json::from_str(volumes_json).map_err(to_js_err)?;
    let returns: Vec<f64> = serde_json::from_str(returns_json).map_err(to_js_err)?;
    Ok(
        finstack_quant_portfolio::liquidity::KyleLambdaModel::lambda_from_series(
            &volumes,
            &returns,
            reference_price,
        ),
    )
}

/// Host-target unit tests.
///
/// Only exports that return plain Rust values (`String`, `f64`,
/// `Option<f64>`) can be exercised here: every export converted to a
/// structured object returns a `JsValue`, and `JsValue` construction aborts
/// off `wasm32`. Those live in `tests/wasm_portfolio.rs` under
/// `#[wasm_bindgen_test]`, where they also gate the `JSON.stringify`
/// round-trip that catches an ES-`Map` serialization regression.
#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_portfolio_spec_json() -> String {
        serde_json::json!({
            "id": "test_portfolio",
            "name": "Test",
            "base_currency": "USD",
            "as_of": "2024-01-15",
            "entities": {},
            "positions": []
        })
        .to_string()
    }

    #[test]
    fn parse_portfolio_spec_json_roundtrip() {
        let json = minimal_portfolio_spec_json();
        let result = parse_portfolio_spec_json(&json).expect("parse");
        let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
        assert_eq!(parsed["id"], "test_portfolio");
    }

    #[test]
    fn build_portfolio_from_spec_json_empty() {
        let json = minimal_portfolio_spec_json();
        let result = build_portfolio_from_spec_json(&json).expect("build");
        let parsed: serde_json::Value = serde_json::from_str(&result).expect("valid json");
        assert_eq!(parsed["id"], "test_portfolio");
    }

    #[test]
    fn parse_and_rebuild_roundtrip() {
        let json = minimal_portfolio_spec_json();
        let canonical = parse_portfolio_spec_json(&json).expect("parse");
        let rebuilt = build_portfolio_from_spec_json(&canonical).expect("rebuild");
        let a: serde_json::Value = serde_json::from_str(&canonical).expect("a");
        let b: serde_json::Value = serde_json::from_str(&rebuilt).expect("b");
        assert_eq!(a["id"], b["id"]);
    }

    fn empty_market_json() -> String {
        let ctx = finstack_quant_core::market_data::context::MarketContext::new();
        serde_json::to_string(&ctx).expect("serialize")
    }

    #[test]
    fn portfolio_handle_exposes_spec_metadata_and_roundtrips() {
        let spec_json = minimal_portfolio_spec_json();
        let handle = JsPortfolio::from_spec(&spec_json).expect("build handle");
        assert_eq!(handle.id(), "test_portfolio");
        assert_eq!(handle.base_currency(), "USD");
        assert_eq!(handle.as_of(), "2024-01-15");
        assert_eq!(handle.num_positions(), 0);

        let round = handle.to_json().expect("to spec json");
        let parsed: serde_json::Value = serde_json::from_str(&round).expect("json");
        assert_eq!(parsed["id"], "test_portfolio");
    }

    #[test]
    fn portfolio_result_total_value_from_valuation() {
        let spec: finstack_quant_portfolio::portfolio::PortfolioSpec =
            serde_json::from_str(&minimal_portfolio_spec_json()).expect("parse spec");
        let portfolio =
            finstack_quant_portfolio::Portfolio::from_spec(spec).expect("build portfolio");
        let market: finstack_quant_core::market_data::context::MarketContext =
            serde_json::from_str(&empty_market_json()).expect("parse market");
        let valuation = finstack_quant_portfolio::valuation::value_portfolio(
            &portfolio,
            &market,
            &finstack_quant_core::config::FinstackConfig::default(),
            &finstack_quant_portfolio::valuation::PortfolioValuationOptions::default(),
        )
        .expect("value");
        let result = finstack_quant_portfolio::results::PortfolioResult::new(
            valuation,
            Default::default(),
            Default::default(),
        );
        let result_json = serde_json::to_string(&result).expect("ser");
        let total = portfolio_result_total_value(&result_json).expect("total");
        assert!(total.is_finite());
    }

    /// Tests the replay_portfolio WASM binding logic by exercising the same
    /// JSON parsing / domain call / serialization pipeline directly.
    /// We call the domain functions instead of the wasm wrapper because
    /// `JsValue::from_str` panics on non-wasm32 targets when an error is
    /// produced.
    #[test]
    fn replay_portfolio_empty_portfolio() {
        let spec_json = minimal_portfolio_spec_json();
        let spec: finstack_quant_portfolio::portfolio::PortfolioSpec =
            serde_json::from_str(&spec_json).expect("parse spec");
        let portfolio =
            finstack_quant_portfolio::Portfolio::from_spec(spec).expect("build portfolio");

        let market_val: serde_json::Value =
            serde_json::from_str(&empty_market_json()).expect("parse market");
        let snapshots_json = serde_json::json!([
            {"date": "2024-01-15", "market": market_val},
            {"date": "2024-01-16", "market": market_val}
        ])
        .to_string();

        let timeline =
            finstack_quant_portfolio::replay::ReplayTimeline::from_json_snapshots(&snapshots_json)
                .expect("build timeline");

        let config_json = serde_json::json!({
            "mode": "pv_only",
            "attribution_method": "parallel"
        })
        .to_string();
        let config: finstack_quant_portfolio::replay::ReplayConfig =
            serde_json::from_str(&config_json).expect("parse config");

        let finstack_config = finstack_quant_core::config::FinstackConfig::default();

        let result = finstack_quant_portfolio::replay::replay_portfolio(
            &portfolio,
            &timeline,
            &config,
            &finstack_config,
        )
        .expect("replay");

        let json = serde_json::to_string(&result).expect("serialize");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("parse json");
        assert!(parsed["steps"].is_array());
        assert_eq!(parsed["steps"].as_array().expect("array").len(), 2);
    }

    #[test]
    fn twrr_modified_dietz_matches_gips_example() {
        let period = serde_json::json!({
            "beginning_market_value": 10_000_000.0,
            "ending_market_value": 10_500_000.0,
            "cashflows": [
                {
                    "amount": 1_000_000.0,
                    "fraction_of_period_remaining": 0.60
                }
            ]
        });

        let result = twrr_modified_dietz(&period.to_string()).expect("modified dietz");
        let expected = -500_000.0 / 10_600_000.0;
        assert!((result - expected).abs() < 1e-12);
    }

    #[test]
    fn mo24_liquidity_estimators_return_none_for_missing_estimates() {
        assert_eq!(roll_effective_spread("[0.01]").expect("valid json"), None);
        assert_eq!(
            amihud_illiquidity("[0.01]", "[0.0]").expect("valid json"),
            None
        );
        assert_eq!(
            kyle_lambda("[0.0]", "[0.01]", 100.0).expect("valid json"),
            None
        );
    }

    #[test]
    fn kyle_lambda_calibrates_in_price_space() {
        let lambda = kyle_lambda("[100.0, 200.0]", "[0.01, -0.02]", 50.0)
            .expect("valid JSON")
            .expect("valid price-space inputs");
        assert!((lambda - 0.005).abs() < 1e-15);
    }

    #[test]
    fn mo28_liquidity_tier_uses_default_config_thresholds() {
        let config = finstack_quant_portfolio::liquidity::LiquidityConfig::default();
        let threshold = config.tier_thresholds[0];
        let expected =
            finstack_quant_portfolio::liquidity::classify_tier(threshold, &config.tier_thresholds)
                .as_binding_str()
                .to_string();

        assert_eq!(liquidity_tier(threshold), expected);
    }

    #[test]
    fn mwr_xirr_solves_money_weighted_return() {
        let cashflows = serde_json::json!([
            {"date": "2025-01-01", "amount": -100.0},
            {"date": "2026-01-01", "amount": 110.0}
        ]);

        let result = mwr_xirr(&cashflows.to_string()).expect("xirr");
        assert!((result - 0.10).abs() < 1e-6);
    }
}
