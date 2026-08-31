//! CDS index Python wrappers and fluent builder.

use pyo3::prelude::*;

use crate::bindings::core::money::PyMoney;
use crate::errors::{core_to_py, value_error};
use finstack_quant_core::types::InstrumentId;
use finstack_quant_valuations::instruments::credit_derivatives::cds_index::IndexPricing;
use finstack_quant_valuations::instruments::{Instrument, InstrumentJson};
use finstack_quant_valuations::market::conventions::CdsConvention;

use super::super::instruments::{
    enum_from_str, json_field, parse_typed_instrument_json, serialize_typed_instrument_json,
};
use super::super::typed_legs::{PyPremiumLegSpec, PyProtectionLegSpec};

type CdsIndexBuilderInner =
    finstack_quant_valuations::instruments::credit_derivatives::cds_index::CDSIndexBuilder;

/// Typed wrapper for the Rust `CDSIndex` instrument.
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "CDSIndex",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyCDSIndex {
    /// Inner canonical Rust CDS index.
    pub(crate) inner: finstack_quant_valuations::instruments::CDSIndex,
}

impl PyCDSIndex {
    /// Serialize as the canonical instrument envelope accepted by the JSON loader.
    pub(crate) fn envelope_json(&self) -> PyResult<String> {
        serialize_typed_instrument_json(InstrumentJson::CDSIndex(self.inner.clone()), "CDSIndex")
    }
}

#[pymethods]
impl PyCDSIndex {
    /// Create a fluent builder (mirrors Rust ``CDSIndex::builder()``).
    ///
    /// The builder pre-seeds an empty ``constituents`` list (the Rust field
    /// has no default) so ``build()`` succeeds without calling
    /// ``constituents_json`` when the index is priced in ``"single_curve"``
    /// mode.
    ///
    /// Returns
    /// -------
    /// CDSIndexBuilder
    ///     A builder with fluent, consuming setter methods.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import CDSIndex
    /// >>> builder = CDSIndex.builder()
    /// >>> builder.id("EXAMPLE") is builder
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn builder() -> PyCDSIndexBuilder {
        PyCDSIndexBuilder {
            inner: Some(
                finstack_quant_valuations::instruments::CDSIndex::builder()
                    .constituents(Vec::new()),
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

    /// Deserialize a validated CDS index from its canonical v1 envelope.
    ///
    /// Parameters
    /// ----------
    /// json : str
    ///     A ``finstack_quant.instrument/1`` envelope containing an exact
    ///     ``"cds_index"`` payload. The UTF-8 input must not exceed 16 MiB.
    ///     Bare payloads and cross-type coercion are rejected.
    ///
    /// Returns
    /// -------
    /// CDSIndex
    ///     The validated CDS index represented by the exact ``"cds_index"`` payload.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the input exceeds 16 MiB, is malformed, has an unsupported
    ///     envelope schema, carries another type, or fails CDS-index validation.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import CDSIndex
    /// >>> try:
    /// ...     CDSIndex.from_json("{}")
    /// ... except ValueError as exc:
    /// ...     print("schema" in str(exc))
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        match parse_typed_instrument_json(json)? {
            InstrumentJson::CDSIndex(inner) => Ok(Self { inner }),
            _ => Err(value_error(
                "expected instrument type \"cds_index\", got a different instrument type",
            )),
        }
    }

    /// Serialize to a canonical ``finstack_quant.instrument/1`` envelope.
    ///
    /// Returns
    /// -------
    /// str
    ///     Canonical instrument envelope accepted by ``price_instrument`` and
    ///     ``CDSIndex.from_json``.
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
        format!("CDSIndex(id={:?})", self.inner.id.as_str())
    }
}

/// Fluent builder for [`PyCDSIndex`]; wraps the Rust
/// `FinancialBuilder`-generated builder (consuming setters).
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "CDSIndexBuilder",
    skip_from_py_object
)]
pub struct PyCDSIndexBuilder {
    inner: Option<CdsIndexBuilderInner>,
}

/// Take the wrapped Rust builder or fail if `build()` already consumed it.
fn take_cds_index(b: &mut PyCDSIndexBuilder) -> PyResult<CdsIndexBuilderInner> {
    b.inner
        .take()
        .ok_or_else(|| value_error("builder already consumed by build()"))
}

#[pymethods]
impl PyCDSIndexBuilder {
    /// Set the instrument identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Unique identifier for the index trade.
    ///
    /// Returns
    /// -------
    /// CDSIndexBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn id<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds_index(&mut slf)?;
        slf.inner = Some(b.id(InstrumentId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the index name.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Index name, e.g. ``"CDX.NA.IG"``, ``"CDX.NA.HY"``, ``"iTraxx Europe"``.
    ///
    /// Returns
    /// -------
    /// CDSIndexBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn index_name<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds_index(&mut slf)?;
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
    /// CDSIndexBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn series<'py>(mut slf: PyRefMut<'py, Self>, value: u16) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds_index(&mut slf)?;
        slf.inner = Some(b.series(value));
        Ok(slf)
    }

    /// Set the version number within the series.
    ///
    /// Parameters
    /// ----------
    /// value : int
    ///     Version number, e.g. ``1``.
    ///
    /// Returns
    /// -------
    /// CDSIndexBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn version<'py>(mut slf: PyRefMut<'py, Self>, value: u16) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds_index(&mut slf)?;
        slf.inner = Some(b.version(value));
        Ok(slf)
    }

