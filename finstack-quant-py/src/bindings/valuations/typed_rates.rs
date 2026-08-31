//! Typed rates instruments: `InterestRateSwap`, `Swaption` and `CapFloor`.
//! Mirrors the `PyBond` pattern in `instruments.rs`.

use pyo3::prelude::*;

use crate::bindings::core::dates::daycount::PyDayCount;
use crate::bindings::core::dates::tenor::PyTenor;
use crate::bindings::core::dates::utils::py_to_date;
use crate::bindings::core::money::PyMoney;
use crate::errors::{core_to_py, value_error};
use finstack_quant_core::types::{CalendarId, CurveId, InstrumentId};
use finstack_quant_valuations::instruments::{Instrument, InstrumentJson};

use super::instruments::{
    decimal_from_f64, enum_from_str, json_field, parse_typed_instrument_json,
    serialize_typed_instrument_json,
};
use super::typed_legs::{PyFixedLegSpec, PyFloatLegSpec};

type IrsBuilder = finstack_quant_valuations::instruments::rates::irs::InterestRateSwapBuilder;
type SwaptionBuilderInner =
    finstack_quant_valuations::instruments::rates::swaption::SwaptionBuilder;
type CapFloorBuilderInner =
    finstack_quant_valuations::instruments::rates::cap_floor::CapFloorBuilder;

/// Typed wrapper for the Rust `InterestRateSwap` instrument.
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "InterestRateSwap",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyInterestRateSwap {
    /// Inner canonical Rust swap.
    pub(crate) inner: finstack_quant_valuations::instruments::InterestRateSwap,
}

impl PyInterestRateSwap {
    /// Serialize as the canonical instrument envelope accepted by the JSON loader.
    pub(crate) fn envelope_json(&self) -> PyResult<String> {
        serialize_typed_instrument_json(
            InstrumentJson::InterestRateSwap(self.inner.clone()),
            "InterestRateSwap",
        )
    }
}

#[pymethods]
impl PyInterestRateSwap {
    /// Create a fluent builder (mirrors Rust ``InterestRateSwap::builder()``).
    ///
    /// Returns
    /// -------
    /// InterestRateSwapBuilder
    ///     A builder with fluent, consuming setter methods.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import InterestRateSwap
    /// >>> builder = InterestRateSwap.builder()
    /// >>> builder.id("EXAMPLE") is builder
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn builder() -> PyInterestRateSwapBuilder {
        PyInterestRateSwapBuilder {
            inner: Some(finstack_quant_valuations::instruments::InterestRateSwap::builder()),
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

    /// Deserialize a validated swap from its canonical v1 envelope.
    ///
    /// Parameters
    /// ----------
    /// json : str
    ///     A ``finstack_quant.instrument/1`` envelope containing an exact
    ///     ``"interest_rate_swap"`` payload. The UTF-8 input must not exceed
    ///     16 MiB. Bare payloads and cross-type coercion are rejected.
    ///
    /// Returns
    /// -------
    /// InterestRateSwap
    ///     The validated swap represented by the exact ``"interest_rate_swap"`` payload.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the input exceeds 16 MiB, is malformed, has an unsupported
    ///     envelope schema, carries another type, or fails swap validation.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import InterestRateSwap
    /// >>> try:
    /// ...     InterestRateSwap.from_json("{}")
    /// ... except ValueError as exc:
    /// ...     print("schema" in str(exc))
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        match parse_typed_instrument_json(json)? {
            InstrumentJson::InterestRateSwap(inner) => Ok(Self { inner }),
            _ => Err(value_error(
                "expected instrument type \"interest_rate_swap\", got a different instrument type",
            )),
        }
    }

    /// Serialize to a canonical ``finstack_quant.instrument/1`` envelope.
    ///
    /// Returns
    /// -------
    /// str
    ///     Canonical instrument envelope accepted by ``price_instrument`` and
    ///     ``InterestRateSwap.from_json``.
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
        format!("InterestRateSwap(id={:?})", self.inner.id.as_str())
    }
}

