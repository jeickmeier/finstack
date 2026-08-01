//! Typed structured-credit deal-modeling surface: `RepLine`, `AssetPool`,
//! `Tranche`, `TrancheStructure`, and the `StructuredCredit` instrument.
//!
//! Mirrors the `PyBond` pattern in `instruments.rs` for `StructuredCredit`
//! (the `Instrument`) and the `PyFixedLegSpec` pattern in `typed_legs.rs` for
//! the flat sub-models. Deep/optional sub-configs (`WaterfallRules`,
//! `CreditModelConfig`, `DealFees`, `MarketConditions`/`CreditFactors`,
//! loan-level `PoolAsset`, floating `TrancheCoupon`) stay JSON sub-fields per
//! the nested-spec rule; only the flat deal-modeling types above are typed.
//!
//! `DealType` and `TrancheSeniority` have no `#[serde(rename_all)]` in Rust —
//! their wire representation is PascalCase/acronym (`"abs"`, `"clo"`,
//! `"Senior"`, ...). This binding accepts exactly that wire casing at the
//! Python surface, routed through the generic [`enum_from_str`] helper like
//! every other typed instrument on this branch, so `to_json()` output round-
//! trips directly back into these constructors without any translation.

use pyo3::prelude::*;
use pyo3::types::PyType;

use crate::bindings::core::currency::PyCurrency;
use crate::bindings::core::dates::daycount::PyDayCount;
use crate::bindings::core::dates::tenor::PyTenor;
use crate::bindings::core::dates::utils::py_to_date;
use crate::bindings::core::money::PyMoney;
use crate::errors::{core_to_py, value_error};
use finstack_quant_core::dates::BusinessDayConvention;
use finstack_quant_core::types::{CurveId, InstrumentId};
use finstack_quant_valuations::instruments::fixed_income::structured_credit::{
    AssetPool, CreditFactors, DealFees, DealType, DefaultAssumptions, MarketConditions, Metadata,
    Overrides, PoolAsset, RepLine, StructuredCredit, Tranche, TrancheCoupon, TrancheSeniority,
    TrancheStructure, WaterfallRules,
};
use finstack_quant_valuations::instruments::{Instrument, InstrumentJson};

use super::instruments::{
    enum_from_str, json_field, parse_typed_instrument_json, serialize_typed_instrument_json,
};

type TrancheBuilderInner =
    finstack_quant_valuations::instruments::fixed_income::structured_credit::TrancheBuilder;
type StructuredCreditBuilderInner =
    finstack_quant_valuations::instruments::fixed_income::structured_credit::StructuredCreditBuilder;

// ---------------------------------------------------------------------------
// RepLine
// ---------------------------------------------------------------------------

/// Typed wrapper for the Rust `RepLine` (aggregated representative pool line).
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "RepLine",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyRepLine {
    /// Inner canonical Rust rep line.
    pub(crate) inner: RepLine,
}

#[pymethods]
impl PyRepLine {
    /// Aggregated representative line for pool modeling.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique identifier for the rep line.
    /// balance : Money
    ///     Aggregated balance.
    /// rate : float
    ///     Weighted average coupon as an annual decimal rate (e.g. ``0.07``
    ///     = 7%).
    /// maturity : datetime.date
    ///     Weighted average maturity date.
    /// seasoning_months : int
    ///     Weighted average seasoning in months.
    /// day_count : DayCount
    ///     Day count convention.
    /// spread_bp : float, optional
    ///     Weighted average spread over the reference index, in basis
    ///     points (e.g. ``150.0`` = 150bp), for floating-rate lines.
    /// index_id : str, optional
    ///     Reference index identifier, if floating.
    /// cpr : float, optional
    ///     Constant prepayment rate override, as an annual decimal (e.g.
    ///     ``0.10`` = 10% CPR).
    /// cdr : float, optional
    ///     Constant default rate override, as an annual decimal (e.g.
    ///     ``0.02`` = 2% CDR).
    /// recovery_rate : float, optional
    ///     Recovery rate override, as a decimal fraction (e.g. ``0.45`` =
    ///     45%).
    ///
    /// Returns
    /// -------
    /// RepLine
    ///     The rep line.
    ///
    /// Raises
    /// ------
    /// TypeError
    ///     If ``maturity`` is not a date-like object (``datetime.date``,
    ///     ``datetime.datetime``, or ``pandas.Timestamp``).
    ///
    /// Examples
    /// --------
    /// >>> import datetime
    /// >>> from finstack_quant.core.currency import Currency
    /// >>> from finstack_quant.core.dates import DayCount
    /// >>> from finstack_quant.core.money import Money
    /// >>> from finstack_quant.valuations.instruments import RepLine
    /// >>> line = RepLine(
    /// ...     "LINE-1", Money(80_000_000.0, Currency("USD")), 0.07,
    /// ...     datetime.date(2031, 1, 15), 12, DayCount.ACT_360,
    /// ...     cpr=0.10, cdr=0.02, recovery_rate=0.45,
    /// ... )
    /// >>> "LINE-1" in repr(line)
    /// True
    #[new]
    #[pyo3(signature = (id, balance, rate, maturity, seasoning_months, day_count, *,
                        spread_bp = None, index_id = None, cpr = None, cdr = None,
                        recovery_rate = None))]
    #[pyo3(
        text_signature = "(id, balance, rate, maturity, seasoning_months, day_count, *, \
spread_bp=None, index_id=None, cpr=None, cdr=None, recovery_rate=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn new(
        id: &str,
        balance: PyRef<'_, PyMoney>,
        rate: f64,
        maturity: &Bound<'_, PyAny>,
        seasoning_months: u32,
        day_count: PyRef<'_, PyDayCount>,
        spread_bp: Option<f64>,
        index_id: Option<String>,
        cpr: Option<f64>,
        cdr: Option<f64>,
        recovery_rate: Option<f64>,
    ) -> PyResult<Self> {
        let maturity = py_to_date(maturity)?;
        let mut inner = RepLine::new(
            id,
            balance.inner,
            rate,
            spread_bp,
            index_id,
            maturity,
            seasoning_months,
            day_count.inner,
        );
        if let Some(cpr) = cpr {
            inner = inner.with_cpr(cpr);
        }
        if let Some(cdr) = cdr {
            inner = inner.with_cdr(cdr);
        }
        if let Some(recovery_rate) = recovery_rate {
            inner = inner.with_recovery_rate(recovery_rate);
        }
        Ok(Self { inner })
    }

    /// Return ``repr(self)``.
    fn __repr__(&self) -> String {
        format!(
            "RepLine(id={:?}, balance={}, rate={})",
            self.inner.id, self.inner.balance, self.inner.rate
        )
    }
}

