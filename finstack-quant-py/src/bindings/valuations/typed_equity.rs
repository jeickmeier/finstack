//! Typed equity instruments: `EquityOption`.
//! Mirrors the `PyInterestRateSwap` pattern in `typed_rates.rs`.

use pyo3::prelude::*;

use crate::bindings::core::dates::utils::py_to_date;
use crate::bindings::core::money::PyMoney;
use crate::errors::{core_to_py, value_error};
use finstack_quant_core::types::{CurveId, InstrumentId, PriceId};
use finstack_quant_valuations::instruments::{Instrument, InstrumentJson};

use super::instruments::{
    enum_from_str, parse_typed_instrument_json, serialize_typed_instrument_json,
};

type EquityOptionBuilderInner =
    finstack_quant_valuations::instruments::equity::equity_option::EquityOptionBuilder;

/// Typed wrapper for the Rust `EquityOption` instrument.
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "EquityOption",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyEquityOption {
    /// Inner canonical Rust equity option.
    pub(crate) inner: finstack_quant_valuations::instruments::EquityOption,
}

impl PyEquityOption {
    /// Serialize as the canonical instrument envelope accepted by the JSON loader.
    pub(crate) fn envelope_json(&self) -> PyResult<String> {
        serialize_typed_instrument_json(
            InstrumentJson::EquityOption(self.inner.clone()),
            "EquityOption",
        )
    }
}

#[pymethods]
impl PyEquityOption {
    /// Create a fluent builder (mirrors Rust ``EquityOption::builder()``).
    ///
    /// Returns
    /// -------
    /// EquityOptionBuilder
    ///     A builder with fluent, consuming setter methods.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import EquityOption
    /// >>> builder = EquityOption.builder()
    /// >>> builder.id("EXAMPLE") is builder
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn builder() -> PyEquityOptionBuilder {
        PyEquityOptionBuilder {
            inner: Some(finstack_quant_valuations::instruments::EquityOption::builder()),
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

    /// Deserialize a validated equity option from its canonical v1 envelope.
    ///
    /// Parameters
    /// ----------
    /// json : str
    ///     A ``finstack_quant.instrument/1`` envelope containing an exact
    ///     ``"equity_option"`` payload. The UTF-8 input must not exceed 16 MiB.
    ///     Bare payloads and cross-type coercion are rejected.
    ///
    /// Returns
    /// -------
    /// EquityOption
    ///     The validated equity option represented by the exact ``"equity_option"`` payload.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the input exceeds 16 MiB, is malformed, has an unsupported
    ///     envelope schema, carries another type, or fails equity-option
    ///     validation.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import EquityOption
    /// >>> try:
    /// ...     EquityOption.from_json("{}")
    /// ... except ValueError as exc:
    /// ...     print("schema" in str(exc))
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        match parse_typed_instrument_json(json)? {
            InstrumentJson::EquityOption(inner) => {
                inner.validate_for_pricing().map_err(core_to_py)?;
                Ok(Self { inner })
            }
            _ => Err(value_error(
                "expected instrument type \"equity_option\", got a different instrument type",
            )),
        }
    }

    /// Serialize to a canonical ``finstack_quant.instrument/1`` envelope.
    ///
    /// Returns
    /// -------
    /// str
    ///     Canonical instrument envelope accepted by ``price_instrument`` and
    ///     ``EquityOption.from_json``.
    #[pyo3(text_signature = "($self)")]
    fn to_json(&self) -> PyResult<String> {
        self.envelope_json()
    }

    /// Instrument identifier.
    #[getter]
    fn id(&self) -> String {
        self.inner.id.to_string()
    }

    /// Return ``repr(self)``.
    fn __repr__(&self) -> String {
        format!("EquityOption(id={:?})", self.inner.id.as_str())
    }
}

/// Fluent builder for [`PyEquityOption`]; wraps the Rust
/// `FinancialBuilder`-generated builder (consuming setters).
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "EquityOptionBuilder",
    skip_from_py_object
)]
pub struct PyEquityOptionBuilder {
    inner: Option<EquityOptionBuilderInner>,
}

/// Take the wrapped Rust builder or fail if `build()` already consumed it.
fn take_equity_option(b: &mut PyEquityOptionBuilder) -> PyResult<EquityOptionBuilderInner> {
    b.inner
        .take()
        .ok_or_else(|| value_error("builder already consumed by build()"))
}

#[pymethods]
impl PyEquityOptionBuilder {
    /// Set the instrument identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Unique identifier for the equity option.
    ///
    /// Returns
    /// -------
    /// EquityOptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn id<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_equity_option(&mut slf)?;
        slf.inner = Some(b.id(InstrumentId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the underlying equity ticker symbol.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Underlying equity ticker symbol.
    ///
    /// Returns
    /// -------
    /// EquityOptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn underlying_ticker<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_equity_option(&mut slf)?;
        slf.inner = Some(b.underlying_ticker(value.to_string()));
        Ok(slf)
    }

    /// Set the strike price.
    ///
    /// Parameters
    /// ----------
    /// value : float
    ///     Strike price. Must be finite and positive.
    ///
    /// Returns
    /// -------
    /// EquityOptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn strike<'py>(mut slf: PyRefMut<'py, Self>, value: f64) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_equity_option(&mut slf)?;
        slf.inner = Some(b.strike(value));
        Ok(slf)
    }

