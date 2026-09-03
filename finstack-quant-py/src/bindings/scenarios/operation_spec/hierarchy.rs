//! `HierarchyTarget` wrapper for hierarchy-targeted scenario operations.

use finstack_quant_core::market_data::hierarchy::{HierarchyTarget, TagFilter, TagPredicate};
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Path into the market-data hierarchy, with an optional tag filter, that a
/// hierarchy-targeted operation resolves against the execution context's
/// ``MarketDataHierarchy``.
///
/// Parameters
/// ----------
/// path : list[str]
///     Hierarchy path from the root, e.g. ``["Credit", "US", "IG"]``. Every
///     curve in that subtree is targeted.
/// tag_filter : dict[str, str] | None, default None
///     Optional ``{key: value}`` equality predicates (AND semantics) that a
///     node must satisfy for its subtree to be included. Use ``from_json``
///     for ``in`` / ``exists`` predicates.
///
/// Examples
/// --------
/// >>> from finstack_quant.scenarios import HierarchyTarget
/// >>> target = HierarchyTarget(["Credit", "US"], {"sector": "financials"})
/// >>> target.path
/// ['Credit', 'US']
/// >>> HierarchyTarget.from_json(target.to_json()) == target
/// True
#[pyclass(
    name = "HierarchyTarget",
    module = "finstack_quant.scenarios",
    eq,
    frozen,
    from_py_object
)]
#[derive(Clone, PartialEq)]
pub struct PyHierarchyTarget {
    pub(crate) inner: HierarchyTarget,
}

#[pymethods]
impl PyHierarchyTarget {
    #[new]
    #[pyo3(signature = (path, tag_filter=None))]
    fn new(path: Vec<String>, tag_filter: Option<&Bound<'_, PyDict>>) -> PyResult<Self> {
        let tag_filter = match tag_filter {
            None => None,
            Some(dict) => {
                let predicates = dict
                    .iter()
                    .map(|(k, v)| {
                        Ok(TagPredicate::Equals {
                            key: k.extract::<String>()?,
                            value: v.extract::<String>()?,
                        })
                    })
                    .collect::<PyResult<Vec<_>>>()?;
                Some(TagFilter { predicates })
            }
        };
        Ok(Self {
            inner: HierarchyTarget { path, tag_filter },
        })
    }

    /// Hierarchy path from the root.
    #[getter]
    fn path(&self) -> Vec<String> {
        self.inner.path.clone()
    }

    /// Tag filter as ``{key: value}`` for equality predicates; ``None`` when
    /// no filter is set. Non-equality predicates are visible via ``to_json``.
    #[getter]
    fn tag_filter(&self) -> Option<Vec<(String, String)>> {
        self.inner.tag_filter.as_ref().map(|filter| {
            filter
                .predicates
                .iter()
                .filter_map(|predicate| match predicate {
                    TagPredicate::Equals { key, value } => Some((key.clone(), value.clone())),
                    _ => None,
                })
                .collect()
        })
    }

    /// Serialize to canonical JSON (``{"path": [...], "tag_filter": {...}}``).
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.inner).map_err(crate::errors::display_to_py)
    }

    /// Deserialize from canonical JSON, including ``in`` / ``exists`` tag
    /// predicates that the constructor does not express.
    ///
    /// Raises
    /// ------
    /// ValueError
    ///     If the JSON does not match the ``HierarchyTarget`` contract.
    #[staticmethod]
    #[pyo3(text_signature = "(json)")]
    fn from_json(json: &str) -> PyResult<Self> {
        let inner: HierarchyTarget = serde_json::from_str(json).map_err(|e| {
            crate::errors::value_error(format!("Invalid HierarchyTarget JSON: {e}"))
        })?;
        Ok(Self { inner })
    }

    fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<(Bound<'py, PyAny>, (String,))> {
        let from_json = py.get_type::<Self>().getattr("from_json")?;
        crate::bindings::pickle_support::reduce_via_json(from_json, self.to_json()?)
    }

    fn __repr__(&self) -> String {
        format!(
            "HierarchyTarget(path={:?}, tag_filter={})",
            self.inner.path,
            match &self.inner.tag_filter {
                None => "None".to_string(),
                Some(filter) => format!("<{} predicates>", filter.predicates.len()),
            }
        )
    }
}
