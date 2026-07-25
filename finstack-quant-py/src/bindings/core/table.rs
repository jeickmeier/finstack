//! Bindings for `finstack_quant_core::table`: Arrow interchange surface.
//!
//! `ArrowTable` wraps an Arrow `RecordBatch` built from a
//! `TableEnvelope` and implements the Arrow PyCapsule C-stream protocol
//! (`__arrow_c_stream__`), so `pyarrow.table(obj)`, `polars.DataFrame(obj)`,
//! DuckDB, and pandas can consume finstack tabular results zero-copy with no
//! extra Python dependencies.
//!
//! Spec: <https://arrow.apache.org/docs/format/CDataInterface/PyCapsuleInterface.html>

use arrow::ffi_stream::FFI_ArrowArrayStream;
use arrow::record_batch::{RecordBatch, RecordBatchIterator};
use finstack_quant_core::table::TableEnvelope;
use pyo3::prelude::*;
use pyo3::types::{PyCapsule, PyList};

/// Arrow `RecordBatch` wrapper exposing the Arrow PyCapsule C-stream protocol.
#[pyclass(module = "finstack_quant.core.table", name = "ArrowTable", frozen)]
pub struct PyArrowTable {
    pub(crate) batch: RecordBatch,
}

impl PyArrowTable {
    /// Convert a core table envelope into an Arrow-backed Python table.
    pub(crate) fn from_envelope(table: &TableEnvelope) -> PyResult<Self> {
        let batch =
            finstack_quant_arrow::to_record_batch(table).map_err(crate::errors::core_to_py)?;
        Ok(Self { batch })
    }
}

#[pymethods]
impl PyArrowTable {
    /// Number of rows in the table.
    #[getter]
    fn num_rows(&self) -> usize {
        self.batch.num_rows()
    }

    /// Number of columns in the table.
    #[getter]
    fn num_columns(&self) -> usize {
        self.batch.num_columns()
    }

    /// Column names in declaration order.
    #[pyo3(text_signature = "($self)")]
    fn column_names(&self) -> Vec<String> {
        self.batch
            .schema()
            .fields()
            .iter()
            .map(|f| f.name().clone())
            .collect()
    }

    /// Export the table via the Arrow PyCapsule C-stream protocol.
    ///
    /// Returns a ``PyCapsule`` named ``arrow_array_stream`` containing an
    /// ``ArrowArrayStream`` with a single record batch. Each call produces a
    /// fresh stream (the underlying buffers are shared, not copied), so the
    /// table can be consumed by multiple consumers. ``requested_schema`` is
    /// accepted per the protocol and ignored: the native schema is always
    /// exported. See the PyCapsule Interface spec for the full protocol:
    /// <https://arrow.apache.org/docs/format/CDataInterface/PyCapsuleInterface.html>
    #[pyo3(signature = (requested_schema=None))]
    fn __arrow_c_stream__<'py>(
        &self,
        py: Python<'py>,
        requested_schema: Option<Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyCapsule>> {
        let _ = requested_schema; // schema negotiation unsupported; export native schema
        let schema = self.batch.schema();
        let reader = RecordBatchIterator::new([Ok(self.batch.clone())], schema);
        let stream = FFI_ArrowArrayStream::new(Box::new(reader));
        PyCapsule::new_with_value(py, stream, c"arrow_array_stream")
    }

    /// Return a debug-style representation.
    fn __repr__(&self) -> String {
        format!(
            "ArrowTable(rows={}, columns={})",
            self.batch.num_rows(),
            self.batch.num_columns()
        )
    }
}

/// Build the `finstack_quant.core.table` submodule.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "table")?;
    m.setattr(
        "__doc__",
        "Arrow interchange surface for finstack-quant tabular results.",
    )?;
    m.add_class::<PyArrowTable>()?;
    let all = PyList::new(py, ["ArrowTable"])?;
    m.setattr("__all__", all)?;
    crate::bindings::module_utils::register_submodule(
        py,
        parent,
        &m,
        "table",
        "finstack_quant.core",
        crate::bindings::module_utils::ParentNameSource::Package,
    )?;
    Ok(())
}
