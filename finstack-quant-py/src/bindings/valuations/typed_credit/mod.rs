//! Typed credit instruments: `CreditDefaultSwap`, `CDSIndex`, `CDSTranche`
//! and `ConvertibleBond`.
//! Mirrors the `PyBond` pattern in `instruments.rs`.

mod cds;
mod cds_index;
mod cds_tranche;
mod convertible;

use pyo3::prelude::*;

pub(crate) use cds::PyCreditDefaultSwap;
use cds::PyCreditDefaultSwapBuilder;
pub(crate) use cds_index::PyCDSIndex;
use cds_index::PyCDSIndexBuilder;
pub(crate) use cds_tranche::PyCDSTranche;
use cds_tranche::PyCDSTrancheBuilder;
pub(crate) use convertible::PyConvertibleBond;
use convertible::PyConvertibleBondBuilder;

/// Register the typed credit-derivative instruments on the instruments
/// submodule.
pub fn register(_py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyCreditDefaultSwap>()?;
    m.add_class::<PyCreditDefaultSwapBuilder>()?;
    m.add_class::<PyCDSIndex>()?;
    m.add_class::<PyCDSIndexBuilder>()?;
    m.add_class::<PyCDSTranche>()?;
    m.add_class::<PyCDSTrancheBuilder>()?;
    m.add_class::<PyConvertibleBond>()?;
    m.add_class::<PyConvertibleBondBuilder>()?;
    Ok(())
}
