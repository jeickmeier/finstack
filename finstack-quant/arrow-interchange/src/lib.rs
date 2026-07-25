//! Apache Arrow interchange for finstack-quant tabular outputs.
//!
//! Converts [`finstack_quant_core::table::TableEnvelope`] to and from Arrow
//! [`RecordBatch`](arrow::record_batch::RecordBatch) values and Arrow IPC
//! stream bytes. Column roles and metadata round-trip losslessly through
//! Arrow field/schema metadata (keys `finstack:role` and `finstack:metadata`).
//!
//! This is a supporting crate: it is not re-exported by the `finstack-quant`
//! umbrella crate and has no WASM binding (arrow-rs is not built for wasm32).
//!
//! # Quick Example
//! ```rust
//! use finstack_quant_core::table::{TableColumn, TableColumnData, TableEnvelope};
//! use finstack_quant_arrow::{from_record_batch, to_record_batch};
//!
//! let table = TableEnvelope::new(vec![TableColumn::new(
//!     "pv",
//!     TableColumnData::Float64(vec![101.5, 99.25]),
//! )])
//! .unwrap();
//! let batch = to_record_batch(&table).unwrap();
//! assert_eq!(from_record_batch(&batch).unwrap(), table);
//! ```

#![forbid(unsafe_code)]
#![warn(clippy::float_cmp)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unreachable)]
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::float_cmp,
    )
)]
#![doc(test(attr(allow(clippy::unwrap_used))))]

use arrow::record_batch::RecordBatch;
use finstack_quant_core::table::TableEnvelope;
use finstack_quant_core::{Error, Result};

/// Convert a [`TableEnvelope`] into an Arrow [`RecordBatch`].
///
/// Column order, names, and roles are preserved: each Arrow field carries the
/// source column's role (if any) and per-column metadata as field metadata
/// (`finstack:role`, `finstack:metadata`), and table-level metadata is carried
/// as schema metadata (`finstack:metadata`).
///
/// # Errors
///
/// Returns [`Error::Internal`] for any table this crate cannot yet convert.
/// The full column-type mapping lands in a follow-up task; today every call
/// fails.
///
/// # Examples
///
/// ```should_panic
/// use finstack_quant_core::table::{TableColumn, TableColumnData, TableEnvelope};
/// use finstack_quant_arrow::to_record_batch;
///
/// let table = TableEnvelope::new(vec![TableColumn::new(
///     "pv",
///     TableColumnData::Float64(vec![101.5, 99.25]),
/// )])
/// .unwrap();
/// // Not yet implemented in this crate version.
/// let _batch = to_record_batch(&table).unwrap();
/// ```
pub fn to_record_batch(_table: &TableEnvelope) -> Result<RecordBatch> {
    Err(Error::Internal("unimplemented".into()))
}

/// Convert an Arrow [`RecordBatch`] into a [`TableEnvelope`].
///
/// This is the inverse of [`to_record_batch`]: role and metadata annotations
/// stored in Arrow field/schema metadata are restored onto the resulting
/// [`TableEnvelope`].
///
/// # Errors
///
/// Returns [`Error::Internal`] for any batch this crate cannot yet convert.
/// The full column-type mapping lands in a follow-up task; today every call
/// fails.
///
/// # Examples
///
/// ```should_panic
/// use arrow::array::Float64Array;
/// use arrow::datatypes::{DataType, Field, Schema};
/// use arrow::record_batch::RecordBatch;
/// use finstack_quant_arrow::from_record_batch;
/// use std::sync::Arc;
///
/// let schema = Arc::new(Schema::new(vec![Field::new("pv", DataType::Float64, false)]));
/// let batch = RecordBatch::try_new(
///     schema,
///     vec![Arc::new(Float64Array::from(vec![101.5, 99.25]))],
/// )
/// .unwrap();
/// // Not yet implemented in this crate version.
/// let _table = from_record_batch(&batch).unwrap();
/// ```
pub fn from_record_batch(_batch: &RecordBatch) -> Result<TableEnvelope> {
    Err(Error::Internal("unimplemented".into()))
}

#[cfg(test)]
mod tests {
    use super::{from_record_batch, to_record_batch, Error};
    use arrow::array::Float64Array;
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow::record_batch::RecordBatch;
    use finstack_quant_core::table::{TableColumn, TableColumnData, TableEnvelope};
    use std::sync::Arc;

    fn sample_table() -> TableEnvelope {
        TableEnvelope::new(vec![TableColumn::new(
            "pv",
            TableColumnData::Float64(vec![101.5, 99.25]),
        )])
        .unwrap()
    }

    fn sample_batch() -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![Field::new(
            "pv",
            DataType::Float64,
            false,
        )]));
        RecordBatch::try_new(
            schema,
            vec![Arc::new(Float64Array::from(vec![101.5, 99.25]))],
        )
        .unwrap()
    }

    #[test]
    fn to_record_batch_is_unimplemented_stub() {
        let err = to_record_batch(&sample_table()).unwrap_err();
        assert!(matches!(err, Error::Internal(_)));
    }

    #[test]
    fn from_record_batch_is_unimplemented_stub() {
        let err = from_record_batch(&sample_batch()).unwrap_err();
        assert!(matches!(err, Error::Internal(_)));
    }
}
