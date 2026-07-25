//! Typed leg-spec wrappers (`FixedLegSpec`, `FloatLegSpec`) shared by the
//! typed `InterestRateSwap` and `Swaption` builders.
//!
//! Mirrors the `PyBond` pattern in `instruments.rs`: thin frozen wrappers,
//! construction and validation in Rust.

use pyo3::prelude::*;

use crate::bindings::core::dates::daycount::PyDayCount;
use crate::bindings::core::dates::tenor::PyTenor;
use crate::bindings::core::dates::utils::py_to_date;
use crate::errors::core_to_py;

use super::instruments::{decimal_from_f64, enum_from_str};

/// Typed wrapper for the Rust `FixedLegSpec` (fixed leg of an IRS/swaption).
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "FixedLegSpec",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFixedLegSpec {
    /// Inner canonical Rust fixed-leg spec.
    pub(crate) inner: finstack_quant_valuations::instruments::FixedLegSpec,
}

#[pymethods]
impl PyFixedLegSpec {
    /// Fixed leg of an interest-rate swap.
    ///
    /// Parameters
    /// ----------
    /// discount_curve_id : str
    ///     Discount curve identifier for pricing this leg.
    /// rate : float
    ///     Fixed rate as a decimal (0.04 = 4%).
    /// frequency : Tenor
    ///     Payment frequency.
    /// day_count : DayCount
    ///     Day count convention for accrual.
    /// start : datetime.date
    ///     Start date of the fixed leg.
    /// end : datetime.date
    ///     End date of the fixed leg.
    /// bdc : str, default "modified_following"
    ///     Business day convention for payment dates.
    /// calendar_id : str, optional
    ///     Calendar used for business day adjustments.
    /// stub : str, default "ShortFront"
    ///     Stub period handling rule.
    /// compounding_simple : bool
    ///     If true, use simple interest on the accrual fraction. Required:
    ///     the canonical Rust ``FixedLegSpec`` field has no default.
    /// payment_lag_days : int, default 0
    ///     Payment lag in business days after period end.
    /// end_of_month : bool, default False
    ///     End-of-month roll convention.
    ///
    /// Returns
    /// -------
    /// FixedLegSpec
    ///     The validated fixed-leg specification.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If an enum value is invalid or the accrual period is malformed
    ///     (``start >= end``).
    ///
    /// Examples
    /// --------
    /// >>> import datetime
    /// >>> from finstack_quant.core.dates import DayCount, Tenor
    /// >>> from finstack_quant.valuations.instruments import FixedLegSpec
    /// >>> leg = FixedLegSpec(
    /// ...     "USD-OIS", 0.04, Tenor.semi_annual(), DayCount.THIRTY_360,
    /// ...     datetime.date(2024, 1, 15), datetime.date(2029, 1, 15),
    /// ...     compounding_simple=False,
    /// ... )
    /// >>> "0.04" in repr(leg)
    /// True
    #[new]
    #[pyo3(signature = (discount_curve_id, rate, frequency, day_count, start, end, *,
                        compounding_simple, bdc = "modified_following", calendar_id = None,
                        stub = "ShortFront", payment_lag_days = 0, end_of_month = false))]
    #[pyo3(
        text_signature = "(discount_curve_id, rate, frequency, day_count, start, end, *, \
compounding_simple, bdc='modified_following', calendar_id=None, stub='ShortFront', \
payment_lag_days=0, end_of_month=False)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn new(
        discount_curve_id: &str,
        rate: f64,
        frequency: PyRef<'_, PyTenor>,
        day_count: PyRef<'_, PyDayCount>,
        start: &Bound<'_, PyAny>,
        end: &Bound<'_, PyAny>,
        compounding_simple: bool,
        bdc: &str,
        calendar_id: Option<String>,
        stub: &str,
        payment_lag_days: i32,
        end_of_month: bool,
    ) -> PyResult<Self> {
        let inner = finstack_quant_valuations::instruments::FixedLegSpec {
            discount_curve_id: finstack_quant_core::types::CurveId::new(
                discount_curve_id.to_string(),
            ),
            rate: decimal_from_f64(rate, "rate")?,
            frequency: frequency.inner,
            day_count: day_count.inner,
            bdc: enum_from_str(bdc, "bdc")?,
            calendar_id,
            stub: enum_from_str(stub, "stub")?,
            start: py_to_date(start)?,
            end: py_to_date(end)?,
            par_method: None,
            compounding_simple,
            payment_lag_days,
            end_of_month,
        };
        inner.validate().map_err(core_to_py)?;
        Ok(Self { inner })
    }

    /// Return ``repr(self)``.
    fn __repr__(&self) -> String {
        format!(
            "FixedLegSpec(rate={}, start={}, end={})",
            self.inner.rate, self.inner.start, self.inner.end
        )
    }
}

