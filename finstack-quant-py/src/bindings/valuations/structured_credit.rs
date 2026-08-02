//! Structured-credit tranche analytics: DM, break-even CDR, OAS, metrics,
//! scenario table.
//!
//! These take a tranche id, so they are not reachable through
//! `price_instrument_with_metrics`. Signatures mirror the WASM facade;
//! `OasResult`, `TrancheMetrics`, and `ScenarioTable` are returned as JSON
//! strings by the five functions below, and can be decoded into the typed
//! `OasResult` / `TrancheMetrics` / `ScenarioTable` wrapper classes in this
//! module via `from_json`.

use crate::bindings::extract::{extract_instrument_json, extract_market};
use crate::errors::display_to_py;
use finstack_quant_valuations::instruments::fixed_income::structured_credit::{
    OasResult, ScenarioTable, TrancheMetrics,
};
use pyo3::prelude::*;
use pyo3::types::PyModule;
use std::collections::BTreeMap;

/// Solve a z-spread-equivalent discount margin for a floating-rate tranche.
///
/// Contractual cashflows are projected without changing coupon projection, then
/// a constant additive spread is applied to the discount curve. The result is
/// zero at model PV, negative for a richer (higher) target PV, and positive for
/// a cheaper (lower) target PV; it is not the contractual quoted margin.
///
/// Parameters
/// ----------
/// instrument_json : str | StructuredCredit
///     Tagged JSON for a ``StructuredCredit`` deal, or a typed
///     ``StructuredCredit`` instance.
/// tranche_id : str
///     Identifier of the floating-rate tranche whose contractual cashflows are
///     spread-discounted.
/// market : MarketContext
///     Market context supplying the discount curve and any forward curves or
///     historical fixings required for cashflow projection.
/// as_of : str
///     Valuation date used for projection and discounting, ``YYYY-MM-DD``.
/// target_pv : float
///     Target present value in the tranche's currency. Values above model PV
///     produce a negative result; values below model PV produce a positive
///     result.
///
/// Returns
/// -------
/// float
///     Z-spread-equivalent discount margin in decimal (``0.015`` = 150 bp).
///
/// Raises
/// ------
/// ValueError
///     If the JSON or date is malformed, the deal is invalid, the tranche is
///     missing or fixed-rate, ``target_pv`` is not finite, required market data
///     is unavailable, or the spread solve fails or exceeds ±5000 bp.
#[pyfunction]
#[pyo3(
    name = "structured_credit_tranche_discount_margin",
    text_signature = "(instrument_json, tranche_id, market, as_of, target_pv)"
)]
fn structured_credit_tranche_discount_margin(
    py: Python<'_>,
    instrument_json: &Bound<'_, PyAny>,
    tranche_id: &str,
    market: &Bound<'_, PyAny>,
    as_of: &str,
    target_pv: f64,
) -> PyResult<f64> {
    let instrument_json = extract_instrument_json(instrument_json)?;
    let market = extract_market(py, market)?;
    let tranche_id = tranche_id.to_owned();
    let as_of = as_of.to_owned();
    py.detach(move || {
        finstack_quant_valuations::pricer::structured_credit_tranche_discount_margin_json(
            &instrument_json,
            &tranche_id,
            &market,
            &as_of,
            target_pv,
        )
        .map_err(display_to_py)
    })
}

/// Solve the constant default rate at which a tranche first takes a writedown.
///
/// Parameters
/// ----------
/// instrument_json : str | StructuredCredit
///     Tagged JSON for a ``StructuredCredit`` deal, or a typed
///     ``StructuredCredit`` instance.
/// tranche_id : str
///     Identifier of the tranche.
/// market : MarketContext
///     Market context supplying curves and fixings.
/// as_of : str
///     Valuation date, ``YYYY-MM-DD``.
///
/// Returns
/// -------
/// float
///     Break-even annual CDR in decimal.
#[pyfunction]
#[pyo3(
    name = "structured_credit_tranche_breakeven_cdr",
    text_signature = "(instrument_json, tranche_id, market, as_of)"
)]
fn structured_credit_tranche_breakeven_cdr(
    py: Python<'_>,
    instrument_json: &Bound<'_, PyAny>,
    tranche_id: &str,
    market: &Bound<'_, PyAny>,
    as_of: &str,
) -> PyResult<f64> {
    let instrument_json = extract_instrument_json(instrument_json)?;
    let market = extract_market(py, market)?;
    let tranche_id = tranche_id.to_owned();
    let as_of = as_of.to_owned();
    py.detach(move || {
        finstack_quant_valuations::pricer::structured_credit_tranche_breakeven_cdr_json(
            &instrument_json,
            &tranche_id,
            &market,
            &as_of,
        )
        .map_err(display_to_py)
    })
}

