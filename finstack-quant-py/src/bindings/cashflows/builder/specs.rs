//! Spec-type bindings for `finstack_quant_cashflows::builder::specs`.

use finstack_quant_cashflows::builder::{
    AmortizationSpec, CouponType, DefaultModelSpec, FeeAccrualBasis, FeeBase, FeeSpec,
    FixedCouponSpec, FixedWindow, FloatingCouponSpec, FloatingRateFallback, FloatingRateSpec,
    Notional, OvernightCompoundingMethod, OvernightIndexConstraintApplication, PrepaymentModelSpec,
    PrincipalExchange, RecoveryModelSpec, RollRule, ScheduleParams, StepUpCouponSpec,
};
use finstack_quant_core::dates::{BusinessDayConvention, Date, StubKind};
use finstack_quant_core::money::Money;
use pyo3::prelude::*;
use pyo3::types::{PyModule, PyType};
use rust_decimal::Decimal;

use crate::bindings::core::currency::extract_currency;
use crate::bindings::core::currency::PyCurrency;
use crate::bindings::core::dates::calendar::extract_business_day_convention;
use crate::bindings::core::dates::daycount::PyDayCount;
use crate::bindings::core::dates::schedule::PyStubKind;
use crate::bindings::core::dates::tenor::{extract_tenor, PyTenor};
use crate::bindings::core::money::{decimal_from_py, decimal_to_py, is_python_decimal, PyMoney};
use crate::bindings::date_utils::py_to_date;
use crate::errors::core_to_py;

/// Extract a `rust_decimal::Decimal` from `decimal.Decimal`, `float`, or `int`.
pub(crate) fn decimal_from_any(obj: &Bound<'_, PyAny>) -> PyResult<Decimal> {
    if is_python_decimal(obj)? {
        return decimal_from_py(obj);
    }
    let value: f64 = obj.extract().map_err(|_| {
        pyo3::exceptions::PyTypeError::new_err("expected decimal.Decimal, float, or int")
    })?;
    Decimal::try_from(value).map_err(|e| {
        crate::errors::value_error(format!(
            "value {value} is not representable as Decimal: {e}"
        ))
    })
}

/// Extract a list of `(date, Decimal)` pairs.
pub(crate) fn date_decimal_pairs(
    items: Vec<(Bound<'_, PyAny>, Bound<'_, PyAny>)>,
) -> PyResult<Vec<(Date, Decimal)>> {
    items
        .iter()
        .map(|(d, v)| Ok((py_to_date(d)?, decimal_from_any(v)?)))
        .collect()
}

/// Extract a list of `(date, Money)` pairs.
fn date_money_pairs(items: Vec<(Bound<'_, PyAny>, PyMoney)>) -> PyResult<Vec<(Date, Money)>> {
    items
        .iter()
        .map(|(d, m)| Ok((py_to_date(d)?, m.inner)))
        .collect()
}

/// Wrapper for [`RollRule`] (`finstack_quant.cashflows.builder.RollRule`).
#[pyclass(
    name = "RollRule",
    module = "finstack_quant.cashflows.builder",
    frozen,
    eq,
    hash,
    skip_from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyRollRule {
    /// Inner roll-date rule.
    pub(crate) inner: RollRule,
}

#[pymethods]
impl PyRollRule {
    /// Plain tenor stepping (default).
    #[classattr]
    const NONE: PyRollRule = PyRollRule {
        inner: RollRule::None,
    };
    /// Standard IMM dates: third Wednesday of Mar/Jun/Sep/Dec.
    #[classattr]
    const IMM: PyRollRule = PyRollRule {
        inner: RollRule::Imm,
    };
    /// CDS IMM dates: 20th of Mar/Jun/Sep/Dec with Big-Bang front accrual.
    #[classattr]
    const CDS_IMM: PyRollRule = PyRollRule {
        inner: RollRule::CdsImm,
    };

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!("RollRule({:?})", self.inner)
    }
}

/// Wrapper for [`PrincipalExchange`]
/// (`finstack_quant.cashflows.builder.PrincipalExchange`).
#[pyclass(
    name = "PrincipalExchange",
    module = "finstack_quant.cashflows.builder",
    frozen,
    eq,
    hash,
    skip_from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyPrincipalExchange {
    /// Inner principal-exchange policy.
    pub(crate) inner: PrincipalExchange,
}

#[pymethods]
impl PyPrincipalExchange {
    /// Do not emit issue or redemption notionals.
    #[classattr]
    const NONE: PyPrincipalExchange = PyPrincipalExchange {
        inner: PrincipalExchange::None,
    };
    /// Emit issue funding and maturity redemption (default).
    #[classattr]
    const INITIAL_AND_FINAL: PyPrincipalExchange = PyPrincipalExchange {
        inner: PrincipalExchange::InitialAndFinal,
    };

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!("PrincipalExchange({:?})", self.inner)
    }
}

/// Wrapper for [`CouponType`] (`finstack_quant.cashflows.builder.CouponType`).
#[pyclass(
    name = "CouponType",
    module = "finstack_quant.cashflows.builder",
    frozen,
    eq,
    hash,
    skip_from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyCouponType {
    /// Inner coupon settlement type.
    pub(crate) inner: CouponType,
}

#[pymethods]
impl PyCouponType {
    /// 100% paid in cash.
    #[classattr]
    const CASH: PyCouponType = PyCouponType {
        inner: CouponType::Cash,
    };
    /// 100% capitalized into principal.
    #[classattr]
    const PIK: PyCouponType = PyCouponType {
        inner: CouponType::Pik,
    };