    /// Set the notional amount of the index.
    ///
    /// Parameters
    /// ----------
    /// value : Money
    ///     Notional amount of the index.
    ///
    /// Returns
    /// -------
    /// CDSIndexBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn notional<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyMoney>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds_index(&mut slf)?;
        slf.inner = Some(b.notional(value.inner));
        Ok(slf)
    }

    /// Set the index factor (fraction of surviving notional).
    ///
    /// Parameters
    /// ----------
    /// value : float
    ///     Index factor in ``[0.0, 1.0]``. ``1.0`` means no constituent has
    ///     defaulted since series inception.
    ///
    /// Returns
    /// -------
    /// CDSIndexBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn index_factor<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: f64,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds_index(&mut slf)?;
        slf.inner = Some(b.index_factor(value));
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
    /// CDSIndexBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not a recognized side.
    #[pyo3(text_signature = "($self, value)")]
    fn side<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let side = enum_from_str(value, "side")?;
        let b = take_cds_index(&mut slf)?;
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
    /// CDSIndexBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not a recognized convention.
    #[pyo3(text_signature = "($self, value)")]
    fn convention<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let convention: CdsConvention = enum_from_str(value, "convention")?;
        let b = take_cds_index(&mut slf)?;
        slf.inner = Some(b.convention(convention));
        Ok(slf)
    }

    /// Set the premium leg specification.
    ///
    /// Parameters
    /// ----------
    /// value : PremiumLegSpec
    ///     Premium leg specification (coupon schedule and discounting).
    ///
    /// Returns
    /// -------
    /// CDSIndexBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn premium<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyPremiumLegSpec>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds_index(&mut slf)?;
        slf.inner = Some(b.premium(value.inner.clone()));
        Ok(slf)
    }

    /// Set the protection leg specification.
    ///
    /// Parameters
    /// ----------
    /// value : ProtectionLegSpec
    ///     Protection leg specification (credit curve and settlement).
    ///
    /// Returns
    /// -------
    /// CDSIndexBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn protection<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyProtectionLegSpec>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds_index(&mut slf)?;
        slf.inner = Some(b.protection(value.inner.clone()));
        Ok(slf)
    }

    /// Set the pricing aggregation mode.
    ///
    /// Parameters
    /// ----------
    /// value : {"single_curve", "constituents"}
    ///     ``"single_curve"`` prices the index against a single index hazard
    ///     curve (synthetic CDS). ``"constituents"`` prices each issuer
    ///     separately and aggregates by weight; requires
    ///     ``constituents_json`` to be set.
    ///
    /// Returns
    /// -------
    /// CDSIndexBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not a recognized pricing mode.
    #[pyo3(text_signature = "($self, value)")]
    fn pricing<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let pricing: IndexPricing = enum_from_str(value, "pricing")?;
        let b = take_cds_index(&mut slf)?;
        slf.inner = Some(b.pricing(pricing));
        Ok(slf)
    }

    /// Set the index constituents from a JSON array.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     JSON array of ``CDSIndexConstituent`` objects (``credit``,
    ///     ``weight``, and optional ``defaulted``).
    ///
    /// Returns
    /// -------
    /// CDSIndexBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not valid JSON for the constituent-list shape.
    #[pyo3(text_signature = "($self, value)")]
    fn constituents_json<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let constituents: Vec<
            finstack_quant_valuations::instruments::credit_derivatives::cds_index::CDSIndexConstituent,
        > = json_field(value, "constituents")?;
        let b = take_cds_index(&mut slf)?;
        slf.inner = Some(b.constituents(constituents));
        Ok(slf)
    }

    /// Set the number of reference entities in the index pool.
    ///
    /// Parameters
    /// ----------
    /// value : int
    ///     Number of names in the index pool, e.g. ``125`` for CDX.NA.IG.
    ///     Required for portfolio-level analytics (e.g. jump-to-default)
    ///     when ``constituents`` is empty.
    ///
    /// Returns
    /// -------
    /// CDSIndexBuilder
    ///     ``self``, for chaining.
    #[pyo3(text_signature = "($self, value)")]
    fn num_constituents<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: u32,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_cds_index(&mut slf)?;
        slf.inner = Some(b.num_constituents(value));
        Ok(slf)
    }

    /// Build the validated CDS index.
    ///
    /// Returns
    /// -------
    /// CDSIndex
    ///     The validated CDS index.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the builder was already consumed, a required field is missing,
    ///     or the completed CDS index fails pricing validation.
    #[pyo3(text_signature = "($self)")]
    fn build(mut slf: PyRefMut<'_, Self>) -> PyResult<PyCDSIndex> {
        let b = take_cds_index(&mut slf)?;
        let inner = b.build().map_err(core_to_py)?;
        inner.validate_for_pricing().map_err(core_to_py)?;
        Ok(PyCDSIndex { inner })
    }
}