// ---------------------------------------------------------------------------
// AssetPool
// ---------------------------------------------------------------------------

/// Typed wrapper for the Rust `AssetPool` (structured-credit collateral pool).
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "AssetPool",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyAssetPool {
    /// Inner canonical Rust asset pool.
    pub(crate) inner: AssetPool,
}

#[pymethods]
impl PyAssetPool {
    /// Structured-credit collateral pool.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Pool identifier.
    /// deal_type : {"clo", "cbo", "abs", "rmbs", "cmbs", "auto", "card"}
    ///     Deal classification for pool-level assumptions.
    /// base_currency : Currency
    ///     Base currency for every asset and pool-level account.
    ///
    /// Returns
    /// -------
    /// AssetPool
    ///     A new, empty asset pool. Use :meth:`with_rep_lines` and/or
    ///     :meth:`assets_json` to attach collateral.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``deal_type`` is not a recognized deal type.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.core.currency import Currency
    /// >>> from finstack_quant.valuations.instruments import AssetPool
    /// >>> pool = AssetPool("POOL-1", "abs", Currency("USD"))
    /// >>> "POOL-1" in repr(pool)
    /// True
    #[new]
    #[pyo3(text_signature = "(id, deal_type, base_currency)")]
    fn new(id: &str, deal_type: &str, base_currency: PyRef<'_, PyCurrency>) -> PyResult<Self> {
        let deal_type: DealType = enum_from_str(deal_type, "deal_type")?;
        let inner = AssetPool::new(id, deal_type, base_currency.inner);
        Ok(Self { inner })
    }

    /// Attach representative pool lines, returning a new pool.
    ///
    /// Parameters
    /// ----------
    /// rep_lines : list[RepLine]
    ///     Aggregated representative lines the pricing engine will use
    ///     instead of individual assets.
    ///
    /// Returns
    /// -------
    /// AssetPool
    ///     A new pool with ``rep_lines`` set (the original is unchanged).
    ///
    /// Raises
    /// ------
    /// TypeError
    ///     If an element of ``rep_lines`` is not a ``RepLine``.
    ///
    /// Examples
    /// --------
    /// >>> import datetime
    /// >>> from finstack_quant.core.currency import Currency
    /// >>> from finstack_quant.core.dates import DayCount
    /// >>> from finstack_quant.core.money import Money
    /// >>> from finstack_quant.valuations.instruments import AssetPool, RepLine
    /// >>> pool = AssetPool("POOL-1", "abs", Currency("USD")).with_rep_lines([
    /// ...     RepLine(
    /// ...         "LINE-1", Money(80_000_000.0, Currency("USD")), 0.07,
    /// ...         datetime.date(2031, 1, 15), 12, DayCount.ACT_360,
    /// ...     )
    /// ... ])
    /// >>> "POOL-1" in repr(pool)
    /// True
    #[pyo3(text_signature = "($self, rep_lines)")]
    fn with_rep_lines(&self, rep_lines: Vec<PyRef<'_, PyRepLine>>) -> Self {
        let mut inner = self.inner.clone();
        inner.rep_lines = Some(rep_lines.iter().map(|line| line.inner.clone()).collect());
        Self { inner }
    }

    /// Attach loan-level assets from a JSON array, returning a new pool.
    ///
    /// Loan-level ``PoolAsset`` records carry ~30 fields and stay JSON per
    /// the nested-spec rule; use :meth:`with_rep_lines` for the typed,
    /// aggregated path.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     JSON array of ``PoolAsset`` objects.
    ///
    /// Returns
    /// -------
    /// AssetPool
    ///     A new pool with ``assets`` set (the original is unchanged).
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not valid JSON for the ``PoolAsset`` list shape.
    #[pyo3(text_signature = "($self, value)")]
    fn assets_json(&self, value: &str) -> PyResult<Self> {
        let assets: Vec<PoolAsset> = json_field(value, "assets")?;
        let mut inner = self.inner.clone();
        inner.assets = assets;
        Ok(Self { inner })
    }

    /// Return ``repr(self)``.
    fn __repr__(&self) -> String {
        format!(
            "AssetPool(id={:?}, deal_type={:?})",
            self.inner.id.as_str(),
            self.inner.deal_type
        )
    }
}

// ---------------------------------------------------------------------------
// Tranche
// ---------------------------------------------------------------------------

/// Typed wrapper for the Rust `Tranche`.
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "Tranche",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyTranche {
    /// Inner canonical Rust tranche.
    pub(crate) inner: Tranche,
}

