//! Typed FX instruments: `FxForward` and `FxOption`.
//! Mirrors the `PyInterestRateSwap` pattern in `typed_rates.rs`.
//!
//! Both classes also carry the pricing methods their WASM twins expose
//! (`price`, `price_with_metrics`, and — for `FxOption` — the standard Greek
//! accessors), delegating to the same canonical Rust pricer entry points.

use pyo3::prelude::*;
use pyo3::types::PyDict;

use crate::bindings::core::currency::PyCurrency;
use crate::bindings::core::dates::utils::py_to_date;
use crate::bindings::core::money::PyMoney;
use crate::bindings::extract::extract_market;
use crate::errors::{core_to_py, value_error};
use finstack_quant_core::types::{CurveId, InstrumentId};
use finstack_quant_valuations::instruments::{Instrument, InstrumentJson};

use super::instruments::{
    enum_from_str, parse_typed_instrument_json, serialize_typed_instrument_json,
};
use super::PyValuationResult;

/// Price a typed instrument envelope through the canonical Rust pricer.
fn price_envelope(
    py: Python<'_>,
    envelope_json: String,
    market: &Bound<'_, PyAny>,
    as_of: &Bound<'_, PyAny>,
    model: &str,
) -> PyResult<PyValuationResult> {
    let market = extract_market(py, market)?;
    let as_of = crate::bindings::date_utils::extract_date_iso(as_of)?;
    let model = model.to_owned();
    let inner = py
        .detach(move || {
            finstack_quant_valuations::pricer::price_instrument_json(
                &envelope_json,
                &market,
                &as_of,
                &model,
            )
        })
        .map_err(core_to_py)?;
    Ok(PyValuationResult { inner })
}

/// Price a typed instrument envelope with explicit metric requests.
// Mirrors the Python keyword-argument API of `price_instrument_with_metrics`.
#[allow(clippy::too_many_arguments)]
fn price_envelope_with_metrics(
    py: Python<'_>,
    envelope_json: String,
    market: &Bound<'_, PyAny>,
    as_of: &Bound<'_, PyAny>,
    model: &str,
    metrics: Vec<String>,
    pricing_options: Option<&str>,
    market_history: Option<&str>,
) -> PyResult<PyValuationResult> {
    let market = extract_market(py, market)?;
    let as_of = crate::bindings::date_utils::extract_date_iso(as_of)?;
    let model = model.to_owned();
    let pricing_options = pricing_options.map(str::to_owned);
    let market_history = market_history.map(str::to_owned);
    let inner = py
        .detach(move || {
            finstack_quant_valuations::pricer::price_instrument_json_with_metrics_and_history(
                &envelope_json,
                &market,
                &as_of,
                &model,
                &metrics,
                pricing_options.as_deref(),
                market_history.as_deref(),
            )
        })
        .map_err(core_to_py)?;
    Ok(PyValuationResult { inner })
}

/// Compute one scalar metric for a typed instrument envelope.
fn envelope_metric_value(
    py: Python<'_>,
    envelope_json: String,
    market: &Bound<'_, PyAny>,
    as_of: &Bound<'_, PyAny>,
    model: &str,
    metric: &'static str,
) -> PyResult<f64> {
    let market = extract_market(py, market)?;
    let as_of = crate::bindings::date_utils::extract_date_iso(as_of)?;
    let model = model.to_owned();
    py.detach(move || {
        finstack_quant_valuations::pricer::metric_value_from_instrument_json(
            &envelope_json,
            &market,
            &as_of,
            &model,
            metric,
        )
    })
    .map_err(core_to_py)
}

/// Compute the standard option Greek set for a typed instrument envelope.
///
/// Mirrors the WASM `greeks` method: non-finite Greeks are rejected rather
/// than returned, so both hosts fail identically instead of one silently
/// yielding `NaN`.
fn envelope_option_greeks<'py>(
    py: Python<'py>,
    envelope_json: String,
    market: &Bound<'py, PyAny>,
    as_of: &Bound<'py, PyAny>,
    model: &str,
) -> PyResult<Bound<'py, PyDict>> {
    let market = extract_market(py, market)?;
    let as_of = crate::bindings::date_utils::extract_date_iso(as_of)?;
    let model = model.to_owned();
    let pairs = py
        .detach(move || {
            finstack_quant_valuations::pricer::present_standard_option_greeks_from_instrument_json(
                &envelope_json,
                &market,
                &as_of,
                &model,
            )
        })
        .map_err(core_to_py)?;
    let out = PyDict::new(py);
    for (metric, value) in pairs {
        if !value.is_finite() {
            return Err(value_error(format!(
                "greek '{metric}' evaluated to a non-finite value ({value})"
            )));
        }
        out.set_item(metric, value)?;
    }
    Ok(out)
}

