//! Python wrappers for strict portfolio materialization.

use std::borrow::Cow;
use std::sync::Arc;

use finstack_quant_core::contract::LoadLimits;
use finstack_quant_portfolio::{
    InstrumentArtifactCache, MaterializationReport, Portfolio as RustPortfolio,
};
use pyo3::exceptions::PyTypeError;
use pyo3::prelude::*;
use pyo3::types::{PyByteArray, PyBytes, PyDict, PyModule, PyString};

use super::types::PyPortfolio;
use crate::errors::{diagnostics_to_py, materialization_to_py};

/// Reusable, bounded cache of decoded instrument artifacts.
#[pyclass(
    name = "InstrumentArtifactCache",
    module = "finstack_quant.portfolio",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyInstrumentArtifactCache {
    inner: Arc<InstrumentArtifactCache>,
}

#[pymethods]
impl PyInstrumentArtifactCache {
    /// Create an empty cache with an explicit entry capacity.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum retained artifacts. Defaults to 4,096.
    ///
    /// # Returns
    ///
    /// A reusable cache with the requested entry bound and a 64 MiB encoded
    /// source-data bound.
    #[new]
    #[pyo3(signature = (capacity = 4096))]
    fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(InstrumentArtifactCache::with_capacity(capacity)),
        }
    }

    /// Number of retained decoded artifacts.
    #[getter]
    fn size(&self) -> usize {
        self.inner.len()
    }

    /// Cumulative number of successful cache-miss decodes.
    #[getter]
    fn decode_count(&self) -> usize {
        self.inner.decode_count()
    }

    /// Return the number of retained decoded artifacts.
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "InstrumentArtifactCache(size={}, decode_count={})",
            self.inner.len(),
            self.inner.decode_count()
        )
    }
}

/// Immutable outcome metadata for one successful materialization.
#[pyclass(
    name = "MaterializationReport",
    module = "finstack_quant.portfolio",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub struct PyMaterializationReport {
    inner: Arc<MaterializationReport>,
}

impl PyMaterializationReport {
    fn from_inner(inner: MaterializationReport) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }
}

#[pymethods]
impl PyMaterializationReport {
    /// Structured non-fatal diagnostics retained by the loader.
    #[getter]
    fn diagnostics(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        diagnostics_to_py(py, &self.inner.report)
    }

    /// Whether additional diagnostics were omitted by the configured bound.
    #[getter]
    fn truncated(&self) -> bool {
        self.inner.report.truncated
    }

    /// Number of unique instrument artifacts in the source bundle.
    #[getter]
    fn unique_instruments(&self) -> usize {
        self.inner.unique_instruments
    }

    /// Number of ordered positions materialized.
    #[getter]
    fn positions(&self) -> usize {
        self.inner.positions
    }

    /// Sum of normalized market-dependency keys over unique artifacts.
    #[getter]
    fn dependencies(&self) -> usize {
        self.inner.dependencies
    }

    /// Number of unique artifacts served from the supplied cache.
    #[getter]
    fn cache_hits(&self) -> usize {
        self.inner.cache_hits
    }

    /// Number of encoded bytes in the source bundle.
    #[getter]
    fn input_bytes(&self) -> usize {
        self.inner.input_bytes
    }

    /// Whether every phase used an available monotonic host clock.
    #[getter]
    fn timing_available(&self) -> bool {
        self.inner.timing_available
    }

    /// Sequential phase timings in nanoseconds.
    #[getter]
    fn phase_nanos(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let phases = PyDict::new(py);
        phases.set_item("parse", self.inner.phase_nanos.parse)?;
        phases.set_item(
            "validate_versions",
            self.inner.phase_nanos.validate_versions,
        )?;
        phases.set_item(
            "decode_instruments",
            self.inner.phase_nanos.decode_instruments,
        )?;
        phases.set_item("build_positions", self.inner.phase_nanos.build_positions)?;
        phases.set_item("index_build", self.inner.phase_nanos.index_build)?;
        Ok(phases.into_any().unbind())
    }

    fn __repr__(&self) -> String {
        format!(
            "MaterializationReport(unique_instruments={}, positions={}, dependencies={}, cache_hits={}, input_bytes={})",
            self.inner.unique_instruments,
            self.inner.positions,
            self.inner.dependencies,
            self.inner.cache_hits,
            self.inner.input_bytes
        )
    }
}

#[pymethods]
impl PyPortfolio {
    /// Build a reusable portfolio handle from one strict materialization bundle.
    ///
    /// Parsing, validation, instrument decoding, and portfolio construction run
    /// in one native call while the Python GIL is released.
    ///
    /// # Arguments
    ///
    /// * `bundle` - Complete UTF-8 materialization JSON supplied as ``str``,
    ///   ``bytes``, or ``bytearray``.
    /// * `cache` - Optional reusable artifact cache. When omitted, an ephemeral
    ///   bounded cache is used for this call.
    ///
    /// # Returns
    ///
    /// A frozen portfolio handle and immutable materialization report.
    ///
    /// # Errors
    ///
    /// Returns ``TypeError`` for unsupported Python input types,
    /// ``ContractValidationError`` (or a typed subclass) for malformed or
    /// invalid persisted contracts, and the documented portfolio exceptions
    /// for other native construction failures.
    #[staticmethod]
    #[pyo3(signature = (bundle, cache = None))]
    fn from_materialization(
        py: Python<'_>,
        bundle: &Bound<'_, PyAny>,
        cache: Option<&PyInstrumentArtifactCache>,
    ) -> PyResult<(PyPortfolio, PyMaterializationReport)> {
        let bytes = extract_bundle_bytes(bundle)?;
        let cache = cache
            .map(|value| Arc::clone(&value.inner))
            .unwrap_or_else(|| Arc::new(InstrumentArtifactCache::new()));
        let limits = LoadLimits::default();
        let (portfolio, report) = py
            .detach(move || RustPortfolio::from_materialization(&bytes, &cache, &limits))
            .map_err(|error| materialization_to_py(py, error))?;
        Ok((
            PyPortfolio::from_inner(portfolio),
            PyMaterializationReport::from_inner(report),
        ))
    }
}

fn extract_bundle_bytes(bundle: &Bound<'_, PyAny>) -> PyResult<Vec<u8>> {
    if bundle.cast::<PyString>().is_ok() {
        return bundle.extract::<String>().map(|value| value.into_bytes());
    }
    if bundle.cast::<PyBytes>().is_ok() || bundle.cast::<PyByteArray>().is_ok() {
        let bytes: Cow<'_, [u8]> = bundle.extract()?;
        return Ok(bytes.into_owned());
    }
    Err(PyTypeError::new_err(
        "bundle must be bytes, bytearray, or str",
    ))
}

/// Register portfolio materialization classes.
pub fn register(_py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyInstrumentArtifactCache>()?;
    module.add_class::<PyMaterializationReport>()?;
    Ok(())
}