#[pymethods]
impl PyTranche {
    /// Create a fluent builder (mirrors Rust ``Tranche::builder()``).
    ///
    /// Returns
    /// -------
    /// TrancheBuilder
    ///     A builder with fluent, consuming setter methods.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import Tranche
    /// >>> callable(Tranche.builder)
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn builder() -> PyTrancheBuilder {
        PyTrancheBuilder {
            inner: Some(Tranche::builder()),
            attachment_point: None,
            detachment_point: None,
        }
    }

    /// Return ``repr(self)``.
    fn __repr__(&self) -> String {
        format!(
            "Tranche(id={:?}, seniority={:?}, attachment_point={}, detachment_point={})",
            self.inner.id.as_str(),
            self.inner.seniority,
            self.inner.attachment_point,
            self.inner.detachment_point
        )
    }
}

/// Fluent builder for [`PyTranche`]; wraps the hand-written Rust
/// `TrancheBuilder` (consuming setters).
///
/// ``attachment_point`` and ``detachment_point`` are tracked separately from
/// the wrapped Rust builder (which only exposes a combined
/// `attachment_detachment(a, d)` setter) and applied together on
/// :meth:`build`, so either call order works.
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "TrancheBuilder",
    skip_from_py_object
)]
pub struct PyTrancheBuilder {
    inner: Option<TrancheBuilderInner>,
    attachment_point: Option<f64>,
    detachment_point: Option<f64>,
}

/// Take the wrapped Rust builder or fail if `build()` already consumed it.
fn take_tranche(b: &mut PyTrancheBuilder) -> PyResult<TrancheBuilderInner> {
    b.inner
        .take()
        .ok_or_else(|| value_error("builder already consumed by build()"))
}

#[pymethods]
impl PyTrancheBuilder {
    /// Set the tranche identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Unique identifier for the tranche.
    ///
    /// Returns
    /// -------
    /// TrancheBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If this builder was already consumed by a prior call to
    ///     :meth:`TrancheBuilder.build`.
    #[pyo3(text_signature = "($self, value)")]
    fn id<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_tranche(&mut slf)?;
        slf.inner = Some(b.id(value));
        Ok(slf)
    }

    /// Set the attachment point.
    ///
    /// Parameters
    /// ----------
    /// value : float
    ///     Attachment point quoted in percent on a 0-100 scale (e.g. ``0.0``
    ///     for equity, ``10.0`` for a tranche attaching at 10%).
    ///
    /// Returns
    /// -------
    /// TrancheBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If this builder was already consumed by a prior call to
    ///     :meth:`TrancheBuilder.build`.
    #[pyo3(text_signature = "($self, value)")]
    fn attachment_point<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: f64,
    ) -> PyResult<PyRefMut<'py, Self>> {
        if slf.inner.is_none() {
            return Err(value_error("builder already consumed by build()"));
        }
        slf.attachment_point = Some(value);
        Ok(slf)
    }

    /// Set the detachment point.
    ///
    /// Parameters
    /// ----------
    /// value : float
    ///     Detachment point quoted in percent on a 0-100 scale (e.g.
    ///     ``100.0`` for the most senior tranche).
    ///
    /// Returns
    /// -------
    /// TrancheBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If this builder was already consumed by a prior call to
    ///     :meth:`TrancheBuilder.build`.
    #[pyo3(text_signature = "($self, value)")]
    fn detachment_point<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: f64,
    ) -> PyResult<PyRefMut<'py, Self>> {
        if slf.inner.is_none() {
            return Err(value_error("builder already consumed by build()"));
        }
        slf.detachment_point = Some(value);
        Ok(slf)
    }

    /// Set the tranche seniority.
    ///
    /// Parameters
    /// ----------
    /// value : {"senior", "mezzanine", "subordinated", "equity"}
    ///     Structural seniority of the tranche.
    ///
    /// Returns
    /// -------
    /// TrancheBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not a recognized seniority.
    #[pyo3(text_signature = "($self, value)")]
    fn seniority<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let seniority: TrancheSeniority = enum_from_str(value, "seniority")?;
        let b = take_tranche(&mut slf)?;
        slf.inner = Some(b.seniority(seniority));
        Ok(slf)
    }

    /// Set the original tranche balance.
    ///
    /// Maps to the Rust ``TrancheBuilder::balance`` setter; named
    /// ``original_balance`` here to match the ``Tranche::original_balance``
    /// field it populates.
    ///
    /// Parameters
    /// ----------
    /// value : Money
    ///     Original tranche balance. Must be positive.
    ///
    /// Returns
    /// -------
    /// TrancheBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If this builder was already consumed by a prior call to
    ///     :meth:`TrancheBuilder.build`.
    #[pyo3(text_signature = "($self, value)")]
    fn original_balance<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyMoney>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_tranche(&mut slf)?;
        slf.inner = Some(b.balance(value.inner));
        Ok(slf)
    }

    /// Set a fixed-rate coupon.
    ///
    /// Parameters
    /// ----------
    /// rate : float
    ///     Fixed interest rate as an annual decimal (e.g. ``0.05`` = 5%).
    ///
    /// Returns
    /// -------
    /// TrancheBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If this builder was already consumed by a prior call to
    ///     :meth:`TrancheBuilder.build`.
    #[pyo3(text_signature = "($self, rate)")]
    fn coupon_fixed<'py>(mut slf: PyRefMut<'py, Self>, rate: f64) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_tranche(&mut slf)?;
        slf.inner = Some(b.coupon(TrancheCoupon::Fixed { rate }));
        Ok(slf)
    }

    /// Set a floating-rate coupon from a JSON ``TrancheCoupon::Floating`` payload.
    ///
    /// The floating-rate spec (``FloatingRateSpec``: index, spread, gearing,
    /// floors/caps, reset conventions) stays JSON per the nested-spec rule —
    /// the typed cashflows plan owns that shape.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     JSON-encoded, externally-tagged ``TrancheCoupon`` value, e.g.
    ///     ``{"floating": {...FloatingRateSpec fields...}}``.
    ///
    /// Returns
    /// -------
    /// TrancheBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not valid JSON for the ``TrancheCoupon`` shape.
    #[pyo3(text_signature = "($self, value)")]
    fn coupon_floating_json<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let coupon: TrancheCoupon = json_field(value, "coupon")?;
        let b = take_tranche(&mut slf)?;
        slf.inner = Some(b.coupon(coupon));
        Ok(slf)
    }

    /// Set the legal final maturity date.
    ///
    /// Parameters
    /// ----------
    /// value : datetime.date
    ///     Legal final maturity date.
    ///
    /// Returns
    /// -------
    /// TrancheBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If this builder was already consumed by a prior call to
    ///     :meth:`TrancheBuilder.build`.
    #[pyo3(text_signature = "($self, value)")]
    fn maturity<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let maturity = py_to_date(value)?;
        let b = take_tranche(&mut slf)?;
        slf.inner = Some(b.maturity(maturity));
        Ok(slf)
    }

    /// Set the payment frequency.
    ///
    /// Parameters
    /// ----------
    /// value : Tenor
    ///     Payment frequency. Defaults to quarterly when never set.
    ///
    /// Returns
    /// -------
    /// TrancheBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If this builder was already consumed by a prior call to
    ///     :meth:`TrancheBuilder.build`.
    #[pyo3(text_signature = "($self, value)")]
    fn frequency<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyTenor>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_tranche(&mut slf)?;
        slf.inner = Some(b.frequency(value.inner));
        Ok(slf)
    }

    /// Set the day count convention for interest accrual.
    ///
    /// Parameters
    /// ----------
    /// value : DayCount
    ///     Day count convention. Defaults to Act/360 when never set.
    ///
    /// Returns
    /// -------
    /// TrancheBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If this builder was already consumed by a prior call to
    ///     :meth:`TrancheBuilder.build`.
    #[pyo3(text_signature = "($self, value)")]
    fn day_count<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyDayCount>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_tranche(&mut slf)?;
        slf.inner = Some(b.day_count(value.inner));
        Ok(slf)
    }

    /// Build the validated tranche.
    ///
    /// Returns
    /// -------
    /// Tranche
    ///     The validated tranche.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If a required field is missing, or attachment/detachment points
    ///     are invalid (negative, out of the ``[0, 100]`` range, or
    ///     detachment not strictly above attachment).
    #[pyo3(text_signature = "($self)")]
    fn build(mut slf: PyRefMut<'_, Self>) -> PyResult<PyTranche> {
        let mut b = take_tranche(&mut slf)?;
        if let (Some(attachment), Some(detachment)) = (slf.attachment_point, slf.detachment_point) {
            b = b.attachment_detachment(attachment, detachment);
        }
        let inner = b.build().map_err(core_to_py)?;
        Ok(PyTranche { inner })
    }
}

