//! Convertible bond Python wrappers and fluent builder.

use pyo3::prelude::*;

use crate::bindings::core::dates::utils::py_to_date;
use crate::bindings::core::money::PyMoney;
use crate::errors::{core_to_py, value_error};
use finstack_quant_core::types::{CurveId, InstrumentId};
use finstack_quant_valuations::instruments::{Instrument, InstrumentJson};

use super::super::instruments::{
    json_field, parse_typed_instrument_json, serialize_typed_instrument_json,
};

type ConvertibleBondBuilderInner =
    finstack_quant_valuations::instruments::fixed_income::convertible::ConvertibleBondBuilder;

/// Typed wrapper for the Rust `ConvertibleBond` instrument.
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "ConvertibleBond",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyConvertibleBond {
    /// Inner canonical Rust convertible bond.
    pub(crate) inner: finstack_quant_valuations::instruments::ConvertibleBond,
}

impl PyConvertibleBond {
    /// Serialize as the canonical instrument envelope accepted by the JSON loader.
    pub(crate) fn envelope_json(&self) -> PyResult<String> {
        serialize_typed_instrument_json(
            InstrumentJson::ConvertibleBond(self.inner.clone()),
            "ConvertibleBond",
        )
    }
}

#[pymethods]
impl PyConvertibleBond {
    /// Create a fluent builder (mirrors Rust ``ConvertibleBond::builder()``).
    ///
    /// Returns
    /// -------
    /// ConvertibleBondBuilder
    ///     A builder with fluent, consuming setter methods.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import ConvertibleBond
    /// >>> builder = ConvertibleBond.builder()
    /// >>> builder.id("EXAMPLE") is builder
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn builder() -> PyConvertibleBondBuilder {
        PyConvertibleBondBuilder {
            inner: Some(finstack_quant_valuations::instruments::ConvertibleBond::builder()),
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

    /// Deserialize a validated convertible bond from its canonical v1 envelope.
    ///
    /// Parameters
    /// ----------
    /// json : str
    ///     A ``finstack_quant.instrument/1`` envelope containing an exact
    ///     ``"convertible_bond"`` payload. The UTF-8 input must not exceed
    ///     16 MiB. Bare payloads and cross-type coercion are rejected.
    ///
    /// Returns
    /// -------
    /// ConvertibleBond
    ///     The validated bond represented by the exact ``"convertible_bond"`` payload.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the input exceeds 16 MiB, is malformed, has an unsupported
    ///     envelope schema, carries another type, or fails convertible-bond
    ///     validation.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import ConvertibleBond
    /// >>> try:
    /// ...     ConvertibleBond.from_json("{}")
    /// ... except ValueError as exc:
    /// ...     print("schema" in str(exc))
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        match parse_typed_instrument_json(json)? {
            InstrumentJson::ConvertibleBond(inner) => {
                inner.validate_for_pricing().map_err(core_to_py)?;
                Ok(Self { inner })
            }
            _ => Err(value_error(
                "expected instrument type \"convertible_bond\", got a different instrument type",
            )),
        }
    }

    /// Serialize to a canonical ``finstack_quant.instrument/1`` envelope.
    ///
    /// Returns
    /// -------
    /// str
    ///     Canonical instrument envelope accepted by ``price_instrument`` and
    ///     ``ConvertibleBond.from_json``.
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
        format!("ConvertibleBond(id={:?})", self.inner.id.as_str())
    }
}

/// Fluent builder for [`PyConvertibleBond`]; wraps the Rust
/// `FinancialBuilder`-generated builder (consuming setters).
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "ConvertibleBondBuilder",
    skip_from_py_object
)]
pub struct PyConvertibleBondBuilder {
    inner: Option<ConvertibleBondBuilderInner>,
}

/// Take the wrapped Rust builder or fail if `build()` already consumed it.
fn take_convertible(b: &mut PyConvertibleBondBuilder) -> PyResult<ConvertibleBondBuilderInner> {
    b.inner
        .take()
        .ok_or_else(|| value_error("builder already consumed by build()"))
}