/// Fluent builder for [`PyInterestRateSwap`]; wraps the Rust
/// `FinancialBuilder`-generated builder (consuming setters).
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "InterestRateSwapBuilder",
    skip_from_py_object
)]
pub struct PyInterestRateSwapBuilder {
    inner: Option<IrsBuilder>,
}

/// Take the wrapped Rust builder or fail if `build()` already consumed it.
fn take_irs(b: &mut PyInterestRateSwapBuilder) -> PyResult<IrsBuilder> {
    b.inner
        .take()
        .ok_or_else(|| value_error("builder already consumed by build()"))
}

#[pymethods]
impl PyInterestRateSwapBuilder {
    /// Set the instrument identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Unique identifier for the swap.
    ///
    /// Returns
    /// -------
    /// InterestRateSwapBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn id<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_irs(&mut slf)?;
        slf.inner = Some(b.id(finstack_quant_core::types::InstrumentId::new(
            value.to_string(),
        )));
        Ok(slf)
    }

    /// Set the notional (both legs).
    ///
    /// Parameters
    /// ----------
    /// value : Money
    ///     Notional amount shared by both legs.
    ///
    /// Returns
    /// -------
    /// InterestRateSwapBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyMoney>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_irs(&mut slf)?;
        slf.inner = Some(b.notional(value.inner));
        Ok(slf)
    }

    /// Set the swap direction: ``"pay"`` or ``"receive"`` (fixed leg).
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     ``"pay"`` to pay fixed/receive floating, ``"receive"`` for the
    ///     opposite.
    ///
    /// Returns
    /// -------
    /// InterestRateSwapBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not a recognized side.
    #[pyo3(text_signature = "($self, value)")]
    fn side<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let side = enum_from_str(value, "side")?;
        let b = take_irs(&mut slf)?;
        slf.inner = Some(b.side(side));
        Ok(slf)
    }

    /// Set the fixed leg specification.
    ///
    /// Parameters
    /// ----------
    /// value : FixedLegSpec
    ///     Fixed leg specification.
    ///
    /// Returns
    /// -------
    /// InterestRateSwapBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn fixed<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyFixedLegSpec>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_irs(&mut slf)?;
        slf.inner = Some(b.fixed(value.inner.clone()));
        Ok(slf)
    }

    /// Set the floating leg specification.
    ///
    /// Parameters
    /// ----------
    /// value : FloatLegSpec
    ///     Floating leg specification.
    ///
    /// Returns
    /// -------
    /// InterestRateSwapBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn float<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyFloatLegSpec>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_irs(&mut slf)?;
        slf.inner = Some(b.float(value.inner.clone()));
        Ok(slf)
    }

    /// Build the validated swap.
    ///
    /// Returns
    /// -------
    /// InterestRateSwap
    ///     The validated swap.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the builder was already consumed, a required field is missing,
    ///     or the completed swap fails pricing validation.
    #[pyo3(text_signature = "($self)")]
    fn build(mut slf: PyRefMut<'_, Self>) -> PyResult<PyInterestRateSwap> {
        let b = take_irs(&mut slf)?;
        let inner = b.build().map_err(core_to_py)?;
        inner.validate_for_pricing().map_err(core_to_py)?;
        Ok(PyInterestRateSwap { inner })
    }
}

/// Typed wrapper for the Rust `Swaption` instrument.
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "Swaption",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PySwaption {
    /// Inner canonical Rust swaption.
    pub(crate) inner: finstack_quant_valuations::instruments::Swaption,
}

impl PySwaption {
    /// Serialize as the canonical instrument envelope accepted by the JSON loader.
    pub(crate) fn envelope_json(&self) -> PyResult<String> {
        serialize_typed_instrument_json(InstrumentJson::Swaption(self.inner.clone()), "Swaption")
    }
}