/// Compute the option-adjusted spread for a tranche at a market price.
///
/// Parameters
/// ----------
/// instrument_json : str | StructuredCredit
///     Tagged JSON for a ``StructuredCredit`` deal, or a typed
///     ``StructuredCredit`` instance.
/// tranche_id : str
///     Identifier of the tranche.
/// market_price_pct : float
///     Market price as a percentage of original balance (100.0 = par).
/// market : MarketContext
///     Market context supplying curves and fixings.
/// as_of : str
///     Valuation date, ``YYYY-MM-DD``.
/// config_json : str, optional
///     Serialized ``OasConfig``. All fields are currently required when
///     supplied.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``OasResult``. Decode with ``OasResult.from_json``.
#[pyfunction]
#[pyo3(
    name = "structured_credit_tranche_oas",
    signature = (instrument_json, tranche_id, market_price_pct, market, as_of, config_json=None),
    text_signature = "(instrument_json, tranche_id, market_price_pct, market, as_of, config_json=None)"
)]
fn structured_credit_tranche_oas(
    py: Python<'_>,
    instrument_json: &Bound<'_, PyAny>,
    tranche_id: &str,
    market_price_pct: f64,
    market: &Bound<'_, PyAny>,
    as_of: &str,
    config_json: Option<&str>,
) -> PyResult<String> {
    let instrument_json = extract_instrument_json(instrument_json)?;
    let market = extract_market(py, market)?;
    let tranche_id = tranche_id.to_owned();
    let as_of = as_of.to_owned();
    let config_json = config_json.map(str::to_owned);
    py.detach(move || {
        let result = finstack_quant_valuations::pricer::structured_credit_tranche_oas_json(
            &instrument_json,
            &tranche_id,
            market_price_pct,
            &market,
            &as_of,
            config_json.as_deref(),
        )
        .map_err(display_to_py)?;
        serde_json::to_string(&result).map_err(display_to_py)
    })
}

/// Compute the summary risk/pricing metrics for a tranche.
///
/// Parameters
/// ----------
/// instrument_json : str | StructuredCredit
///     Tagged JSON for a ``StructuredCredit`` deal, or a typed
///     ``StructuredCredit`` instance.
/// tranche_id : str
///     Identifier of the tranche.
/// market : MarketContext
///     Market context supplying curves and fixings.
/// as_of : str
///     Valuation date, ``YYYY-MM-DD``.
/// market_price_pct : float, optional
///     Market price as a percentage of original balance; when omitted the
///     model price is used.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``TrancheMetrics``. Decode with
///     ``TrancheMetrics.from_json``.
#[pyfunction]
#[pyo3(
    name = "structured_credit_tranche_metrics",
    signature = (instrument_json, tranche_id, market, as_of, market_price_pct=None),
    text_signature = "(instrument_json, tranche_id, market, as_of, market_price_pct=None)"
)]
fn structured_credit_tranche_metrics(
    py: Python<'_>,
    instrument_json: &Bound<'_, PyAny>,
    tranche_id: &str,
    market: &Bound<'_, PyAny>,
    as_of: &str,
    market_price_pct: Option<f64>,
) -> PyResult<String> {
    let instrument_json = extract_instrument_json(instrument_json)?;
    let market = extract_market(py, market)?;
    let tranche_id = tranche_id.to_owned();
    let as_of = as_of.to_owned();
    py.detach(move || {
        let result = finstack_quant_valuations::pricer::structured_credit_tranche_metrics_json(
            &instrument_json,
            &tranche_id,
            &market,
            &as_of,
            market_price_pct,
        )
        .map_err(display_to_py)?;
        serde_json::to_string(&result).map_err(display_to_py)
    })
}

