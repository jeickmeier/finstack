//! Typed instrument classes for the `finstack_quant.valuations.instruments`
//! namespace.
//!
//! Thin wrappers over the canonical Rust structs
//! [`finstack_quant_valuations::instruments::Bond`] and
//! [`finstack_quant_valuations::instruments::TermLoan`]. Construction and
//! validation stay in Rust; the wrappers only convert to and from the tagged
//! instrument JSON accepted by the JSON loader (`{"type": "bond", "spec":
//! ...}` / `{"type": "term_loan", "spec": ...}`).

use pyo3::prelude::*;
use pyo3::types::PyType;

use crate::bindings::core::dates::utils::py_to_date;
use crate::bindings::core::money::PyMoney;
use crate::bindings::core::types::{PyBps, PyRate};
use crate::errors::{core_to_py, serde_json_to_py};
use finstack_quant_valuations::instruments::{Instrument, InstrumentJson};

/// Parse tagged instrument JSON through the JSON-loader path and run the
/// same pricing-boundary validation the loader applies.
fn parse_tagged(json: &str) -> PyResult<InstrumentJson> {
    serde_json::from_str::<InstrumentJson>(json)
        .map_err(|err| serde_json_to_py(err, "invalid instrument JSON"))
}

// ---------------------------------------------------------------------------
// Shared helpers for typed instrument builders (bond/term_loan today; every
// later typed-instrument task reuses these three).
//
// `#[allow(dead_code)]`: `json_field` has no consumer yet (nested-spec
// setters land in a later typed-instrument task), so
// `clippy --all-targets -D warnings` flags it as unused. Remove the
// `#[allow(dead_code)]` the moment it gets a first call site.
// ---------------------------------------------------------------------------

/// Parse a serde-tagged unit-enum value from its snake_case string form.
///
/// Used by typed builders so Python passes plain strings (typed as
/// ``Literal[...]`` in the stubs) for Rust enums like ``PayReceive``.
pub(crate) fn enum_from_str<T: serde::de::DeserializeOwned>(
    value: &str,
    what: &str,
) -> PyResult<T> {
    serde_json::from_value(serde_json::Value::String(value.to_string()))
        .map_err(|err| crate::errors::value_error(format!("invalid {what}: {err}")))
}

/// Convert a Python float to `Decimal`, rejecting non-finite values.
pub(crate) fn decimal_from_f64(value: f64, what: &str) -> PyResult<rust_decimal::Decimal> {
    rust_decimal::Decimal::try_from(value)
        .map_err(|err| crate::errors::value_error(format!("invalid {what}: {err}")))
}

/// Parse a JSON sub-field string into a typed Rust spec value.
///
/// Used by ``*_json`` builder setters for deep nested config (margin specs,
/// waterfall rules, conversion terms) per the nested-spec rule in the plan.
#[allow(dead_code)]
pub(crate) fn json_field<T: serde::de::DeserializeOwned>(json: &str, what: &str) -> PyResult<T> {
    serde_json::from_str(json)
        .map_err(|err| crate::errors::serde_json_to_py(err, &format!("invalid {what} JSON")))
}

#[cfg(test)]
mod helper_tests {
    use super::*;

    #[test]
    fn enum_from_str_parses_pay_receive() {
        let side: finstack_quant_valuations::instruments::PayReceive =
            enum_from_str("pay", "side").expect("parses");
        assert!(matches!(
            side,
            finstack_quant_valuations::instruments::PayReceive::Pay
        ));
    }

    #[test]
    fn enum_from_str_rejects_unknown_variant() {
        let err =
            enum_from_str::<finstack_quant_valuations::instruments::PayReceive>("sideways", "side")
                .unwrap_err();
        // `cargo test` on this crate runs outside a Python process, so the
        // interpreter is not yet attached; inspecting `PyErr`'s value (or its
        // `Display`/`Debug` impls, which also call `Python::attach`
        // internally) requires initializing it first. `Python::initialize`
        // does not require the `auto-initialize` feature (unlike the
        // deprecated `prepare_freethreaded_python`), and this crate cannot
        // enable that feature because it conflicts with `extension-module`,
        // which the published wheel requires.
        pyo3::Python::initialize();
        pyo3::Python::attach(|py| {
            assert!(err.value(py).to_string().contains("invalid side"));
        });
    }

    #[test]
    fn decimal_from_f64_rejects_nan() {
        assert!(decimal_from_f64(f64::NAN, "strike").is_err());
        assert!(decimal_from_f64(0.05, "strike").is_ok());
    }
}

// ---------------------------------------------------------------------------
// Bond
// ---------------------------------------------------------------------------