type FxForwardBuilderInner =
    finstack_quant_valuations::instruments::fx::fx_forward::FxForwardBuilder;
type FxOptionBuilderInner = finstack_quant_valuations::instruments::fx::fx_option::FxOptionBuilder;

// FxForward

/// Typed wrapper for the Rust `FxForward` instrument.
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "FxForward",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFxForward {
    /// Inner canonical Rust FX forward.
    pub(crate) inner: finstack_quant_valuations::instruments::FxForward,
}

impl PyFxForward {
    /// Serialize as the canonical instrument envelope accepted by the JSON loader.
    pub(crate) fn envelope_json(&self) -> PyResult<String> {
        serialize_typed_instrument_json(InstrumentJson::FxForward(self.inner.clone()), "FxForward")
    }
}

#[pymethods]
impl PyFxForward {
    /// Create a fluent builder (mirrors Rust ``FxForward::builder()``).
    ///
    /// Returns
    /// -------
    /// FxForwardBuilder
    ///     A builder with fluent, consuming setter methods.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import FxForward
    /// >>> builder = FxForward.builder()
    /// >>> builder.id("EXAMPLE") is builder
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn builder() -> PyFxForwardBuilder {
        PyFxForwardBuilder {
            inner: Some(finstack_quant_valuations::instruments::FxForward::builder()),
        }
    }

    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Deserialize a validated FX forward from its canonical v1 envelope.
    ///
    /// Parameters
    /// ----------
    /// json : str
    ///     A ``finstack_quant.instrument/1`` envelope containing an exact
    ///     ``"fx_forward"`` payload. The UTF-8 input must not exceed 16 MiB.
    ///     Bare payloads and cross-type coercion are rejected.
    ///
    /// Returns
    /// -------
    /// FxForward
    ///     The validated FX forward represented by the exact ``"fx_forward"`` payload.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the input exceeds 16 MiB, is malformed, has an unsupported
    ///     envelope schema, carries another type, or fails FX-forward validation.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import FxForward
    /// >>> try:
    /// ...     FxForward.from_json("{}")
    /// ... except ValueError as exc:
    /// ...     print("schema" in str(exc))
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        match parse_typed_instrument_json(json)? {
            InstrumentJson::FxForward(inner) => {
                inner.validate_for_pricing().map_err(core_to_py)?;
                Ok(Self { inner })
            }
            _ => Err(value_error(
                "expected instrument type \"fx_forward\", got a different instrument type",
            )),
        }
    }

    /// Serialize to a canonical ``finstack_quant.instrument/1`` envelope.
    ///
    /// Returns
    /// -------
    /// str
    ///     Canonical instrument envelope accepted by ``price_instrument`` and
    ///     ``FxForward.from_json``.
    #[pyo3(text_signature = "($self)")]
    fn to_json(&self) -> PyResult<String> {
        self.envelope_json()
    }

    /// Instrument identifier.
    #[getter]
    fn id(&self) -> String {
        self.inner.id.to_string()
    }

    /// Price this FX forward and return a typed ``ValuationResult``.
    ///
    /// Delegates to the same canonical Rust pricer entry point as
    /// ``price_instrument(self, market, as_of, model)``.
    ///
    /// Parameters
    /// ----------
    /// market : MarketContext | str
    ///     A ``MarketContext`` object or serialized market-context JSON.
    /// as_of : datetime.date | str
    ///     Valuation date, either a date-like object or an ISO 8601 string.
    /// model : str, optional
    ///     Model key (default ``"default"`` — the instrument-native model).
    ///
    /// Returns
    /// -------
    /// ValuationResult
    ///     Typed valuation envelope carrying value, currency, and metrics.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the market JSON, ``as_of``, or ``model`` is invalid, required
    ///     market data is missing, or the selected pricer fails.
    #[pyo3(signature = (market, as_of, model="default"))]
    fn price(
        &self,
        py: Python<'_>,
        market: &Bound<'_, PyAny>,
        as_of: &Bound<'_, PyAny>,
        model: &str,
    ) -> PyResult<PyValuationResult> {
        price_envelope(py, self.envelope_json()?, market, as_of, model)
    }

    /// Price this FX forward with explicit metric requests.
    ///
    /// Parameters
    /// ----------
    /// market : MarketContext | str
    ///     A ``MarketContext`` object or serialized market-context JSON.
    /// as_of : datetime.date | str
    ///     Valuation date, either a date-like object or an ISO 8601 string.
    /// model : str, optional
    ///     Model key (default ``"default"``).
    /// metrics : list[str], optional
    ///     Metric identifiers to compute (e.g. ``["dv01", "theta"]``).
    /// pricing_options : str | None
    ///     Optional JSON ``MetricPricingOverrides`` merged into the
    ///     instrument's ``pricing_overrides`` before pricing.
    /// market_history : str | None
    ///     Optional JSON ``MarketHistory`` scenarios required by ``hvar`` and
    ///     ``expected_shortfall`` metrics.
    ///
    /// Returns
    /// -------
    /// ValuationResult
    ///     Typed valuation envelope including the requested metrics.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If any input payload is invalid, required market data is missing,
    ///     or pricing or a metric calculation fails.
    #[pyo3(signature = (market, as_of, model="default", metrics=vec![], pricing_options=None, market_history=None))]
    #[allow(clippy::too_many_arguments)]
    fn price_with_metrics(
        &self,
        py: Python<'_>,
        market: &Bound<'_, PyAny>,
        as_of: &Bound<'_, PyAny>,
        model: &str,
        metrics: Vec<String>,
        pricing_options: Option<&str>,
        market_history: Option<&str>,
    ) -> PyResult<PyValuationResult> {
        price_envelope_with_metrics(
            py,
            self.envelope_json()?,
            market,
            as_of,
            model,
            metrics,
            pricing_options,
            market_history,
        )
    }

    /// Return ``repr(self)``.
    fn __repr__(&self) -> String {
        format!("FxForward(id={:?})", self.inner.id.as_str())
    }
}