    /// Split settlement: explicit cash and PIK fractions in ``[0, 1]``.
    #[staticmethod]
    #[pyo3(text_signature = "(cash_pct, pik_pct)")]
    fn split(cash_pct: &Bound<'_, PyAny>, pik_pct: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: CouponType::Split {
                cash_pct: decimal_from_any(cash_pct)?,
                pik_pct: decimal_from_any(pik_pct)?,
            },
        })
    }

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!("CouponType({:?})", self.inner)
    }
}

/// Wrapper for [`OvernightCompoundingMethod`].
#[pyclass(
    name = "OvernightCompoundingMethod",
    module = "finstack_quant.cashflows.builder",
    frozen,
    eq,
    hash,
    skip_from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyOvernightCompoundingMethod {
    /// Inner compounding method.
    pub(crate) inner: OvernightCompoundingMethod,
}

#[pymethods]
impl PyOvernightCompoundingMethod {
    /// Arithmetic average of daily fixings weighted by accrual days.
    #[classattr]
    const SIMPLE_AVERAGE: PyOvernightCompoundingMethod = PyOvernightCompoundingMethod {
        inner: OvernightCompoundingMethod::SimpleAverage,
    };
    /// Compounded in arrears (ISDA 2021 standard; default).
    #[classattr]
    const COMPOUNDED_IN_ARREARS: PyOvernightCompoundingMethod = PyOvernightCompoundingMethod {
        inner: OvernightCompoundingMethod::CompoundedInArrears,
    };

    /// Compounded in arrears with a lookback of ``lookback_days`` business days.
    #[staticmethod]
    #[pyo3(text_signature = "(lookback_days)")]
    fn compounded_with_lookback(lookback_days: u32) -> Self {
        Self {
            inner: OvernightCompoundingMethod::CompoundedWithLookback { lookback_days },
        }
    }

    /// Compounded in arrears with a rate lockout of ``lockout_days`` business days.
    #[staticmethod]
    #[pyo3(text_signature = "(lockout_days)")]
    fn compounded_with_lockout(lockout_days: u32) -> Self {
        Self {
            inner: OvernightCompoundingMethod::CompoundedWithLockout { lockout_days },
        }
    }

    /// Compounded in arrears with an observation shift of ``shift_days`` business days.
    #[staticmethod]
    #[pyo3(text_signature = "(shift_days)")]
    fn compounded_with_observation_shift(shift_days: u32) -> Self {
        Self {
            inner: OvernightCompoundingMethod::CompoundedWithObservationShift { shift_days },
        }
    }

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!("OvernightCompoundingMethod({:?})", self.inner)
    }
}

/// Wrapper for [`OvernightIndexConstraintApplication`].
#[pyclass(
    name = "OvernightIndexConstraintApplication",
    module = "finstack_quant.cashflows.builder",
    frozen,
    eq,
    hash,
    skip_from_py_object
)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PyOvernightIndexConstraintApplication {
    /// Inner constraint-application policy.
    pub(crate) inner: OvernightIndexConstraintApplication,
}

#[pymethods]
impl PyOvernightIndexConstraintApplication {
    /// Apply index floors/caps to each daily fixing before compounding (default).
    #[classattr]
    const DAILY: PyOvernightIndexConstraintApplication = PyOvernightIndexConstraintApplication {
        inner: OvernightIndexConstraintApplication::Daily,
    };
    /// Apply index floors/caps once to the compounded period index rate.
    #[classattr]
    const PERIOD: PyOvernightIndexConstraintApplication = PyOvernightIndexConstraintApplication {
        inner: OvernightIndexConstraintApplication::Period,
    };

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!("OvernightIndexConstraintApplication({:?})", self.inner)
    }
}

/// Wrapper for [`FloatingRateFallback`].
#[pyclass(
    name = "FloatingRateFallback",
    module = "finstack_quant.cashflows.builder",
    frozen,
    eq,
    hash,
    skip_from_py_object
)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PyFloatingRateFallback {
    /// Inner fallback policy.
    pub(crate) inner: FloatingRateFallback,
}

#[pymethods]
impl PyFloatingRateFallback {
    /// Fail the build when the forward curve lookup fails (default, safest).
    #[allow(non_snake_case)]
    #[classattr]
    fn ERROR() -> Self {
        Self {
            inner: FloatingRateFallback::Error,
        }
    }
    /// Project spread-only when no forward curve is available (explicit opt-in).
    #[allow(non_snake_case)]
    #[classattr]
    fn SPREAD_ONLY() -> Self {
        Self {
            inner: FloatingRateFallback::SpreadOnly,
        }
    }

    /// Use ``rate`` (decimal annual rate, e.g. ``0.045``) as the index component.
    #[staticmethod]
    #[pyo3(text_signature = "(rate)")]
    fn fixed_rate(rate: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: FloatingRateFallback::FixedRate(decimal_from_any(rate)?),
        })
    }

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!("FloatingRateFallback({:?})", self.inner)
    }
}

/// Wrapper for [`FeeAccrualBasis`].
#[pyclass(
    name = "FeeAccrualBasis",
    module = "finstack_quant.cashflows.builder",
    frozen,
    eq,
    hash,
    skip_from_py_object
)]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PyFeeAccrualBasis {
    /// Inner fee accrual basis.
    pub(crate) inner: FeeAccrualBasis,
}