/// Typed wrapper for the Rust `Bond` instrument.
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "Bond",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyBond {
    /// Inner canonical Rust bond.
    pub(crate) inner: finstack_quant_valuations::instruments::Bond,
}

impl PyBond {
    /// Serialize as the tagged instrument JSON accepted by the JSON loader.
    pub(crate) fn tagged_json(&self) -> PyResult<String> {
        serde_json::to_string(&InstrumentJson::Bond(self.inner.clone()))
            .map_err(|err| serde_json_to_py(err, "failed to serialize Bond"))
    }
}

#[pymethods]
impl PyBond {
    /// Create a standard fixed-rate bond (semi-annual, 30/360, T+2).
    ///
    /// Mirrors Rust ``Bond::fixed``.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique instrument identifier.
    /// notional : Money
    ///     Principal amount of the bond.
    /// coupon_rate : Rate
    ///     Annual coupon rate.
    /// issue : datetime.date
    ///     Issue date.
    /// maturity : datetime.date
    ///     Maturity date.
    /// discount_curve_id : str
    ///     Discount curve identifier used for pricing.
    ///
    /// Returns
    /// -------
    /// Bond
    ///     A validated fixed-rate bond.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If validation fails (e.g. maturity not after issue).
    ///
    /// Examples
    /// --------
    /// >>> import datetime
    /// >>> from finstack_quant.core.currency import Currency
    /// >>> from finstack_quant.core.money import Money
    /// >>> from finstack_quant.core.types import Rate
    /// >>> from finstack_quant.valuations.instruments import Bond
    /// >>> bond = Bond.fixed(
    /// ...     "BOND-1",
    /// ...     Money(1_000_000.0, Currency("USD")),
    /// ...     Rate(0.05),
    /// ...     datetime.date(2024, 1, 1),
    /// ...     datetime.date(2034, 1, 1),
    /// ...     "USD-OIS",
    /// ... )
    /// >>> bond.id
    /// 'BOND-1'
    #[staticmethod]
    #[pyo3(text_signature = "(id, notional, coupon_rate, issue, maturity, discount_curve_id)")]
    fn fixed(
        id: &str,
        notional: PyRef<'_, PyMoney>,
        coupon_rate: PyRef<'_, PyRate>,
        issue: &Bound<'_, PyAny>,
        maturity: &Bound<'_, PyAny>,
        discount_curve_id: &str,
    ) -> PyResult<Self> {
        let inner = finstack_quant_valuations::instruments::Bond::fixed(
            id,
            notional.inner,
            coupon_rate.inner,
            py_to_date(issue)?,
            py_to_date(maturity)?,
            discount_curve_id,
        )
        .map_err(core_to_py)?;
        Ok(Self { inner })
    }

    /// Create a floating-rate bond (FRN) linked to a forward index.
    ///
    /// Mirrors Rust ``Bond::floating``.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique instrument identifier.
    /// notional : Money
    ///     Principal amount of the bond.
    /// index_id : str
    ///     Forward curve identifier (e.g. ``"USD-SOFR-3M"``).
    /// margin_bp : Bps
    ///     Spread over the index in basis points.
    /// issue : datetime.date
    ///     Issue date.
    /// maturity : datetime.date
    ///     Maturity date.
    /// freq : Tenor
    ///     Payment frequency (e.g. ``Tenor.quarterly()``).
    /// dc : DayCount
    ///     Day count convention (e.g. ``DayCount.act360()``).
    /// discount_curve_id : str
    ///     Discount curve identifier used for pricing.
    ///
    /// Returns
    /// -------
    /// Bond
    ///     A validated floating-rate note.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If validation fails.
    #[staticmethod]
    #[pyo3(
        text_signature = "(id, notional, index_id, margin_bp, issue, maturity, freq, dc, discount_curve_id)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn floating(
        id: &str,
        notional: PyRef<'_, PyMoney>,
        index_id: &str,
        margin_bp: PyRef<'_, PyBps>,
        issue: &Bound<'_, PyAny>,
        maturity: &Bound<'_, PyAny>,
        freq: PyRef<'_, crate::bindings::core::dates::tenor::PyTenor>,
        dc: PyRef<'_, crate::bindings::core::dates::daycount::PyDayCount>,
        discount_curve_id: &str,
    ) -> PyResult<Self> {
        let inner = finstack_quant_valuations::instruments::Bond::floating(
            id,
            notional.inner,
            index_id,
            margin_bp.inner,
            py_to_date(issue)?,
            py_to_date(maturity)?,
            freq.inner,
            dc.inner,
            discount_curve_id,
        )
        .map_err(core_to_py)?;
        Ok(Self { inner })
    }

    /// Deserialize a bond from tagged instrument JSON.
    ///
    /// Accepts the same ``{"type": "bond", "spec": {...}}`` payload the
    /// JSON loader accepts; the loader's validation runs on the result.
    ///
    /// Parameters
    /// ----------
    /// json : str
    ///     Tagged instrument JSON with type ``"bond"``.
    ///
    /// Returns
    /// -------
    /// Bond
    ///     The validated bond.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the JSON is malformed, has a different instrument type, or
    ///     fails validation.
    #[classmethod]
    #[pyo3(text_signature = "(cls, json)")]
    fn from_json(_cls: &Bound<'_, PyType>, json: &str) -> PyResult<Self> {
        match parse_tagged(json)? {
            InstrumentJson::Bond(inner) => {
                inner.validate_for_pricing().map_err(core_to_py)?;
                Ok(Self { inner })
            }
            _ => Err(crate::errors::value_error(
                "expected instrument type \"bond\", got a different instrument type",
            )),
        }
    }

    /// Serialize to tagged instrument JSON (``{"type": "bond", "spec": ...}``).
    ///
    /// Returns
    /// -------
    /// str
    ///     Tagged instrument JSON accepted by ``price_instrument`` and
    ///     ``Bond.from_json``.
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
        format!(
            "Bond(id={:?}, maturity={})",
            self.inner.id.as_str(),
            self.inner.maturity
        )
    }
}

