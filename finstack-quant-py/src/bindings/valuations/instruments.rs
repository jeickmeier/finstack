//! Typed instrument classes for the `finstack_quant.valuations.instruments`
//! namespace.
//!
//! Thin wrappers over the canonical Rust structs
//! [`finstack_quant_valuations::instruments::Bond`] and
//! [`finstack_quant_valuations::instruments::TermLoan`]. Construction and
//! validation stay in Rust; the wrappers only convert to and from the canonical
//! `finstack_quant.instrument/1` envelope accepted by the JSON loader.

use pyo3::prelude::*;

use crate::bindings::core::dates::schedule::PyStubKind;
use crate::bindings::core::dates::utils::py_to_date;
use crate::bindings::core::money::PyMoney;
use crate::bindings::core::types::{PyBps, PyRate};
use crate::bindings::valuations::merton_mc::{PyMertonMcConfig, PyMertonMcResult};
use crate::errors::{core_to_py, serde_json_to_py};
use finstack_quant_valuations::instruments::{InstrumentEnvelope, InstrumentJson};

/// Parse a canonical typed-instrument envelope through the shared Rust path.
pub(crate) fn parse_typed_instrument_json(json: &str) -> PyResult<InstrumentJson> {
    finstack_quant_valuations::pricer::json::parse_instrument_from_json(json).map_err(core_to_py)
}

/// Serialize a typed instrument as the canonical v1 persistence envelope.
pub(crate) fn serialize_typed_instrument_json(
    instrument: InstrumentJson,
    what: &str,
) -> PyResult<String> {
    serde_json::to_string(&InstrumentEnvelope::new(instrument)).map_err(|err| {
        serde_json_to_py(
            err,
            &format!("failed to serialize {what} instrument envelope"),
        )
    })
}

// Shared helpers for typed instrument builders (bond/term_loan today; every
// later typed-instrument task reuses these three).

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
pub(crate) fn json_field<T: serde::de::DeserializeOwned>(json: &str, what: &str) -> PyResult<T> {
    serde_json::from_str(json)
        .map_err(|err| crate::errors::serde_json_to_py(err, &format!("invalid {what} JSON")))
}

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
    /// Serialize as the canonical instrument envelope accepted by the JSON loader.
    pub(crate) fn envelope_json(&self) -> PyResult<String> {
        serialize_typed_instrument_json(InstrumentJson::Bond(self.inner.clone()), "Bond")
    }
}

