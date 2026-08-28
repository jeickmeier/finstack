//! Instrument pricing pipeline: canonical instrument envelope + market → ValuationResult.

use super::PyValuationResult;
use crate::bindings::extract::{extract_instrument_json, extract_market};
use crate::errors::core_to_py;
use pyo3::prelude::*;

/// Price an instrument from its canonical envelope and return a ``ValuationResult``.
///
/// Parameters
/// ----------
/// instrument_json : str | Bond | TermLoan | InterestRateSwap | Swaption |
///     CapFloor | CreditDefaultSwap | CDSIndex | FxForward | FxOption |
///     CDSTranche | ConvertibleBond | EquityOption | StructuredCredit
///     A ``finstack_quant.instrument/1`` envelope or a typed
///     ``Bond`` / ``TermLoan`` / ``InterestRateSwap`` / ``Swaption`` /
///     ``CapFloor`` / ``CreditDefaultSwap`` / ``CDSIndex`` / ``FxForward`` /
///     ``FxOption`` / ``CDSTranche`` / ``ConvertibleBond`` / ``EquityOption`` /
///     ``StructuredCredit`` instance.
/// market : MarketContext | str
///     A ``MarketContext`` object or a JSON string.
/// as_of : datetime.date | str
///     Valuation date, either a date-like object (``datetime.date``,
///     ``pandas.Timestamp``) or an ISO 8601 string.
/// model : str
///     Model key: ``"default"`` (default), ``"discounting"``, ``"black76"``, ``"hazard_rate"``,
///     ``"hull_white_1f"``, ``"tree"``, ``"normal"``, ``"monte_carlo_gbm"``,
///     ``"bond_future_clean_price_proxy"``, etc.
/// metrics : list[str]
///     Optional metric identifiers to compute (e.g. ``["ytm", "dv01"]``).
///     Empty or omitted means valuation only.
/// pricing_options : str | None
///     Optional JSON string of ``MetricPricingOverrides`` merged into the instrument's
///     ``pricing_overrides`` before pricing.  Supported fields include
///     ``"theta_period"`` (e.g. ``"1D"``, ``"1W"``, ``"1M"``) and
///     ``"breakeven_config"`` (e.g. ``{"target": "z_spread", "mode": "linear"}``).
///     If omitted, the instrument's own overrides (if any) are used unchanged.
/// market_history : str | None
///     Optional JSON string of ``MarketHistory`` scenarios required by ``hvar`` and
///     ``expected_shortfall`` metrics.
///
/// Returns
/// -------
/// ValuationResult
///     Typed valuation envelope carrying value, currency, metrics, and
///     covenant flags.
///
/// Notes
/// -----
/// The wire payload is still one call away: ``result.to_json()`` returns the
/// JSON that ``ValuationResult.from_json`` accepts, for pipelines that
/// serialize results.
#[pyfunction]
#[pyo3(signature = (instrument_json, market, as_of, model="default", metrics=None, pricing_options=None, market_history=None))]
// PyO3 binding: the argument list mirrors the Python keyword-argument API, so
// it cannot be collapsed into a parameter struct without changing that API.
#[allow(clippy::too_many_arguments)]
fn price_instrument(
    py: Python<'_>,
    instrument_json: &Bound<'_, PyAny>,
    market: &Bound<'_, PyAny>,
    as_of: &Bound<'_, PyAny>,
    model: &str,
    metrics: Option<Vec<String>>,
    pricing_options: Option<&str>,
    market_history: Option<&str>,
) -> PyResult<PyValuationResult> {
    let instrument_json = extract_instrument_json(instrument_json)?;
    let pricing_options = pricing_options.map(str::to_owned);
    let instrument = py.detach(move || {
        finstack_quant_valuations::pricer::parse_boxed_instrument_json(
            &instrument_json,
            pricing_options.as_deref(),
        )
        .map_err(core_to_py)
    })?;
    let market = extract_market(py, market)?;
    let as_of = crate::bindings::date_utils::extract_date_iso(as_of)?;
    let model = model.to_owned();
    let metrics = metrics.unwrap_or_default();
    let market_history = market_history.map(str::to_owned);

    let inner = py
        .detach(move || {
            finstack_quant_valuations::pricer::price_instrument(
                &instrument,
                &market,
                &as_of,
                &model,
                &metrics,
                market_history.as_deref(),
                finstack_quant_valuations::instruments::PricingOptions::default()
                    .with_recalibration_provider(std::sync::Arc::new(
                        finstack_quant_calibration::recalibration::CachedRecalibrationProvider::new(
                        ),
                    )),
            )
        })
        .map_err(core_to_py)?;
    Ok(PyValuationResult { inner })
}

/// List all metric IDs in the standard metric registry.
///
/// Returns
/// -------
/// list[str]
///     All registered metric identifiers (sorted alphabetically).
#[pyfunction]
fn list_standard_metrics() -> Vec<String> {
    finstack_quant_valuations::pricer::list_standard_metrics()
}

/// List all standard metrics organized by group.
///
/// Returns a dict `{ group_name: [metric_id, ...], ... }` where each key
/// is a human-readable group name (e.g. "Pricing", "Greeks", "Sensitivity")
/// and the value is a sorted list of metric ID strings.
///
/// Returns
/// -------
/// dict[str, list[str]]
///     Metrics grouped by category.
#[pyfunction]
fn list_standard_metrics_grouped() -> std::collections::BTreeMap<String, Vec<String>> {
    finstack_quant_valuations::pricer::list_standard_metrics_grouped()
}

