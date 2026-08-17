//! CDS tranche Python wrappers and fluent builder.

use pyo3::prelude::*;

use crate::bindings::core::dates::daycount::PyDayCount;
use crate::bindings::core::dates::tenor::PyTenor;
use crate::bindings::core::dates::utils::py_to_date;
use crate::bindings::core::money::PyMoney;
use crate::errors::{core_to_py, value_error};
use finstack_quant_core::types::{CurveId, InstrumentId};
use finstack_quant_valuations::instruments::credit_derivatives::cds_tranche::TrancheSide;
use finstack_quant_valuations::instruments::{Instrument, InstrumentJson};

use super::super::instruments::{
    enum_from_str, parse_typed_instrument_json, serialize_typed_instrument_json,
};

type CdsTrancheBuilderInner =
    finstack_quant_valuations::instruments::credit_derivatives::cds_tranche::CDSTrancheBuilder;

/// Typed wrapper for the Rust `CDSTranche` instrument.
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "CDSTranche",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyCDSTranche {
    /// Inner canonical Rust CDS tranche.
    pub(crate) inner: finstack_quant_valuations::instruments::CDSTranche,
}

impl PyCDSTranche {
    /// Serialize as the canonical instrument envelope accepted by the JSON loader.
    pub(crate) fn envelope_json(&self) -> PyResult<String> {
        serialize_typed_instrument_json(
            InstrumentJson::CDSTranche(self.inner.clone()),
            "CDSTranche",
        )
    }
}

#[pymethods]
impl PyCDSTranche {
    /// Create a fluent builder (mirrors Rust ``CDSTranche::builder()``).
    ///
    /// The builder pre-seeds ``accumulated_loss(0.0)``. Coupon dates follow
    /// the supplied schedule (``standard_imm_dates`` defaults to ``False``),
    /// matching ``CDSTranche::new``. Call
    /// :meth:`CDSTrancheBuilder.standard_imm_dates` for IMM rolls.
    ///
    /// Returns
    /// -------
    /// CDSTrancheBuilder
    ///     A builder with fluent, consuming setter methods.
    ///
    /// Notes
    /// -----
    /// This factory does not raise; it returns a new instance with the documented defaults.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import CDSTranche
    /// >>> builder = CDSTranche.builder()
    /// >>> builder.id("EXAMPLE") is builder
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn builder() -> PyCDSTrancheBuilder {
        PyCDSTrancheBuilder {
            inner: Some(
                finstack_quant_valuations::instruments::CDSTranche::builder().accumulated_loss(0.0),
            ),
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

    /// Deserialize a validated CDS tranche from its canonical v1 envelope.
    ///
    /// Parameters
    /// ----------
    /// json : str
    ///     A ``finstack_quant.instrument/1`` envelope containing an exact
    ///     ``"cds_tranche"`` payload. The UTF-8 input must not exceed 16 MiB.
    ///     Bare payloads and cross-type coercion are rejected.
    ///
    /// Returns
    /// -------
    /// CDSTranche
    ///     The validated CDS tranche represented by the exact ``"cds_tranche"`` payload.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the input exceeds 16 MiB, is malformed, has an unsupported
    ///     envelope schema, carries another type, or fails CDS-tranche validation.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import CDSTranche
    /// >>> try:
    /// ...     CDSTranche.from_json("{}")
    /// ... except ValueError as exc:
    /// ...     print("schema" in str(exc))
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        match parse_typed_instrument_json(json)? {
            InstrumentJson::CDSTranche(inner) => {
                inner.validate_for_pricing().map_err(core_to_py)?;
                Ok(Self { inner })
            }
            _ => Err(value_error(
                "expected instrument type \"cds_tranche\", got a different instrument type",
            )),
        }
    }

    /// Serialize to a canonical ``finstack_quant.instrument/1`` envelope.
    ///
    /// Returns
    /// -------
    /// str
    ///     Canonical instrument envelope accepted by ``price_instrument`` and
    ///     ``CDSTranche.from_json``.
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
        format!("CDSTranche(id={:?})", self.inner.id.as_str())
    }
}