#[pymethods]
impl PyBond {
    /// Create a US corporate fixed-rate bond (semi-annual, 30/360, T+1).
    ///
    /// Mirrors Rust ``Bond::fixed`` and requires an explicit stub policy.
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
    /// stub : StubKind
    ///     Placement and length policy for an irregular coupon period.
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
    /// >>> from finstack_quant.core.dates import StubKind
    /// >>> from finstack_quant.valuations.instruments import Bond
    /// >>> bond = Bond.fixed(
    /// ...     "BOND-1",
    /// ...     Money(1_000_000.0, Currency("USD")),
    /// ...     Rate(0.05),
    /// ...     datetime.date(2024, 1, 1),
    /// ...     datetime.date(2034, 1, 1),
    /// ...     StubKind.NONE,
    /// ...     "USD-OIS",
    /// ... )
    /// >>> bond.id
    /// 'BOND-1'
    #[staticmethod]
    #[pyo3(
        text_signature = "(id, notional, coupon_rate, issue, maturity, stub, discount_curve_id)"
    )]
    fn fixed(
        id: &str,
        notional: PyRef<'_, PyMoney>,
        coupon_rate: PyRef<'_, PyRate>,
        issue: &Bound<'_, PyAny>,
        maturity: &Bound<'_, PyAny>,
        stub: PyRef<'_, PyStubKind>,
        discount_curve_id: &str,
    ) -> PyResult<Self> {
        let inner = finstack_quant_valuations::instruments::Bond::fixed(
            id,
            notional.inner,
            coupon_rate.inner,
            py_to_date(issue)?,
            py_to_date(maturity)?,
            stub.inner,
            discount_curve_id,
        )
        .map_err(core_to_py)?;
        Ok(Self { inner })
    }

    /// Create a floating-rate bond (FRN) linked to a forward index.
    ///
    /// Mirrors Rust ``Bond::floating``. Settlement, calendar, and
    /// business-day convention come from the notional currency:
    /// USD ``UsCorporate`` (T+1, ``usny``), EUR ``EurCorporate`` (T+2,
    /// ``target2``), GBP ``UkGilt`` (T+1), JPY ``Jgb`` (T+2). Other
    /// currencies raise ``ValueError``; use
    /// ``Bond.floating_with_convention`` when that constructor is
    /// exposed, or the builder.
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
    /// frequency : Tenor
    ///     Payment frequency (e.g. ``Tenor.quarterly()``).
    /// day_count : DayCount
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
    ///     If the notional currency has no mapped settlement convention,
    ///     ``notional`` is not finite and positive, or ``issue`` is not
    ///     strictly before ``maturity``.
    #[staticmethod]
    #[pyo3(
        text_signature = "(id, notional, index_id, margin_bp, issue, maturity, frequency, day_count, discount_curve_id)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn floating(
        id: &str,
        notional: PyRef<'_, PyMoney>,
        index_id: &str,
        margin_bp: PyRef<'_, PyBps>,
        issue: &Bound<'_, PyAny>,
        maturity: &Bound<'_, PyAny>,
        frequency: PyRef<'_, crate::bindings::core::dates::tenor::PyTenor>,
        day_count: PyRef<'_, crate::bindings::core::dates::daycount::PyDayCount>,
        discount_curve_id: &str,
    ) -> PyResult<Self> {
        let inner = finstack_quant_valuations::instruments::Bond::floating(
            id,
            notional.inner,
            index_id,
            margin_bp.inner,
            py_to_date(issue)?,
            py_to_date(maturity)?,
            frequency.inner,
            day_count.inner,
            discount_curve_id,
        )
        .map_err(core_to_py)?;
        Ok(Self { inner })
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

    /// Deserialize a validated bond from its canonical v1 envelope.
    ///
    /// Parameters
    /// ----------
    /// json : str
    ///     A ``finstack_quant.instrument/1`` envelope containing an exact
    ///     ``"bond"`` payload. The UTF-8 input must not exceed 16 MiB.
    ///     Bare payloads and cross-type coercion are rejected.
    ///
    /// Returns
    /// -------
    /// Bond
    ///     The validated bond represented by the exact ``"bond"`` payload.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the input exceeds 16 MiB, is malformed, has an unsupported
    ///     envelope schema, carries a type other than ``"bond"``, or fails
    ///     bond validation.
    ///
    /// Examples
    /// --------
    /// >>> import datetime
    /// >>> from finstack_quant.core.currency import Currency
    /// >>> from finstack_quant.core.dates import StubKind
    /// >>> from finstack_quant.core.money import Money
    /// >>> from finstack_quant.core.types import Rate
    /// >>> from finstack_quant.valuations.instruments import Bond
    /// >>> bond = Bond.fixed(
    /// ...     "B",
    /// ...     Money(1_000.0, Currency("USD")),
    /// ...     Rate(0.05),
    /// ...     datetime.date(2024, 1, 1),
    /// ...     datetime.date(2029, 1, 1),
    /// ...     StubKind.NONE,
    /// ...     "USD-OIS",
    /// ... )
    /// >>> Bond.from_json(bond.to_json()).id
    /// 'B'
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        match parse_typed_instrument_json(json)? {
            InstrumentJson::Bond(inner) => Ok(Self { inner }),
            _ => Err(crate::errors::value_error(
                "expected instrument type \"bond\", got a different instrument type",
            )),
        }
    }

    /// Serialize to a canonical ``finstack_quant.instrument/1`` envelope.
    ///
    /// Returns
    /// -------
    /// str
    ///     Canonical instrument envelope accepted by ``price_instrument`` and
    ///     ``Bond.from_json``.
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
        format!(
            "Bond(id={:?}, maturity={})",
            self.inner.id.as_str(),
            self.inner.maturity
        )
    }

    /// Price this bond with the Merton Monte Carlo structural credit engine.
    ///
    /// Uses geometric Brownian motion asset dynamics only. Floating-rate and
    /// amortizing cashflow specs are rejected. When the config's PIK schedule
    /// is the default uniform cash mode, the bond's ``CouponType`` overrides
    /// the schedule; otherwise the config schedule takes precedence.
    ///
    /// # Arguments
    ///
    /// * `config` - Merton MC simulation configuration including the structural model.
    /// * `discount_rate` - Flat continuously compounded risk-free rate as a decimal
    ///   used to discount simulated cashflows (unless term-structure DFs are set on
    ///   the config via JSON).
    /// * `as_of` - Valuation date (`datetime.date` or ISO 8601 string).
    fn price_merton_mc(
        &self,
        config: PyRef<'_, PyMertonMcConfig>,
        discount_rate: f64,
        as_of: &Bound<'_, PyAny>,
    ) -> PyResult<PyMertonMcResult> {
        let as_of = py_to_date(as_of)?;
        let result = self
            .inner
            .price_merton_mc(&config.inner, discount_rate, as_of)
            .map_err(core_to_py)?;
        Ok(PyMertonMcResult::from_inner(result))
    }
}

/// Typed wrapper for the Rust `TermLoan` instrument.
///
/// Rust has no ``fixed``/``floating`` convenience constructors for term
/// loans; construct via ``TermLoan.from_json`` with a canonical
/// ``finstack_quant.instrument/1`` envelope or start from ``TermLoan.example()``.
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
    /// Serialize as the canonical instrument envelope accepted by the JSON loader.
    pub(crate) fn envelope_json(&self) -> PyResult<String> {
        serialize_typed_instrument_json(InstrumentJson::TermLoan(self.inner.clone()), "TermLoan")
    }
}

#[pymethods]
impl PyTermLoan {
    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    /// Deserialize a validated term loan from its canonical v1 envelope.
    ///
    /// Parameters
    /// ----------
    /// json : str
    ///     A ``finstack_quant.instrument/1`` envelope containing an exact
    ///     ``"term_loan"`` payload. The UTF-8 input must not exceed 16 MiB.
    ///     Bare payloads and cross-type coercion are rejected.
    ///
    /// Returns
    /// -------
    /// TermLoan
    ///     The validated term loan represented by the exact ``"term_loan"`` payload.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the input exceeds 16 MiB, is malformed, has an unsupported
    ///     envelope schema, carries a type other than ``"term_loan"``, or
    ///     fails term-loan validation.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import TermLoan
    /// >>> TermLoan.from_json(TermLoan.example().to_json()).id
    /// 'TERM-LOAN-USD-5Y'
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        match parse_typed_instrument_json(json)? {
            InstrumentJson::TermLoan(inner) => Ok(Self { inner }),
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

    /// Serialize to a canonical ``finstack_quant.instrument/1`` envelope.
    ///
    /// Returns
    /// -------
    /// str
    ///     Canonical instrument envelope accepted by ``price_instrument`` and
    ///     ``TermLoan.from_json``.
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