/// Fluent builder for [`PyFxForward`]; wraps the Rust
/// `FinancialBuilder`-generated builder (consuming setters).
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "FxForwardBuilder",
    skip_from_py_object
)]
pub struct PyFxForwardBuilder {
    inner: Option<FxForwardBuilderInner>,
}

/// Take the wrapped Rust builder or fail if `build()` already consumed it.
fn take_fx_forward(b: &mut PyFxForwardBuilder) -> PyResult<FxForwardBuilderInner> {
    b.inner
        .take()
        .ok_or_else(|| value_error("builder already consumed by build()"))
}

#[pymethods]
impl PyFxForwardBuilder {
    /// Set the instrument identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Unique identifier for the FX forward.
    ///
    /// Returns
    /// -------
    /// FxForwardBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn id<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_fx_forward(&mut slf)?;
        slf.inner = Some(b.id(InstrumentId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the base currency (foreign currency, numerator of the pair).
    ///
    /// Parameters
    /// ----------
    /// value : Currency
    ///     Base (foreign) currency.
    ///
    /// Returns
    /// -------
    /// FxForwardBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn base_currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyCurrency>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_fx_forward(&mut slf)?;
        slf.inner = Some(b.base_currency(value.inner));
        Ok(slf)
    }

    /// Set the quote currency (domestic currency, denominator of the pair).
    ///
    /// Parameters
    /// ----------
    /// value : Currency
    ///     Quote (domestic) currency; also the PV currency.
    ///
    /// Returns
    /// -------
    /// FxForwardBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn quote_currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyCurrency>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_fx_forward(&mut slf)?;
        slf.inner = Some(b.quote_currency(value.inner));
        Ok(slf)
    }