#[pymethods]
impl PyFeeAccrualBasis {
    /// Sample the outstanding balance at accrual-period start (default).
    #[classattr]
    const POINT_IN_TIME: PyFeeAccrualBasis = PyFeeAccrualBasis {
        inner: FeeAccrualBasis::PointInTime,
    };
    /// Time-weighted average outstanding over the accrual period.
    #[classattr]
    const TIME_WEIGHTED_AVERAGE: PyFeeAccrualBasis = PyFeeAccrualBasis {
        inner: FeeAccrualBasis::TimeWeightedAverage,
    };

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!("FeeAccrualBasis({:?})", self.inner)
    }
}

/// Wrapper for [`ScheduleParams`] (`finstack_quant.cashflows.builder.ScheduleParams`).
#[pyclass(
    name = "ScheduleParams",
    module = "finstack_quant.cashflows.builder",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyScheduleParams {
    /// Inner schedule-generation parameters.
    pub(crate) inner: ScheduleParams,
}

impl PyScheduleParams {
    /// Build from an existing Rust [`ScheduleParams`].
    pub(crate) fn from_inner(inner: ScheduleParams) -> Self {
        Self { inner }
    }
}

macro_rules! schedule_params_presets {
    ($( $(#[$doc:meta])* $name:ident ),+ $(,)?) => {
        #[pymethods]
        impl PyScheduleParams {
            $(
                $(#[$doc])*
                #[staticmethod]
                #[pyo3(text_signature = "()")]
                fn $name() -> Self {
                    Self::from_inner(ScheduleParams::$name())
                }
            )+
        }
    };
}

schedule_params_presets! {
    /// Quarterly, Act/360, Modified Following, weekends-only calendar.
    quarterly_act360,
    /// Semi-annual, 30/360, Modified Following, weekends-only calendar.
    semiannual_30360,
    /// Annual, Act/Act, Following, weekends-only calendar.
    annual_actact,
    /// USD SOFR swap: quarterly, Act/360, MF, USNY, T+2 lag, adjusted accruals.
    usd_sofr_swap,
    /// USD corporate bond: semi-annual, 30/360, Following, USNY.
    usd_corporate_bond,
    /// USD Treasury: semi-annual, Act/Act ISMA, Following, USNY.
    usd_treasury,
    /// EUR ESTR swap: annual, Act/360, MF, TARGET2, T+2 lag, adjusted accruals.
    eur_estr_swap,
    /// EUR government bond: annual, Act/Act ISMA, Following, TARGET2.
    eur_gov_bond,
    /// GBP SONIA swap: annual, Act/365F, MF, GBLO, no lag, adjusted accruals.
    gbp_sonia_swap,
    /// JPY TONA swap: annual, Act/365F, MF, JPTO, T+2 lag, adjusted accruals.
    jpy_tona_swap,
}

#[pymethods]
impl PyScheduleParams {
    /// Construct schedule-generation parameters.
    ///
    /// Parameters
    /// ----------
    /// frequency : Tenor or str
    ///     Accrual and payment frequency (e.g. ``"3M"``).
    /// day_count : DayCount
    ///     Day-count convention for accrual year fractions.
    /// calendar_id : str
    ///     Holiday calendar id (``"weekends_only"`` for weekend-only rolling).
    /// business_day_convention : BusinessDayConvention or str, optional
    ///     Payment-date rolling convention (default Modified Following).
    /// stub : StubKind, optional
    ///     Stub rule (default short-front).
    /// end_of_month : bool, default False
    ///     Preserve end-of-month rolling.
    /// payment_lag_days : int, default 0
    ///     Payment lag in business days.
    /// adjust_accrual_dates : bool, default False
    ///     Roll accrual boundaries with ``business_day_convention`` (swap/ISDA convention).
    /// roll_rule : RollRule, optional
    ///     IMM/CDS-IMM anchor grid (default none).
    #[new]
    #[pyo3(
        signature = (frequency, day_count, calendar_id, business_day_convention=None, stub=None, end_of_month=false, payment_lag_days=0, adjust_accrual_dates=false, roll_rule=None),
        text_signature = "(frequency, day_count, calendar_id, business_day_convention=None, stub=None, end_of_month=False, payment_lag_days=0, adjust_accrual_dates=False, roll_rule=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn new(
        frequency: &Bound<'_, PyAny>,
        day_count: PyRef<'_, PyDayCount>,
        calendar_id: &str,
        business_day_convention: Option<&Bound<'_, PyAny>>,
        stub: Option<PyRef<'_, PyStubKind>>,
        end_of_month: bool,
        payment_lag_days: i32,
        adjust_accrual_dates: bool,
        roll_rule: Option<PyRef<'_, PyRollRule>>,
    ) -> PyResult<Self> {
        let business_day_convention = match business_day_convention {
            Some(obj) => extract_business_day_convention(obj)?,
            None => BusinessDayConvention::ModifiedFollowing,
        };
        Ok(Self::from_inner(ScheduleParams {
            frequency: extract_tenor(frequency)?,
            day_count: day_count.inner,
            business_day_convention,
            calendar_id: calendar_id.to_string(),
            stub: stub.map_or(StubKind::ShortFront, |s| s.inner),
            end_of_month,
            payment_lag_days,
            adjust_accrual_dates,
            roll_rule: roll_rule.map_or(RollRule::None, |r| r.inner),
        }))
    }

    /// Accrual and payment frequency.
    #[getter]
    fn frequency(&self) -> PyTenor {
        PyTenor::from_inner(self.inner.frequency)
    }

    /// Day-count convention.
    #[getter]
    fn day_count(&self) -> PyDayCount {
        PyDayCount::from_inner(self.inner.day_count)
    }

    /// Holiday calendar identifier.
    #[getter]
    fn calendar_id(&self) -> String {
        self.inner.calendar_id.clone()
    }

    /// Payment lag in business days.
    #[getter]
    fn payment_lag_days(&self) -> i32 {
        self.inner.payment_lag_days
    }

    /// Whether end-of-month rolling is preserved.
    #[getter]
    fn end_of_month(&self) -> bool {
        self.inner.end_of_month
    }

    /// Whether accrual boundaries are business-day adjusted.
    #[getter]
    fn adjust_accrual_dates(&self) -> bool {
        self.inner.adjust_accrual_dates
    }

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!("ScheduleParams({:?})", self.inner)
    }
}

/// Wrapper for [`FixedWindow`].
#[pyclass(
    name = "FixedWindow",
    module = "finstack_quant.cashflows.builder",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFixedWindow {
    /// Inner fixed-rate window.
    pub(crate) inner: FixedWindow,
}

#[pymethods]
impl PyFixedWindow {
    /// Fixed-rate coupon window with a shared schedule.
    ///
    /// Parameters
    /// ----------
    /// rate : decimal.Decimal or float
    ///     Annual coupon rate as a decimal (``0.05`` for 5%).
    /// schedule : ScheduleParams
    ///     Accrual and payment schedule conventions for the window.
    #[new]
    #[pyo3(text_signature = "(rate, schedule)")]
    fn new(rate: &Bound<'_, PyAny>, schedule: PyRef<'_, PyScheduleParams>) -> PyResult<Self> {
        Ok(Self {
            inner: FixedWindow {
                rate: decimal_from_any(rate)?,
                schedule: schedule.inner.clone(),
            },
        })
    }

    /// Annual coupon rate as ``decimal.Decimal``.
    #[getter]
    fn rate<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        decimal_to_py(py, self.inner.rate)
    }

    /// Schedule conventions for this window.
    #[getter]
    fn schedule(&self) -> PyScheduleParams {
        PyScheduleParams::from_inner(self.inner.schedule.clone())
    }

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!("FixedWindow(rate={}, ...)", self.inner.rate)
    }
}