    /// Set the option type.
    ///
    /// Parameters
    /// ----------
    /// value : {"call", "put"}
    ///     Option type of the equity option.
    ///
    /// Returns
    /// -------
    /// EquityOptionBuilder
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
        let b = take_equity_option(&mut slf)?;
        slf.inner = Some(b.option_type(option_type));
        Ok(slf)
    }

    /// Set the exercise style.
    ///
    /// Parameters
    /// ----------
    /// value : {"european", "american", "bermudan"}
    ///     Exercise style of the equity option. Defaults to ``"european"``
    ///     when never set.
    ///
    /// Returns
    /// -------
    /// EquityOptionBuilder
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
        let b = take_equity_option(&mut slf)?;
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
    /// EquityOptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn expiry<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let expiry = py_to_date(value)?;
        let b = take_equity_option(&mut slf)?;
        slf.inner = Some(b.expiry(expiry));
        Ok(slf)
    }

    /// Set the notional amount for valuation scaling.
    ///
    /// Parameters
    /// ----------
    /// value : Money
    ///     Notional amount for valuation scaling.
    ///
    /// Returns
    /// -------
    /// EquityOptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyMoney>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_equity_option(&mut slf)?;
        slf.inner = Some(b.notional(value.inner));
        Ok(slf)
    }

    /// Set the discount curve identifier for present value calculations.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Discount curve identifier.
    ///
    /// Returns
    /// -------
    /// EquityOptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn discount_curve_id<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_equity_option(&mut slf)?;
        slf.inner = Some(b.discount_curve_id(CurveId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the equity spot price identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Equity spot price identifier.
    ///
    /// Returns
    /// -------
    /// EquityOptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn spot_id<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_equity_option(&mut slf)?;
        slf.inner = Some(b.spot_id(PriceId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the equity volatility surface identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Equity volatility surface identifier.
    ///
    /// Returns
    /// -------
    /// EquityOptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn vol_surface_id<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_equity_option(&mut slf)?;
        slf.inner = Some(b.vol_surface_id(CurveId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the continuous dividend yield identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Continuous dividend yield identifier. If never set, the pricer
    ///     treats the underlying as having zero continuous dividend yield.
    ///
    /// Returns
    /// -------
    /// EquityOptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn div_yield_id<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_equity_option(&mut slf)?;
        slf.inner = Some(b.div_yield_id(CurveId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the discrete dividend schedule.
    ///
    /// Parameters
    /// ----------
    /// value : list[tuple[datetime.date, float]]
    ///     Discrete dividend schedule as ``(ex_date, dividend_amount)`` pairs.
    ///     When provided, the escrowed dividend model is used for pricing.
    ///
    /// Returns
    /// -------
    /// EquityOptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn discrete_dividends<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: Vec<(Bound<'py, PyAny>, f64)>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let dividends = value
            .into_iter()
            .map(|(date, amount)| Ok((py_to_date(&date)?, amount)))
            .collect::<PyResult<Vec<_>>>()?;
        let b = take_equity_option(&mut slf)?;
        slf.inner = Some(b.discrete_dividends(dividends));
        Ok(slf)
    }

    /// Set the exercise schedule for Bermudan options.
    ///
    /// Parameters
    /// ----------
    /// value : list[datetime.date]
    ///     Dates on which early exercise is permitted. Required when
    ///     ``exercise_style`` is ``"bermudan"``.
    ///
    /// Returns
    /// -------
    /// EquityOptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn exercise_schedule<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: Vec<Bound<'py, PyAny>>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let dates = value.iter().map(py_to_date).collect::<PyResult<Vec<_>>>()?;
        let b = take_equity_option(&mut slf)?;
        slf.inner = Some(b.exercise_schedule(dates));
        Ok(slf)
    }

    /// Build the validated equity option.
    ///
    /// Returns
    /// -------
    /// EquityOption
    ///     The validated equity option.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the builder was already consumed, a required field is missing,
    ///     or the completed option fails pricing validation.
    #[pyo3(text_signature = "($self)")]
    fn build(mut slf: PyRefMut<'_, Self>) -> PyResult<PyEquityOption> {
        let b = take_equity_option(&mut slf)?;
        let inner = b.build().map_err(core_to_py)?;
        inner.validate_for_pricing().map_err(core_to_py)?;
        Ok(PyEquityOption { inner })
    }
}

/// Register the typed equity instruments on the instruments submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyEquityOption>()?;
    m.add_class::<PyEquityOptionBuilder>()?;
    Ok(())
}