// ---------------------------------------------------------------------------
// TermLoan
// ---------------------------------------------------------------------------

/// Typed wrapper for the Rust `TermLoan` instrument.
///
/// Rust has no ``fixed``/``floating`` convenience constructors for term
/// loans; construct via ``TermLoan.from_json`` with tagged JSON
/// (``{"type": "term_loan", "spec": ...}``) or start from
/// ``TermLoan.example()``.
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "TermLoan",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyTermLoan {
    /// Inner canonical Rust term loan.
    pub(crate) inner: finstack_quant_valuations::instruments::TermLoan,
}

impl PyTermLoan {
    /// Serialize as the tagged instrument JSON accepted by the JSON loader.
    pub(crate) fn tagged_json(&self) -> PyResult<String> {
        serde_json::to_string(&InstrumentJson::TermLoan(self.inner.clone()))
            .map_err(|err| serde_json_to_py(err, "failed to serialize TermLoan"))
    }
}

#[pymethods]
impl PyTermLoan {
    /// Deserialize a term loan from tagged instrument JSON.
    ///
    /// Accepts the same ``{"type": "term_loan", "spec": {...}}`` payload the
    /// JSON loader accepts; the loader's validation runs on the result.
    ///
    /// Parameters
    /// ----------
    /// json : str
    ///     Tagged instrument JSON with type ``"term_loan"``.
    ///
    /// Returns
    /// -------
    /// TermLoan
    ///     The validated term loan.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the JSON is malformed, has a different instrument type, or
    ///     fails validation.
    #[classmethod]
    #[pyo3(text_signature = "(cls, json)")]
    fn from_json(_cls: &Bound<'_, PyType>, json: &str) -> PyResult<Self> {
        match parse_tagged(json)? {
            InstrumentJson::TermLoan(inner) => {
                inner.validate_for_pricing().map_err(core_to_py)?;
                Ok(Self { inner })
            }
            _ => Err(crate::errors::value_error(
                "expected instrument type \"term_loan\", got a different instrument type",
            )),
        }
    }

    /// Canonical example term loan (mirrors Rust ``TermLoan::example``).
    ///
    /// Returns a 5-year USD fixed-rate loan (6%, quarterly, Act/360, 2.5%
    /// per-period amortization) useful as a starting point and in tests.
    ///
    /// Returns
    /// -------
    /// TermLoan
    ///     The example loan.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If construction fails (should not occur).
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn example() -> PyResult<Self> {
        finstack_quant_valuations::instruments::TermLoan::example()
            .map(|inner| Self { inner })
            .map_err(core_to_py)
    }

    /// Serialize to tagged instrument JSON (``{"type": "term_loan", "spec": ...}``).
    ///
    /// Returns
    /// -------
    /// str
    ///     Tagged instrument JSON accepted by ``price_instrument`` and
    ///     ``TermLoan.from_json``.
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
        format!(
            "TermLoan(id={:?}, maturity={})",
            self.inner.id.as_str(),
            self.inner.maturity
        )
    }
}

/// Register the typed instrument classes on the instruments submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyBond>()?;
    m.add_class::<PyTermLoan>()?;
    Ok(())
}