/// List every pricing model key registered in the standard pricer registry.
///
/// The list is registry-derived rather than enum-derived: it reflects real
/// dispatch coverage, so a model with no registered pricer is omitted. The
/// returned names are the canonical keys accepted by the ``model`` argument of
/// :func:`price_instrument`.
///
/// Returns
/// -------
/// list[str]
///     Canonical model keys (e.g. ``"discounting"``, ``"black76"``), sorted.
#[pyfunction]
fn list_models() -> Vec<String> {
    finstack_quant_valuations::pricer::list_models()
}

/// List the standard registry's pricing models grouped by instrument type.
///
/// Returns a dict ``{ instrument_type: [model_key, ...], ... }``. Only
/// instrument types with at least one registered pricer appear, and each entry
/// lists only the models that can actually price that instrument.
///
/// Returns
/// -------
/// dict[str, list[str]]
///     Model keys grouped by canonical instrument-type name.
#[pyfunction]
fn list_models_grouped() -> std::collections::BTreeMap<String, Vec<String>> {
    finstack_quant_valuations::pricer::list_models_grouped()
}

/// Return the maintained liquid listed-derivatives coverage catalog.
///
/// Parameters
/// ----------
/// exchange : str | None, optional
///     Exact venue filter: ``"cme"``, ``"eurex"``, ``"montreal"``, or
///     ``"sgx"``. ``None`` returns all venues.
///
/// Returns
/// -------
/// list[dict[str, object]]
///     Product-family rows with the canonical instrument type, exercised
///     features, source URL, and any residual modelling gap.
///
/// Raises
/// ------
/// ValueError
///     If ``exchange`` is not one of the accepted canonical venue names, or
///     if the embedded listed-product sidecar is invalid.
///
/// Examples
/// --------
/// >>> from finstack_quant.valuations.market import listed_product_catalog
/// >>> rows = listed_product_catalog("cme")
/// >>> all(row["exchange"] == "cme" for row in rows)
/// True
#[pyfunction(signature = (exchange=None))]
fn listed_product_catalog<'py>(
    py: Python<'py>,
    exchange: Option<&str>,
) -> PyResult<Bound<'py, PyAny>> {
    let exchange = exchange
        .map(str::parse::<finstack_quant_valuations::market::listed::ListedExchange>)
        .transpose()
        .map_err(pyo3::exceptions::PyValueError::new_err)?;
    let rows = finstack_quant_valuations::market::listed::listed_product_catalog(exchange)
        .map_err(core_to_py)?;
    crate::bindings::pandas_utils::serde_to_py(py, &rows)
}

/// Per-flow cashflow envelope (DF / survival / PV) for a discountable instrument.
///
/// Supported ``model`` values are ``"discounting"`` (DF-only PV) and
/// ``"hazard_rate"`` (DF × survival + recovery on principal). Any other model
/// key, or an instrument type that isn't priced under the chosen model in the
/// standard registry, raises ``ValueError``. For the supported combinations,
/// the returned envelope's ``total_pv`` reconciles with the instrument's
/// ``base_value``.
///
/// Parameters
/// ----------
/// instrument_json : str | Bond | TermLoan | InterestRateSwap | Swaption |
///     CapFloor | CreditDefaultSwap | CDSIndex | FxForward | FxOption |
///     CDSTranche | ConvertibleBond | EquityOption | StructuredCredit
///     A ``finstack_quant.instrument/1`` envelope or a typed ``Bond`` / ``TermLoan`` /
///     ``InterestRateSwap`` / ``Swaption`` / ``CapFloor`` /
///     ``CreditDefaultSwap`` / ``CDSIndex`` / ``FxForward`` / ``FxOption`` /
///     ``CDSTranche`` / ``ConvertibleBond`` / ``EquityOption`` /
///     ``StructuredCredit`` instance.
/// market : MarketContext | str
///     A ``MarketContext`` object or a JSON string.
/// as_of : datetime.date | str
///     Valuation date, either a date-like object (``datetime.date``,
///     ``pandas.Timestamp``) or an ISO 8601 string.
/// model : str
///     ``"discounting"`` or ``"hazard_rate"``. ``"default"`` is not accepted.
///
/// Returns
/// -------
/// str
///     JSON-serialized ``InstrumentCashflowEnvelope``. Parse and wrap in a
///     DataFrame via :func:`finstack_quant.valuations.instrument_cashflows`.
#[pyfunction]
fn instrument_cashflows_json(
    py: Python<'_>,
    instrument_json: &Bound<'_, PyAny>,
    market: &Bound<'_, PyAny>,
    as_of: &Bound<'_, PyAny>,
    model: &str,
) -> PyResult<String> {
    let instrument_json = extract_instrument_json(instrument_json)?;
    let instrument = py.detach(move || {
        finstack_quant_valuations::pricer::parse_boxed_instrument_json(&instrument_json, None)
            .map_err(core_to_py)
    })?;
    let market = extract_market(py, market)?;
    let as_of = crate::bindings::date_utils::extract_date_iso(as_of)?;
    let model = model.to_owned();

    py.detach(move || {
        finstack_quant_valuations::instruments::cashflow_export::instrument_cashflows(
            &instrument,
            &market,
            &as_of,
            &model,
        )
        .map_err(core_to_py)
    })
}

/// Register pricing functions on the valuations submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(price_instrument, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(list_models, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(list_models_grouped, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(list_standard_metrics, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(list_standard_metrics_grouped, m)?)?;
    m.add_function(pyo3::wrap_pyfunction!(instrument_cashflows_json, m)?)?;
    Ok(())
}

/// Register listed-market catalog functions on the valuations market submodule.
pub fn register_market(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(pyo3::wrap_pyfunction!(listed_product_catalog, m)?)?;
    Ok(())
}