    /// Set the maturity/settlement date.
    ///
    /// Parameters
    /// ----------
    /// value : datetime.date
    ///     Maturity/settlement date.
    ///
    /// Returns
    /// -------
    /// FxForwardBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn maturity<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let maturity = py_to_date(value)?;
        let b = take_fx_forward(&mut slf)?;
        slf.inner = Some(b.maturity(maturity));
        Ok(slf)
    }

    /// Set the notional amount in base currency.
    ///
    /// Parameters
    /// ----------
    /// value : Money
    ///     Notional amount, denominated in the base currency.
    ///
    /// Returns
    /// -------
    /// FxForwardBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyMoney>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_fx_forward(&mut slf)?;
        slf.inner = Some(b.notional(value.inner));
        Ok(slf)
    }

    /// Set the contract forward rate (quote per base).
    ///
    /// If not set, the forward is valued at-market (zero PV at inception).
    ///
    /// Parameters
    /// ----------
    /// value : float
    ///     Contract forward rate, quote currency per unit of base currency.
    ///
    /// Returns
    /// -------
    /// FxForwardBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn contract_rate<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: f64,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_fx_forward(&mut slf)?;
        slf.inner = Some(b.contract_rate(value));
        Ok(slf)
    }

    /// Set the domestic (quote currency) discount curve identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Domestic (quote currency) discount curve identifier.
    ///
    /// Returns
    /// -------
    /// FxForwardBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn domestic_discount_curve_id<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_fx_forward(&mut slf)?;
        slf.inner = Some(b.domestic_discount_curve_id(CurveId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the foreign (base currency) discount curve identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Foreign (base currency) discount curve identifier.
    ///
    /// Returns
    /// -------
    /// FxForwardBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn foreign_discount_curve_id<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_fx_forward(&mut slf)?;
        slf.inner = Some(b.foreign_discount_curve_id(CurveId::new(value.to_string())));
        Ok(slf)
    }

    /// Set an explicit spot rate override (quote per base).
    ///
    /// If not set, the spot rate is sourced from the market's FX matrix.
    ///
    /// Parameters
    /// ----------
    /// value : float
    ///     Spot FX rate, quote currency per unit of base currency.
    ///
    /// Returns
    /// -------
    /// FxForwardBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn spot_rate_override<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: f64,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_fx_forward(&mut slf)?;
        slf.inner = Some(b.spot_rate_override(value));
        Ok(slf)
    }

    /// Set the base currency calendar identifier for business day adjustment.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Base currency holiday calendar identifier.
    ///
    /// Returns
    /// -------
    /// FxForwardBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn base_calendar_id<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_fx_forward(&mut slf)?;
        slf.inner = Some(b.base_calendar_id(value.to_string()));
        Ok(slf)
    }

    /// Set the quote currency calendar identifier for business day adjustment.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Quote currency holiday calendar identifier.
    ///
    /// Returns
    /// -------
    /// FxForwardBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn quote_calendar_id<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_fx_forward(&mut slf)?;
        slf.inner = Some(b.quote_calendar_id(value.to_string()));
        Ok(slf)
    }

    /// Build the validated FX forward.
    ///
    /// Returns
    /// -------
    /// FxForward
    ///     The validated FX forward.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the builder was already consumed, a required field is missing,
    ///     or the completed FX forward fails pricing validation (for example,
    ///     ``base_currency`` equals ``quote_currency``).
    #[pyo3(text_signature = "($self)")]
    fn build(mut slf: PyRefMut<'_, Self>) -> PyResult<PyFxForward> {
        let b = take_fx_forward(&mut slf)?;
        let inner = b.build().map_err(core_to_py)?;
        inner.validate_for_pricing().map_err(core_to_py)?;
        Ok(PyFxForward { inner })
    }
}

// FxOption

/// Typed wrapper for the Rust `FxOption` instrument.
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "FxOption",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFxOption {
    /// Inner canonical Rust FX option.
    pub(crate) inner: finstack_quant_valuations::instruments::FxOption,
}

impl PyFxOption {
    /// Serialize as the canonical instrument envelope accepted by the JSON loader.
    pub(crate) fn envelope_json(&self) -> PyResult<String> {
        serialize_typed_instrument_json(InstrumentJson::FxOption(self.inner.clone()), "FxOption")
    }
}