#[pymethods]
impl PySwaption {
    /// Create a fluent builder (mirrors Rust ``Swaption::builder()``).
    ///
    /// Returns
    /// -------
    /// SwaptionBuilder
    ///     A builder with fluent, consuming setter methods.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import Swaption
    /// >>> builder = Swaption.builder()
    /// >>> builder.id("EXAMPLE") is builder
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn builder() -> PySwaptionBuilder {
        PySwaptionBuilder {
            inner: Some(finstack_quant_valuations::instruments::Swaption::builder()),
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

    /// Deserialize a validated swaption from its canonical v1 envelope.
    ///
    /// Parameters
    /// ----------
    /// json : str
    ///     A ``finstack_quant.instrument/1`` envelope containing an exact
    ///     ``"swaption"`` payload. The UTF-8 input must not exceed 16 MiB.
    ///     Bare payloads and cross-type coercion are rejected.
    ///
    /// Returns
    /// -------
    /// Swaption
    ///     The validated swaption represented by the exact ``"swaption"`` payload.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the input exceeds 16 MiB, is malformed, has an unsupported
    ///     envelope schema, carries another type, or fails swaption validation.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import Swaption
    /// >>> try:
    /// ...     Swaption.from_json("{}")
    /// ... except ValueError as exc:
    /// ...     print("schema" in str(exc))
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        match parse_typed_instrument_json(json)? {
            InstrumentJson::Swaption(inner) => Ok(Self { inner }),
            _ => Err(value_error(
                "expected instrument type \"swaption\", got a different instrument type",
            )),
        }
    }

    /// Serialize to a canonical ``finstack_quant.instrument/1`` envelope.
    ///
    /// Returns
    /// -------
    /// str
    ///     Canonical instrument envelope accepted by ``price_instrument`` and
    ///     ``Swaption.from_json``.
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
        format!("Swaption(id={:?})", self.inner.id.as_str())
    }
}

/// Fluent builder for [`PySwaption`]; wraps the Rust
/// `FinancialBuilder`-generated builder (consuming setters).
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "SwaptionBuilder",
    skip_from_py_object
)]
pub struct PySwaptionBuilder {
    inner: Option<SwaptionBuilderInner>,
}

/// Take the wrapped Rust builder or fail if `build()` already consumed it.
fn take_swaption(b: &mut PySwaptionBuilder) -> PyResult<SwaptionBuilderInner> {
    b.inner
        .take()
        .ok_or_else(|| value_error("builder already consumed by build()"))
}

#[pymethods]
impl PySwaptionBuilder {
    /// Set the instrument identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Unique identifier for the swaption.
    ///
    /// Returns
    /// -------
    /// SwaptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn id<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_swaption(&mut slf)?;
        slf.inner = Some(b.id(InstrumentId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the option type: ``"call"`` (payer) or ``"put"`` (receiver).
    ///
    /// Parameters
    /// ----------
    /// value : {"call", "put"}
    ///     Option type of the swaption.
    ///
    /// Returns
    /// -------
    /// SwaptionBuilder
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
        let b = take_swaption(&mut slf)?;
        slf.inner = Some(b.option_type(option_type));
        Ok(slf)
    }

    /// Set the notional amount of the underlying swap.
    ///
    /// Parameters
    /// ----------
    /// value : Money
    ///     Notional amount of the underlying swap.
    ///
    /// Returns
    /// -------
    /// SwaptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyMoney>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_swaption(&mut slf)?;
        slf.inner = Some(b.notional(value.inner));
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
    /// SwaptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn expiry<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let expiry = py_to_date(value)?;
        let b = take_swaption(&mut slf)?;
        slf.inner = Some(b.expiry(expiry));
        Ok(slf)
    }