/// Wrapper for [`FixedCouponSpec`].
#[pyclass(
    name = "FixedCouponSpec",
    module = "finstack_quant.cashflows.builder",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFixedCouponSpec {
    /// Inner fixed-coupon spec.
    pub(crate) inner: FixedCouponSpec,
}

#[pymethods]
impl PyFixedCouponSpec {
    /// Fixed-rate coupon specification.
    ///
    /// Parameters
    /// ----------
    /// rate : decimal.Decimal or float
    ///     Annual coupon rate as a decimal (``0.05`` for 5%).
    /// schedule : ScheduleParams
    ///     Accrual and payment schedule conventions.
    /// coupon_type : CouponType, optional
    ///     Cash (default), PIK, or split settlement.
    #[new]
    #[pyo3(
        signature = (rate, schedule, coupon_type=None),
        text_signature = "(rate, schedule, coupon_type=None)"
    )]
    fn new(
        rate: &Bound<'_, PyAny>,
        schedule: PyRef<'_, PyScheduleParams>,
        coupon_type: Option<PyRef<'_, PyCouponType>>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: FixedCouponSpec {
                coupon_type: coupon_type.map_or(CouponType::Cash, |c| c.inner),
                rate: decimal_from_any(rate)?,
                schedule: schedule.inner.clone(),
            },
        })
    }

    /// Annual coupon rate as ``decimal.Decimal``.
    #[getter]
    fn rate<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        decimal_to_py(py, self.inner.rate)
    }

    /// Schedule conventions.
    #[getter]
    fn schedule(&self) -> PyScheduleParams {
        PyScheduleParams::from_inner(self.inner.schedule.clone())
    }

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!("FixedCouponSpec(rate={}, ...)", self.inner.rate)
    }
}

/// Wrapper for [`FloatingRateSpec`].
#[pyclass(
    name = "FloatingRateSpec",
    module = "finstack_quant.cashflows.builder",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFloatingRateSpec {
    /// Inner floating-rate spec.
    pub(crate) inner: FloatingRateSpec,
}

