//! Typed credit instruments: `CreditDefaultSwap`, `CDSIndex`, `CDSTranche`
//! and `ConvertibleBond`, plus their small typed helper classes
//! (`CDSIndexParams`, `CDSIndexConstituent`, `CDSTrancheParams`,
//! `CallPutSchedule`, `ConversionSpec`).
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
pub fn register(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyCreditDefaultSwap>()?;
    m.add_class::<PyCreditDefaultSwapBuilder>()?;
    m.add_class::<PyCDSIndex>()?;
    m.add_class::<PyCDSIndexBuilder>()?;
    m.add_class::<PyCDSTranche>()?;
    m.add_class::<PyCDSTrancheBuilder>()?;
    m.add_class::<PyConvertibleBond>()?;
    m.add_class::<PyConvertibleBondBuilder>()?;
    cds_index::register(py, m)?;
    cds_tranche::register(py, m)?;
    convertible::register(py, m)?;
    Ok(())
}

/// Names this module contributes to `finstack_quant.valuations.instruments.__all__`.
///
/// Extend this list (sorted) when adding a class or function here; `mod.rs`
/// merges every submodule list so registration stays in one place per file.
/// The four instrument classes and their builders are already listed by the
/// parent; only the helper classes are new names.
pub(crate) const EXPORTS: &[&str] = &[
    "CDSIndexConstituent",
    "CDSIndexParams",
    "CDSTrancheParams",
    "CallPutSchedule",
    "ConversionSpec",
];
