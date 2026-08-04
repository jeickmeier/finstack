use pyo3::prelude::*;

use crate::bindings::core::currency::PyCurrency;
use finstack_quant_valuations::instruments::fixed_income::structured_credit::{
    AssetPool, DealType, PoolAsset,
};

use super::super::instruments::{enum_from_str, json_field};
use super::PyRepLine;

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