/// Price a tranche across a CPR x CDR x severity scenario grid.
///
/// Parameters
/// ----------
/// instrument_json : str | StructuredCredit
///     Tagged JSON for a ``StructuredCredit`` deal, or a typed
///     ``StructuredCredit`` instance.
/// tranche_id : str
///     Identifier of the tranche.
/// market : MarketContext
///     Market context supplying curves and fixings.
/// as_of : str
///     Valuation date, ``YYYY-MM-DD``.
/// grid_json : str
///     Serialized ``ScenarioGrid``. The grid is capped at 10,000 cells because
///     each cell reprices the entire deal.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``ScenarioTable``. Decode with
///     ``ScenarioTable.from_json``.
#[pyfunction]
#[pyo3(
    name = "structured_credit_tranche_scenario_table",
    text_signature = "(instrument_json, tranche_id, market, as_of, grid_json)"
)]
fn structured_credit_tranche_scenario_table(
    py: Python<'_>,
    instrument_json: &Bound<'_, PyAny>,
    tranche_id: &str,
    market: &Bound<'_, PyAny>,
    as_of: &str,
    grid_json: &str,
) -> PyResult<String> {
    let instrument_json = extract_instrument_json(instrument_json)?;
    let market = extract_market(py, market)?;
    let tranche_id = tranche_id.to_owned();
    let as_of = as_of.to_owned();
    let grid_json = grid_json.to_owned();
    py.detach(move || {
        let result =
            finstack_quant_valuations::pricer::structured_credit_tranche_scenario_table_json(
                &instrument_json,
                &tranche_id,
                &market,
                &as_of,
                &grid_json,
            )
            .map_err(display_to_py)?;
        serde_json::to_string(&result).map_err(display_to_py)
    })
}

// ---------------------------------------------------------------------------
// OasResult
// ---------------------------------------------------------------------------

/// Result of an option-adjusted-spread calculation for a structured-credit
/// tranche ([`structured_credit_tranche_oas`]'s decoded return value).
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "OasResult",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyOasResult {
    pub(crate) inner: OasResult,
}

#[pymethods]
impl PyOasResult {
    /// Deserialize from the JSON returned by ``structured_credit_tranche_oas``.
    ///
    /// Parameters
    /// ----------
    /// json : str
    ///     JSON-encoded ``OasResult``.
    ///
    /// Returns
    /// -------
    /// OasResult
    ///     The decoded result.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``json`` is not valid JSON for the ``OasResult`` shape.
    ///
    /// Examples
    /// --------
    /// >>> import json
    /// >>> from finstack_quant.valuations.instruments import OasResult
    /// >>> payload = json.dumps({
    /// ...     "oas": 0.0125, "model_price": 99.5, "market_price": 98.75,
    /// ...     "num_paths": 256, "price_std_error": 0.05,
    /// ... })
    /// >>> result = OasResult.from_json(payload)
    /// >>> result.oas
    /// 0.0125
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: OasResult = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize back to the same JSON shape ``from_json`` accepts.
    ///
    /// Returns
    /// -------
    /// str
    ///     JSON-encoded ``OasResult``.
    #[pyo3(text_signature = "($self)")]
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Option-adjusted spread, as an annual decimal (``0.01`` = 100 bp).
    #[getter]
    fn oas(&self) -> f64 {
        self.inner.oas
    }

    /// Model price at the solved OAS, as a percentage of original balance.
    #[getter]
    fn model_price(&self) -> f64 {
        self.inner.model_price
    }

    /// Target market price, as a percentage of original balance.
    #[getter]
    fn market_price(&self) -> f64 {
        self.inner.market_price
    }

    /// Number of Monte-Carlo scenarios used.
    #[getter]
    fn num_paths(&self) -> usize {
        self.inner.num_paths
    }

    /// Monte-Carlo standard error of the mean price, as a percentage of
    /// original balance.
    #[getter]
    fn price_std_error(&self) -> f64 {
        self.inner.price_std_error
    }

    /// Return ``repr(self)``.
    fn __repr__(&self) -> String {
        format!(
            "OasResult(oas={}, model_price={}, market_price={})",
            self.inner.oas, self.inner.model_price, self.inner.market_price
        )
    }
}

