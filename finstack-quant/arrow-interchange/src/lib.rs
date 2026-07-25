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

use std::collections::HashMap;
use std::sync::Arc;

use arrow::array::{ArrayRef, Float64Array, Int64Array, StringArray, UInt32Array};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::error::ArrowError;
use arrow::record_batch::{RecordBatch, RecordBatchOptions};
use finstack_quant_core::table::{TableColumn, TableColumnData, TableColumnRole, TableEnvelope};
use finstack_quant_core::{Error, Result};

/// Metadata key carrying a column's semantic role on an Arrow field.
pub const ROLE_METADATA_KEY: &str = "finstack:role";
/// Metadata key carrying serialized envelope/column metadata.
pub const METADATA_KEY: &str = "finstack:metadata";

/// Map an [`ArrowError`] into the core validation error with context.
fn arrow_err(context: &str, err: &ArrowError) -> Error {
    Error::Validation(format!("arrow {context}: {err}"))
}

/// Serde snake_case name for a column role (matches `TableColumnRole` serde).
fn role_to_str(role: TableColumnRole) -> &'static str {
    match role {
        TableColumnRole::Dimension => "dimension",
        TableColumnRole::Index => "index",
        TableColumnRole::Measure => "measure",
        TableColumnRole::Attribute => "attribute",
    }
}

/// Build the Arrow field + array for one envelope column.
fn column_to_arrow(column: &TableColumn) -> Result<(Field, ArrayRef)> {
    let (data_type, nullable, array): (DataType, bool, ArrayRef) = match &column.data {
        TableColumnData::String(values) => (
            DataType::Utf8,
            false,
            Arc::new(StringArray::from_iter_values(
                values.iter().map(String::as_str),
            )),
        ),
        TableColumnData::NullableString(values) => (
            DataType::Utf8,
            true,
            Arc::new(values.iter().map(|v| v.as_deref()).collect::<StringArray>()),
        ),
        TableColumnData::Float64(values) => (
            DataType::Float64,
            false,
            Arc::new(Float64Array::from(values.clone())),
        ),
        TableColumnData::NullableFloat64(values) => (
            DataType::Float64,
            true,
            Arc::new(Float64Array::from(values.clone())),
        ),
        TableColumnData::UInt32(values) => (
            DataType::UInt32,
            false,
            Arc::new(UInt32Array::from(values.clone())),
        ),
        TableColumnData::NullableUInt32(values) => (
            DataType::UInt32,
            true,
            Arc::new(UInt32Array::from(values.clone())),
        ),
        TableColumnData::Int64(values) => (
            DataType::Int64,
            false,
            Arc::new(Int64Array::from(values.clone())),
        ),
        TableColumnData::NullableInt64(values) => (
            DataType::Int64,
            true,
            Arc::new(Int64Array::from(values.clone())),
        ),
    };

    let mut metadata = HashMap::new();
    if let Some(role) = column.role {
        metadata.insert(ROLE_METADATA_KEY.to_string(), role_to_str(role).to_string());
    }
    if !column.metadata.is_empty() {
        let json = serde_json::to_string(&column.metadata).map_err(|e| {
            Error::Validation(format!(
                "column '{}': metadata serialization failed: {e}",
                column.name
            ))
        })?;
        metadata.insert(METADATA_KEY.to_string(), json);
    }
    Ok((
        Field::new(column.name.clone(), data_type, nullable).with_metadata(metadata),
        array,
    ))
}

