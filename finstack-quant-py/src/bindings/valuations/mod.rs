//! Python bindings for the `finstack-quant-valuations` crate.
//!
//! Exposes the [`PyValuationResult`] envelope for pricing output,
//! JSON-based instrument loading and the standard pricer pipeline.

pub(crate) mod composite;
mod credit_derivatives;
mod exotic_rates;
pub(crate) mod instruments;
mod merton_mc;
mod pricing;
mod schema;
mod structured_credit;
pub(crate) mod typed_credit;
pub(crate) mod typed_equity;
pub(crate) mod typed_fx;
mod typed_legs;
pub(crate) mod typed_rates;
pub(crate) mod typed_structured_credit;

use crate::bindings::pandas_utils::{
    serde_object_to_single_row_dataframe_with_schema, serde_to_py,
};
use crate::errors::display_to_py;
use pyo3::prelude::*;
use pyo3::types::PyList;

#[pyclass(
    name = "ValuationResult",
    module = "finstack_quant.valuations",
    skip_from_py_object
)]
#[derive(Clone)]
struct PyValuationResult {
    pub(crate) inner: finstack_quant_valuations::results::ValuationResult,
}

#[pymethods]
impl PyValuationResult {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    #[staticmethod]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: finstack_quant_valuations::results::ValuationResult =
            serde_json::from_str(json).map_err(display_to_py)?;
        Ok(Self { inner })
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(display_to_py)
    }

    #[getter]
    fn instrument_id(&self) -> &str {
        &self.inner.instrument_id
    }

    /// Valuation date (T+0) for the calculation, as ``datetime.date``.
    #[getter]
    fn as_of<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        crate::bindings::core::dates::utils::date_to_py(py, self.inner.as_of)
    }

    /// Wire-format schema version of the result envelope (currently ``1``).
    #[getter]
    fn schema_version(&self) -> u32 {
        self.inner.schema_version.into()
    }

    #[getter]
    fn get_price(&self) -> f64 {
        self.inner.value.amount()
    }

    /// Return the exact Decimal price as a string, without a float round-trip.
    ///
    /// Unlike the ``price`` property (a lossy ``float``), this preserves the
    /// internal Decimal representation exactly. Pass the result to
    /// ``decimal.Decimal`` for lossless arithmetic in Python.
    ///
    /// Returns
    /// -------
    /// str
    ///     Exact decimal string of the valuation amount, e.g. ``"1000000.00"``.
    #[pyo3(text_signature = "($self)")]
    fn price_decimal(&self) -> String {
        self.inner.value.amount_decimal().to_string()
    }

    #[getter]
    fn currency(&self) -> String {
        self.inner.value.currency().to_string()
    }

    fn get_metric(&self, key: &str) -> Option<f64> {
        self.inner.metric_str(key)
    }

    /// Decoded component vectors and values for a composite base metric.
    ///
    /// Despite the ``_series`` suffix (which mirrors the Rust name) this is a
    /// plain ``list`` of tuples, not a :class:`pandas.Series`. Use
    /// :meth:`to_dataframe` for the tabular view.
    ///
    /// Results preserve the underlying ``measures`` insertion order. Legacy
    /// malformed escapes remain literal, and decoded-coordinate collisions
    /// fall back to literal wire components so every value remains visible.
    fn metric_series(&self, base: &str) -> Vec<(Vec<String>, f64)> {
        let base = finstack_quant_valuations::metrics::MetricId::custom(base);
        self.inner.metric_series(&base)
    }

    fn metric_keys(&self) -> Vec<String> {
        self.inner.measures.keys().map(|k| k.to_string()).collect()
    }

    fn metric_count(&self) -> usize {
        self.inner.measures.len()
    }

    fn all_covenants_passed(&self) -> bool {
        self.inner.all_covenants_passed()
    }

    fn failed_covenants(&self) -> Vec<String> {
        self.inner
            .failed_covenants()
            .into_iter()
            .map(String::from)
            .collect()
    }

    /// Export the headline result as a single-row pandas ``DataFrame``.
    ///
    /// Columns: ``instrument_id``, ``as_of_date`` (ISO 8601 string), ``pv``,
    /// ``currency``, then one column per metric key in ``measures`` insertion
    /// order.
    ///
    /// This is the default export. It is built from the Rust crate's own
    /// ``ValuationResult::to_row`` flattener, so the Python frame and the
    /// Rust-side DataFrame rows cannot drift apart. Stack a book with
    /// ``pd.concat([r.to_dataframe() for r in results])``; instruments with
    /// different metric sets align on column name and leave ``NaN`` elsewhere.
    #[pyo3(text_signature = "($self)")]
    fn to_dataframe<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let row = self.inner.to_row();
        serde_object_to_single_row_dataframe_with_schema(
            py,
            &row,
            &["instrument_id", "as_of_date", "pv", "currency"],
        )
    }

    /// Policy stamps: numeric mode, rounding context, FX policy, and timing.
    ///
    /// Same serde shape as the Rust ``ResultsMeta`` object already present on
    /// the WASM ``ValuationResult``.
    #[getter]
    fn meta<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        serde_to_py(py, &self.inner.meta)
    }

    /// Model-specific structured pricing detail, or ``None``.
    ///
    /// Same tagged ``{type, data}`` shape as the Rust ``ValuationDetails``
    /// enum. Absent when the pricer emitted only the scalar envelope.
    #[getter]
    fn details<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyAny>>> {
        self.inner
            .details
            .as_ref()
            .map(|details| serde_to_py(py, details))
            .transpose()
    }

    fn __repr__(&self) -> String {
        format!(
            "ValuationResult(id={:?}, price={:.4}, currency={}, metrics={})",
            self.inner.instrument_id,
            self.inner.value.amount(),
            self.inner.value.currency(),
            self.inner.measures.len()
        )
    }
}

