use pyo3::prelude::*;

use crate::errors::core_to_py;
use finstack_quant_valuations::instruments::fixed_income::structured_credit::{
    Tranche, TrancheStructure,
};

use super::PyTranche;

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