#[pymethods]
impl PyFxOption {
    /// Create a fluent builder (mirrors Rust ``FxOption::builder()``).
    ///
    /// Returns
    /// -------
    /// FxOptionBuilder
    ///     A builder with fluent, consuming setter methods.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import FxOption
    /// >>> builder = FxOption.builder()
    /// >>> builder.id("EXAMPLE") is builder
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn builder() -> PyFxOptionBuilder {
        PyFxOptionBuilder {
            inner: Some(finstack_quant_valuations::instruments::FxOption::builder()),
        }
    }

    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Deserialize a validated FX option from its canonical v1 envelope.
    ///
    /// Parameters
    /// ----------
    /// json : str
    ///     A ``finstack_quant.instrument/1`` envelope containing an exact
    ///     ``"fx_option"`` payload. The UTF-8 input must not exceed 16 MiB.
    ///     Bare payloads and cross-type coercion are rejected.
    ///
    /// Returns
    /// -------
    /// FxOption
    ///     The validated FX option represented by the exact ``"fx_option"`` payload.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the input exceeds 16 MiB, is malformed, has an unsupported
    ///     envelope schema, carries another type, or fails FX-option validation.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import FxOption
    /// >>> try:
    /// ...     FxOption.from_json("{}")
    /// ... except ValueError as exc:
    /// ...     print("schema" in str(exc))
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        match parse_typed_instrument_json(json)? {
            InstrumentJson::FxOption(inner) => {
                inner.validate_for_pricing().map_err(core_to_py)?;
                Ok(Self { inner })
            }
            _ => Err(value_error(
                "expected instrument type \"fx_option\", got a different instrument type",
            )),
        }
    }

    /// Serialize to a canonical ``finstack_quant.instrument/1`` envelope.
    ///
    /// Returns
    /// -------
    /// str
    ///     Canonical instrument envelope accepted by ``price_instrument`` and
    ///     ``FxOption.from_json``.
    #[pyo3(text_signature = "($self)")]
    fn to_json(&self) -> PyResult<String> {
        self.envelope_json()
    }

    /// Instrument identifier.
    #[getter]
    fn id(&self) -> String {
        self.inner.id.to_string()
    }

    /// Price this FX option and return a typed ``ValuationResult``.
    ///
    /// Delegates to the same canonical Rust pricer entry point as
    /// ``price_instrument(self, market, as_of, model)``.
    ///
    /// Parameters
    /// ----------
    /// market : MarketContext | str
    ///     A ``MarketContext`` object or serialized market-context JSON.
    /// as_of : datetime.date | str
    ///     Valuation date, either a date-like object or an ISO 8601 string.
    /// model : str, optional
    ///     Model key (default ``"default"`` — the instrument-native model).
    ///
    /// Returns
    /// -------
    /// ValuationResult
    ///     Typed valuation envelope carrying value, currency, and metrics.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the market JSON, ``as_of``, or ``model`` is invalid, required
    ///     market data is missing, or the selected pricer fails.
    #[pyo3(signature = (market, as_of, model="default"))]
    fn price(
        &self,
        py: Python<'_>,
        market: &Bound<'_, PyAny>,
        as_of: &Bound<'_, PyAny>,
        model: &str,
    ) -> PyResult<PyValuationResult> {
        price_envelope(py, self.envelope_json()?, market, as_of, model)
    }

    /// Price this FX option with explicit metric requests.
    ///
    /// Parameters
    /// ----------
    /// market : MarketContext | str
    ///     A ``MarketContext`` object or serialized market-context JSON.
    /// as_of : datetime.date | str
    ///     Valuation date, either a date-like object or an ISO 8601 string.
    /// model : str, optional
    ///     Model key (default ``"default"``).
    /// metrics : list[str], optional
    ///     Metric identifiers to compute (e.g. ``["delta", "vega"]``).
    /// pricing_options : str | None
    ///     Optional JSON ``MetricPricingOverrides`` merged into the
    ///     instrument's ``pricing_overrides`` before pricing.
    /// market_history : str | None
    ///     Optional JSON ``MarketHistory`` scenarios required by ``hvar`` and
    ///     ``expected_shortfall`` metrics.
    ///
    /// Returns
    /// -------
    /// ValuationResult
    ///     Typed valuation envelope including the requested metrics.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If any input payload is invalid, required market data is missing,
    ///     or pricing or a metric calculation fails.
    #[pyo3(signature = (market, as_of, model="default", metrics=vec![], pricing_options=None, market_history=None))]
    #[allow(clippy::too_many_arguments)]
    fn price_with_metrics(
        &self,
        py: Python<'_>,
        market: &Bound<'_, PyAny>,
        as_of: &Bound<'_, PyAny>,
        model: &str,
        metrics: Vec<String>,
        pricing_options: Option<&str>,
        market_history: Option<&str>,
    ) -> PyResult<PyValuationResult> {
        price_envelope_with_metrics(
            py,
            self.envelope_json()?,
            market,
            as_of,
            model,
            metrics,
            pricing_options,
            market_history,
        )
    }

    /// Spot delta of the option.
    ///
    /// Returns
    /// -------
    /// float
    ///     Spot delta produced by the selected model.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If an input is invalid, required market data is missing, pricing
    ///     fails, or the model does not produce delta.
    #[pyo3(signature = (market, as_of, model="default"))]
    fn delta(
        &self,
        py: Python<'_>,
        market: &Bound<'_, PyAny>,
        as_of: &Bound<'_, PyAny>,
        model: &str,
    ) -> PyResult<f64> {
        envelope_metric_value(py, self.envelope_json()?, market, as_of, model, "delta")
    }

    /// Spot gamma of the option.
    ///
    /// Returns
    /// -------
    /// float
    ///     Spot gamma produced by the selected model.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If an input is invalid, required market data is missing, pricing
    ///     fails, or the model does not produce gamma.
    #[pyo3(signature = (market, as_of, model="default"))]
    fn gamma(
        &self,
        py: Python<'_>,
        market: &Bound<'_, PyAny>,
        as_of: &Bound<'_, PyAny>,
        model: &str,
    ) -> PyResult<f64> {
        envelope_metric_value(py, self.envelope_json()?, market, as_of, model, "gamma")
    }

    /// Vega of the option.
    ///
    /// Returns
    /// -------
    /// float
    ///     Vega produced by the selected model.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If an input is invalid, required market data is missing, pricing
    ///     fails, or the model does not produce vega.
    #[pyo3(signature = (market, as_of, model="default"))]
    fn vega(
        &self,
        py: Python<'_>,
        market: &Bound<'_, PyAny>,
        as_of: &Bound<'_, PyAny>,
        model: &str,
    ) -> PyResult<f64> {
        envelope_metric_value(py, self.envelope_json()?, market, as_of, model, "vega")
    }

    /// Theta of the option.
    ///
    /// Returns
    /// -------
    /// float
    ///     Theta produced by the selected model.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If an input is invalid, required market data is missing, pricing
    ///     fails, or the model does not produce theta.
    #[pyo3(signature = (market, as_of, model="default"))]
    fn theta(
        &self,
        py: Python<'_>,
        market: &Bound<'_, PyAny>,
        as_of: &Bound<'_, PyAny>,
        model: &str,
    ) -> PyResult<f64> {
        envelope_metric_value(py, self.envelope_json()?, market, as_of, model, "theta")
    }

    /// Domestic-rate rho of the option.
    ///
    /// Returns
    /// -------
    /// float
    ///     Domestic-rate rho produced by the selected model.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If an input is invalid, required market data is missing, pricing
    ///     fails, or the model does not produce rho.
    #[pyo3(signature = (market, as_of, model="default"))]
    fn rho(
        &self,
        py: Python<'_>,
        market: &Bound<'_, PyAny>,
        as_of: &Bound<'_, PyAny>,
        model: &str,
    ) -> PyResult<f64> {
        envelope_metric_value(py, self.envelope_json()?, market, as_of, model, "rho")
    }

    /// Foreign-rate rho of the option.
    ///
    /// Returns
    /// -------
    /// float
    ///     Foreign-rate rho produced by the selected model.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If an input is invalid, required market data is missing, pricing
    ///     fails, or the model does not produce foreign rho.
    #[pyo3(signature = (market, as_of, model="default"))]
    fn foreign_rho(
        &self,
        py: Python<'_>,
        market: &Bound<'_, PyAny>,
        as_of: &Bound<'_, PyAny>,
        model: &str,
    ) -> PyResult<f64> {
        envelope_metric_value(
            py,
            self.envelope_json()?,
            market,
            as_of,
            model,
            "foreign_rho",
        )
    }

    /// Vanna of the option.
    ///
    /// Returns
    /// -------
    /// float
    ///     Vanna produced by the selected model.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If an input is invalid, required market data is missing, pricing
    ///     fails, or the model does not produce vanna.
    #[pyo3(signature = (market, as_of, model="default"))]
    fn vanna(
        &self,
        py: Python<'_>,
        market: &Bound<'_, PyAny>,
        as_of: &Bound<'_, PyAny>,
        model: &str,
    ) -> PyResult<f64> {
        envelope_metric_value(py, self.envelope_json()?, market, as_of, model, "vanna")
    }

    /// Volga of the option.
    ///
    /// Returns
    /// -------
    /// float
    ///     Volga produced by the selected model.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If an input is invalid, required market data is missing, pricing
    ///     fails, or the model does not produce volga.
    #[pyo3(signature = (market, as_of, model="default"))]
    fn volga(
        &self,
        py: Python<'_>,
        market: &Bound<'_, PyAny>,
        as_of: &Bound<'_, PyAny>,
        model: &str,
    ) -> PyResult<f64> {
        envelope_metric_value(py, self.envelope_json()?, market, as_of, model, "volga")
    }

    /// Compute the standard FX option Greek set as a dict.
    ///
    /// Mirrors the WASM ``greeks`` method: Greeks the selected model cannot
    /// produce are omitted, and any non-finite Greek raises rather than being
    /// returned.
    ///
    /// Returns
    /// -------
    /// dict[str, float]
    ///     Mapping of Greek name to value for every Greek the model produced.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If an input is invalid, required market data is missing, pricing
    ///     fails, or a returned Greek is non-finite.
    #[pyo3(signature = (market, as_of, model="default"))]
    fn greeks<'py>(
        &self,
        py: Python<'py>,
        market: &Bound<'py, PyAny>,
        as_of: &Bound<'py, PyAny>,
        model: &str,
    ) -> PyResult<Bound<'py, PyDict>> {
        envelope_option_greeks(py, self.envelope_json()?, market, as_of, model)
    }

    /// Return ``repr(self)``.
    fn __repr__(&self) -> String {
        format!("FxOption(id={:?})", self.inner.id.as_str())
    }
}