    /// Set the exercise style.
    ///
    /// Parameters
    /// ----------
    /// value : {"european", "bermudan", "american"}
    ///     Exercise style of the swaption.
    ///
    /// Returns
    /// -------
    /// SwaptionBuilder
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
        let b = take_swaption(&mut slf)?;
        slf.inner = Some(b.exercise_style(exercise_style));
        Ok(slf)
    }

    /// Set the settlement method.
    ///
    /// Parameters
    /// ----------
    /// value : {"physical", "cash"}
    ///     Settlement method of the swaption.
    ///
    /// Returns
    /// -------
    /// SwaptionBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not a recognized settlement method.
    #[pyo3(text_signature = "($self, value)")]
    fn settlement<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let settlement = enum_from_str(value, "settlement")?;
        let b = take_swaption(&mut slf)?;
        slf.inner = Some(b.settlement(settlement));
        Ok(slf)
    }

    /// Set the cash settlement annuity method.
    ///
    /// Only affects pricing when ``settlement`` is ``"cash"``.
    ///
    /// Parameters
    /// ----------
    /// value : {"collateralized_cash_price", "par_yield", "isda_par_par", "zero_coupon"}
    ///     Cash settlement annuity method. ``"collateralized_cash_price"`` is
    ///     the default and discounts the physical fixed-leg annuity.
    ///
    /// Returns
    /// -------
    /// SwaptionBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not a recognized cash settlement method.
    #[pyo3(text_signature = "($self, value)")]
    fn cash_settlement_method<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let method = enum_from_str(value, "cash_settlement_method")?;
        let b = take_swaption(&mut slf)?;
        slf.inner = Some(b.cash_settlement_method(method));
        Ok(slf)
    }

    /// Set the volatility model.
    ///
    /// Parameters
    /// ----------
    /// value : {"black", "normal"}
    ///     Volatility model used for pricing.
    ///
    /// Returns
    /// -------
    /// SwaptionBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not a recognized volatility model.
    #[pyo3(text_signature = "($self, value)")]
    fn vol_model<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let vol_model = enum_from_str(value, "vol_model")?;
        let b = take_swaption(&mut slf)?;
        slf.inner = Some(b.vol_model(vol_model));
        Ok(slf)
    }

    /// Set the volatility surface identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Volatility surface identifier for option pricing.
    ///
    /// Returns
    /// -------
    /// SwaptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn vol_surface_id<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_swaption(&mut slf)?;
        slf.inner = Some(b.vol_surface_id(CurveId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the complete fixed leg of the underlying swap.
    ///
    /// Parameters
    /// ----------
    /// value : FixedLegSpec
    ///     Fixed leg of the underlying swap.
    ///
    /// Returns
    /// -------
    /// SwaptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn underlying_fixed_leg<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyFixedLegSpec>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_swaption(&mut slf)?;
        slf.inner = Some(b.underlying_fixed_leg(value.inner.clone()));
        Ok(slf)
    }

    /// Set the complete floating leg of the underlying swap.
    ///
    /// Parameters
    /// ----------
    /// value : FloatLegSpec
    ///     Floating leg of the underlying swap.
    ///
    /// Returns
    /// -------
    /// SwaptionBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn underlying_float_leg<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyFloatLegSpec>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_swaption(&mut slf)?;
        slf.inner = Some(b.underlying_float_leg(value.inner.clone()));
        Ok(slf)
    }

    /// Set the SABR volatility model parameters from a JSON object.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     JSON-encoded SABR parameters object with fields ``alpha``,
    ///     ``beta``, ``nu``, ``rho`` and optional ``shift``.
    ///
    /// Returns
    /// -------
    /// SwaptionBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not valid JSON for the SABR parameters shape.
    #[pyo3(text_signature = "($self, value)")]
    fn sabr_params_json<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let sabr_params: finstack_quant_models::volatility::SabrParameters =
            json_field(value, "sabr_params")?;
        let b = take_swaption(&mut slf)?;
        slf.inner = Some(b.sabr_params(sabr_params));
        Ok(slf)
    }

    /// Build the validated swaption.
    ///
    /// Returns
    /// -------
    /// Swaption
    ///     The validated swaption.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the builder was already consumed, a required field is missing,
    ///     or the completed swaption fails pricing validation.
    #[pyo3(text_signature = "($self)")]
    fn build(mut slf: PyRefMut<'_, Self>) -> PyResult<PySwaption> {
        let b = take_swaption(&mut slf)?;
        let inner = b.build().map_err(core_to_py)?;
        inner.validate_for_pricing().map_err(core_to_py)?;
        Ok(PySwaption { inner })
    }
}