/// Typed wrapper for the Rust `FloatLegSpec` (floating leg of an IRS/swaption).
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "FloatLegSpec",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyFloatLegSpec {
    /// Inner canonical Rust floating-leg spec.
    pub(crate) inner: finstack_quant_valuations::instruments::FloatLegSpec,
}

#[pymethods]
impl PyFloatLegSpec {
    /// Floating leg of an interest-rate swap.
    ///
    /// Parameters
    /// ----------
    /// discount_curve_id : str
    ///     Discount curve identifier for pricing this leg.
    /// forward_curve_id : str
    ///     Forward curve identifier for rate projections.
    /// spread_bp : float
    ///     Spread over the index in basis points.
    /// frequency : Tenor
    ///     Payment frequency.
    /// day_count : DayCount
    ///     Day count convention for accrual.
    /// start : datetime.date
    ///     Start date of the floating leg.
    /// end : datetime.date
    ///     End date of the floating leg.
    /// bdc : str, default "modified_following"
    ///     Business day convention for payment dates.
    /// calendar_id : str, optional
    ///     Calendar used for business day adjustments.
    /// stub : str, default "ShortFront"
    ///     Stub period handling rule.
    /// reset_lag_days : int, default -1
    ///     Reset lag in business days for the floating rate fixing.
    ///     ``-1`` is a sentinel meaning "use the convention default".
    /// fixing_calendar_id : str, optional
    ///     Calendar used for rate fixing (reset lag).
    /// payment_lag_days : int, default 0
    ///     Payment lag in business days after period end.
    /// end_of_month : bool, default False
    ///     End-of-month roll convention.
    ///
    /// Returns
    /// -------
    /// FloatLegSpec
    ///     The validated floating-leg specification.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If an enum value is invalid or the accrual period is malformed
    ///     (``start >= end``).
    ///
    /// Examples
    /// --------
    /// >>> import datetime
    /// >>> from finstack_quant.core.dates import DayCount, Tenor
    /// >>> from finstack_quant.valuations.instruments import FloatLegSpec
    /// >>> leg = FloatLegSpec(
    /// ...     "USD-OIS", "USD-SOFR-3M", 0.0, Tenor.quarterly(), DayCount.ACT_360,
    /// ...     datetime.date(2024, 1, 15), datetime.date(2029, 1, 15),
    /// ... )
    /// >>> "spread_bp=0" in repr(leg)
    /// True
    #[new]
    #[pyo3(signature = (discount_curve_id, forward_curve_id, spread_bp, frequency, day_count,
                        start, end, *, bdc = "modified_following", calendar_id = None,
                        stub = "ShortFront", reset_lag_days = -1, fixing_calendar_id = None,
                        payment_lag_days = 0, end_of_month = false))]
    #[pyo3(
        text_signature = "(discount_curve_id, forward_curve_id, spread_bp, frequency, \
day_count, start, end, *, bdc='modified_following', calendar_id=None, stub='ShortFront', \
reset_lag_days=-1, fixing_calendar_id=None, payment_lag_days=0, end_of_month=False)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn new(
        discount_curve_id: &str,
        forward_curve_id: &str,
        spread_bp: f64,
        frequency: PyRef<'_, PyTenor>,
        day_count: PyRef<'_, PyDayCount>,
        start: &Bound<'_, PyAny>,
        end: &Bound<'_, PyAny>,
        bdc: &str,
        calendar_id: Option<String>,
        stub: &str,
        reset_lag_days: i32,
        fixing_calendar_id: Option<String>,
        payment_lag_days: i32,
        end_of_month: bool,
    ) -> PyResult<Self> {
        let inner = finstack_quant_valuations::instruments::FloatLegSpec {
            discount_curve_id: finstack_quant_core::types::CurveId::new(
                discount_curve_id.to_string(),
            ),
            forward_curve_id: finstack_quant_core::types::CurveId::new(
                forward_curve_id.to_string(),
            ),
            spread_bp: decimal_from_f64(spread_bp, "spread_bp")?,
            frequency: frequency.inner,
            day_count: day_count.inner,
            bdc: enum_from_str(bdc, "bdc")?,
            calendar_id,
            stub: enum_from_str(stub, "stub")?,
            reset_lag_days,
            fixing_calendar_id,
            start: py_to_date(start)?,
            end: py_to_date(end)?,
            compounding: Default::default(),
            payment_lag_days,
            end_of_month,
        };
        inner.validate().map_err(core_to_py)?;
        Ok(Self { inner })
    }

    /// Return ``repr(self)``.
    fn __repr__(&self) -> String {
        format!(
            "FloatLegSpec(spread_bp={}, start={}, end={})",
            self.inner.spread_bp, self.inner.start, self.inner.end
        )
    }
}

