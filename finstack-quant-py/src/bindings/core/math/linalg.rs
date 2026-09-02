//! Python bindings for `finstack_quant_core::math::linalg`.

use finstack_quant_core::math::linalg;
use numpy::{PyReadonlyArray2, PyUntypedArrayMethods};
use pyo3::prelude::*;
use pyo3::types::{PyList, PyModule};

pyo3::create_exception!(
    finstack_quant.core.math.linalg,
    CholeskyError,
    crate::errors::FinstackError,
    "Cholesky decomposition failure (inherits FinstackError, ValueError)."
);

/// Map a core linear-algebra error to a Python `CholeskyError` exception.
fn cholesky_err(e: impl std::fmt::Display) -> PyErr {
    CholeskyError::new_err(e.to_string())
}

/// Flatten a `list[list[float]]` into a row-major `Vec<f64>` and return `(flat, n)`.
///
/// Returns a `PyResult::Err` when the input is not a square matrix.
fn flatten_matrix(rows: Vec<Vec<f64>>) -> PyResult<(Vec<f64>, usize)> {
    let n = rows.len();
    for (i, row) in rows.iter().enumerate() {
        if row.len() != n {
            return Err(crate::errors::value_error(format!(
                "Row {i} has length {} but expected {n} for a square matrix",
                row.len()
            )));
        }
    }
    let flat: Vec<f64> = rows.into_iter().flatten().collect();
    Ok((flat, n))
}

/// Extract an n×n matrix from a C-contiguous NumPy array or a nested list.
///
/// NumPy input is copied from the backing buffer in bulk instead of
/// per-element extraction; nested lists take the canonical
/// `list[list[float]]` path.
fn extract_square_matrix(matrix: &Bound<'_, PyAny>) -> PyResult<(Vec<f64>, usize)> {
    if let Ok(array) = matrix.extract::<PyReadonlyArray2<'_, f64>>() {
        let shape = array.shape();
        let (rows, cols) = (shape[0], shape[1]);
        if rows != cols {
            return Err(crate::errors::value_error(format!(
                "Matrix must be square, got {rows}x{cols}"
            )));
        }
        let flat = match array.as_slice() {
            Ok(slice) => slice.to_vec(),
            // Strided views cannot borrow as a contiguous slice; fall back to
            // logical-order iteration.
            Err(_) => array.as_array().iter().copied().collect(),
        };
        return Ok((flat, rows));
    }
    flatten_matrix(matrix.extract()?)
}

/// Compute the Cholesky decomposition L of a symmetric positive-definite matrix
/// such that A = L L^T.
///
/// Accepts a square matrix as a ``numpy.ndarray`` (``float64``) or
/// ``list[list[float]]`` and returns the lower-triangular factor in the same
/// shape.
///
/// Raises ``CholeskyError`` when the matrix is not positive-definite, is singular,
/// or has mismatched dimensions.
#[pyfunction]
#[pyo3(text_signature = "(matrix)")]
fn cholesky_decomposition(py: Python<'_>, matrix: &Bound<'_, PyAny>) -> PyResult<Vec<Vec<f64>>> {
    let (flat, n) = extract_square_matrix(matrix)?;
    let result = py
        .detach(|| linalg::cholesky_decomposition(&flat, n))
        .map_err(cholesky_err)?;
    Ok(linalg::unflatten_square(&result, n))
}

/// Solve a symmetric positive-definite linear system A x = b given the Cholesky
/// factor L of A (where A = L L^T).
///
/// Accepts L as a ``numpy.ndarray`` (``float64``) or ``list[list[float]]`` and b
/// as ``list[float]``. Returns x as ``list[float]``.
///
/// Raises ``CholeskyError`` on dimension mismatch or singular factor.
#[pyfunction]
#[pyo3(text_signature = "(chol, b)")]
fn cholesky_solve(py: Python<'_>, chol: &Bound<'_, PyAny>, b: Vec<f64>) -> PyResult<Vec<f64>> {
    let (flat, n) = extract_square_matrix(chol)?;
    if b.len() != n {
        return Err(crate::errors::value_error(format!(
            "Right-hand side has length {} but Cholesky factor is {n}x{n}",
            b.len()
        )));
    }
    let mut x = vec![0.0; n];
    py.detach(|| linalg::cholesky_solve(&flat, &b, &mut x))
        .map_err(cholesky_err)?;
    Ok(x)
}

/// Apply a lower-triangular factor L to a vector z, returning L z.
///
/// This is the Cholesky "apply" step that turns independent standard normals into
/// correlated normals: if A = L L^T and z ~ N(0, I), then L z ~ N(0, A).
///
/// Accepts L as a ``numpy.ndarray`` (``float64``) or ``list[list[float]]``
/// (only the lower triangle is read; the upper triangle is assumed zero) and z
/// as ``list[float]``. Returns L z as ``list[float]``.
///
/// Raises ``ValueError`` when L is not square, and ``CholeskyError`` when z's length
/// does not match L's dimension.
#[pyfunction]
#[pyo3(text_signature = "(l, z)")]
fn apply_lower_triangular(py: Python<'_>, l: &Bound<'_, PyAny>, z: Vec<f64>) -> PyResult<Vec<f64>> {
    let (flat, n) = extract_square_matrix(l)?;
    py.detach(|| linalg::apply_lower_triangular(&flat, n, &z))
        .map_err(cholesky_err)
}

/// Build the `finstack_quant.core.math.linalg` submodule.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "linalg")?;
    m.setattr(
        "__doc__",
        "Linear algebra utilities: Cholesky decomposition and triangular solves.",
    )?;

    m.add_function(wrap_pyfunction!(apply_lower_triangular, &m)?)?;
    m.add_function(wrap_pyfunction!(cholesky_decomposition, &m)?)?;
    m.add_function(wrap_pyfunction!(cholesky_solve, &m)?)?;

    m.add("CholeskyError", py.get_type::<CholeskyError>())?;

    m.add("SINGULAR_THRESHOLD", linalg::SINGULAR_THRESHOLD)?;
    m.add("DIAGONAL_TOLERANCE", linalg::DIAGONAL_TOLERANCE)?;
    m.add("SYMMETRY_TOLERANCE", linalg::SYMMETRY_TOLERANCE)?;

    let all = PyList::new(
        py,
        [
            "CholeskyError",
            "DIAGONAL_TOLERANCE",
            "SINGULAR_THRESHOLD",
            "SYMMETRY_TOLERANCE",
            "apply_lower_triangular",
            "cholesky_decomposition",
            "cholesky_solve",
        ],
    )?;
    m.setattr("__all__", all)?;
    crate::bindings::module_utils::register_submodule(
        py,
        parent,
        &m,
        "linalg",
        "finstack_quant.core.math",
        crate::bindings::module_utils::ParentNameSource::Package,
    )?;

    Ok(())
}