// ---------------------------------------------------------------------------
// TrancheStructure
// ---------------------------------------------------------------------------

/// Typed wrapper for the Rust `TrancheStructure` (capital structure).
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "TrancheStructure",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyTrancheStructure {
    /// Inner canonical Rust tranche structure.
    pub(crate) inner: TrancheStructure,
}

#[pymethods]
impl PyTrancheStructure {
    /// Capital structure formed from a list of tranches.
    ///
    /// Validates that attachment/detachment points tile ``[0, 100]`` without
    /// gaps or overlaps, that every tranche shares one currency, and assigns
    /// each tranche a distinct, strictly-increasing ``payment_priority``
    /// ranked by seniority (see Rust ``TrancheStructure::new``).
    ///
    /// Parameters
    /// ----------
    /// tranches : list[Tranche]
    ///     Tranches forming the capital structure.
    ///
    /// Returns
    /// -------
    /// TrancheStructure
    ///     The validated tranche structure.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``tranches`` is empty, has non-finite attachment/detachment
    ///     points, leaves a gap/overlap, doesn't tile to 100%, or mixes
    ///     currencies.
    ///
    /// Examples
    /// --------
    /// >>> import datetime
    /// >>> from finstack_quant.core.currency import Currency
    /// >>> from finstack_quant.core.money import Money
    /// >>> from finstack_quant.valuations.instruments import Tranche, TrancheStructure
    /// >>> senior = (
    /// ...     Tranche.builder()
    /// ...     .id("A")
    /// ...     .attachment_point(10.0)
    /// ...     .detachment_point(100.0)
    /// ...     .seniority("senior")
    /// ...     .original_balance(Money(72_000_000.0, Currency("USD")))
    /// ...     .coupon_fixed(0.05)
    /// ...     .maturity(datetime.date(2031, 1, 15))
    /// ...     .build()
    /// ... )
    /// >>> equity = (
    /// ...     Tranche.builder()
    /// ...     .id("E")
    /// ...     .attachment_point(0.0)
    /// ...     .detachment_point(10.0)
    /// ...     .seniority("equity")
    /// ...     .original_balance(Money(8_000_000.0, Currency("USD")))
    /// ...     .coupon_fixed(0.0)
    /// ...     .maturity(datetime.date(2031, 1, 15))
    /// ...     .build()
    /// ... )
    /// >>> structure = TrancheStructure([senior, equity])
    /// >>> "tranches=2" in repr(structure)
    /// True
    #[new]
    #[pyo3(text_signature = "(tranches)")]
    fn new(tranches: Vec<PyRef<'_, PyTranche>>) -> PyResult<Self> {
        let tranches: Vec<Tranche> = tranches.iter().map(|t| t.inner.clone()).collect();
        let inner = TrancheStructure::new(tranches).map_err(core_to_py)?;
        Ok(Self { inner })
    }

