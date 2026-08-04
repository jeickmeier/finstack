use pyo3::prelude::*;
use pyo3::types::PyType;

use finstack_quant_portfolio::factor_model::{
    FactorAssignmentReport, PositionAssignment, UnmatchedEntry,
};

use super::super::json_bridge::{deserialize_json, serialize_json};

/// Matched factor assignments for a single portfolio position.
///
/// The `mappings` field carries ``(MarketDependency, FactorId, beta)`` triples whose
/// dependency variant tree is wide enough that the binding exposes it as a
/// JSON-serialized vector via :meth:`mappings_json` rather than a fully
/// structured Python type.
#[pyclass(
    name = "PositionAssignment",
    module = "finstack_quant.portfolio",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyPositionAssignment {
    pub(crate) inner: PositionAssignment,
}

impl PyPositionAssignment {
    fn from_inner(inner: PositionAssignment) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyPositionAssignment {
    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        let inner: PositionAssignment = deserialize_json(json_str)?;
        Ok(Self::from_inner(inner))
    }

    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serialize_json(&self.inner)
    }

    #[getter]
    fn position_id(&self) -> String {
        self.inner.position_id.as_str().to_owned()
    }

    /// Number of matched ``(dependency, factor_id, beta)`` triples.
    #[getter]
    fn n_mappings(&self) -> usize {
        self.inner.mappings.len()
    }

    /// Matched ``(dependency, factor_id, beta)`` triples as a JSON string.
    #[pyo3(text_signature = "(self)")]
    fn mappings_json(&self) -> PyResult<String> {
        serialize_json(&self.inner.mappings)
    }

    /// Matched factor identifiers (in mapping order).
    #[getter]
    fn factor_ids(&self) -> Vec<String> {
        self.inner
            .mappings
            .iter()
            .map(|(_, fid, _)| fid.as_str().to_owned())
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "PositionAssignment(position_id={:?}, n_mappings={})",
            self.inner.position_id.as_str(),
            self.inner.mappings.len(),
        )
    }
}

/// Single unmatched dependency surfaced during assignment.
#[pyclass(
    name = "UnmatchedEntry",
    module = "finstack_quant.portfolio",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyUnmatchedEntry {
    pub(crate) inner: UnmatchedEntry,
}

impl PyUnmatchedEntry {
    fn from_inner(inner: UnmatchedEntry) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyUnmatchedEntry {
    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        let inner: UnmatchedEntry = deserialize_json(json_str)?;
        Ok(Self::from_inner(inner))
    }

    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serialize_json(&self.inner)
    }

    #[getter]
    fn position_id(&self) -> String {
        self.inner.position_id.as_str().to_owned()
    }

    /// Unmatched dependency as a JSON string.
    #[pyo3(text_signature = "(self)")]
    fn dependency_json(&self) -> PyResult<String> {
        serialize_json(&self.inner.dependency)
    }

    fn __repr__(&self) -> String {
        format!(
            "UnmatchedEntry(position_id={:?})",
            self.inner.position_id.as_str(),
        )
    }
}

/// Assignment results for a portfolio-level factor mapping pass.
#[pyclass(
    name = "FactorAssignmentReport",
    module = "finstack_quant.portfolio",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(super) struct PyFactorAssignmentReport {
    pub(crate) inner: FactorAssignmentReport,
}

impl PyFactorAssignmentReport {
    pub(crate) fn from_inner(inner: FactorAssignmentReport) -> Self {
        Self { inner }
    }
}

#[pymethods]
impl PyFactorAssignmentReport {
    #[classmethod]
    #[pyo3(text_signature = "(cls, json_str)")]
    fn from_json(_cls: &Bound<'_, PyType>, json_str: &str) -> PyResult<Self> {
        let inner: FactorAssignmentReport = deserialize_json(json_str)?;
        Ok(Self::from_inner(inner))
    }

    #[pyo3(text_signature = "(self)")]
    fn to_json(&self) -> PyResult<String> {
        serialize_json(&self.inner)
    }

    #[getter]
    fn assignments(&self) -> Vec<PyPositionAssignment> {
        self.inner
            .assignments
            .iter()
            .cloned()
            .map(PyPositionAssignment::from_inner)
            .collect()
    }

    #[getter]
    fn unmatched(&self) -> Vec<PyUnmatchedEntry> {
        self.inner
            .unmatched
            .iter()
            .cloned()
            .map(PyUnmatchedEntry::from_inner)
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "FactorAssignmentReport(assignments={}, unmatched={})",
            self.inner.assignments.len(),
            self.inner.unmatched.len(),
        )
    }
}
