//! Typed credit instruments: `CreditDefaultSwap`, `CDSIndex`, `CDSTranche`
//! and `ConvertibleBond`.
//! Mirrors the `PyBond` pattern in `instruments.rs`.

use pyo3::prelude::*;
use pyo3::types::PyType;

use crate::bindings::core::dates::daycount::PyDayCount;
use crate::bindings::core::dates::tenor::PyTenor;
use crate::bindings::core::dates::utils::py_to_date;
use crate::bindings::core::money::PyMoney;
use crate::errors::{core_to_py, value_error};
use finstack_quant_core::types::{CurveId, InstrumentId};
use finstack_quant_valuations::instruments::credit_derivatives::cds::CDSConvention;
use finstack_quant_valuations::instruments::credit_derivatives::cds_index::IndexPricing;
use finstack_quant_valuations::instruments::credit_derivatives::cds_tranche::TrancheSide;
use finstack_quant_valuations::instruments::{Instrument, InstrumentJson};
use finstack_quant_valuations::market::conventions::ids::CdsDocClause;

use super::instruments::{
    enum_from_str, json_field, parse_typed_instrument_json, serialize_typed_instrument_json,
};
use super::typed_legs::{PyPremiumLegSpec, PyProtectionLegSpec};

type CdsBuilderInner =
    finstack_quant_valuations::instruments::credit_derivatives::cds::CreditDefaultSwapBuilder;
type CdsIndexBuilderInner =
    finstack_quant_valuations::instruments::credit_derivatives::cds_index::CDSIndexBuilder;
type CdsTrancheBuilderInner =
    finstack_quant_valuations::instruments::credit_derivatives::cds_tranche::CDSTrancheBuilder;
type ConvertibleBondBuilderInner =
    finstack_quant_valuations::instruments::fixed_income::convertible::ConvertibleBondBuilder;