/// Typed wrapper for the Rust `CapFloor` instrument.
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "CapFloor",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyCapFloor {
    /// Inner canonical Rust cap/floor.
    pub(crate) inner: finstack_quant_valuations::instruments::CapFloor,
}

impl PyCapFloor {
    /// Serialize as the canonical instrument envelope accepted by the JSON loader.
    pub(crate) fn envelope_json(&self) -> PyResult<String> {
        serialize_typed_instrument_json(InstrumentJson::CapFloor(self.inner.clone()), "CapFloor")
    }
}

#[pymethods]
impl PyCapFloor {
    /// Create a fluent builder (mirrors Rust ``CapFloor::builder()``).
    ///
    /// Returns
    /// -------
    /// CapFloorBuilder
    ///     A builder with fluent, consuming setter methods.
    ///
    /// Notes
    /// -----
    /// This factory does not raise; it returns a new instance with the documented defaults.
    /// Unset ``vol_type`` defaults to ``"auto"``: the surface is treated as
    /// a lognormal quote. Each caplet uses Black-76 when forward and strike
    /// are positive; otherwise the lognormal vol is converted to an
    /// equivalent normal vol and priced with Bachelier. A normal-vol
    /// surface must set ``vol_type`` to ``"normal"``.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import CapFloor
    /// >>> builder = CapFloor.builder()
    /// >>> builder.id("EXAMPLE") is builder
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn builder() -> PyCapFloorBuilder {
        PyCapFloorBuilder {
            inner: Some(finstack_quant_valuations::instruments::CapFloor::builder()),
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

    /// Deserialize a validated cap/floor from its canonical v1 envelope.
    ///
    /// Parameters
    /// ----------
    /// json : str
    ///     A ``finstack_quant.instrument/1`` envelope containing an exact
    ///     ``"cap_floor"`` payload. The UTF-8 input must not exceed 16 MiB.
    ///     Bare payloads and cross-type coercion are rejected.
    ///
    /// Returns
    /// -------
    /// CapFloor
    ///     The validated cap/floor represented by the exact ``"cap_floor"`` payload.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the input exceeds 16 MiB, is malformed, has an unsupported
    ///     envelope schema, carries another type, or fails cap/floor validation.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import CapFloor
    /// >>> try:
    /// ...     CapFloor.from_json("{}")
    /// ... except ValueError as exc:
    /// ...     print("schema" in str(exc))
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        match parse_typed_instrument_json(json)? {
            InstrumentJson::CapFloor(inner) => Ok(Self { inner }),
            _ => Err(value_error(
                "expected instrument type \"cap_floor\", got a different instrument type",
            )),
        }
    }

    /// Serialize to a canonical ``finstack_quant.instrument/1`` envelope.
    ///
    /// Returns
    /// -------
    /// str
    ///     Canonical instrument envelope accepted by ``price_instrument`` and
    ///     ``CapFloor.from_json``.
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
        format!("CapFloor(id={:?})", self.inner.id.as_str())
    }
}

/// Fluent builder for [`PyCapFloor`]; wraps the Rust
/// `FinancialBuilder`-generated builder (consuming setters).
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "CapFloorBuilder",
    skip_from_py_object
)]
pub struct PyCapFloorBuilder {
    inner: Option<CapFloorBuilderInner>,
}

/// Take the wrapped Rust builder or fail if `build()` already consumed it.
fn take_cap_floor(b: &mut PyCapFloorBuilder) -> PyResult<CapFloorBuilderInner> {
    b.inner
        .take()
        .ok_or_else(|| value_error("builder already consumed by build()"))
}