// ---------------------------------------------------------------------------
// TrancheMetrics
// ---------------------------------------------------------------------------

/// Summary risk/pricing metrics for a structured-credit tranche
/// ([`structured_credit_tranche_metrics`]'s decoded return value).
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "TrancheMetrics",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyTrancheMetrics {
    pub(crate) inner: TrancheMetrics,
}

#[pymethods]
impl PyTrancheMetrics {
    /// Deserialize from the JSON returned by
    /// ``structured_credit_tranche_metrics``.
    ///
    /// Parameters
    /// ----------
    /// json : str
    ///     JSON-encoded ``TrancheMetrics``.
    ///
    /// Returns
    /// -------
    /// TrancheMetrics
    ///     The decoded metrics bundle.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``json`` is not valid JSON for the ``TrancheMetrics`` shape.
    ///
    /// Examples
    /// --------
    /// >>> import json
    /// >>> from finstack_quant.valuations.instruments import TrancheMetrics
    /// >>> payload = json.dumps({
    /// ...     "tranche_id": "A", "currency": "USD", "pv": 1000.0,
    /// ...     "price_pct": 100.0, "wal": 3.0, "z_spread_bp": 0.0,
    /// ...     "cs01": -1.0, "spread_duration": 3.0, "modified_duration": 3.0,
    /// ...     "convexity": 12.0, "target_price_pct": 100.0,
    /// ... })
    /// >>> metrics = TrancheMetrics.from_json(payload)
    /// >>> metrics.tranche_id
    /// 'A'
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: TrancheMetrics = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize back to the same JSON shape ``from_json`` accepts.
    ///
    /// Returns
    /// -------
    /// str
    ///     JSON-encoded ``TrancheMetrics``.
    #[pyo3(text_signature = "($self)")]
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Identifier of the tranche.
    #[getter]
    fn tranche_id(&self) -> &str {
        &self.inner.tranche_id
    }

    /// ISO-4217 code of the currency ``pv`` and ``cs01`` are denominated in.
    #[getter]
    fn currency(&self) -> &str {
        &self.inner.currency
    }

    /// Present value of the tranche, in ``currency`` units.
    #[getter]
    fn pv(&self) -> f64 {
        self.inner.pv
    }

    /// Model price, as a percentage of original balance.
    #[getter]
    fn price_pct(&self) -> f64 {
        self.inner.price_pct
    }

    /// Weighted-average life, in years.
    #[getter]
    fn wal(&self) -> f64 {
        self.inner.wal
    }

    /// Z-spread to ``target_price_pct``, in basis points.
    #[getter]
    fn z_spread_bp(&self) -> f64 {
        self.inner.z_spread_bp
    }

    /// Credit-spread DV01 — currency change for a +1 bp z-spread shock,
    /// in ``currency`` units. Negative for a long tranche.
    #[getter]
    fn cs01(&self) -> f64 {
        self.inner.cs01
    }

    /// Spread duration, in years (``-cs01 / (pv * 1bp)``).
    #[getter]
    fn spread_duration(&self) -> f64 {
        self.inner.spread_duration
    }

    /// Modified (rate) duration of the projected cashflows, in years.
    #[getter]
    fn modified_duration(&self) -> f64 {
        self.inner.modified_duration
    }

    /// Modified convexity of the projected cashflows, in years squared.
    #[getter]
    fn convexity(&self) -> f64 {
        self.inner.convexity
    }

    /// Price the z-spread/CS01 were solved against, as a percentage of
    /// original balance.
    #[getter]
    fn target_price_pct(&self) -> f64 {
        self.inner.target_price_pct
    }

    /// Return ``repr(self)``.
    fn __repr__(&self) -> String {
        format!(
            "TrancheMetrics(tranche_id={:?}, pv={}, price_pct={})",
            self.inner.tranche_id, self.inner.pv, self.inner.price_pct
        )
    }
}

// ---------------------------------------------------------------------------
// ScenarioTable
// ---------------------------------------------------------------------------

/// Scenario/yield table for a single structured-credit tranche
/// ([`structured_credit_tranche_scenario_table`]'s decoded return value).
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "ScenarioTable",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyScenarioTable {
    pub(crate) inner: ScenarioTable,
}