    /// Return ``repr(self)``.
    fn __repr__(&self) -> String {
        format!(
            "TrancheStructure(tranches={}, total_size={})",
            self.inner.tranches.len(),
            self.inner.total_size
        )
    }
}

// ---------------------------------------------------------------------------
// StructuredCredit
// ---------------------------------------------------------------------------

/// Typed wrapper for the Rust `StructuredCredit` instrument (ABS/CLO/CMBS/RMBS).
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "StructuredCredit",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyStructuredCredit {
    /// Inner canonical Rust structured-credit deal.
    pub(crate) inner: StructuredCredit,
}

impl PyStructuredCredit {
    /// Serialize as the canonical instrument envelope accepted by the JSON loader.
    pub(crate) fn envelope_json(&self) -> PyResult<String> {
        serialize_typed_instrument_json(
            InstrumentJson::StructuredCredit(Box::new(self.inner.clone())),
            "StructuredCredit",
        )
    }
}

#[pymethods]
impl PyStructuredCredit {
    /// Create a fluent builder (mirrors Rust ``StructuredCredit::builder()``).
    ///
    /// The builder pre-seeds ``market_conditions``, ``credit_factors``,
    /// ``deal_metadata``, ``behavior_overrides``, ``default_assumptions``,
    /// and ``hedge_swaps`` with their Rust ``Default`` values (the Rust
    /// builder fields have no default), which the corresponding ``*_json``
    /// setters can override. Prefer :meth:`new_abs` / :meth:`new_clo` /
    /// :meth:`new_cmbs` / :meth:`new_rmbs` for registry-calibrated deal-type
    /// defaults; use this builder for full manual control.
    ///
    /// Returns
    /// -------
    /// StructuredCreditBuilder
    ///     A builder with fluent, consuming setter methods.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import StructuredCredit
    /// >>> callable(StructuredCredit.builder)
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn builder() -> PyStructuredCreditBuilder {
        PyStructuredCreditBuilder {
            inner: Some(
                StructuredCredit::builder()
                    .market_conditions(MarketConditions::default())
                    .credit_factors(CreditFactors::default())
                    .deal_metadata(Metadata::default())
                    .behavior_overrides(Overrides::default())
                    .default_assumptions(DefaultAssumptions::default())
                    .hedge_swaps(Vec::new()),
            ),
        }
    }

    /// Create a new ABS deal with registry-calibrated defaults.
    ///
    /// Parameters
    /// ----------
    /// id : str
    ///     Unique instrument identifier.
    /// pool : AssetPool
    ///     Asset pool definition.
    /// tranches : TrancheStructure
    ///     Tranche capital structure.
    /// closing_date : datetime.date
    ///     Deal closing date (issuance).
    /// maturity : datetime.date
    ///     Legal final maturity date.
    /// discount_curve_id : str
    ///     Discount curve identifier for valuation.
    ///
    /// Returns
    /// -------
    /// StructuredCredit
    ///     The validated ABS deal.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the deal fails pricing validation.
    ///
    /// Examples
    /// --------
    /// >>> import datetime
    /// >>> from finstack_quant.core.currency import Currency
    /// >>> from finstack_quant.core.dates import DayCount
    /// >>> from finstack_quant.core.money import Money
    /// >>> from finstack_quant.valuations.instruments import (
    /// ...     AssetPool, RepLine, StructuredCredit, Tranche, TrancheStructure,
    /// ... )
    /// >>> pool = AssetPool("POOL-1", "abs", Currency("USD")).with_rep_lines([
    /// ...     RepLine(
    /// ...         "LINE-1", Money(80_000_000.0, Currency("USD")), 0.07,
    /// ...         datetime.date(2031, 1, 15), 12, DayCount.ACT_360,
    /// ...     )
    /// ... ])
    /// >>> senior = (
    /// ...     Tranche.builder().id("A").attachment_point(10.0).detachment_point(100.0)
    /// ...     .seniority("senior").original_balance(Money(72_000_000.0, Currency("USD")))
    /// ...     .coupon_fixed(0.05).maturity(datetime.date(2031, 1, 15)).build()
    /// ... )
    /// >>> equity = (
    /// ...     Tranche.builder().id("E").attachment_point(0.0).detachment_point(10.0)
    /// ...     .seniority("equity").original_balance(Money(8_000_000.0, Currency("USD")))
    /// ...     .coupon_fixed(0.0).maturity(datetime.date(2031, 1, 15)).build()
    /// ... )
    /// >>> deal = StructuredCredit.new_abs(
    /// ...     "ABS-1", pool, TrancheStructure([senior, equity]),
    /// ...     datetime.date(2024, 1, 15), datetime.date(2031, 1, 15), "USD-SOFR-DISC",
    /// ... )
    /// >>> "ABS-1" in repr(deal)
    /// True
    #[staticmethod]
    #[pyo3(text_signature = "(id, pool, tranches, closing_date, maturity, discount_curve_id)")]
    fn new_abs(
        id: &str,
        pool: PyRef<'_, PyAssetPool>,
        tranches: PyRef<'_, PyTrancheStructure>,
        closing_date: &Bound<'_, PyAny>,
        maturity: &Bound<'_, PyAny>,
        discount_curve_id: &str,
    ) -> PyResult<Self> {
        let inner = StructuredCredit::new_abs(
            id,
            pool.inner.clone(),
            tranches.inner.clone(),
            py_to_date(closing_date)?,
            py_to_date(maturity)?,
            discount_curve_id,
        );
        inner.validate_for_pricing().map_err(core_to_py)?;
        Ok(Self { inner })
    }