#[pymethods]
impl PyCapFloorBuilder {
    /// Set the instrument identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Unique identifier for the cap/floor.
    ///
    /// Returns
    /// -------
    /// CapFloorBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn id<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cap_floor(&mut slf)?;
        slf.inner = Some(b.id(InstrumentId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the option type.
    ///
    /// Parameters
    /// ----------
    /// value : {"cap", "floor", "caplet", "floorlet"}
    ///     Option type of the instrument: ``"cap"``/``"floor"`` for a series
    ///     of caplets/floorlets, or ``"caplet"``/``"floorlet"`` for a single
    ///     period.
    ///
    /// Returns
    /// -------
    /// CapFloorBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not a recognized option type.
    #[pyo3(text_signature = "($self, value)")]
    fn rate_option_type<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let rate_option_type = enum_from_str(value, "rate_option_type")?;
        let b = take_cap_floor(&mut slf)?;
        slf.inner = Some(b.rate_option_type(rate_option_type));
        Ok(slf)
    }

    /// Set the notional amount.
    ///
    /// Parameters
    /// ----------
    /// value : Money
    ///     Notional amount.
    ///
    /// Returns
    /// -------
    /// CapFloorBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyMoney>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cap_floor(&mut slf)?;
        slf.inner = Some(b.notional(value.inner));
        Ok(slf)
    }

