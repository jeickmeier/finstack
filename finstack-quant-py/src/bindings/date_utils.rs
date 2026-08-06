//! Date parsing helpers shared by Python bindings.

use pyo3::prelude::*;

use crate::errors::display_to_py;

/// Parse an ISO 8601 date string into a `time::Date`.
pub(crate) fn parse_iso_date_py(s: &str) -> PyResult<time::Date> {
    finstack_quant_core::dates::parse_iso_date(s).map_err(display_to_py)
}

/// Accept either an ISO 8601 string or a Python date-like object.
///
/// Date-valued parameters split into two historically disjoint groups: typed
/// constructors take `datetime.date` (via
/// [`py_to_date`](crate::bindings::core::dates::utils::py_to_date)) while
/// `as_of` parameters took only strings. Quants hit that seam constantly,
/// because the object they already hold is whichever one the *other* group
/// wanted. This accepts both.
///
/// String parsing is tried only after the attribute probe fails, so a
/// `datetime.date`, `datetime.datetime` or `pandas.Timestamp` never pays for a
/// failed parse.
///
/// # Errors
///
/// Returns `TypeError` when `obj` is neither a string nor date-like, and
/// `ValueError` when a string is not valid ISO 8601.
pub(crate) fn extract_date(obj: &Bound<'_, PyAny>) -> PyResult<time::Date> {
    if let Ok(s) = obj.extract::<std::borrow::Cow<'_, str>>() {
        return parse_iso_date_py(&s);
    }
    crate::bindings::core::dates::utils::py_to_date(obj)
}

/// Like [`extract_date`], but yields the ISO 8601 string the canonical crate
/// entry points take.
///
/// A string argument is validated and passed through unchanged, so the only
/// formatting happens on the date-object path. The output is built from the
/// calendar parts rather than `time`'s `Iso8601` formatter, because the `time`
/// dependency declares no features and its formatting support is therefore not
/// guaranteed to be enabled.
pub(crate) fn extract_date_iso(obj: &Bound<'_, PyAny>) -> PyResult<String> {
    if let Ok(s) = obj.extract::<std::borrow::Cow<'_, str>>() {
        parse_iso_date_py(&s)?;
        return Ok(s.into_owned());
    }
    let date = crate::bindings::core::dates::utils::py_to_date(obj)?;
    Ok(format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        date.month() as u8,
        date.day()
    ))
}