/// Fluent builder for [`PyFxOption`]; wraps the Rust
/// `FinancialBuilder`-generated builder (consuming setters).
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "FxOptionBuilder",
    skip_from_py_object
)]
pub struct PyFxOptionBuilder {
    inner: Option<FxOptionBuilderInner>,
}

/// Take the wrapped Rust builder or fail if `build()` already consumed it.
fn take_fx_option(b: &mut PyFxOptionBuilder) -> PyResult<FxOptionBuilderInner> {
    b.inner
        .take()
        .ok_or_else(|| value_error("builder already consumed by build()"))
}

#[pymethods]
impl PyFxOptionBuilder {
    /// Set the instrument identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Unique identifier for the FX option.
    ///
    /// Returns
    /// -------
    /// FxOptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn id<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_fx_option(&mut slf)?;
        slf.inner = Some(b.id(InstrumentId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the base currency (foreign currency).
    ///
    /// Parameters
    /// ----------
    /// value : Currency
    ///     Base (foreign) currency.
    ///
    /// Returns
    /// -------
    /// FxOptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn base_currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyCurrency>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_fx_option(&mut slf)?;
        slf.inner = Some(b.base_currency(value.inner));
        Ok(slf)
    }

    /// Set the quote currency (domestic currency).
    ///
    /// Parameters
    /// ----------
    /// value : Currency
    ///     Quote (domestic) currency.
    ///
    /// Returns
    /// -------
    /// FxOptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn quote_currency<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyCurrency>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_fx_option(&mut slf)?;
        slf.inner = Some(b.quote_currency(value.inner));
        Ok(slf)
    }