#[pymethods]
impl PyFloatingRateSpec {
    /// Canonical floating rate specification.
    ///
    /// Parameters mirror the Rust struct field-for-field; ``*_bp`` values are
    /// basis points and accept ``decimal.Decimal`` (lossless) or ``float``.
    #[new]
    #[pyo3(
        signature = (index_id, spread_bp, reset_frequency, gearing=None, gearing_includes_spread=true, index_floor_bp=None, all_in_floor_bp=None, all_in_cap_bp=None, index_cap_bp=None, overnight_index_constraints=None, index_tenor=None, reset_lag_days=2, fixing_calendar_id=None, overnight_compounding=None, overnight_basis=None, fallback=None),
        text_signature = "(index_id, spread_bp, reset_frequency, gearing=None, gearing_includes_spread=True, index_floor_bp=None, all_in_floor_bp=None, all_in_cap_bp=None, index_cap_bp=None, overnight_index_constraints=None, index_tenor=None, reset_lag_days=2, fixing_calendar_id=None, overnight_compounding=None, overnight_basis=None, fallback=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn new(
        index_id: &str,
        spread_bp: &Bound<'_, PyAny>,
        reset_frequency: &Bound<'_, PyAny>,
        gearing: Option<&Bound<'_, PyAny>>,
        gearing_includes_spread: bool,
        index_floor_bp: Option<&Bound<'_, PyAny>>,
        all_in_floor_bp: Option<&Bound<'_, PyAny>>,
        all_in_cap_bp: Option<&Bound<'_, PyAny>>,
        index_cap_bp: Option<&Bound<'_, PyAny>>,
        overnight_index_constraints: Option<PyRef<'_, PyOvernightIndexConstraintApplication>>,
        index_tenor: Option<&Bound<'_, PyAny>>,
        reset_lag_days: i32,
        fixing_calendar_id: Option<String>,
        overnight_compounding: Option<PyRef<'_, PyOvernightCompoundingMethod>>,
        overnight_basis: Option<PyRef<'_, PyDayCount>>,
        fallback: Option<PyRef<'_, PyFloatingRateFallback>>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: FloatingRateSpec {
                index_id: finstack_quant_core::types::CurveId::from(index_id),
                spread_bp: decimal_from_any(spread_bp)?,
                gearing: gearing.map_or(Ok(Decimal::ONE), decimal_from_any)?,
                gearing_includes_spread,
                index_floor_bp: index_floor_bp.map(decimal_from_any).transpose()?,
                all_in_floor_bp: all_in_floor_bp.map(decimal_from_any).transpose()?,
                all_in_cap_bp: all_in_cap_bp.map(decimal_from_any).transpose()?,
                index_cap_bp: index_cap_bp.map(decimal_from_any).transpose()?,
                overnight_index_constraints: overnight_index_constraints
                    .map_or(OvernightIndexConstraintApplication::Daily, |c| c.inner),
                reset_frequency: extract_tenor(reset_frequency)?,
                index_tenor: index_tenor.map(extract_tenor).transpose()?,
                reset_lag_days,
                fixing_calendar_id,
                overnight_compounding: overnight_compounding.map(|m| m.inner),
                overnight_basis: overnight_basis.map(|d| d.inner),
                fallback: fallback.map_or(FloatingRateFallback::Error, |f| f.inner.clone()),
            },
        })
    }

    /// Forward curve identifier.
    #[getter]
    fn index_id(&self) -> String {
        self.inner.index_id.to_string()
    }

    /// Spread over the index in basis points, as ``decimal.Decimal``.
    #[getter]
    fn spread_bp<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        decimal_to_py(py, self.inner.spread_bp)
    }

    /// Validate reset lag and floor/cap ordering.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the reset lag is negative or a floor exceeds its cap.
    #[pyo3(text_signature = "(self)")]
    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(core_to_py)
    }

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!(
            "FloatingRateSpec(index_id='{}', spread_bp={})",
            self.inner.index_id, self.inner.spread_bp
        )
    }
}

/// Wrapper for [`FloatingCouponSpec`].
#[pyclass(
    name = "FloatingCouponSpec",
    module = "finstack_quant.cashflows.builder",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFloatingCouponSpec {
    /// Inner floating-coupon spec.
    pub(crate) inner: FloatingCouponSpec,
}

#[pymethods]
impl PyFloatingCouponSpec {
    /// Floating coupon specification composing a [`FloatingRateSpec`].
    #[new]
    #[pyo3(
        signature = (rate_spec, schedule, coupon_type=None),
        text_signature = "(rate_spec, schedule, coupon_type=None)"
    )]
    fn new(
        rate_spec: PyRef<'_, PyFloatingRateSpec>,
        schedule: PyRef<'_, PyScheduleParams>,
        coupon_type: Option<PyRef<'_, PyCouponType>>,
    ) -> Self {
        Self {
            inner: FloatingCouponSpec {
                rate_spec: rate_spec.inner.clone(),
                coupon_type: coupon_type.map_or(CouponType::Cash, |c| c.inner),
                schedule: schedule.inner.clone(),
            },
        }
    }

    /// Floating rate specification.
    #[getter]
    fn rate_spec(&self) -> PyFloatingRateSpec {
        PyFloatingRateSpec {
            inner: self.inner.rate_spec.clone(),
        }
    }

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!(
            "FloatingCouponSpec(index_id='{}')",
            self.inner.rate_spec.index_id
        )
    }
}

/// Wrapper for [`StepUpCouponSpec`].
#[pyclass(
    name = "StepUpCouponSpec",
    module = "finstack_quant.cashflows.builder",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyStepUpCouponSpec {
    /// Inner step-up coupon spec.
    pub(crate) inner: StepUpCouponSpec,
}

