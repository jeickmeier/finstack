//! Credit default swap Python wrappers and fluent builder.

use pyo3::prelude::*;

use crate::bindings::core::money::PyMoney;
use crate::errors::{core_to_py, value_error};
use finstack_quant_core::types::InstrumentId;
use finstack_quant_valuations::instruments::{Instrument, InstrumentJson};
use finstack_quant_valuations::market::conventions::ids::CdsDocClause;
use finstack_quant_valuations::market::conventions::CdsConvention;

use super::super::instruments::{
    enum_from_str, parse_typed_instrument_json, serialize_typed_instrument_json,
};
use super::super::typed_legs::{PyPremiumLegSpec, PyProtectionLegSpec};

type CdsBuilderInner =
    finstack_quant_valuations::instruments::credit_derivatives::cds::CreditDefaultSwapBuilder;

/// Typed wrapper for the Rust `CreditDefaultSwap` instrument.
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "CreditDefaultSwap",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyCreditDefaultSwap {
    /// Inner canonical Rust CDS.
    pub(crate) inner: finstack_quant_valuations::instruments::CreditDefaultSwap,
}

impl PyCreditDefaultSwap {
    /// Serialize as the canonical instrument envelope accepted by the JSON loader.
    pub(crate) fn envelope_json(&self) -> PyResult<String> {
        serialize_typed_instrument_json(
            InstrumentJson::CreditDefaultSwap(self.inner.clone()),
            "CreditDefaultSwap",
        )
    }
}

#[pymethods]
impl PyCreditDefaultSwap {
    /// Create a fluent builder (mirrors Rust ``CreditDefaultSwap::builder()``).
    ///
    /// Returns
    /// -------
    /// CreditDefaultSwapBuilder
    ///     A builder with fluent, consuming setter methods.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import CreditDefaultSwap
    /// >>> builder = CreditDefaultSwap.builder()
    /// >>> builder.id("EXAMPLE") is builder
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn builder() -> PyCreditDefaultSwapBuilder {
        PyCreditDefaultSwapBuilder {
            inner: Some(finstack_quant_valuations::instruments::CreditDefaultSwap::builder()),
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

    /// Deserialize a validated CDS from its canonical v1 envelope.
    ///
    /// Parameters
    /// ----------
    /// json : str
    ///     A ``finstack_quant.instrument/1`` envelope containing an exact
    ///     ``"credit_default_swap"`` payload. The UTF-8 input must not exceed
    ///     16 MiB. Bare payloads and cross-type coercion are rejected.
    ///
    /// Returns
    /// -------
    /// CreditDefaultSwap
    ///     The validated CDS represented by the exact ``"credit_default_swap"`` payload.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the input exceeds 16 MiB, is malformed, has an unsupported
    ///     envelope schema, carries another type, or fails CDS validation.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import CreditDefaultSwap
    /// >>> try:
    /// ...     CreditDefaultSwap.from_json("{}")
    /// ... except ValueError as exc:
    /// ...     print("schema" in str(exc))
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        match parse_typed_instrument_json(json)? {
            InstrumentJson::CreditDefaultSwap(inner) => Ok(Self { inner }),
            _ => Err(value_error(
                "expected instrument type \"credit_default_swap\", got a different instrument type",
            )),
        }
    }

    /// Serialize to a canonical ``finstack_quant.instrument/1`` envelope.
    ///
    /// Returns
    /// -------
    /// str
    ///     Canonical instrument envelope accepted by ``price_instrument`` and
    ///     ``CreditDefaultSwap.from_json``.
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
        format!("CreditDefaultSwap(id={:?})", self.inner.id.as_str())
    }
}

/// Fluent builder for [`PyCreditDefaultSwap`]; wraps the Rust
/// `FinancialBuilder`-generated builder (consuming setters).
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "CreditDefaultSwapBuilder",
    skip_from_py_object
)]
pub struct PyCreditDefaultSwapBuilder {
    inner: Option<CdsBuilderInner>,
}

/// Take the wrapped Rust builder or fail if `build()` already consumed it.
fn take_cds(b: &mut PyCreditDefaultSwapBuilder) -> PyResult<CdsBuilderInner> {
    b.inner
        .take()
        .ok_or_else(|| value_error("builder already consumed by build()"))
}

