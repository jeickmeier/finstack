//! Typed credit-derivative instruments: `CreditDefaultSwap` and `CDSIndex`.
//! Mirrors the `PyBond` pattern in `instruments.rs`.

use pyo3::prelude::*;
use pyo3::types::PyType;

use crate::bindings::core::money::PyMoney;
use crate::errors::{core_to_py, serde_json_to_py, value_error};
use finstack_quant_core::types::InstrumentId;
use finstack_quant_valuations::instruments::credit_derivatives::cds::CDSConvention;
use finstack_quant_valuations::instruments::credit_derivatives::cds_index::IndexPricing;
use finstack_quant_valuations::instruments::{Instrument, InstrumentJson};
use finstack_quant_valuations::market::conventions::ids::CdsDocClause;

use super::instruments::{enum_from_str, json_field};
use super::typed_legs::{PyPremiumLegSpec, PyProtectionLegSpec};

type CdsBuilderInner =
    finstack_quant_valuations::instruments::credit_derivatives::cds::CreditDefaultSwapBuilder;
type CdsIndexBuilderInner =
    finstack_quant_valuations::instruments::credit_derivatives::cds_index::CDSIndexBuilder;

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
    /// Serialize as the tagged instrument JSON accepted by the JSON loader.
    pub(crate) fn tagged_json(&self) -> PyResult<String> {
        serde_json::to_string(&InstrumentJson::CreditDefaultSwap(self.inner.clone()))
            .map_err(|err| serde_json_to_py(err, "failed to serialize CreditDefaultSwap"))
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

    /// Deserialize from tagged instrument JSON (``{"type": "credit_default_swap", ...}``).
    ///
    /// Parameters
    /// ----------
    /// json : str
    ///     Tagged instrument JSON with type ``"credit_default_swap"``.
    ///
    /// Returns
    /// -------
    /// CreditDefaultSwap
    ///     The validated CDS.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the JSON is malformed, has a different instrument type, or
    ///     fails validation.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import CreditDefaultSwap
    /// >>> callable(CreditDefaultSwap.from_json)
    /// True
    #[classmethod]
    #[pyo3(text_signature = "(cls, json)")]
    fn from_json(_cls: &Bound<'_, PyType>, json: &str) -> PyResult<Self> {
        match serde_json::from_str::<InstrumentJson>(json)
            .map_err(|err| serde_json_to_py(err, "invalid instrument JSON"))?
        {
            InstrumentJson::CreditDefaultSwap(inner) => {
                inner.validate_for_pricing().map_err(core_to_py)?;
                Ok(Self { inner })
            }
            _ => Err(value_error(
                "expected instrument type \"credit_default_swap\", got a different instrument type",
            )),
        }
    }

    /// Serialize to tagged instrument JSON accepted by ``price_instrument``.
    ///
    /// Returns
    /// -------
    /// str
    ///     Tagged instrument JSON accepted by ``price_instrument`` and
    ///     ``CreditDefaultSwap.from_json``.
    #[pyo3(text_signature = "($self)")]
    fn to_json(&self) -> PyResult<String> {
        self.tagged_json()
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
    /// value : {"isda_na", "isda_eu", "isda_as"}
    ///     ISDA CDS convention (North American, European, or Asian).
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
    /// value : {"cr14", "mr14", "mm14", "xr14"}
    ///     Restructuring documentation clause. If never set, the effective
    ///     clause is derived from the CDS convention (see Rust
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
    /// Serialize as the tagged instrument JSON accepted by the JSON loader.
    pub(crate) fn tagged_json(&self) -> PyResult<String> {
        serde_json::to_string(&InstrumentJson::CDSIndex(self.inner.clone()))
            .map_err(|err| serde_json_to_py(err, "failed to serialize CDSIndex"))
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

    /// Deserialize from tagged instrument JSON (``{"type": "cds_index", ...}``).
    ///
    /// Parameters
    /// ----------
    /// json : str
    ///     Tagged instrument JSON with type ``"cds_index"``.
    ///
    /// Returns
    /// -------
    /// CDSIndex
    ///     The validated CDS index.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the JSON is malformed, has a different instrument type, or
    ///     fails validation.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import CDSIndex
    /// >>> callable(CDSIndex.from_json)
    /// True
    #[classmethod]
    #[pyo3(text_signature = "(cls, json)")]
    fn from_json(_cls: &Bound<'_, PyType>, json: &str) -> PyResult<Self> {
        match serde_json::from_str::<InstrumentJson>(json)
            .map_err(|err| serde_json_to_py(err, "invalid instrument JSON"))?
        {
            InstrumentJson::CDSIndex(inner) => {
                inner.validate_for_pricing().map_err(core_to_py)?;
                Ok(Self { inner })
            }
            _ => Err(value_error(
                "expected instrument type \"cds_index\", got a different instrument type",
            )),
        }
    }

    /// Serialize to tagged instrument JSON accepted by ``price_instrument``.
    ///
    /// Returns
    /// -------
    /// str
    ///     Tagged instrument JSON accepted by ``price_instrument`` and
    ///     ``CDSIndex.from_json``.
    #[pyo3(text_signature = "($self)")]
    fn to_json(&self) -> PyResult<String> {
        self.tagged_json()
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
    /// value : {"isda_na", "isda_eu", "isda_as"}
    ///     ISDA CDS convention (North American, European, or Asian).
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
    /// value : {"SingleCurve", "Constituents"}
    ///     ``"SingleCurve"`` prices the index against a single index hazard
    ///     curve (synthetic CDS). ``"Constituents"`` prices each issuer
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

/// Register the typed credit-derivative instruments on the instruments
/// submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyCreditDefaultSwap>()?;
    m.add_class::<PyCreditDefaultSwapBuilder>()?;
    m.add_class::<PyCDSIndex>()?;
    m.add_class::<PyCDSIndexBuilder>()?;
    Ok(())
}