#[pymethods]
impl PyStepUpCouponSpec {
    /// Step-up/step-down coupon specification.
    ///
    /// Parameters
    /// ----------
    /// initial_rate : decimal.Decimal or float
    ///     Rate until the first step date.
    /// step_schedule : list[tuple[datetime.date, decimal.Decimal | float]]
    ///     ``(effective_date, new_rate)`` pairs, strictly increasing by date.
    /// schedule : ScheduleParams
    ///     Accrual and payment schedule conventions.
    /// coupon_type : CouponType, optional
    ///     Cash (default), PIK, or split settlement.
    #[new]
    #[pyo3(
        signature = (initial_rate, step_schedule, schedule, coupon_type=None),
        text_signature = "(initial_rate, step_schedule, schedule, coupon_type=None)"
    )]
    fn new(
        initial_rate: &Bound<'_, PyAny>,
        step_schedule: Vec<(Bound<'_, PyAny>, Bound<'_, PyAny>)>,
        schedule: PyRef<'_, PyScheduleParams>,
        coupon_type: Option<PyRef<'_, PyCouponType>>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: StepUpCouponSpec {
                coupon_type: coupon_type.map_or(CouponType::Cash, |c| c.inner),
                initial_rate: decimal_from_any(initial_rate)?,
                step_schedule: date_decimal_pairs(step_schedule)?,
                schedule: schedule.inner.clone(),
            },
        })
    }

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!(
            "StepUpCouponSpec(initial_rate={}, steps={})",
            self.inner.initial_rate,
            self.inner.step_schedule.len()
        )
    }
}

/// Wrapper for [`AmortizationSpec`].
#[pyclass(
    name = "AmortizationSpec",
    module = "finstack_quant.cashflows.builder",
    frozen,
    eq,
    hash,
    skip_from_py_object
)]
#[derive(Clone, Debug, PartialEq, Hash)]
pub struct PyAmortizationSpec {
    /// Inner amortization rule.
    pub(crate) inner: AmortizationSpec,
}

#[pymethods]
impl PyAmortizationSpec {
    /// No amortization (bullet).
    #[allow(non_snake_case)]
    #[classattr]
    fn NONE() -> Self {
        Self {
            inner: AmortizationSpec::None,
        }
    }

    /// Linear paydown towards ``final_notional`` over all coupon periods.
    #[staticmethod]
    #[pyo3(text_signature = "(final_notional)")]
    fn linear_to(final_notional: PyMoney) -> Self {
        Self {
            inner: AmortizationSpec::LinearTo {
                final_notional: final_notional.inner,
            },
        }
    }

    /// Explicit ``(date, remaining_principal_after_date)`` schedule.
    #[staticmethod]
    #[pyo3(text_signature = "(schedule)")]
    fn step_remaining(schedule: Vec<(Bound<'_, PyAny>, PyMoney)>) -> PyResult<Self> {
        Ok(Self {
            inner: AmortizationSpec::StepRemaining {
                schedule: date_money_pairs(schedule)?,
            },
        })
    }

    /// Fixed percentage of original notional paid each period.
    #[staticmethod]
    #[pyo3(text_signature = "(pct)")]
    fn percent_of_original_per_period(pct: f64) -> Self {
        Self {
            inner: AmortizationSpec::PercentOfOriginalPerPeriod { pct },
        }
    }

    /// Custom principal exchanges on specific dates (absolute cash amounts).
    #[staticmethod]
    #[pyo3(text_signature = "(items)")]
    fn custom_principal(items: Vec<(Bound<'_, PyAny>, PyMoney)>) -> PyResult<Self> {
        Ok(Self {
            inner: AmortizationSpec::CustomPrincipal {
                items: date_money_pairs(items)?,
            },
        })
    }

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!("AmortizationSpec({:?})", self.inner)
    }
}

/// Wrapper for [`Notional`].
#[pyclass(
    name = "Notional",
    module = "finstack_quant.cashflows.builder",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyNotional {
    /// Inner notional with amortization rule.
    pub(crate) inner: Notional,
}

impl PyNotional {
    /// Build from an existing Rust [`Notional`].
    pub(crate) fn from_inner(inner: Notional) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyNotional {
    /// Notional amount with an optional amortization rule.
    ///
    /// Parameters
    /// ----------
    /// initial : Money
    ///     Initial principal amount outstanding at leg inception.
    /// amort : AmortizationSpec, optional
    ///     Amortization rule applied after each period (default: none).
    #[new]
    #[pyo3(signature = (initial, amort=None), text_signature = "(initial, amort=None)")]
    fn new(initial: PyMoney, amort: Option<PyRef<'_, PyAmortizationSpec>>) -> Self {
        Self {
            inner: Notional {
                initial: initial.inner,
                amort: amort.map_or(AmortizationSpec::None, |a| a.inner.clone()),
            },
        }
    }

    /// Plain (non-amortising) notional helper.
    ///
    /// Parameters
    /// ----------
    /// amount : float
    ///     Initial principal amount.
    /// currency : Currency or str
    ///     Currency of the notional, as a ``Currency`` instance or ISO code.
    ///
    /// Returns
    /// -------
    /// Notional
    ///     A bullet notional with no amortization.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If *currency* is not a valid ISO 4217 code.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.cashflows.builder import Notional
    /// >>> Notional.par(1_000_000.0, "USD").initial.amount
    /// 1000000.0
    #[classmethod]
    #[pyo3(text_signature = "(cls, amount, currency)")]
    fn par(_cls: &Bound<'_, PyType>, amount: f64, currency: &Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: Notional::par(amount, extract_currency(currency)?),
        })
    }

    /// Initial principal amount.
    #[getter]
    fn initial(&self) -> PyMoney {
        PyMoney::from_inner(self.inner.initial)
    }