/// Typed wrapper for the Rust `PremiumLegSpec` (CDS/CDS-index premium leg).
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "PremiumLegSpec",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyPremiumLegSpec {
    /// Inner canonical Rust premium-leg spec.
    pub(crate) inner: finstack_quant_valuations::instruments::PremiumLegSpec,
}

#[pymethods]
impl PyPremiumLegSpec {
    /// Premium (fixed coupon) leg of a CDS or CDS index.
    ///
    /// Parameters
    /// ----------
    /// start : datetime.date
    ///     Start date of protection / premium accrual.
    /// end : datetime.date
    ///     End date of protection / premium accrual.
    /// frequency : Tenor
    ///     Payment frequency.
    /// day_count : DayCount
    ///     Day count convention for accrual.
    /// spread_bp : float
    ///     Fixed running spread in basis points (e.g. 100.0 = 100bp = 1%).
    /// discount_curve_id : str
    ///     Discount curve identifier for pricing this leg.
    /// stub : str, default "ShortFront"
    ///     Stub period handling rule.
    /// bdc : str, default "modified_following"
    ///     Business day convention for payment dates.
    /// calendar_id : str, optional
    ///     Calendar used for business day adjustments.
    ///
    /// Returns
    /// -------
    /// PremiumLegSpec
    ///     The premium-leg specification.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If an enum value is invalid.
    ///
    /// Examples
    /// --------
    /// >>> import datetime
    /// >>> from finstack_quant.core.dates import DayCount, Tenor
    /// >>> from finstack_quant.valuations.instruments import PremiumLegSpec
    /// >>> leg = PremiumLegSpec(
    /// ...     datetime.date(2024, 3, 20),
    /// ...     datetime.date(2029, 6, 20),
    /// ...     Tenor.quarterly(),
    /// ...     DayCount.ACT_360,
    /// ...     100.0,
    /// ...     "USD-OIS",
    /// ... )
    /// >>> "spread_bp=100" in repr(leg)
    /// True
    #[new]
    #[pyo3(signature = (start, end, frequency, day_count, spread_bp, discount_curve_id, *,
                        stub = "ShortFront", bdc = "modified_following", calendar_id = None))]
    #[pyo3(
        text_signature = "(start, end, frequency, day_count, spread_bp, discount_curve_id, *, \
stub='ShortFront', bdc='modified_following', calendar_id=None)"
    )]
    #[allow(clippy::too_many_arguments)]
    fn new(
        start: &Bound<'_, PyAny>,
        end: &Bound<'_, PyAny>,
        frequency: PyRef<'_, PyTenor>,
        day_count: PyRef<'_, PyDayCount>,
        spread_bp: f64,
        discount_curve_id: &str,
        stub: &str,
        bdc: &str,
        calendar_id: Option<String>,
    ) -> PyResult<Self> {
        let inner = finstack_quant_valuations::instruments::PremiumLegSpec {
            start: py_to_date(start)?,
            end: py_to_date(end)?,
            frequency: frequency.inner,
            stub: enum_from_str(stub, "stub")?,
            bdc: enum_from_str(bdc, "bdc")?,
            calendar_id,
            day_count: day_count.inner,
            spread_bp: decimal_from_f64(spread_bp, "spread_bp")?,
            discount_curve_id: finstack_quant_core::types::CurveId::new(
                discount_curve_id.to_string(),
            ),
        };
        Ok(Self { inner })
    }

    /// Return ``repr(self)``.
    fn __repr__(&self) -> String {
        format!(
            "PremiumLegSpec(spread_bp={}, start={}, end={})",
            self.inner.spread_bp, self.inner.start, self.inner.end
        )
    }
}