/// Fluent builder for [`PyCDSTranche`]; wraps the Rust
/// `FinancialBuilder`-generated builder (consuming setters).
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "CDSTrancheBuilder",
    skip_from_py_object
)]
pub struct PyCDSTrancheBuilder {
    inner: Option<CdsTrancheBuilderInner>,
}

/// Take the wrapped Rust builder or fail if `build()` already consumed it.
fn take_cds_tranche(b: &mut PyCDSTrancheBuilder) -> PyResult<CdsTrancheBuilderInner> {
    b.inner
        .take()
        .ok_or_else(|| value_error("builder already consumed by build()"))
}

#[pymethods]
impl PyCDSTrancheBuilder {
    /// Set the instrument identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Unique identifier for the tranche trade.
    ///
    /// Returns
    /// -------
    /// CDSTrancheBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn id<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds_tranche(&mut slf)?;
        slf.inner = Some(b.id(InstrumentId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the underlying index name.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Index name, e.g. ``"CDX.NA.IG"``, ``"CDX.NA.HY"``, ``"iTraxx EUR"``.
    ///
    /// Returns
    /// -------
    /// CDSTrancheBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn index_name<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds_tranche(&mut slf)?;
        slf.inner = Some(b.index_name(value.to_string()));
        Ok(slf)
    }

    /// Set the series number.
    ///
    /// Parameters
    /// ----------
    /// value : int
    ///     Series number, e.g. ``42``.
    ///
    /// Returns
    /// -------
    /// CDSTrancheBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn series<'py>(mut slf: PyRefMut<'py, Self>, value: u16) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds_tranche(&mut slf)?;
        slf.inner = Some(b.series(value));
        Ok(slf)
    }

