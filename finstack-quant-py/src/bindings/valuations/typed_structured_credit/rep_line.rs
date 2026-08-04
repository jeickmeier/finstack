use pyo3::prelude::*;

use crate::bindings::core::dates::daycount::PyDayCount;
use crate::bindings::core::dates::utils::py_to_date;
use crate::bindings::core::money::PyMoney;
use finstack_quant_valuations::instruments::fixed_income::structured_credit::RepLine;

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