#[pymethods]
impl PyConvertibleBondBuilder {
    /// Set the instrument identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Unique identifier for the convertible bond.
    ///
    /// Returns
    /// -------
    /// ConvertibleBondBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn id<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_convertible(&mut slf)?;
        slf.inner = Some(b.id(InstrumentId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the principal amount.
    ///
    /// Parameters
    /// ----------
    /// value : Money
    ///     Principal amount.
    ///
    /// Returns
    /// -------
    /// ConvertibleBondBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyMoney>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_convertible(&mut slf)?;
        slf.inner = Some(b.notional(value.inner));
        Ok(slf)
    }

    /// Set the issue date.
    ///
    /// Parameters
    /// ----------
    /// value : datetime.date
    ///     Issue date.
    ///
    /// Returns
    /// -------
    /// ConvertibleBondBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn issue_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let issue_date = py_to_date(value)?;
        let b = take_convertible(&mut slf)?;
        slf.inner = Some(b.issue_date(issue_date));
        Ok(slf)
    }

    /// Set the maturity date.
    ///
    /// Parameters
    /// ----------
    /// value : datetime.date
    ///     Maturity date.
    ///
    /// Returns
    /// -------
    /// ConvertibleBondBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn maturity<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let maturity = py_to_date(value)?;
        let b = take_convertible(&mut slf)?;
        slf.inner = Some(b.maturity(maturity));
        Ok(slf)
    }

    /// Set the discount curve identifier for the debt component.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Discount curve identifier for the debt component (risk-free or
    ///     funding).
    ///
    /// Returns
    /// -------
    /// ConvertibleBondBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn discount_curve_id<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_convertible(&mut slf)?;
        slf.inner = Some(b.discount_curve_id(CurveId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the credit curve identifier for risky discounting (bond floor).
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Credit curve identifier. If not provided, falls back to
    ///     ``discount_curve_id`` (implies no credit spread). Must represent
    ///     zero-recovery (pure hazard) risky discounting.
    ///
    /// Returns
    /// -------
    /// ConvertibleBondBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn credit_curve_id<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_convertible(&mut slf)?;
        slf.inner = Some(b.credit_curve_id(CurveId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the conversion terms from a JSON object.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     JSON-encoded ``ConversionSpec`` object with fields ``ratio``,
    ///     ``price``, ``policy``, ``anti_dilution``, ``dividend_adjustment``
    ///     and ``dilution_events``. At least one of ``ratio`` / ``price``
    ///     must be set.
    ///
    /// Returns
    /// -------
    /// ConvertibleBondBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not valid JSON for the ``ConversionSpec`` shape.
    #[pyo3(text_signature = "($self, value)")]
    fn conversion_json<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let conversion: finstack_quant_valuations::instruments::fixed_income::convertible::ConversionSpec =
            json_field(value, "conversion")?;
        let b = take_convertible(&mut slf)?;
        slf.inner = Some(b.conversion(conversion));
        Ok(slf)
    }

    /// Set the underlying equity identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Underlying equity identifier (ticker or instrument id).
    ///
    /// Returns
    /// -------
    /// ConvertibleBondBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn underlying_equity_id<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_convertible(&mut slf)?;
        slf.inner = Some(b.underlying_equity_id(value.to_string()));
        Ok(slf)
    }

    /// Set the call/put schedule from a JSON object.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     JSON-encoded ``CallPutSchedule`` object with ``calls`` and
    ///     ``puts`` arrays of call/put windows.
    ///
    /// Returns
    /// -------
    /// ConvertibleBondBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not valid JSON for the ``CallPutSchedule`` shape.
    #[pyo3(text_signature = "($self, value)")]
    fn call_put_json<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let call_put: finstack_quant_valuations::instruments::fixed_income::bond::CallPutSchedule =
            json_field(value, "call_put")?;
        let b = take_convertible(&mut slf)?;
        slf.inner = Some(b.call_put(call_put));
        Ok(slf)
    }

    /// Set the soft-call trigger condition from a JSON object.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     JSON-encoded ``SoftCallTrigger`` object with fields
    ///     ``threshold_pct``, ``observation_days`` and
    ///     ``required_days_above``.
    ///
    /// Returns
    /// -------
    /// ConvertibleBondBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not valid JSON for the ``SoftCallTrigger`` shape.
    #[pyo3(text_signature = "($self, value)")]
    fn soft_call_trigger_json<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let trigger: finstack_quant_valuations::instruments::fixed_income::convertible::SoftCallTrigger =
            json_field(value, "soft_call_trigger")?;
        let b = take_convertible(&mut slf)?;
        slf.inner = Some(b.soft_call_trigger(trigger));
        Ok(slf)
    }

    /// Set the settlement lag.
    ///
    /// Parameters
    /// ----------
    /// value : int
    ///     Number of business days from trade date to settlement date
    ///     (e.g. ``2`` for US corporate convertibles). If never set,
    ///     settlement is assumed same-day.
    ///
    /// Returns
    /// -------
    /// ConvertibleBondBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn settlement_days<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: u32,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_convertible(&mut slf)?;
        slf.inner = Some(b.settlement_days(value));
        Ok(slf)
    }

    /// Set the assumed recovery rate on default.
    ///
    /// Parameters
    /// ----------
    /// value : float
    ///     Recovery rate as a fraction (e.g. ``0.40`` = 40%). Used in the
    ///     Tsiveriotis-Zhang credit model; only relevant when
    ///     ``credit_curve_id`` is set.
    ///
    /// Returns
    /// -------
    /// ConvertibleBondBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn recovery_rate<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: f64,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_convertible(&mut slf)?;
        slf.inner = Some(b.recovery_rate(value));
        Ok(slf)
    }

    /// Set the fixed coupon specification from a JSON object.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     JSON-encoded ``FixedCouponSpec`` object.
    ///
    /// Returns
    /// -------
    /// ConvertibleBondBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not valid JSON for the ``FixedCouponSpec`` shape.
    #[pyo3(text_signature = "($self, value)")]
    fn fixed_coupon_json<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let spec: finstack_quant_cashflows::builder::FixedCouponSpec =
            json_field(value, "fixed_coupon")?;
        let b = take_convertible(&mut slf)?;
        slf.inner = Some(b.fixed_coupon(spec));
        Ok(slf)
    }

    /// Set the floating coupon specification from a JSON object.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     JSON-encoded ``FloatingCouponSpec`` object.
    ///
    /// Returns
    /// -------
    /// ConvertibleBondBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not valid JSON for the ``FloatingCouponSpec`` shape.
    #[pyo3(text_signature = "($self, value)")]
    fn floating_coupon_json<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let spec: finstack_quant_cashflows::builder::FloatingCouponSpec =
            json_field(value, "floating_coupon")?;
        let b = take_convertible(&mut slf)?;
        slf.inner = Some(b.floating_coupon(spec));
        Ok(slf)
    }

    /// Build the validated convertible bond.
    ///
    /// Returns
    /// -------
    /// ConvertibleBond
    ///     The validated convertible bond.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the builder was already consumed, a required field is missing,
    ///     or the completed convertible bond fails pricing validation (for
    ///     example, conversion terms set neither ``ratio`` nor ``price``).
    #[pyo3(text_signature = "($self)")]
    fn build(mut slf: PyRefMut<'_, Self>) -> PyResult<PyConvertibleBond> {
        let b = take_convertible(&mut slf)?;
        let inner = b.build().map_err(core_to_py)?;
        inner.validate_for_pricing().map_err(core_to_py)?;
        Ok(PyConvertibleBond { inner })
    }
}