#[pymethods]
impl PyScenarioTable {
    /// Deserialize from the JSON returned by
    /// ``structured_credit_tranche_scenario_table``.
    ///
    /// Parameters
    /// ----------
    /// json : str
    ///     JSON-encoded ``ScenarioTable``.
    ///
    /// Returns
    /// -------
    /// ScenarioTable
    ///     The decoded scenario table.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``json`` is not valid JSON for the ``ScenarioTable`` shape.
    ///
    /// Examples
    /// --------
    /// >>> import json
    /// >>> from finstack_quant.valuations.instruments import ScenarioTable
    /// >>> payload = json.dumps({
    /// ...     "tranche_id": "A",
    /// ...     "cells": [{"cpr": 0.06, "cdr": 0.02, "severity": 0.6,
    /// ...                "price": 98.2, "wal": 4.1, "writedown": 0.0}],
    /// ... })
    /// >>> table = ScenarioTable.from_json(payload)
    /// >>> table.tranche_id
    /// 'A'
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: ScenarioTable = serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    /// Serialize back to the same JSON shape ``from_json`` accepts.
    ///
    /// Returns
    /// -------
    /// str
    ///     JSON-encoded ``ScenarioTable``.
    #[pyo3(text_signature = "($self)")]
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    /// Identifier of the tranche evaluated.
    #[getter]
    fn tranche_id(&self) -> &str {
        &self.inner.tranche_id
    }

    /// Evaluated cells, in CPR-major, then CDR, then severity order.
    ///
    /// Each cell is a dict with keys ``cpr`` (annual decimal), ``cdr``
    /// (annual decimal), ``severity`` (decimal), ``price`` (percentage of
    /// original balance), ``wal`` (years), and ``writedown`` (currency
    /// units).
    ///
    /// Returns
    /// -------
    /// list[dict[str, float]]
    ///     One dict per evaluated scenario cell.
    #[pyo3(text_signature = "($self)")]
    fn cells(&self) -> Vec<BTreeMap<String, f64>> {
        self.inner
            .cells
            .iter()
            .map(|cell| {
                let mut map = BTreeMap::new();
                map.insert("cpr".to_string(), cell.cpr);
                map.insert("cdr".to_string(), cell.cdr);
                map.insert("severity".to_string(), cell.severity);
                map.insert("price".to_string(), cell.price);
                map.insert("wal".to_string(), cell.wal);
                map.insert("writedown".to_string(), cell.writedown);
                map
            })
            .collect()
    }

    /// Return ``repr(self)``.
    fn __repr__(&self) -> String {
        format!(
            "ScenarioTable(tranche_id={:?}, cells={})",
            self.inner.tranche_id,
            self.inner.cells.len()
        )
    }
}

/// Names registered by this module, in the order they appear in `__all__`.
pub(crate) const EXPORTS: &[&str] = &[
    "OasResult",
    "ScenarioTable",
    "TrancheMetrics",
    "structured_credit_tranche_breakeven_cdr",
    "structured_credit_tranche_discount_margin",
    "structured_credit_tranche_metrics",
    "structured_credit_tranche_oas",
    "structured_credit_tranche_scenario_table",
];

/// Register the structured-credit tranche analytics on the instruments module.
pub(crate) fn register(parent: &Bound<'_, PyModule>) -> PyResult<()> {
    parent.add_function(wrap_pyfunction!(
        structured_credit_tranche_discount_margin,
        parent
    )?)?;
    parent.add_function(wrap_pyfunction!(
        structured_credit_tranche_breakeven_cdr,
        parent
    )?)?;
    parent.add_function(wrap_pyfunction!(structured_credit_tranche_oas, parent)?)?;
    parent.add_function(wrap_pyfunction!(structured_credit_tranche_metrics, parent)?)?;
    parent.add_function(wrap_pyfunction!(
        structured_credit_tranche_scenario_table,
        parent
    )?)?;
    parent.add_class::<PyOasResult>()?;
    parent.add_class::<PyTrancheMetrics>()?;
    parent.add_class::<PyScenarioTable>()?;
    for &name in EXPORTS {
        parent
            .getattr(name)?
            .setattr("__module__", "finstack_quant.valuations.instruments")?;
    }
    Ok(())
}