    /// Set the strike.
    ///
    /// Parameters
    /// ----------
    /// value : float
    ///     Strike as a decimal (0.05 = 5%).
    ///
    /// Returns
    /// -------
    /// CapFloorBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not finite.
    #[pyo3(text_signature = "($self, value)")]
    fn strike<'py>(mut slf: PyRefMut<'py, Self>, value: f64) -> PyResult<PyRefMut<'py, Self>> {
        let strike = decimal_from_f64(value, "strike")?;
        let b = take_cap_floor(&mut slf)?;
        slf.inner = Some(b.strike(strike));
        Ok(slf)
    }

    /// Set the contractual spread added to the referenced rate.
    ///
    /// Parameters
    /// ----------
    /// value : float
    ///     Spread in decimal rate units, added after projecting the index.
    ///
    /// Returns
    /// -------
    /// CapFloorBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not finite.
    #[pyo3(text_signature = "($self, value)")]
    fn spread<'py>(mut slf: PyRefMut<'py, Self>, value: f64) -> PyResult<PyRefMut<'py, Self>> {
        let spread = decimal_from_f64(value, "spread")?;
        let b = take_cap_floor(&mut slf)?;
        slf.inner = Some(b.spread(spread));
        Ok(slf)
    }
    /// Set the dated premium paid by the cap/floor holder.
    ///
    /// Parameters
    /// ----------
    /// payment_date : datetime.date
    ///     Contractual premium payment date. Payments on or before the valuation
    ///     date are treated as settled and excluded from NPV.
    /// amount : Money
    ///     Non-negative premium outflow in the notional currency.
    ///
    /// Returns
    /// -------
    /// CapFloorBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``payment_date`` cannot be converted to a date or the builder
    ///     was already consumed. Premium amount and currency validation occurs
    ///     when :meth:`CapFloorBuilder.build` is called.
    #[pyo3(text_signature = "($self, payment_date, amount)")]
    fn premium<'py>(
        mut slf: PyRefMut<'py, Self>,
        payment_date: &Bound<'_, PyAny>,
        amount: PyRef<'_, PyMoney>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let payment_date = py_to_date(payment_date)?;
        let b = take_cap_floor(&mut slf)?;
        slf.inner = Some(b.premium((payment_date, amount.inner)));
        Ok(slf)
    }

    /// Set the start date of the underlying period.
    ///
    /// Parameters
    /// ----------
    /// value : datetime.date
    ///     Start date of the underlying period.
    ///
    /// Returns
    /// -------
    /// CapFloorBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn start_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let start_date = py_to_date(value)?;
        let b = take_cap_floor(&mut slf)?;
        slf.inner = Some(b.start_date(start_date));
        Ok(slf)
    }

    /// Set the end date of the underlying period.
    ///
    /// Parameters
    /// ----------
    /// value : datetime.date
    ///     End date of the underlying period.
    ///
    /// Returns
    /// -------
    /// CapFloorBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn maturity<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let maturity = py_to_date(value)?;
        let b = take_cap_floor(&mut slf)?;
        slf.inner = Some(b.maturity(maturity));
        Ok(slf)
    }

    /// Set the payment frequency.
    ///
    /// Parameters
    /// ----------
    /// value : Tenor
    ///     Payment frequency for caps/floors.
    ///
    /// Returns
    /// -------
    /// CapFloorBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn frequency<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyTenor>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cap_floor(&mut slf)?;
        slf.inner = Some(b.frequency(value.inner));
        Ok(slf)
    }

    /// Set the day count convention.
    ///
    /// Parameters
    /// ----------
    /// value : DayCount
    ///     Day count convention.
    ///
    /// Returns
    /// -------
    /// CapFloorBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn day_count<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyDayCount>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cap_floor(&mut slf)?;
        slf.inner = Some(b.day_count(value.inner));
        Ok(slf)
    }

    /// Set the holiday calendar identifier for schedule and roll conventions.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Holiday calendar identifier.
    ///
    /// Returns
    /// -------
    /// CapFloorBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn calendar_id<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cap_floor(&mut slf)?;
        slf.inner = Some(b.calendar_id(CalendarId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the discount curve identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Discount curve identifier.
    ///
    /// Returns
    /// -------
    /// CapFloorBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn discount_curve_id<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cap_floor(&mut slf)?;
        slf.inner = Some(b.discount_curve_id(CurveId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the forward curve identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Forward curve identifier.
    ///
    /// Returns
    /// -------
    /// CapFloorBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn forward_curve_id<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cap_floor(&mut slf)?;
        slf.inner = Some(b.forward_curve_id(CurveId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the volatility surface identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Volatility surface identifier.
    ///
    /// Returns
    /// -------
    /// CapFloorBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn vol_surface_id<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cap_floor(&mut slf)?;
        slf.inner = Some(b.vol_surface_id(CurveId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the volatility type convention.
    ///
    /// Parameters
    /// ----------
    /// value : {"lognormal", "shifted_lognormal", "normal", "auto"}
    ///     Volatility convention. Must match the convention of the
    ///     configured volatility surface. ``"auto"`` resolves to
    ///     ``"lognormal"``, pricing each caplet with Black-76 where
    ///     well-defined and falling back to an equivalent Bachelier price
    ///     otherwise (e.g. a cap whose schedule crosses a zero forward
    ///     rate).
    ///
    /// Returns
    /// -------
    /// CapFloorBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not a recognized volatility type.
    #[pyo3(text_signature = "($self, value)")]
    fn vol_type<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let vol_type = enum_from_str(value, "vol_type")?;
        let b = take_cap_floor(&mut slf)?;
        slf.inner = Some(b.vol_type(vol_type));
        Ok(slf)
    }

    /// Set the displacement shift used for shifted-lognormal pricing.
    ///
    /// Parameters
    /// ----------
    /// value : float
    ///     Displacement added to forward and strike. Must be non-negative.
    ///
    /// Returns
    /// -------
    /// CapFloorBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn vol_shift<'py>(mut slf: PyRefMut<'py, Self>, value: f64) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cap_floor(&mut slf)?;
        slf.inner = Some(b.vol_shift(value));
        Ok(slf)
    }

    /// Build the validated cap/floor.
    ///
    /// Returns
    /// -------
    /// CapFloor
    ///     The validated cap/floor.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the builder was already consumed, a required field is missing,
    ///     or the completed cap/floor fails pricing validation.
    #[pyo3(text_signature = "($self)")]
    fn build(mut slf: PyRefMut<'_, Self>) -> PyResult<PyCapFloor> {
        let b = take_cap_floor(&mut slf)?;
        let inner = b.build().map_err(core_to_py)?;
        inner.validate_for_pricing().map_err(core_to_py)?;
        Ok(PyCapFloor { inner })
    }
}

/// Register the typed rates instruments on the instruments submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyInterestRateSwap>()?;
    m.add_class::<PyInterestRateSwapBuilder>()?;
    m.add_class::<PySwaption>()?;
    m.add_class::<PySwaptionBuilder>()?;
    m.add_class::<PyCapFloor>()?;
    m.add_class::<PyCapFloorBuilder>()?;
    Ok(())
}