// ---------------------------------------------------------------------------
// CreditDefaultSwap
// ---------------------------------------------------------------------------

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
    /// >>> callable(CreditDefaultSwap.builder)
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn builder() -> PyCreditDefaultSwapBuilder {
        PyCreditDefaultSwapBuilder {
            inner: Some(finstack_quant_valuations::instruments::CreditDefaultSwap::builder()),
        }
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
    /// >>> callable(CreditDefaultSwap.from_json)
    /// True
    #[classmethod]
    #[pyo3(text_signature = "(cls, json)")]
    fn from_json(_cls: &Bound<'_, PyType>, json: &str) -> PyResult<Self> {
        match parse_typed_instrument_json(json)? {
            InstrumentJson::CreditDefaultSwap(inner) => {
                inner.validate_for_pricing().map_err(core_to_py)?;
                Ok(Self { inner })
            }
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
        let convention: CDSConvention = enum_from_str(value, "convention")?;
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
        let date = crate::bindings::core::dates::utils::py_to_date(value)?;
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
    ///     If a required field is missing or Rust validation fails.
    #[pyo3(text_signature = "($self)")]
    fn build(mut slf: PyRefMut<'_, Self>) -> PyResult<PyCreditDefaultSwap> {
        let b = take_cds(&mut slf)?;
        let inner = b.build().map_err(core_to_py)?;
        inner.validate_for_pricing().map_err(core_to_py)?;
        Ok(PyCreditDefaultSwap { inner })
    }
}

// ---------------------------------------------------------------------------
// CDSIndex
// ---------------------------------------------------------------------------

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
    /// >>> callable(CDSIndex.builder)
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
    /// >>> callable(CDSIndex.from_json)
    /// True
    #[classmethod]
    #[pyo3(text_signature = "(cls, json)")]
    fn from_json(_cls: &Bound<'_, PyType>, json: &str) -> PyResult<Self> {
        match parse_typed_instrument_json(json)? {
            InstrumentJson::CDSIndex(inner) => {
                inner.validate_for_pricing().map_err(core_to_py)?;
                Ok(Self { inner })
            }
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
        let convention: CDSConvention = enum_from_str(value, "convention")?;
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
    ///     If a required field is missing or Rust validation fails.
    #[pyo3(text_signature = "($self)")]
    fn build(mut slf: PyRefMut<'_, Self>) -> PyResult<PyCDSIndex> {
        let b = take_cds_index(&mut slf)?;
        let inner = b.build().map_err(core_to_py)?;
        inner.validate_for_pricing().map_err(core_to_py)?;
        Ok(PyCDSIndex { inner })
    }
}

// ---------------------------------------------------------------------------
// CDSTranche
// ---------------------------------------------------------------------------

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
    /// The builder pre-seeds ``accumulated_loss(0.0)`` and
    /// ``standard_imm_dates(True)`` (the Rust fields have no defaults), which
    /// :meth:`CDSTrancheBuilder.accumulated_loss` and
    /// :meth:`CDSTrancheBuilder.standard_imm_dates` can override.
    ///
    /// Returns
    /// -------
    /// CDSTrancheBuilder
    ///     A builder with fluent, consuming setter methods.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import CDSTranche
    /// >>> callable(CDSTranche.builder)
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn builder() -> PyCDSTrancheBuilder {
        PyCDSTrancheBuilder {
            inner: Some(
                finstack_quant_valuations::instruments::CDSTranche::builder()
                    .accumulated_loss(0.0)
                    .standard_imm_dates(true),
            ),
        }
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
    /// >>> callable(CDSTranche.from_json)
    /// True
    #[classmethod]
    #[pyo3(text_signature = "(cls, json)")]
    fn from_json(_cls: &Bound<'_, PyType>, json: &str) -> PyResult<Self> {
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
    ///     If a required field is missing or Rust validation fails.
    #[pyo3(text_signature = "($self)")]
    fn build(mut slf: PyRefMut<'_, Self>) -> PyResult<PyCDSTranche> {
        let b = take_cds_tranche(&mut slf)?;
        let inner = b.build().map_err(core_to_py)?;
        inner.validate_for_pricing().map_err(core_to_py)?;
        Ok(PyCDSTranche { inner })
    }
}

// ---------------------------------------------------------------------------
// ConvertibleBond
// ---------------------------------------------------------------------------

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
    /// >>> callable(ConvertibleBond.builder)
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn builder() -> PyConvertibleBondBuilder {
        PyConvertibleBondBuilder {
            inner: Some(finstack_quant_valuations::instruments::ConvertibleBond::builder()),
        }
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
    /// >>> callable(ConvertibleBond.from_json)
    /// True
    #[classmethod]
    #[pyo3(text_signature = "(cls, json)")]
    fn from_json(_cls: &Bound<'_, PyType>, json: &str) -> PyResult<Self> {
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
    ///     If a required field is missing or Rust validation fails (e.g.
    ///     neither ``ratio`` nor ``price`` set on the conversion terms).
    #[pyo3(text_signature = "($self)")]
    fn build(mut slf: PyRefMut<'_, Self>) -> PyResult<PyConvertibleBond> {
        let b = take_convertible(&mut slf)?;
        let inner = b.build().map_err(core_to_py)?;
        inner.validate_for_pricing().map_err(core_to_py)?;
        Ok(PyConvertibleBond { inner })
    }
}

/// Register the typed credit-derivative instruments on the instruments
/// submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyCreditDefaultSwap>()?;
    m.add_class::<PyCreditDefaultSwapBuilder>()?;
    m.add_class::<PyCDSIndex>()?;
    m.add_class::<PyCDSIndexBuilder>()?;
    m.add_class::<PyCDSTranche>()?;
    m.add_class::<PyCDSTrancheBuilder>()?;
    m.add_class::<PyConvertibleBond>()?;
    m.add_class::<PyConvertibleBondBuilder>()?;
    Ok(())
}