    /// Create a new CLO deal with registry-calibrated defaults.
    ///
    /// See :meth:`new_abs` for parameter and return documentation; the
    /// signature is identical, only the deal-type calibration differs.
    #[staticmethod]
    #[pyo3(text_signature = "(id, pool, tranches, closing_date, maturity, discount_curve_id)")]
    fn new_clo(
        id: &str,
        pool: PyRef<'_, PyAssetPool>,
        tranches: PyRef<'_, PyTrancheStructure>,
        closing_date: &Bound<'_, PyAny>,
        maturity: &Bound<'_, PyAny>,
        discount_curve_id: &str,
    ) -> PyResult<Self> {
        let inner = StructuredCredit::new_clo(
            id,
            pool.inner.clone(),
            tranches.inner.clone(),
            py_to_date(closing_date)?,
            py_to_date(maturity)?,
            discount_curve_id,
        );
        inner.validate_for_pricing().map_err(core_to_py)?;
        Ok(Self { inner })
    }

    /// Create a new CMBS deal with registry-calibrated defaults.
    ///
    /// See :meth:`new_abs` for parameter and return documentation; the
    /// signature is identical, only the deal-type calibration differs.
    #[staticmethod]
    #[pyo3(text_signature = "(id, pool, tranches, closing_date, maturity, discount_curve_id)")]
    fn new_cmbs(
        id: &str,
        pool: PyRef<'_, PyAssetPool>,
        tranches: PyRef<'_, PyTrancheStructure>,
        closing_date: &Bound<'_, PyAny>,
        maturity: &Bound<'_, PyAny>,
        discount_curve_id: &str,
    ) -> PyResult<Self> {
        let inner = StructuredCredit::new_cmbs(
            id,
            pool.inner.clone(),
            tranches.inner.clone(),
            py_to_date(closing_date)?,
            py_to_date(maturity)?,
            discount_curve_id,
        );
        inner.validate_for_pricing().map_err(core_to_py)?;
        Ok(Self { inner })
    }

    /// Create a new RMBS deal with registry-calibrated defaults.
    ///
    /// See :meth:`new_abs` for parameter and return documentation; the
    /// signature is identical, only the deal-type calibration differs.
    #[staticmethod]
    #[pyo3(text_signature = "(id, pool, tranches, closing_date, maturity, discount_curve_id)")]
    fn new_rmbs(
        id: &str,
        pool: PyRef<'_, PyAssetPool>,
        tranches: PyRef<'_, PyTrancheStructure>,
        closing_date: &Bound<'_, PyAny>,
        maturity: &Bound<'_, PyAny>,
        discount_curve_id: &str,
    ) -> PyResult<Self> {
        let inner = StructuredCredit::new_rmbs(
            id,
            pool.inner.clone(),
            tranches.inner.clone(),
            py_to_date(closing_date)?,
            py_to_date(maturity)?,
            discount_curve_id,
        );
        inner.validate_for_pricing().map_err(core_to_py)?;
        Ok(Self { inner })
    }

    /// Deserialize a validated deal from its canonical v1 envelope.
    ///
    /// Parameters
    /// ----------
    /// json : str
    ///     A ``finstack_quant.instrument/1`` envelope containing an exact
    ///     ``"structured_credit"`` payload. The UTF-8 input must not exceed
    ///     16 MiB. Bare payloads and cross-type coercion are rejected.
    ///
    /// Returns
    /// -------
    /// StructuredCredit
    ///     The validated deal represented by the exact ``"structured_credit"`` payload.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the input exceeds 16 MiB, is malformed, has an unsupported
    ///     envelope schema, carries another type, or fails structured-credit
    ///     validation.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import StructuredCredit
    /// >>> callable(StructuredCredit.from_json)
    /// True
    #[classmethod]
    #[pyo3(text_signature = "(cls, json)")]
    fn from_json(_cls: &Bound<'_, PyType>, json: &str) -> PyResult<Self> {
        match parse_typed_instrument_json(json)? {
            InstrumentJson::StructuredCredit(inner) => {
                let inner = *inner;
                inner.validate_for_pricing().map_err(core_to_py)?;
                Ok(Self { inner })
            }
            _ => Err(value_error(
                "expected instrument type \"structured_credit\", got a different instrument type",
            )),
        }
    }

    /// Serialize to a canonical ``finstack_quant.instrument/1`` envelope.
    ///
    /// Returns
    /// -------
    /// str
    ///     Canonical instrument envelope accepted by ``price_instrument`` and
    ///     ``StructuredCredit.from_json``.
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
            "StructuredCredit(id={:?}, deal_type={:?})",
            self.inner.id.as_str(),
            self.inner.deal_type
        )
    }
}

/// Fluent builder for [`PyStructuredCredit`]; wraps the Rust
/// `FinancialBuilder`-generated builder (consuming setters).
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "StructuredCreditBuilder",
    skip_from_py_object
)]
pub struct PyStructuredCreditBuilder {
    inner: Option<StructuredCreditBuilderInner>,
}