    /// Set the strike exchange rate (quote per base).
    ///
    /// Parameters
    /// ----------
    /// value : float
    ///     Strike exchange rate, quote currency per unit of base currency.
    ///
    /// Returns
    /// -------
    /// FxOptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn strike<'py>(mut slf: PyRefMut<'py, Self>, value: f64) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_fx_option(&mut slf)?;
        slf.inner = Some(b.strike(value));
        Ok(slf)
    }

    /// Set the option type: ``"call"`` or ``"put"`` on base currency.
    ///
    /// Parameters
    /// ----------
    /// value : {"call", "put"}
    ///     Option type of the FX option.
    ///
    /// Returns
    /// -------
    /// FxOptionBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not a recognized option type.
    #[pyo3(text_signature = "($self, value)")]
    fn option_type<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let option_type = enum_from_str(value, "option_type")?;
        let b = take_fx_option(&mut slf)?;
        slf.inner = Some(b.option_type(option_type));
        Ok(slf)
    }

    /// Set the exercise style.
    ///
    /// Parameters
    /// ----------
    /// value : {"european", "american", "bermudan"}
    ///     Exercise style of the FX option. Only ``"european"`` is
    ///     currently priceable; ``"american"`` and ``"bermudan"`` are
    ///     accepted here but rejected with a ``ValueError`` at pricing time
    ///     (specialized pricers are not yet implemented).
    ///
    /// Returns
    /// -------
    /// FxOptionBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not a recognized exercise style.
    #[pyo3(text_signature = "($self, value)")]
    fn exercise_style<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let exercise_style = enum_from_str(value, "exercise_style")?;
        let b = take_fx_option(&mut slf)?;
        slf.inner = Some(b.exercise_style(exercise_style));
        Ok(slf)
    }

    /// Set the option expiry date.
    ///
    /// Parameters
    /// ----------
    /// value : datetime.date
    ///     Option expiry date.
    ///
    /// Returns
    /// -------
    /// FxOptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn expiry<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let expiry = py_to_date(value)?;
        let b = take_fx_option(&mut slf)?;
        slf.inner = Some(b.expiry(expiry));
        Ok(slf)
    }

    /// Set the notional amount in base currency.
    ///
    /// Parameters
    /// ----------
    /// value : Money
    ///     Notional amount, denominated in the base currency.
    ///
    /// Returns
    /// -------
    /// FxOptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyMoney>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_fx_option(&mut slf)?;
        slf.inner = Some(b.notional(value.inner));
        Ok(slf)
    }

    /// Set the domestic currency discount curve identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Domestic currency discount curve identifier.
    ///
    /// Returns
    /// -------
    /// FxOptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn domestic_discount_curve_id<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_fx_option(&mut slf)?;
        slf.inner = Some(b.domestic_discount_curve_id(CurveId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the foreign currency discount curve identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Foreign currency discount curve identifier.
    ///
    /// Returns
    /// -------
    /// FxOptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn foreign_discount_curve_id<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_fx_option(&mut slf)?;
        slf.inner = Some(b.foreign_discount_curve_id(CurveId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the FX volatility surface identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     FX volatility surface identifier for option pricing.
    ///
    /// Returns
    /// -------
    /// FxOptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn vol_surface_id<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_fx_option(&mut slf)?;
        slf.inner = Some(b.vol_surface_id(CurveId::new(value.to_string())));
        Ok(slf)
    }

    /// Build the validated FX option.
    ///
    /// Returns
    /// -------
    /// FxOption
    ///     The validated FX option.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the builder was already consumed, a required field is missing,
    ///     or the completed FX option fails pricing validation (for example,
    ///     ``base_currency`` equals ``quote_currency``).
    #[pyo3(text_signature = "($self)")]
    fn build(mut slf: PyRefMut<'_, Self>) -> PyResult<PyFxOption> {
        let b = take_fx_option(&mut slf)?;
        let inner = b.build().map_err(core_to_py)?;
        inner.validate_for_pricing().map_err(core_to_py)?;
        Ok(PyFxOption { inner })
    }
}

/// Register the typed FX instruments on the instruments submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyFxForward>()?;
    m.add_class::<PyFxForwardBuilder>()?;
    m.add_class::<PyFxOption>()?;
    m.add_class::<PyFxOptionBuilder>()?;
    Ok(())
}