/// Convert a [`TableEnvelope`] into an Arrow [`RecordBatch`].
///
/// Column order, names, and roles are preserved: each Arrow field carries the
/// source column's role (if any) and per-column metadata as field metadata
/// (`finstack:role`, `finstack:metadata`), and table-level metadata is carried
/// as schema metadata (`finstack:metadata`).
///
/// # Errors
///
/// Returns [`Error::Validation`] if metadata cannot be serialized or the
/// batch fails Arrow-side construction (should not happen for a valid
/// envelope, whose column lengths are already validated).
///
/// # Examples
///
/// ```rust
/// use finstack_quant_core::table::{TableColumn, TableColumnData, TableEnvelope};
/// use finstack_quant_arrow::to_record_batch;
///
/// let table = TableEnvelope::new(vec![TableColumn::new(
///     "pv",
///     TableColumnData::Float64(vec![101.5, 99.25]),
/// )])
/// .unwrap();
/// assert_eq!(to_record_batch(&table).unwrap().num_rows(), 2);
/// ```
pub fn to_record_batch(table: &TableEnvelope) -> Result<RecordBatch> {
    let mut fields = Vec::with_capacity(table.columns.len());
    let mut arrays: Vec<ArrayRef> = Vec::with_capacity(table.columns.len());
    for column in &table.columns {
        let (field, array) = column_to_arrow(column)?;
        fields.push(field);
        arrays.push(array);
    }

    let mut schema_metadata = HashMap::new();
    if !table.metadata.is_empty() {
        let json = serde_json::to_string(&table.metadata)
            .map_err(|e| Error::Validation(format!("table metadata serialization failed: {e}")))?;
        schema_metadata.insert(METADATA_KEY.to_string(), json);
    }

    let schema = Arc::new(Schema::new_with_metadata(fields, schema_metadata));
    let options = RecordBatchOptions::new().with_row_count(Some(table.row_count));
    RecordBatch::try_new_with_options(schema, arrays, &options)
        .map_err(|e| arrow_err("record batch construction", &e))
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
    use super::{from_record_batch, to_record_batch, Error, METADATA_KEY, ROLE_METADATA_KEY};
    use arrow::array::Float64Array;
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow::record_batch::RecordBatch;
    use finstack_quant_core::table::{
        TableColumn, TableColumnData, TableColumnRole, TableEnvelope,
    };
    use indexmap::IndexMap;
    use serde_json::json;
    use std::sync::Arc;

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
    fn from_record_batch_is_unimplemented_stub() {
        let err = from_record_batch(&sample_batch()).unwrap_err();
        assert!(matches!(err, Error::Internal(_)));
    }

    /// Envelope exercising all eight column variants, roles, and metadata.
    fn full_envelope() -> TableEnvelope {
        let mut column_meta = IndexMap::new();
        column_meta.insert("unit".to_string(), json!("USD"));
        column_meta.insert("scale".to_string(), json!(2));

        let mut table_meta = IndexMap::new();
        table_meta.insert("kind".to_string(), json!("unit_test"));
        table_meta.insert("orientation".to_string(), json!("long"));

        TableEnvelope::new_with_metadata(
            vec![
                TableColumn::new("id", TableColumnData::String(vec!["a".into(), "b".into()]))
                    .with_role(TableColumnRole::Dimension),
                TableColumn::new(
                    "label",
                    TableColumnData::NullableString(vec![Some("x".into()), None]),
                )
                .with_role(TableColumnRole::Attribute),
                TableColumn::new("pv", TableColumnData::Float64(vec![1.5, -2.25]))
                    .with_role(TableColumnRole::Measure)
                    .with_metadata(column_meta),
                TableColumn::new(
                    "pv_opt",
                    TableColumnData::NullableFloat64(vec![Some(0.5), None]),
                ),
                TableColumn::new("count", TableColumnData::UInt32(vec![1, 2])),
                TableColumn::new(
                    "count_opt",
                    TableColumnData::NullableUInt32(vec![None, Some(7)]),
                ),
                TableColumn::new("qty", TableColumnData::Int64(vec![-5, 5]))
                    .with_role(TableColumnRole::Index),
                TableColumn::new(
                    "qty_opt",
                    TableColumnData::NullableInt64(vec![Some(-1), None]),
                ),
            ],
            table_meta,
        )
        .unwrap()
    }

    #[test]
    fn to_record_batch_maps_types_nullability_and_metadata() {
        let batch = to_record_batch(&full_envelope()).unwrap();
        assert_eq!(batch.num_rows(), 2);
        assert_eq!(batch.num_columns(), 8);

        let schema = batch.schema();
        let expected: [(&str, DataType, bool); 8] = [
            ("id", DataType::Utf8, false),
            ("label", DataType::Utf8, true),
            ("pv", DataType::Float64, false),
            ("pv_opt", DataType::Float64, true),
            ("count", DataType::UInt32, false),
            ("count_opt", DataType::UInt32, true),
            ("qty", DataType::Int64, false),
            ("qty_opt", DataType::Int64, true),
        ];
        for (i, (name, dtype, nullable)) in expected.iter().enumerate() {
            let field = schema.field(i);
            assert_eq!(field.name(), name);
            assert_eq!(field.data_type(), dtype);
            assert_eq!(field.is_nullable(), *nullable, "field {name}");
        }

        assert_eq!(
            schema.field(0).metadata().get(ROLE_METADATA_KEY),
            Some(&"dimension".to_string())
        );
        assert_eq!(
            schema.field(6).metadata().get(ROLE_METADATA_KEY),
            Some(&"index".to_string())
        );
        let pv_meta = schema.field(2).metadata().get(METADATA_KEY).unwrap();
        assert_eq!(pv_meta, r#"{"unit":"USD","scale":2}"#);
        let table_meta = schema.metadata().get(METADATA_KEY).unwrap();
        assert_eq!(table_meta, r#"{"kind":"unit_test","orientation":"long"}"#);
        // Nulls became validity-bitmap nulls.
        assert_eq!(batch.column(1).null_count(), 1);
        assert_eq!(batch.column(3).null_count(), 1);
    }

    #[test]
    fn to_record_batch_preserves_row_count_for_zero_column_table() {
        let table = TableEnvelope::new(vec![]).unwrap();
        let batch = to_record_batch(&table).unwrap();
        assert_eq!(batch.num_rows(), 0);
        assert_eq!(batch.num_columns(), 0);
    }
}