/// Validate a tagged instrument JSON payload and return the canonical envelope.
///
/// Parameters
/// ----------
/// json : str
///     A ``finstack_quant.instrument/1`` envelope. Bare instrument payloads
///     are rejected.
///
/// Returns
/// -------
/// str
///     Canonical instrument envelope for the validated instrument.
///
/// Raises
/// ------
/// ValueError
///     If ``json`` is malformed, is not a canonical v1 envelope, or fails
///     instrument validation.
#[pyfunction]
fn validate_instrument_json(json: &str) -> PyResult<String> {
    finstack_quant_valuations::pricer::validate_instrument_json(json)
        .map_err(crate::errors::display_to_py)
}

/// Validate a payload as one exact instrument type and return the canonical envelope.
///
/// Pure delegation to the Rust
/// ``finstack_quant_valuations::pricer::validate_typed_instrument_json`` used
/// by the WASM typed FX classes' ``fromJson`` constructors.
///
/// Parameters
/// ----------
/// type_tag : str
///     Canonical instrument discriminator (e.g. ``"fx_forward"``).
/// json : str
///     A ``finstack_quant.instrument/1`` envelope for exactly that type.
///
/// Returns
/// -------
/// str
///     The canonical instrument envelope for the validated instrument.
///
/// Raises
/// ------
/// ValueError
///     If ``json`` is malformed, carries a different instrument type, or
///     fails instrument validation.
#[pyfunction]
#[pyo3(text_signature = "(type_tag, json)")]
fn validate_typed_instrument_json(type_tag: &str, json: &str) -> PyResult<String> {
    finstack_quant_valuations::pricer::validate_typed_instrument_json(type_tag, json)
        .map_err(crate::errors::display_to_py)
}

/// Re-render a canonical instrument envelope as pretty-printed JSON.
///
/// Pure delegation to the Rust
/// ``finstack_quant_valuations::pricer::pretty_instrument_json`` used by the
/// WASM typed FX classes' ``toJson`` methods.
///
/// Parameters
/// ----------
/// json : str
///     A canonical ``finstack_quant.instrument/1`` envelope.
///
/// Returns
/// -------
/// str
///     The same envelope, pretty-printed.
///
/// Raises
/// ------
/// ValueError
///     If ``json`` is malformed or cannot be rendered.
#[pyfunction]
#[pyo3(text_signature = "(json)")]
fn pretty_instrument_json(json: &str) -> PyResult<String> {
    finstack_quant_valuations::pricer::pretty_instrument_json(json)
        .map_err(crate::errors::display_to_py)
}

/// Construct tagged bond instrument JSON from a cashflow schedule.
#[pyfunction]
#[pyo3(
    signature = (instrument_id, schedule_json, discount_curve_id, quoted_clean = None),
    text_signature = "(instrument_id, schedule_json, discount_curve_id, quoted_clean=None)"
)]
fn bond_from_cashflows_json(
    py: Python<'_>,
    instrument_id: &str,
    schedule_json: &str,
    discount_curve_id: &str,
    quoted_clean: Option<f64>,
) -> PyResult<String> {
    py.detach(|| {
        finstack_quant_valuations::instruments::fixed_income::bond::bond_from_cashflows_json(
            instrument_id,
            schedule_json,
            discount_curve_id,
            quoted_clean,
        )
        .map_err(crate::errors::core_to_py)
    })
}

pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "valuations")?;
    let qual = crate::bindings::module_utils::set_submodule_package(
        parent,
        &m,
        "valuations",
        crate::bindings::module_utils::ROOT_PACKAGE,
        crate::bindings::module_utils::ParentNameSource::Name,
    )?;
    m.setattr(
        "__doc__",
        "Instrument pricing for bonds, swaps, options, and structured products.",
    )?;

    m.add_class::<PyValuationResult>()?;
    composite::register(py, &m)?;
    exotic_rates::register(py, &m)?;
    credit_derivatives::register(py, &m)?;
    schema::register(py, &m)?;
    register_instruments(py, &m)?;
    register_market(py, &m)?;

    let all = PyList::new(
        py,
        [
            "ValuationResult",
            "tarn_coupon_profile",
            "snowball_coupon_profile",
            "inverse_floater_coupon_profile",
            "cms_spread_option_intrinsic",
            "callable_range_accrual_accrued",
            "composite",
            "credit_derivatives",
            "instruments",
            "market",
            "schema",
        ],
    )?;
    m.setattr("__all__", all)?;
    crate::bindings::module_utils::register_submodule_at(py, parent, &m, &qual)?;

    Ok(())
}

fn register_instruments(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "instruments")?;
    let qual = crate::bindings::module_utils::set_submodule_package_by_package(
        parent,
        &m,
        "instruments",
        "finstack_quant.valuations",
    )?;
    m.setattr(
        "__doc__",
        "JSON validation, pricing, metric, and cashflow helpers for valuation workflows.",
    )?;

    m.add_function(wrap_pyfunction!(validate_instrument_json, &m)?)?;
    m.add_function(wrap_pyfunction!(validate_typed_instrument_json, &m)?)?;
    m.add_function(wrap_pyfunction!(pretty_instrument_json, &m)?)?;
    m.add_function(wrap_pyfunction!(bond_from_cashflows_json, &m)?)?;
    for name in [
        "bond_from_cashflows_json",
        "pretty_instrument_json",
        "validate_typed_instrument_json",
    ] {
        m.getattr(name)?
            .setattr("__module__", "finstack_quant.valuations.instruments")?;
    }
    instruments::register(py, &m)?;
    merton_mc::register(py, &m)?;
    typed_legs::register(py, &m)?;
    typed_rates::register(py, &m)?;
    typed_credit::register(py, &m)?;
    typed_equity::register(py, &m)?;
    typed_fx::register(py, &m)?;
    typed_structured_credit::register(py, &m)?;
    pricing::register(py, &m)?;
    structured_credit::register(&m)?;
    let mut exports = vec![
        "AssetPool",
        "BarrierCrossing",
        "Bond",
        "CDSIndex",
        "CDSIndexBuilder",
        "CDSTranche",
        "CDSTrancheBuilder",
        "CapFloor",
        "CapFloorBuilder",
        "ConvertibleBond",
        "ConvertibleBondBuilder",
        "CreditDefaultSwap",
        "CreditDefaultSwapBuilder",
        "EquityOption",
        "EquityOptionBuilder",
        "FixedLegSpec",
        "FloatLegSpec",
        "FxForward",
        "FxForwardBuilder",
        "FxOption",
        "FxOptionBuilder",
        "InterestRateSwap",
        "InterestRateSwapBuilder",
        "PremiumLegSpec",
        "ProtectionLegSpec",
        "RepLine",
        "StructuredCredit",
        "StructuredCreditBuilder",
        "Swaption",
        "SwaptionBuilder",
        "TermLoan",
        "Tranche",
        "TrancheBuilder",
        "TrancheStructure",
        "bond_from_cashflows_json",
        "instrument_cashflows_json",
        "list_models",
        "list_models_grouped",
        "list_standard_metrics",
        "list_standard_metrics_grouped",
        "pretty_instrument_json",
        "price_instrument",
        "validate_instrument_json",
        "validate_typed_instrument_json",
    ];
    exports.extend_from_slice(merton_mc::EXPORTS);
    exports.extend_from_slice(structured_credit::EXPORTS);
    exports.sort_unstable();
    exports.dedup();
    let all = PyList::new(py, exports)?;
    m.setattr("__all__", all)?;
    crate::bindings::module_utils::register_submodule_at(py, parent, &m, &qual)?;
    Ok(())
}

fn register_market(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "market")?;
    let qual = crate::bindings::module_utils::set_submodule_package_by_package(
        parent,
        &m,
        "market",
        "finstack_quant.valuations",
    )?;
    m.setattr(
        "__doc__",
        "Listed-market product coverage and exchange routing metadata.",
    )?;

    pricing::register_market(py, &m)?;
    let all = PyList::new(py, ["listed_product_catalog"])?;
    m.setattr("__all__", all)?;
    crate::bindings::module_utils::register_submodule_at(py, parent, &m, &qual)?;
    Ok(())
}
