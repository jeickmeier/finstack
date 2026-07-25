//! Python bindings for `finstack_quant_cashflows::aggregation`.

use finstack_quant_cashflows::aggregation::{aggregate_by_period, aggregate_cashflows_checked};
use finstack_quant_cashflows::DatedFlow;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyModule};

use crate::bindings::core::currency::extract_currency;
use crate::bindings::core::dates::periods::PyPeriod;
use crate::bindings::core::dates::utils::py_to_date;
use crate::bindings::core::money::PyMoney;
use crate::errors::core_to_py;

/// Convert Python ``[(date, Money), ...]`` into Rust dated flows.
fn extract_dated_flows(flows: Vec<(Bound<'_, PyAny>, PyMoney)>) -> PyResult<Vec<DatedFlow>> {
    flows
        .iter()
        .map(|(d, m)| Ok((py_to_date(d)?, m.inner)))
        .collect()
}

/// Aggregate dated flows into reporting periods (half-open ``[start, end)``).
///
/// Parameters
/// ----------
/// flows : list[tuple[datetime.date, Money]]
///     Dated cashflows; unsorted input is sorted internally.
/// periods : list[Period]
///     Sorted, disjoint reporting periods.
///
/// Returns
/// -------
/// dict[str, dict[str, Money]]
///     ``{period_id_label: {currency_code: nominal_sum}}``; empty periods
///     are omitted from the result.
///
/// Raises
/// ------
/// ValueError
///     If periods are unsorted, overlapping, or contain duplicate ids, or
///     if a per-currency total is non-finite or exceeds the Decimal range.
///
/// Examples
/// --------
/// >>> import datetime
/// >>> from finstack_quant.cashflows.aggregation import aggregate_by_period
/// >>> from finstack_quant.core.dates import build_periods
/// >>> from finstack_quant.core.money import Money
/// >>> flows = [(datetime.date(2025, 3, 15), Money(100.0, "USD"))]
/// >>> periods = build_periods("2025Q1..Q4").periods
/// >>> aggregate_by_period(flows, periods)["2025Q1"]["USD"].amount
/// 100.0
#[pyfunction(name = "aggregate_by_period")]
#[pyo3(text_signature = "(flows, periods)")]
fn py_aggregate_by_period<'py>(
    py: Python<'py>,
    flows: Vec<(Bound<'py, PyAny>, PyMoney)>,
    periods: Vec<PyRef<'py, PyPeriod>>,
) -> PyResult<Bound<'py, PyDict>> {
    let flows = extract_dated_flows(flows)?;
    let periods: Vec<finstack_quant_core::dates::Period> =
        periods.iter().map(|p| p.inner.clone()).collect();
    let aggregated = py
        .detach(move || aggregate_by_period(&flows, &periods))
        .map_err(core_to_py)?;
    let out = PyDict::new(py);
    for (period_id, per_currency) in aggregated {
        let inner = PyDict::new(py);
        for (currency, amount) in per_currency {
            inner.set_item(currency.to_string(), PyMoney::from_inner(amount))?;
        }
        out.set_item(period_id.to_string(), inner)?;
    }
    Ok(out)
}

/// Currency-checked single-currency aggregation with an explicit target currency.
///
/// Parameters
/// ----------
/// flows : list[tuple[datetime.date, Money]]
///     Dated cashflows; every flow must be in ``target`` currency.
/// target : Currency or str
///     Required currency for every flow and the returned total.
///
/// Returns
/// -------
/// Money
///     Single total in ``target`` currency. Empty ``flows`` returns a zero
///     total.
///
/// Raises
/// ------
/// ValueError
///     If any flow currency differs from ``target`` (currency mismatch is
///     rejected, never silently converted).
///
/// Examples
/// --------
/// >>> import datetime
/// >>> from finstack_quant.cashflows.aggregation import aggregate_cashflows_checked
/// >>> from finstack_quant.core.money import Money
/// >>> flows = [(datetime.date(2025, 1, 15), Money(50_000.0, "USD"))]
/// >>> aggregate_cashflows_checked(flows, "USD").amount
/// 50000.0
#[pyfunction(name = "aggregate_cashflows_checked")]
#[pyo3(text_signature = "(flows, target)")]
fn py_aggregate_cashflows_checked(
    py: Python<'_>,
    flows: Vec<(Bound<'_, PyAny>, PyMoney)>,
    target: &Bound<'_, PyAny>,
) -> PyResult<PyMoney> {
    let flows = extract_dated_flows(flows)?;
    let target = extract_currency(target)?;
    py.detach(move || aggregate_cashflows_checked(&flows, target))
        .map(PyMoney::from_inner)
        .map_err(core_to_py)
}

/// Register the `finstack_quant.cashflows.aggregation` submodule.
pub(crate) fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let module = PyModule::new(py, "aggregation")?;
    module.setattr(
        "__doc__",
        "Currency-preserving aggregation of dated cashflows into periods and totals.",
    )?;
    module.add_function(wrap_pyfunction!(py_aggregate_by_period, &module)?)?;
    module.add_function(wrap_pyfunction!(py_aggregate_cashflows_checked, &module)?)?;

    let all = PyList::new(py, ["aggregate_by_period", "aggregate_cashflows_checked"])?;
    module.setattr("__all__", all)?;

    crate::bindings::module_utils::register_submodule(
        py,
        parent,
        &module,
        "aggregation",
        "finstack_quant.cashflows",
        crate::bindings::module_utils::ParentNameSource::Package,
    )?;
    Ok(())
}
