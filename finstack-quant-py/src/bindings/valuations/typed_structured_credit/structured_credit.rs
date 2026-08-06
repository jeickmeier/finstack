use pyo3::prelude::*;
use pyo3::types::PyType;

use crate::bindings::core::dates::tenor::PyTenor;
use crate::bindings::core::dates::utils::py_to_date;
use crate::errors::{core_to_py, value_error};
use finstack_quant_core::dates::BusinessDayConvention;
use finstack_quant_core::types::{CurveId, InstrumentId};
use finstack_quant_valuations::instruments::fixed_income::structured_credit::{
    CreditFactors, DealFees, DealType, DefaultAssumptions, MarketConditions, Metadata, Overrides,
    StructuredCredit, WaterfallRules,
};
use finstack_quant_valuations::instruments::{Instrument, InstrumentJson};

use super::super::instruments::{
    enum_from_str, json_field, parse_typed_instrument_json, serialize_typed_instrument_json,
};
use super::{PyAssetPool, PyTrancheStructure};

type StructuredCreditBuilderInner =
    finstack_quant_valuations::instruments::fixed_income::structured_credit::StructuredCreditBuilder;

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
    /// >>> builder = StructuredCredit.builder()
    /// >>> builder.id("EXAMPLE") is builder
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

    /// Support `pickle` (and therefore `multiprocessing`, `joblib`, `dask`).
    ///
    /// Reconstruction goes through the same strict serde round-trip as
    /// `to_json` / `from_json`, so an unpickled value is exactly what the wire
    /// format defines — there is no second state format that can drift.
    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        Ok((from_json, (self.to_json()?,)))
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
    /// >>> try:
    /// ...     StructuredCredit.from_json("{}")
    /// ... except ValueError as exc:
    /// ...     print("schema" in str(exc))
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
    ///     If the builder was already consumed, a required field is missing,
    ///     or the completed deal fails pricing validation.
    #[pyo3(text_signature = "($self)")]
    fn build(mut slf: PyRefMut<'_, Self>) -> PyResult<PyStructuredCredit> {
        let b = take_sc(&mut slf)?;
        let inner = b.build().map_err(core_to_py)?;
        inner.validate_for_pricing().map_err(core_to_py)?;
        Ok(PyStructuredCredit { inner })
    }
}