/// Typed wrapper for the Rust `ProtectionLegSpec` (CDS/CDS-index protection leg).
#[pyclass(
    module = "finstack_quant.valuations.instruments",
    name = "ProtectionLegSpec",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyProtectionLegSpec {
    /// Inner canonical Rust protection-leg spec.
    pub(crate) inner: finstack_quant_valuations::instruments::ProtectionLegSpec,
}

#[pymethods]
impl PyProtectionLegSpec {
    /// Protection (default-contingent) leg of a CDS or CDS index.
    ///
    /// Parameters
    /// ----------
    /// credit_curve_id : str
    ///     Hazard/credit curve identifier for default probabilities.
    /// recovery_rate : float
    ///     Recovery rate in ``[0.0, 1.0]`` (e.g. 0.4 = 40%).
    /// settlement_delay : int, default 3
    ///     Settlement delay in business days.
    ///
    /// Returns
    /// -------
    /// ProtectionLegSpec
    ///     The validated protection-leg specification.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If ``recovery_rate`` is outside ``[0.0, 1.0]``.
    ///
    /// Examples
    /// --------
    /// >>> from finstack_quant.valuations.instruments import ProtectionLegSpec
    /// >>> leg = ProtectionLegSpec("ACME-CDS", 0.4, 3)
    /// >>> "recovery_rate=0.4" in repr(leg)
    /// True
    #[new]
    #[pyo3(signature = (credit_curve_id, recovery_rate, settlement_delay = 3))]
    #[pyo3(text_signature = "(credit_curve_id, recovery_rate, settlement_delay=3)")]
    fn new(credit_curve_id: &str, recovery_rate: f64, settlement_delay: u16) -> PyResult<Self> {
        let inner = finstack_quant_valuations::instruments::ProtectionLegSpec::new(
            credit_curve_id.to_string(),
            recovery_rate,
            settlement_delay,
        )
        .map_err(core_to_py)?;
        Ok(Self { inner })
    }

    /// Return ``repr(self)``.
    fn __repr__(&self) -> String {
        format!(
            "ProtectionLegSpec(credit_curve_id={:?}, recovery_rate={})",
            self.inner.credit_curve_id.as_str(),
            self.inner.recovery_rate
        )
    }
}

/// Register the leg-spec classes on the instruments submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyFixedLegSpec>()?;
    m.add_class::<PyFloatLegSpec>()?;
    m.add_class::<PyPremiumLegSpec>()?;
    m.add_class::<PyProtectionLegSpec>()?;
    Ok(())
}