    /// Currency of the notional.
    ///
    /// Returns
    /// -------
    /// Currency
    ///     The currency of the initial notional amount.
    ///
    /// Raises
    /// ------
    /// RuntimeError
    ///     Never raised in practice; present for the shared Python error
    ///     protocol.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.cashflows.builder import Notional
    /// >>> Notional.par(1_000_000.0, "USD").currency().code
    /// 'USD'
    #[pyo3(text_signature = "(self)")]
    fn currency(&self, py: Python<'_>) -> PyResult<Py<PyCurrency>> {
        Py::new(py, PyCurrency::from_inner(self.inner.currency()))
    }

    /// Validate the notional and its amortization rule.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the amortization schedule is inconsistent with the initial
    ///     notional (e.g. a currency mismatch or a target above the initial
    ///     amount).
    #[pyo3(text_signature = "(self)")]
    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(core_to_py)
    }

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!(
            "Notional(initial={} {}, amort={:?})",
            self.inner.initial.amount(),
            self.inner.initial.currency(),
            self.inner.amort
        )
    }
}

/// Wrapper for [`FeeBase`].
#[pyclass(
    name = "FeeBase",
    module = "finstack_quant.cashflows.builder",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFeeBase {
    /// Inner fee base.
    pub(crate) inner: FeeBase,
}

#[pymethods]
impl PyFeeBase {
    /// Fee accrues on the drawn outstanding balance.
    #[allow(non_snake_case)]
    #[classattr]
    fn DRAWN() -> Self {
        Self {
            inner: FeeBase::Drawn,
        }
    }

    /// Fee accrues on undrawn = max(facility_limit - outstanding, 0).
    #[staticmethod]
    #[pyo3(text_signature = "(facility_limit)")]
    fn undrawn(facility_limit: PyMoney) -> Self {
        Self {
            inner: FeeBase::Undrawn {
                facility_limit: facility_limit.inner,
            },
        }
    }

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!("FeeBase({:?})", self.inner)
    }
}

/// Wrapper for [`FeeSpec`].
#[pyclass(
    name = "FeeSpec",
    module = "finstack_quant.cashflows.builder",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyFeeSpec {
    /// Inner fee specification.
    pub(crate) inner: FeeSpec,
}

#[pymethods]
impl PyFeeSpec {
    /// Fixed fee paid once on ``date``. Negative amounts are rebates.
    #[staticmethod]
    #[pyo3(text_signature = "(date, amount)")]
    fn fixed(date: &Bound<'_, PyAny>, amount: PyMoney) -> PyResult<Self> {
        Ok(Self {
            inner: FeeSpec::Fixed {
                date: py_to_date(date)?,
                amount: amount.inner,
            },
        })
    }

    /// Periodic fee quoted in basis points per annum, accrued over generated periods.
    #[staticmethod]
    #[pyo3(
        signature = (base, bp, frequency, day_count, business_day_convention, calendar_id, stub=None, accrual_basis=None),
        text_signature = "(base, bp, frequency, day_count, business_day_convention, calendar_id, stub=None, accrual_basis=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn periodic_bp(
        base: PyRef<'_, PyFeeBase>,
        bp: &Bound<'_, PyAny>,
        frequency: &Bound<'_, PyAny>,
        day_count: PyRef<'_, PyDayCount>,
        business_day_convention: &Bound<'_, PyAny>,
        calendar_id: &str,
        stub: Option<PyRef<'_, PyStubKind>>,
        accrual_basis: Option<PyRef<'_, PyFeeAccrualBasis>>,
    ) -> PyResult<Self> {
        Ok(Self {
            inner: FeeSpec::PeriodicBp {
                base: base.inner.clone(),
                bp: decimal_from_any(bp)?,
                frequency: extract_tenor(frequency)?,
                day_count: day_count.inner,
                business_day_convention: extract_business_day_convention(business_day_convention)?,
                calendar_id: calendar_id.to_string(),
                stub: stub.map_or(StubKind::ShortFront, |s| s.inner),
                accrual_basis: accrual_basis
                    .map_or(FeeAccrualBasis::PointInTime, |a| a.inner.clone()),
            },
        })
    }

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!("FeeSpec({:?})", self.inner)
    }
}

/// Wrapper for [`PrepaymentModelSpec`].
#[pyclass(
    name = "PrepaymentModelSpec",
    module = "finstack_quant.cashflows.builder",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyPrepaymentModelSpec {
    /// Inner prepayment model.
    pub(crate) inner: PrepaymentModelSpec,
}

#[pymethods]
impl PyPrepaymentModelSpec {
    /// Constant CPR (no seasoning curve).
    #[staticmethod]
    #[pyo3(text_signature = "(cpr)")]
    fn constant_cpr(cpr: f64) -> Self {
        Self {
            inner: PrepaymentModelSpec::constant_cpr(cpr),
        }
    }

    /// PSA curve with speed multiplier (1.0 = 100% PSA).
    #[staticmethod]
    #[pyo3(text_signature = "(speed_multiplier)")]
    fn psa(speed_multiplier: f64) -> Self {
        Self {
            inner: PrepaymentModelSpec::psa(speed_multiplier),
        }
    }

    /// 100% PSA (standard prepayment assumption).
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn psa_100() -> Self {
        Self {
            inner: PrepaymentModelSpec::psa_100(),
        }
    }

    /// CMBS lockout: zero prepayment for ``lockout_months``, then constant CPR.
    #[staticmethod]
    #[pyo3(text_signature = "(lockout_months, post_lockout_cpr)")]
    fn cmbs_with_lockout(lockout_months: u32, post_lockout_cpr: f64) -> Self {
        Self {
            inner: PrepaymentModelSpec::cmbs_with_lockout(lockout_months, post_lockout_cpr),
        }
    }