/// Take the wrapped Rust builder or fail if `build()` already consumed it.
fn take_sc(b: &mut PyStructuredCreditBuilder) -> PyResult<StructuredCreditBuilderInner> {
    b.inner
        .take()
        .ok_or_else(|| value_error("builder already consumed by build()"))
}

#[pymethods]
impl PyStructuredCreditBuilder {
    /// Set the instrument identifier.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Unique identifier for the deal.
    ///
    /// Returns
    /// -------
    /// StructuredCreditBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If this builder was already consumed by a prior call to
    ///     :meth:`StructuredCreditBuilder.build`.
    #[pyo3(text_signature = "($self, value)")]
    fn id<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_sc(&mut slf)?;
        slf.inner = Some(b.id(InstrumentId::new(value.to_string())));
        Ok(slf)
    }

    /// Set the deal-type classification.
    ///
    /// Parameters
    /// ----------
    /// value : {"clo", "cbo", "abs", "rmbs", "cmbs", "auto", "card"}
    ///     Deal classification.
    ///
    /// Returns
    /// -------
    /// StructuredCreditBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not a recognized deal type.
    #[pyo3(text_signature = "($self, value)")]
    fn deal_type<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let deal_type: DealType = enum_from_str(value, "deal_type")?;
        let b = take_sc(&mut slf)?;
        slf.inner = Some(b.deal_type(deal_type));
        Ok(slf)
    }

    /// Set the asset pool.
    ///
    /// Parameters
    /// ----------
    /// value : AssetPool
    ///     Asset pool definition.
    ///
    /// Returns
    /// -------
    /// StructuredCreditBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If this builder was already consumed by a prior call to
    ///     :meth:`StructuredCreditBuilder.build`.
    #[pyo3(text_signature = "($self, value)")]
    fn pool<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyAssetPool>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_sc(&mut slf)?;
        slf.inner = Some(b.pool(value.inner.clone()));
        Ok(slf)
    }

    /// Set the tranche capital structure.
    ///
    /// Parameters
    /// ----------
    /// value : TrancheStructure
    ///     Tranche capital structure.
    ///
    /// Returns
    /// -------
    /// StructuredCreditBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If this builder was already consumed by a prior call to
    ///     :meth:`StructuredCreditBuilder.build`.
    #[pyo3(text_signature = "($self, value)")]
    fn tranches<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyTrancheStructure>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_sc(&mut slf)?;
        slf.inner = Some(b.tranches(value.inner.clone()));
        Ok(slf)
    }

    /// Set the deal closing (issuance) date.
    ///
    /// Parameters
    /// ----------
    /// value : datetime.date
    ///     Deal closing date.
    ///
    /// Returns
    /// -------
    /// StructuredCreditBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If this builder was already consumed by a prior call to
    ///     :meth:`StructuredCreditBuilder.build`.
    #[pyo3(text_signature = "($self, value)")]
    fn closing_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let date = py_to_date(value)?;
        let b = take_sc(&mut slf)?;
        slf.inner = Some(b.closing_date(date));
        Ok(slf)
    }

    /// Set the first payment date to tranches.
    ///
    /// Parameters
    /// ----------
    /// value : datetime.date
    ///     First payment date.
    ///
    /// Returns
    /// -------
    /// StructuredCreditBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If this builder was already consumed by a prior call to
    ///     :meth:`StructuredCreditBuilder.build`.
    #[pyo3(text_signature = "($self, value)")]
    fn first_payment_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let date = py_to_date(value)?;
        let b = take_sc(&mut slf)?;
        slf.inner = Some(b.first_payment_date(date));
        Ok(slf)
    }

    /// Set the end of the reinvestment period.
    ///
    /// Parameters
    /// ----------
    /// value : datetime.date
    ///     End date of the reinvestment period. Optional; when never set,
    ///     the deal has no reinvestment period.
    ///
    /// Returns
    /// -------
    /// StructuredCreditBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If this builder was already consumed by a prior call to
    ///     :meth:`StructuredCreditBuilder.build`.
    #[pyo3(text_signature = "($self, value)")]
    fn reinvestment_end_date<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let date = py_to_date(value)?;
        let b = take_sc(&mut slf)?;
        slf.inner = Some(b.reinvestment_end_date(date));
        Ok(slf)
    }

    /// Set the legal final maturity date.
    ///
    /// Parameters
    /// ----------
    /// value : datetime.date
    ///     Legal final maturity date.
    ///
    /// Returns
    /// -------
    /// StructuredCreditBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If this builder was already consumed by a prior call to
    ///     :meth:`StructuredCreditBuilder.build`.
    #[pyo3(text_signature = "($self, value)")]
    fn maturity<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let date = py_to_date(value)?;
        let b = take_sc(&mut slf)?;
        slf.inner = Some(b.maturity(date));
        Ok(slf)
    }

    /// Set the payment frequency for the structure.
    ///
    /// Parameters
    /// ----------
    /// value : Tenor
    ///     Payment frequency.
    ///
    /// Returns
    /// -------
    /// StructuredCreditBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If this builder was already consumed by a prior call to
    ///     :meth:`StructuredCreditBuilder.build`.
    #[pyo3(text_signature = "($self, value)")]
    fn frequency<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: PyRef<'_, PyTenor>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_sc(&mut slf)?;
        slf.inner = Some(b.frequency(value.inner));
        Ok(slf)
    }

    /// Set the payment calendar identifier for schedule adjustments.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Holiday calendar identifier (e.g. ``"nyse"``). Required for
    ///     accurate schedule generation.
    ///
    /// Returns
    /// -------
    /// StructuredCreditBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If this builder was already consumed by a prior call to
    ///     :meth:`StructuredCreditBuilder.build`.
    #[pyo3(text_signature = "($self, value)")]
    fn payment_calendar_id<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_sc(&mut slf)?;
        slf.inner = Some(b.payment_calendar_id(value.to_string()));
        Ok(slf)
    }

    /// Set the business day convention for tranche payments.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Business day convention (e.g. ``"following"``,
    ///     ``"modified_following"``). Defaults to ``"following"`` when
    ///     never set.
    ///
    /// Returns
    /// -------
    /// StructuredCreditBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not a recognized business day convention.
    #[pyo3(text_signature = "($self, value)")]
    fn payment_business_day_convention<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let business_day_convention: BusinessDayConvention =
            enum_from_str(value, "payment_business_day_convention")?;
        let b = take_sc(&mut slf)?;
        slf.inner = Some(b.payment_business_day_convention(business_day_convention));
        Ok(slf)
    }

    /// Set the discount curve identifier for valuation.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     Discount curve identifier.
    ///
    /// Returns
    /// -------
    /// StructuredCreditBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If this builder was already consumed by a prior call to
    ///     :meth:`StructuredCreditBuilder.build`.
    #[pyo3(text_signature = "($self, value)")]
    fn discount_curve_id<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let b = take_sc(&mut slf)?;
        slf.inner = Some(b.discount_curve_id(CurveId::new(value.to_string())));
        Ok(slf)
    }

    /// Set market conditions from a JSON object.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     JSON-encoded ``MarketConditions`` object (refinancing rate, home
    ///     price appreciation, unemployment, seasonal factor, custom
    ///     factors). :meth:`StructuredCredit.builder` pre-seeds the registry
    ///     default, which this overrides.
    ///
    /// Returns
    /// -------
    /// StructuredCreditBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not valid JSON for the ``MarketConditions`` shape.
    #[pyo3(text_signature = "($self, value)")]
    fn market_conditions_json<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let market_conditions: MarketConditions = json_field(value, "market_conditions")?;
        let b = take_sc(&mut slf)?;
        slf.inner = Some(b.market_conditions(market_conditions));
        Ok(slf)
    }

    /// Set credit factors from a JSON object.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     JSON-encoded ``CreditFactors`` object (credit score, DTI, LTV,
    ///     delinquency, unemployment, CMBS NOI/debt-service, custom
    ///     factors). :meth:`StructuredCredit.builder` pre-seeds
    ///     ``CreditFactors::default()``, which this overrides.
    ///
    /// Returns
    /// -------
    /// StructuredCreditBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not valid JSON for the ``CreditFactors`` shape.
    #[pyo3(text_signature = "($self, value)")]
    fn credit_factors_json<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let credit_factors: CreditFactors = json_field(value, "credit_factors")?;
        let b = take_sc(&mut slf)?;
        slf.inner = Some(b.credit_factors(credit_factors));
        Ok(slf)
    }

    /// Set declarative waterfall rules from a JSON object.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     JSON-encoded ``WaterfallRules`` object (available-funds caps,
    ///     step-down, shifting interest, controlled accumulation), layered
    ///     onto the base waterfall.
    ///
    /// Returns
    /// -------
    /// StructuredCreditBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not valid JSON for the ``WaterfallRules`` shape.
    #[pyo3(text_signature = "($self, value)")]
    fn waterfall_rules_json<'py>(
        mut slf: PyRefMut<'py, Self>,
        value: &str,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let waterfall_rules: WaterfallRules = json_field(value, "waterfall_rules")?;
        let b = take_sc(&mut slf)?;
        slf.inner = Some(b.waterfall_rules(waterfall_rules));
        Ok(slf)
    }

    /// Set senior transaction fees from a JSON object.
    ///
    /// Parameters
    /// ----------
    /// value : str
    ///     JSON-encoded ``DealFees`` object (trustee, senior management,
    ///     servicing, and optional master/special servicer fees), paid
    ///     ahead of every note. Skipped (``None``) by default.
    ///
    /// Returns
    /// -------
    /// StructuredCreditBuilder
    ///     ``self``, for chaining.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``value`` is not valid JSON for the ``DealFees`` shape.
    #[pyo3(text_signature = "($self, value)")]
    fn fees_json<'py>(mut slf: PyRefMut<'py, Self>, value: &str) -> PyResult<PyRefMut<'py, Self>> {
        let fees: DealFees = json_field(value, "fees")?;
        let b = take_sc(&mut slf)?;
        slf.inner = Some(b.fees(fees));
        Ok(slf)
    }

    /// Build the validated structured-credit deal.
    ///
    /// Returns
    /// -------
    /// StructuredCredit
    ///     The validated deal.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If a required field is missing or Rust validation fails.
    #[pyo3(text_signature = "($self)")]
    fn build(mut slf: PyRefMut<'_, Self>) -> PyResult<PyStructuredCredit> {
        let b = take_sc(&mut slf)?;
        let inner = b.build().map_err(core_to_py)?;
        inner.validate_for_pricing().map_err(core_to_py)?;
        Ok(PyStructuredCredit { inner })
    }
}

/// Register the typed structured-credit deal-modeling classes on the
/// instruments submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyRepLine>()?;
    m.add_class::<PyAssetPool>()?;
    m.add_class::<PyTranche>()?;
    m.add_class::<PyTrancheBuilder>()?;
    m.add_class::<PyTrancheStructure>()?;
    m.add_class::<PyStructuredCredit>()?;
    m.add_class::<PyStructuredCreditBuilder>()?;
    Ok(())
}