#[pymethods]
impl PyCreditDefaultSwapBuilder {
    /// Set the instrument identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Unique identifier for the CDS.
    ///
    /// Returns
    /// -------
    /// CreditDefaultSwapBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn id<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds(&mut slf)?;
        slf.inner = Some(b.id(InstrumentId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the notional amount.
    ///
    /// Parameters
    /// ----------
    /// value : Money
    ///     Notional amount of protection.
    ///
    /// Returns
    /// -------
    /// CreditDefaultSwapBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyMoney>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds(&mut slf)?;
        slf.inner = Some(b.notional(value.inner));
        Ok(slf)
    }

    /// Set the protection buyer/seller perspective.
    ///
    /// Parameters
    /// ----------
    /// value : {"pay", "receive"}
    ///     ``"pay"`` to buy protection (pay premium), ``"receive"`` to sell
    ///     protection (receive premium).
    ///
    /// Returns
    /// -------
    /// CreditDefaultSwapBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not a recognized side.
    #[pyo3(text_signature = "($self, value)")]
    fn side<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let side = enum_from_str(value, "side")?;
        let b = take_cds(&mut slf)?;
        slf.inner = Some(b.side(side));
        Ok(slf)
    }

    /// Set the ISDA regional convention.
    ///
    /// Parameters
    /// ----------
    /// value : {"isda_na", "isda_eu", "isda_as", "custom"}
    ///     ISDA CDS convention (North American, European, or Asian), or
    ///     ``"custom"`` for a manually configured convention.
    ///
    /// Returns
    /// -------
    /// CreditDefaultSwapBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not a recognized convention.
    #[pyo3(text_signature = "($self, value)")]
    fn convention<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let convention: CdsConvention = enum_from_str(value, "convention")?;
        let b = take_cds(&mut slf)?;
        slf.inner = Some(b.convention(convention));
        Ok(slf)
    }

    /// Set the premium leg specification.
    ///
    /// Parameters
    /// ----------
    /// value : PremiumLegSpec
    ///     Premium leg specification.
    ///
    /// Returns
    /// -------
    /// CreditDefaultSwapBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn premium<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyPremiumLegSpec>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds(&mut slf)?;
        slf.inner = Some(b.premium(value.inner.clone()));
        Ok(slf)
    }

    /// Set the protection leg specification.
    ///
    /// Parameters
    /// ----------
    /// value : ProtectionLegSpec
    ///     Protection leg specification.
    ///
    /// Returns
    /// -------
    /// CreditDefaultSwapBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn protection<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyProtectionLegSpec>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds(&mut slf)?;
        slf.inner = Some(b.protection(value.inner.clone()));
        Ok(slf)
    }

    /// Set the ISDA documentation clause for restructuring credit events.
    ///
    /// Parameters
    /// ----------
    /// value : {"cr14", "mr14", "mm14", "xr14", "isda_na", "isda_eu", "isda_as", "isda_au", "isda_nz", "custom"}
    ///     Restructuring documentation clause: one of the four 2014 ISDA
    ///     restructuring elections (``"cr14"``/``"mr14"``/``"mm14"``/
    ///     ``"xr14"``), a regional ISDA corporate default (``"isda_na"``/
    ///     ``"isda_eu"``/``"isda_as"``/``"isda_au"``/``"isda_nz"``), or
    ///     ``"custom"``. If never set, the effective clause is derived from
    ///     the CDS convention (see Rust
    ///     ``CreditDefaultSwap::doc_clause_effective``).
    ///
    /// Returns
    /// -------
    /// CreditDefaultSwapBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not a recognized documentation clause.
    #[pyo3(text_signature = "($self, value)")]
    fn doc_clause<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let clause: CdsDocClause = enum_from_str(value, "doc_clause")?;
        let b = take_cds(&mut slf)?;
        slf.inner = Some(b.doc_clause(clause));
        Ok(slf)
    }

    /// Set the protection effective date for a forward-starting CDS.
    ///
    /// Parameters
    /// ----------
    /// value : datetime.date
    ///     Date on which credit protection begins. Must satisfy
    ///     ``premium.start <= value <= premium.end``. When never set,
    ///     protection starts on the premium leg start date.
    ///
    /// Returns
    /// -------
    /// CreditDefaultSwapBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn protection_effective_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let date = crate::bindings::date_utils::py_to_date(value)?;
        let b = take_cds(&mut slf)?;
        slf.inner = Some(b.protection_effective_date(date));
        Ok(slf)
    }

    /// Build the validated CDS.
    ///
    /// Returns
    /// -------
    /// CreditDefaultSwap
    ///     The validated CDS.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the builder was already consumed, a required field is missing,
    ///     or the completed CDS fails pricing validation.
    #[pyo3(text_signature = "($self)")]
    fn build(mut slf: PyRefMut<'_, Self>) -> PyResult<PyCreditDefaultSwap> {
        let b = take_cds(&mut slf)?;
        let inner = b.build().map_err(core_to_py)?;
        inner.validate_for_pricing().map_err(core_to_py)?;
        Ok(PyCreditDefaultSwap { inner })
    }
}
