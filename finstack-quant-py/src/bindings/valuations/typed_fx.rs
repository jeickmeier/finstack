//! Typed FX instruments: `FxForward` and `FxOption`.
//! Mirrors the `PyInterestRateSwap` pattern in `typed_rates.rs`.

use pyo3::prelude::*;
use pyo3::types::PyType;

use crate::bindings::core::currency::PyCurrency;
use crate::bindings::core::dates::utils::py_to_date;
use crate::bindings::core::money::PyMoney;
use crate::errors::{core_to_py, value_error};
use finstack_quant_core::types::{CurveId, InstrumentId};
use finstack_quant_valuations::instruments::{Instrument, InstrumentJson};

use super::instruments::{
    enum_from_str, parse_typed_instrument_json, serialize_typed_instrument_json,
};

type FxForwardBuilderInner =
    finstack_quant_valuations::instruments::fx::fx_forward::FxForwardBuilder;
type FxOptionBuilderInner = finstack_quant_valuations::instruments::fx::fx_option::FxOptionBuilder;

// ---------------------------------------------------------------------------
// FxForward
// ---------------------------------------------------------------------------

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
    #[classmethod]
    #[pyo3(text_signature = "(cls, json)")]
    fn from_json(_cls: &Bound<'_, PyType>, json: &str) -> PyResult<Self> {
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

// ---------------------------------------------------------------------------
// FxOption
// ---------------------------------------------------------------------------

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
    #[classmethod]
    #[pyo3(text_signature = "(cls, json)")]
    fn from_json(_cls: &Bound<'_, PyType>, json: &str) -> PyResult<Self> {
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