    /// Annual constant prepayment rate.
    #[getter]
    fn cpr(&self) -> f64 {
        self.inner.cpr
    }

    /// Single-month mortality for the supplied seasoning.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the underlying curve parameters are invalid (e.g. a negative
    ///     PSA speed multiplier).
    #[pyo3(text_signature = "(self, seasoning_months)")]
    fn smm(&self, seasoning_months: u32) -> PyResult<f64> {
        self.inner.smm(seasoning_months).map_err(core_to_py)
    }

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!(
            "PrepaymentModelSpec(cpr={}, curve={:?})",
            self.inner.cpr, self.inner.curve
        )
    }
}

/// Wrapper for [`DefaultModelSpec`].
#[pyclass(
    name = "DefaultModelSpec",
    module = "finstack_quant.cashflows.builder",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyDefaultModelSpec {
    /// Inner default model.
    pub(crate) inner: DefaultModelSpec,
}

#[pymethods]
impl PyDefaultModelSpec {
    /// Constant CDR (no seasoning curve).
    #[staticmethod]
    #[pyo3(text_signature = "(cdr)")]
    fn constant_cdr(cdr: f64) -> Self {
        Self {
            inner: DefaultModelSpec::constant_cdr(cdr),
        }
    }

    /// SDA curve with speed multiplier (1.0 = 100% SDA).
    #[staticmethod]
    #[pyo3(text_signature = "(speed_multiplier)")]
    fn sda(speed_multiplier: f64) -> Self {
        Self {
            inner: DefaultModelSpec::sda(speed_multiplier),
        }
    }

    /// 2% CDR baseline.
    #[staticmethod]
    #[pyo3(text_signature = "()")]
    fn cdr_2pct() -> Self {
        Self {
            inner: DefaultModelSpec::cdr_2pct(),
        }
    }

    /// Annual constant default rate.
    #[getter]
    fn cdr(&self) -> f64 {
        self.inner.cdr
    }

    /// Monthly default rate for the supplied seasoning.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the underlying curve parameters are invalid (e.g. a negative
    ///     SDA speed multiplier).
    #[pyo3(text_signature = "(self, seasoning_months)")]
    fn mdr(&self, seasoning_months: u32) -> PyResult<f64> {
        self.inner.mdr(seasoning_months).map_err(core_to_py)
    }

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!(
            "DefaultModelSpec(cdr={}, curve={:?})",
            self.inner.cdr, self.inner.curve
        )
    }
}

/// Wrapper for [`RecoveryModelSpec`].
#[pyclass(
    name = "RecoveryModelSpec",
    module = "finstack_quant.cashflows.builder",
    frozen,
    skip_from_py_object
)]
#[derive(Clone, Debug)]
pub struct PyRecoveryModelSpec {
    /// Inner recovery model.
    pub(crate) inner: RecoveryModelSpec,
}

#[pymethods]
impl PyRecoveryModelSpec {
    /// Recovery model with rate (fraction in ``[0, 1]``) and lag in months.
    #[new]
    #[pyo3(text_signature = "(rate, recovery_lag)")]
    fn new(rate: f64, recovery_lag: u32) -> Self {
        Self {
            inner: RecoveryModelSpec::with_lag(rate, recovery_lag),
        }
    }

    /// Recovery rate as a fraction.
    #[getter]
    fn rate(&self) -> f64 {
        self.inner.rate
    }

    /// Recovery lag in months.
    #[getter]
    fn recovery_lag(&self) -> u32 {
        self.inner.recovery_lag
    }

    /// Validate that the rate is finite and in ``[0, 1]``.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the rate is not finite or falls outside ``[0, 1]``.
    #[pyo3(text_signature = "(self)")]
    fn validate(&self) -> PyResult<()> {
        self.inner.validate().map_err(core_to_py)
    }

    /// Debug-style representation.
    fn __repr__(&self) -> String {
        format!(
            "RecoveryModelSpec(rate={}, recovery_lag={})",
            self.inner.rate, self.inner.recovery_lag
        )
    }
}

/// Add all spec classes to the builder module.
pub(crate) fn add_classes(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyRollRule>()?;
    module.add_class::<PyPrincipalExchange>()?;
    module.add_class::<PyCouponType>()?;
    module.add_class::<PyOvernightCompoundingMethod>()?;
    module.add_class::<PyOvernightIndexConstraintApplication>()?;
    module.add_class::<PyFloatingRateFallback>()?;
    module.add_class::<PyFeeAccrualBasis>()?;
    module.add_class::<PyScheduleParams>()?;
    module.add_class::<PyFixedWindow>()?;
    module.add_class::<PyFixedCouponSpec>()?;
    module.add_class::<PyFloatingRateSpec>()?;
    module.add_class::<PyFloatingCouponSpec>()?;
    module.add_class::<PyStepUpCouponSpec>()?;
    module.add_class::<PyAmortizationSpec>()?;
    module.add_class::<PyNotional>()?;
    module.add_class::<PyFeeBase>()?;
    module.add_class::<PyFeeSpec>()?;
    module.add_class::<PyPrepaymentModelSpec>()?;
    module.add_class::<PyDefaultModelSpec>()?;
    module.add_class::<PyRecoveryModelSpec>()?;
    Ok(())
}