    /// Set the attachment point.
    ///
    /// Parameters
    /// ----------
    /// value : float
    ///     Attachment point quoted in percent (e.g. ``0.0`` for equity;
    ///     ``3.0`` for a tranche attaching at 3%).
    ///
    /// Returns
    /// -------
    /// CDSTrancheBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn attach_pct<'py>(mut slf: PyRefMut<'py, Self>, value: f64) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds_tranche(&mut slf)?;
        slf.inner = Some(b.attach_pct(value));
        Ok(slf)
    }

    /// Set the detachment point.
    ///
    /// Parameters
    /// ----------
    /// value : float
    ///     Detachment point quoted in percent (e.g. ``3.0`` for a 0-3%
    ///     tranche).
    ///
    /// Returns
    /// -------
    /// CDSTrancheBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn detach_pct<'py>(mut slf: PyRefMut<'py, Self>, value: f64) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds_tranche(&mut slf)?;
        slf.inner = Some(b.detach_pct(value));
        Ok(slf)
    }

    /// Set the notional amount of the tranche.
    ///
    /// Parameters
    /// ----------
    /// value : Money
    ///     Notional amount of the tranche.
    ///
    /// Returns
    /// -------
    /// CDSTrancheBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyMoney>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds_tranche(&mut slf)?;
        slf.inner = Some(b.notional(value.inner));
        Ok(slf)
    }

    /// Set the maturity date of the tranche.
    ///
    /// Parameters
    /// ----------
    /// value : datetime.date
    ///     Maturity date of the tranche.
    ///
    /// Returns
    /// -------
    /// CDSTrancheBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn maturity<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let maturity = py_to_date(value)?;
        let b = take_cds_tranche(&mut slf)?;
        slf.inner = Some(b.maturity(maturity));
        Ok(slf)
    }

    /// Set the running coupon.
    ///
    /// Parameters
    /// ----------
    /// value : float
    ///     Running coupon in basis points (e.g. ``100.0`` = 1.00%).
    ///
    /// Returns
    /// -------
    /// CDSTrancheBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn running_coupon_bp<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: f64,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds_tranche(&mut slf)?;
        slf.inner = Some(b.running_coupon_bp(value));
        Ok(slf)
    }

    /// Set the payment frequency.
    ///
    /// Parameters
    /// ----------
    /// value : Tenor
    ///     Payment frequency (typically quarterly).
    ///
    /// Returns
    /// -------
    /// CDSTrancheBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn frequency<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyTenor>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds_tranche(&mut slf)?;
        slf.inner = Some(b.frequency(value.inner));
        Ok(slf)
    }

    /// Set the day count convention.
    ///
    /// Parameters
    /// ----------
    /// value : DayCount
    ///     Day count convention (typically Act/360).
    ///
    /// Returns
    /// -------
    /// CDSTrancheBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn day_count<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyDayCount>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds_tranche(&mut slf)?;
        slf.inner = Some(b.day_count(value.inner));
        Ok(slf)
    }

    /// Set the holiday calendar identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Holiday calendar identifier.
    ///
    /// Returns
    /// -------
    /// CDSTrancheBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn calendar_id<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds_tranche(&mut slf)?;
        slf.inner = Some(b.calendar_id(value.to_string()));
        Ok(slf)
    }

    /// Set the discount curve identifier (by quote currency).
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Discount curve identifier.
    ///
    /// Returns
    /// -------
    /// CDSTrancheBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn discount_curve_id<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds_tranche(&mut slf)?;
        slf.inner = Some(b.discount_curve_id(CurveId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the credit index identifier for survival/loss modeling.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Credit index identifier.
    ///
    /// Returns
    /// -------
    /// CDSTrancheBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn credit_index_id<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds_tranche(&mut slf)?;
        slf.inner = Some(b.credit_index_id(CurveId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the tranche side (buy/sell protection).
    ///
    /// Parameters
    /// ----------
    /// value : {"buy_protection", "sell_protection"}
    ///     Tranche side.
    ///
    /// Returns
    /// -------
    /// CDSTrancheBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not a recognized side.
    #[pyo3(text_signature = "($self, value)")]
    fn side<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let side: TrancheSide = enum_from_str(value, "side")?;
        let b = take_cds_tranche(&mut slf)?;
        slf.inner = Some(b.side(side));
        Ok(slf)
    }

    /// Set the effective date for schedule anchoring.
    ///
    /// Parameters
    /// ----------
    /// value : datetime.date
    ///     Effective date. If never set, uses the as-of date (or standard
    ///     IMM-date rolling, if ``standard_imm_dates`` is true).
    ///
    /// Returns
    /// -------
    /// CDSTrancheBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn effective_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let effective_date = py_to_date(value)?;
        let b = take_cds_tranche(&mut slf)?;
        slf.inner = Some(b.effective_date(effective_date));
        Ok(slf)
    }

    /// Set the accumulated realized loss.
    ///
    /// Parameters
    /// ----------
    /// value : float
    ///     Accumulated realized loss as a fraction of the original portfolio
    ///     notional. Defaults to ``0.0`` when never set explicitly.
    ///
    /// Returns
    /// -------
    /// CDSTrancheBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn accumulated_loss<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: f64,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds_tranche(&mut slf)?;
        slf.inner = Some(b.accumulated_loss(value));
        Ok(slf)
    }

    /// Set whether to enforce standard IMM dates.
    ///
    /// Parameters
    /// ----------
    /// value : bool
    ///     Whether to enforce standard IMM dates (20th of Mar, Jun, Sep,
    ///     Dec). Defaults to ``True`` when never set explicitly.
    ///
    /// Returns
    /// -------
    /// CDSTrancheBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn standard_imm_dates<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: bool,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds_tranche(&mut slf)?;
        slf.inner = Some(b.standard_imm_dates(value));
        Ok(slf)
    }

    /// Build the validated CDS tranche.
    ///
    /// Returns
    /// -------
    /// CDSTranche
    ///     The validated CDS tranche.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the builder was already consumed, a required field is missing,
    ///     or the completed CDS tranche fails pricing validation.
    #[pyo3(text_signature = "($self)")]
    fn build(mut slf: PyRefMut<'_, Self>) -> PyResult<PyCDSTranche> {
        let b = take_cds_tranche(&mut slf)?;
        let inner = b.build().map_err(core_to_py)?;
        inner.validate_for_pricing().map_err(core_to_py)?;
        Ok(PyCDSTranche { inner })
    }
}
