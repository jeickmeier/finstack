//! Arrow record-batch and IPC-stream interchange for
//! [`finstack_quant_core::table::TableEnvelope`].
//!
//! Canonical envelopes round-trip semantically through [`RecordBatch`] values
//! and stream-format bytes. This supporting crate is not umbrella-re-exported;
//! Python bindings use it for `finstack_quant.core.table.ArrowTable`.
//!
//! # Wire contract
//!
//! Outbound columns map `String` to `Utf8`, `f64` to `Float64`, `u32` to
//! `UInt32`, and `i64` to `Int64`; the nullable envelope variant controls Arrow
//! field nullability. Inbound conversion also accepts `LargeUtf8`, `Utf8View`,
//! and string dictionaries that Arrow can cast to `Utf8`. Other physical types
//! are rejected. Nullability comes from the schema even when an array contains
//! no nulls, and a non-nullable field containing nulls is invalid. Non-finite
//! floats remain representable.
//!
//! Field metadata key [`ROLE_METADATA_KEY`] (`finstack:role`) stores
//! `dimension`, `index`, `measure`, or `attribute`. [`METADATA_KEY`]
//! (`finstack:metadata`) stores a JSON object in field metadata for a column and
//! in schema metadata for the table. Missing keys restore no role or an empty
//! metadata map; unknown roles and malformed JSON are rejected.
//!
//! [`to_ipc_bytes`] writes one batch in Arrow IPC stream format;
//! [`from_ipc_bytes`] concatenates every input batch. A zero-batch stream keeps
//! its schema as a typed zero-row table. A zero-column batch with nonzero rows
//! is not representable and is rejected. IPC compatibility is semantic rather
//! than byte-stable across Arrow versions.

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
use std::io::Cursor;
use std::sync::Arc;

use arrow::array::{
    Array, ArrayRef, Float64Array, GenericStringArray, Int64Array, OffsetSizeTrait, StringArray,
    StringViewArray, UInt32Array,
};
use arrow::compute::{cast, concat_batches};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::error::ArrowError;
use arrow::ipc::reader::StreamReader;
use arrow::ipc::writer::StreamWriter;
use arrow::record_batch::{RecordBatch, RecordBatchOptions};
use finstack_quant_core::table::{TableColumn, TableColumnData, TableColumnRole, TableEnvelope};
use finstack_quant_core::{Error, Result};
use indexmap::IndexMap;

/// Arrow field-metadata key for a column's [`TableColumnRole`] wire name.
///
/// Value is one of `dimension`, `index`, `measure`, or `attribute` — the same
/// snake_case spelling used by `TableColumnRole` serde and
/// [`TableColumnRole::as_str`].
pub const ROLE_METADATA_KEY: &str = "finstack:role";

/// Arrow metadata key for serialized JSON metadata maps.
///
/// On a **field**, the value is the column's `metadata`
/// (`IndexMap<String, serde_json::Value>` as JSON). On the **schema**, the
/// value is the table-level `TableEnvelope::metadata` map.
pub const METADATA_KEY: &str = "finstack:metadata";

/// Map an [`ArrowError`] into the core validation error with context.
fn arrow_err(context: &str, err: &ArrowError) -> Error {
    Error::Validation(format!("arrow {context}: {err}"))
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
        metadata.insert(ROLE_METADATA_KEY.to_string(), role.as_str().to_string());
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
/// # Arguments
///
/// * `table` - Validated envelope to convert; its column order, names, roles,
///   and metadata are carried into the resulting Arrow schema.
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

/// Downcast an Arrow array to a concrete type with a contextual error.
fn downcast<'a, T: 'static>(name: &str, array: &'a ArrayRef) -> Result<&'a T> {
    array.as_any().downcast_ref::<T>().ok_or_else(|| {
        Error::Validation(format!(
            "column '{name}': array does not match its declared data type"
        ))
    })
}

/// True for Arrow string physical types we map onto envelope string columns.
fn is_string_arrow_type(data_type: &DataType) -> bool {
    matches!(
        data_type,
        DataType::Utf8 | DataType::LargeUtf8 | DataType::Utf8View
    )
}

/// Collect owned strings from a `GenericStringArray` (`Utf8` / `LargeUtf8`).
///
/// Non-nullable columns use `value(i)` after the caller has rejected nulls, so
/// a future refactor cannot silently drop rows via `.flatten()`.
fn string_data_from_generic_string<O: OffsetSizeTrait>(
    array: &GenericStringArray<O>,
    nullable: bool,
) -> TableColumnData {
    if nullable {
        TableColumnData::NullableString(array.iter().map(|v| v.map(str::to_owned)).collect())
    } else {
        let mut values = Vec::with_capacity(array.len());
        for i in 0..array.len() {
            values.push(array.value(i).to_owned());
        }
        TableColumnData::String(values)
    }
}

/// Collect owned strings from a [`StringViewArray`] (`Utf8View`).
fn string_data_from_string_view(array: &StringViewArray, nullable: bool) -> TableColumnData {
    if nullable {
        TableColumnData::NullableString(array.iter().map(|v| v.map(str::to_owned)).collect())
    } else {
        let mut values = Vec::with_capacity(array.len());
        for i in 0..array.len() {
            values.push(array.value(i).to_owned());
        }
        TableColumnData::String(values)
    }
}

/// Decode an Arrow string (or string-dictionary) array into envelope column data.
fn string_data_from_arrow(
    name: &str,
    data_type: &DataType,
    array: &ArrayRef,
    nullable: bool,
) -> Result<TableColumnData> {
    match data_type {
        DataType::Utf8 => Ok(string_data_from_generic_string(
            downcast::<StringArray>(name, array)?,
            nullable,
        )),
        DataType::LargeUtf8 => Ok(string_data_from_generic_string(
            downcast::<GenericStringArray<i64>>(name, array)?,
            nullable,
        )),
        DataType::Utf8View => Ok(string_data_from_string_view(
            downcast::<StringViewArray>(name, array)?,
            nullable,
        )),
        DataType::Dictionary(_, value_type) if is_string_arrow_type(value_type.as_ref()) => {
            let utf8 = cast(array, &DataType::Utf8).map_err(|e| {
                Error::Validation(format!(
                    "column '{name}': dictionary to Utf8 cast failed: {e}"
                ))
            })?;
            Ok(string_data_from_generic_string(
                downcast::<StringArray>(name, &utf8)?,
                nullable,
            ))
        }
        other => Err(Error::Validation(format!(
            "column '{name}': unsupported Arrow type {other}; \
             supported types are Utf8, LargeUtf8, Utf8View, \
             Dictionary(*→string), Float64, UInt32, Int64"
        ))),
    }
}

/// Rebuild one envelope column from an Arrow field + array.
fn column_from_arrow(field: &Field, array: &ArrayRef) -> Result<TableColumn> {
    let name = field.name().clone();
    let nullable = field.is_nullable();
    if !nullable && array.null_count() > 0 {
        return Err(Error::Validation(format!(
            "column '{name}': declared non-nullable but contains {} null value(s)",
            array.null_count()
        )));
    }

    let data = match field.data_type() {
        dt if is_string_arrow_type(dt)
            || matches!(dt, DataType::Dictionary(_, v) if is_string_arrow_type(v)) =>
        {
            string_data_from_arrow(&name, dt, array, nullable)?
        }
        DataType::Float64 if !nullable => {
            TableColumnData::Float64(downcast::<Float64Array>(&name, array)?.values().to_vec())
        }
        DataType::Float64 => TableColumnData::NullableFloat64(
            downcast::<Float64Array>(&name, array)?.iter().collect(),
        ),
        DataType::UInt32 if !nullable => {
            TableColumnData::UInt32(downcast::<UInt32Array>(&name, array)?.values().to_vec())
        }
        DataType::UInt32 => {
            TableColumnData::NullableUInt32(downcast::<UInt32Array>(&name, array)?.iter().collect())
        }
        DataType::Int64 if !nullable => {
            TableColumnData::Int64(downcast::<Int64Array>(&name, array)?.values().to_vec())
        }
        DataType::Int64 => {
            TableColumnData::NullableInt64(downcast::<Int64Array>(&name, array)?.iter().collect())
        }
        other => {
            return Err(Error::Validation(format!(
                "column '{name}': unsupported Arrow type {other}; \
                 supported types are Utf8, LargeUtf8, Utf8View, \
                 Dictionary(*→string), Float64, UInt32, Int64"
            )));
        }
    };

    let mut column = TableColumn::new(name, data);
    if let Some(role) = field.metadata().get(ROLE_METADATA_KEY) {
        let parsed = TableColumnRole::from_str_name(role)
            .map_err(|e| Error::Validation(format!("unknown '{ROLE_METADATA_KEY}' value: {e}")))?;
        column = column.with_role(parsed);
    }
    if let Some(meta_json) = field.metadata().get(METADATA_KEY) {
        let metadata: IndexMap<String, serde_json::Value> = serde_json::from_str(meta_json)
            .map_err(|e| {
                Error::Validation(format!(
                    "column '{}': metadata deserialization failed: {e}",
                    column.name
                ))
            })?;
        column = column.with_metadata(metadata);
    }
    Ok(column)
}

/// Convert an Arrow [`RecordBatch`] back into a [`TableEnvelope`].
///
/// The inverse of [`to_record_batch`]. The field's `nullable` flag selects
/// the nullable vs non-nullable envelope variant; `finstack:role` and
/// `finstack:metadata` field/schema metadata are restored.
///
/// # Arguments
///
/// * `batch` - Arrow batch to convert; its schema drives column names, roles,
///   nullability, and the restored table metadata.
///
/// # Errors
///
/// Returns [`Error::Validation`] for unsupported Arrow types (see crate-level
/// supported-type list), nulls in a non-nullable field, unknown role values,
/// malformed metadata JSON, a zero-column batch with a nonzero row count (not
/// representable, since [`TableEnvelope`] derives `row_count` from its first
/// column), or envelope validation failures (duplicate names, mismatched
/// lengths).
///
/// # Examples
/// ```rust
/// use finstack_quant_core::table::{TableColumn, TableColumnData, TableEnvelope};
/// use finstack_quant_arrow::{from_record_batch, to_record_batch};
///
/// let table = TableEnvelope::new(vec![TableColumn::new(
///     "qty",
///     TableColumnData::Int64(vec![1, 2, 3]),
/// )])
/// .unwrap();
/// let batch = to_record_batch(&table).unwrap();
/// assert_eq!(from_record_batch(&batch).unwrap(), table);
/// ```
pub fn from_record_batch(batch: &RecordBatch) -> Result<TableEnvelope> {
    if batch.num_columns() == 0 && batch.num_rows() != 0 {
        return Err(Error::Validation(format!(
            "record batch has 0 columns but {} rows; TableEnvelope cannot \
             represent a column-less table with rows (row_count is derived \
             from the first column)",
            batch.num_rows()
        )));
    }
    let schema = batch.schema();
    let mut columns = Vec::with_capacity(batch.num_columns());
    for (field, array) in schema.fields().iter().zip(batch.columns()) {
        columns.push(column_from_arrow(field, array)?);
    }
    let metadata = match schema.metadata().get(METADATA_KEY) {
        Some(json) => serde_json::from_str(json).map_err(|e| {
            Error::Validation(format!("table metadata deserialization failed: {e}"))
        })?,
        None => IndexMap::new(),
    };
    TableEnvelope::new_with_metadata(columns, metadata)
}

/// Serialize a [`TableEnvelope`] as Arrow IPC **stream-format** bytes.
///
/// The output contains a single record batch and the full schema (including
/// `finstack:*` metadata), suitable for standard Arrow IPC stream consumers.
///
/// # Arguments
///
/// * `table` - Validated envelope to serialize; it is written as a single
///   record batch carrying the full `finstack:*` schema metadata.
///
/// # Errors
///
/// Returns [`Error::Validation`] if conversion or IPC encoding fails.
///
/// # Examples
/// ```rust
/// use finstack_quant_core::table::{TableColumn, TableColumnData, TableEnvelope};
/// use finstack_quant_arrow::{from_ipc_bytes, to_ipc_bytes};
///
/// let table = TableEnvelope::new(vec![TableColumn::new(
///     "pv",
///     TableColumnData::Float64(vec![1.0]),
/// )])
/// .unwrap();
/// let bytes = to_ipc_bytes(&table).unwrap();
/// assert_eq!(from_ipc_bytes(&bytes).unwrap(), table);
/// ```
pub fn to_ipc_bytes(table: &TableEnvelope) -> Result<Vec<u8>> {
    let batch = to_record_batch(table)?;
    let schema = batch.schema();
    let mut writer = StreamWriter::try_new(Vec::new(), schema.as_ref())
        .map_err(|e| arrow_err("ipc writer creation", &e))?;
    writer
        .write(&batch)
        .map_err(|e| arrow_err("ipc write", &e))?;
    writer.finish().map_err(|e| arrow_err("ipc finish", &e))?;
    writer
        .into_inner()
        .map_err(|e| arrow_err("ipc buffer recovery", &e))
}

/// Deserialize Arrow IPC **stream-format** bytes into a [`TableEnvelope`].
///
/// Multiple batches in the stream are concatenated into a single table
/// (column-wise `concat`), preserving schema metadata. This is a deliberate
/// choice over rejecting multi-batch input or silently returning only the
/// first batch: [`arrow::ipc::writer::StreamWriter`] readily produces
/// multi-batch streams (e.g. chunked writers), and concatenation is the
/// behavior a caller reading "the table" from a stream would expect.
///
/// # Arguments
///
/// * `bytes` - Arrow IPC stream-format buffer; every batch it contains is
///   read and concatenated column-wise into one envelope.
///
/// # Errors
///
/// Returns [`Error::Validation`] on malformed IPC bytes, unsupported column
/// types, or envelope validation failures.
///
/// # Examples
/// ```rust
/// use finstack_quant_core::table::{TableColumn, TableColumnData, TableEnvelope};
/// use finstack_quant_arrow::{from_ipc_bytes, to_ipc_bytes};
///
/// let table = TableEnvelope::new(vec![TableColumn::new(
///     "id",
///     TableColumnData::String(vec!["x".into()]),
/// )])
/// .unwrap();
/// let restored = from_ipc_bytes(&to_ipc_bytes(&table).unwrap()).unwrap();
/// assert_eq!(restored, table);
/// ```
pub fn from_ipc_bytes(bytes: &[u8]) -> Result<TableEnvelope> {
    let reader = StreamReader::try_new(Cursor::new(bytes), None)
        .map_err(|e| arrow_err("ipc reader creation", &e))?;
    let schema = reader.schema();
    let batches = reader
        .collect::<std::result::Result<Vec<RecordBatch>, ArrowError>>()
        .map_err(|e| arrow_err("ipc read", &e))?;
    let batch = concat_batches(&schema, &batches).map_err(|e| arrow_err("ipc batch concat", &e))?;
    from_record_batch(&batch)
}

#[cfg(test)]
mod tests {
    use super::{from_record_batch, to_record_batch, METADATA_KEY, ROLE_METADATA_KEY};
    use arrow::datatypes::DataType;
    use finstack_quant_core::table::{
        TableColumn, TableColumnData, TableColumnRole, TableEnvelope,
    };
    use indexmap::IndexMap;
    use serde_json::json;

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

    #[test]
    fn record_batch_round_trip_is_lossless() {
        let table = full_envelope();
        let restored = from_record_batch(&to_record_batch(&table).unwrap()).unwrap();
        assert_eq!(restored, table);
    }

    #[test]
    fn empty_envelope_round_trips() {
        let table = TableEnvelope::new(vec![]).unwrap();
        let restored = from_record_batch(&to_record_batch(&table).unwrap()).unwrap();
        assert_eq!(restored, table);
    }

    /// A zero-column Arrow `RecordBatch` carrying a nonzero row count is
    /// legal Arrow (constructible via `RecordBatchOptions::with_row_count`)
    /// but not representable as a `TableEnvelope`, which derives `row_count`
    /// from its first column. This must be rejected, not silently truncated
    /// to `row_count = 0`.
    #[test]
    fn zero_column_batch_with_nonzero_row_count_is_rejected() {
        use arrow::datatypes::Schema;
        use arrow::record_batch::{RecordBatch, RecordBatchOptions};
        use std::sync::Arc;

        let schema = Arc::new(Schema::empty());
        let options = RecordBatchOptions::new().with_row_count(Some(5));
        let batch = RecordBatch::try_new_with_options(schema, vec![], &options).unwrap();
        assert_eq!(batch.num_columns(), 0);
        assert_eq!(batch.num_rows(), 5);

        let err = from_record_batch(&batch).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains('5'), "error should name the row count: {msg}");
        assert!(
            msg.contains("column-less") || msg.contains("0 columns"),
            "error should explain the column-less constraint: {msg}"
        );
    }

    /// The genuinely-empty case (0 columns AND 0 rows) is representable and
    /// must keep succeeding.
    #[test]
    fn zero_column_zero_row_batch_still_succeeds() {
        use arrow::datatypes::Schema;
        use arrow::record_batch::RecordBatch;
        use std::sync::Arc;

        let schema = Arc::new(Schema::empty());
        let batch = RecordBatch::new_empty(schema);
        assert_eq!(batch.num_columns(), 0);
        assert_eq!(batch.num_rows(), 0);

        let table = from_record_batch(&batch).unwrap();
        assert_eq!(table.row_count, 0);
        assert!(table.columns.is_empty());
    }

    #[test]
    fn nullable_field_without_nulls_stays_nullable_variant() {
        let table = TableEnvelope::new(vec![TableColumn::new(
            "x",
            TableColumnData::NullableFloat64(vec![Some(1.0), Some(2.0)]),
        )])
        .unwrap();
        let restored = from_record_batch(&to_record_batch(&table).unwrap()).unwrap();
        assert_eq!(restored, table); // nullability is schema-driven
    }

    #[test]
    fn unsupported_arrow_type_is_rejected() {
        use arrow::array::Date32Array;
        let field = arrow::datatypes::Field::new("d", DataType::Date32, false);
        let schema = std::sync::Arc::new(arrow::datatypes::Schema::new(vec![field]));
        let batch = arrow::record_batch::RecordBatch::try_new(
            schema,
            vec![std::sync::Arc::new(Date32Array::from(vec![0, 1]))],
        )
        .unwrap();
        let err = from_record_batch(&batch).unwrap_err();
        assert!(err.to_string().contains("unsupported Arrow type"));
    }

    #[test]
    fn unknown_role_metadata_is_rejected() {
        let mut batch = to_record_batch(&full_envelope()).unwrap();
        // Rebuild schema with a bogus role on field 0.
        let schema = batch.schema();
        let mut fields: Vec<arrow::datatypes::Field> =
            schema.fields().iter().map(|f| f.as_ref().clone()).collect();
        let mut md = fields[0].metadata().clone();
        md.insert(ROLE_METADATA_KEY.to_string(), "bogus".to_string());
        fields[0] = fields[0].clone().with_metadata(md);
        let new_schema = std::sync::Arc::new(arrow::datatypes::Schema::new_with_metadata(
            fields,
            schema.metadata().clone(),
        ));
        batch = arrow::record_batch::RecordBatch::try_new(new_schema, batch.columns().to_vec())
            .unwrap();
        assert!(from_record_batch(&batch).is_err());
    }

    /// Closes the reviewer gap from task 2: `to_record_batch` tests checked
    /// types/nullability/metadata but not actual non-null values, and neither
    /// covered non-finite floats. NaN fails `PartialEq` against itself, so
    /// this compares field-by-field instead of a blanket `assert_eq!` on the
    /// whole envelope, using `is_nan()`/`is_infinite()` for the special float.
    #[test]
    fn non_finite_float_values_round_trip() {
        let table = TableEnvelope::new(vec![
            TableColumn::new(
                "pv",
                TableColumnData::Float64(vec![
                    f64::NAN,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    0.0,
                    0.0,
                ]),
            ),
            TableColumn::new(
                "pv_opt",
                TableColumnData::NullableFloat64(vec![
                    Some(f64::NAN),
                    None,
                    Some(1.25),
                    Some(f64::INFINITY),
                    Some(f64::NEG_INFINITY),
                ]),
            ),
        ])
        .unwrap();
        let restored = from_record_batch(&to_record_batch(&table).unwrap()).unwrap();

        assert_eq!(restored.row_count, table.row_count);
        assert_eq!(restored.columns.len(), table.columns.len());

        match &restored.columns[0].data {
            TableColumnData::Float64(values) => {
                assert!(values[0].is_nan());
                assert!(values[1].is_infinite() && values[1].is_sign_positive());
                assert!(values[2].is_infinite() && values[2].is_sign_negative());
            }
            other => panic!("expected Float64, got {other:?}"),
        }
        match &restored.columns[1].data {
            TableColumnData::NullableFloat64(values) => {
                assert!(values[0].is_some_and(|v| v.is_nan()));
                assert_eq!(values[1], None);
                assert_eq!(values[2], Some(1.25));
                assert!(values[3].is_some_and(|v| v.is_infinite() && v.is_sign_positive()));
                assert!(values[4].is_some_and(|v| v.is_infinite() && v.is_sign_negative()));
            }
            other => panic!("expected NullableFloat64, got {other:?}"),
        }
    }

    #[test]
    fn ipc_round_trip_is_semantically_lossless() {
        // Deliberately NOT a byte-golden: Arrow IPC encoding is not guaranteed
        // byte-stable across arrow-rs versions (padding/flatbuffer layout), so
        // we assert semantic equality of the round-tripped envelope instead.
        let table = full_envelope();
        let bytes = super::to_ipc_bytes(&table).unwrap();
        assert_eq!(super::from_ipc_bytes(&bytes).unwrap(), table);
    }

    #[test]
    fn ipc_round_trip_empty_envelope() {
        let table = TableEnvelope::new(vec![]).unwrap();
        let bytes = super::to_ipc_bytes(&table).unwrap();
        assert_eq!(super::from_ipc_bytes(&bytes).unwrap(), table);
    }

    /// Zero rows but non-zero columns: a distinct edge case from the
    /// zero-column empty envelope above (exercises schema/type round-trip
    /// with no data to carry it).
    #[test]
    fn ipc_round_trip_zero_rows_with_columns() {
        let table = TableEnvelope::new(vec![
            TableColumn::new("id", TableColumnData::String(vec![])),
            TableColumn::new("pv", TableColumnData::Float64(vec![])),
        ])
        .unwrap();
        let bytes = super::to_ipc_bytes(&table).unwrap();
        let restored = super::from_ipc_bytes(&bytes).unwrap();
        assert_eq!(restored, table);
        assert_eq!(restored.row_count, 0);
    }

    /// A truncated (but not entirely garbage) IPC stream — the schema
    /// message is present but the body is cut off — must also error
    /// cleanly rather than panic.
    #[test]
    fn truncated_ipc_bytes_error_cleanly() {
        let table = full_envelope();
        let bytes = super::to_ipc_bytes(&table).unwrap();
        let truncated = &bytes[..bytes.len() / 2];
        assert!(super::from_ipc_bytes(truncated).is_err());
    }

    #[test]
    fn multi_batch_ipc_input_is_concatenated() {
        use arrow::ipc::writer::StreamWriter;
        let t1 = TableEnvelope::new(vec![TableColumn::new(
            "v",
            TableColumnData::Int64(vec![1, 2]),
        )])
        .unwrap();
        let t2 = TableEnvelope::new(vec![TableColumn::new("v", TableColumnData::Int64(vec![3]))])
            .unwrap();
        let b1 = to_record_batch(&t1).unwrap();
        let b2 = to_record_batch(&t2).unwrap();
        let schema = b1.schema();
        let mut writer = StreamWriter::try_new(Vec::new(), schema.as_ref()).unwrap();
        writer.write(&b1).unwrap();
        writer.write(&b2).unwrap();
        writer.finish().unwrap();
        let bytes = writer.into_inner().unwrap();

        let merged = super::from_ipc_bytes(&bytes).unwrap();
        assert_eq!(merged.row_count, 3);
        assert_eq!(
            merged.column("v").unwrap().as_i64(),
            Some([1, 2, 3].as_slice())
        );
    }

    #[test]
    fn garbage_ipc_bytes_error_cleanly() {
        assert!(super::from_ipc_bytes(&[0x00, 0x01, 0x02]).is_err());
    }

    /// Closes a task-8 reviewer gap: `multi_batch_ipc_input_is_concatenated`
    /// only checked row values after concatenation. Two single-row batches
    /// sharing an identical schema (same roles, per-column metadata, and
    /// table-level metadata) must retain all of that metadata after the
    /// `concat_batches` merge, not just the numeric payload.
    #[test]
    fn multi_batch_ipc_concat_preserves_roles_and_metadata() {
        let mut column_meta = IndexMap::new();
        column_meta.insert("unit".to_string(), json!("USD"));

        let mut table_meta = IndexMap::new();
        table_meta.insert("kind".to_string(), json!("multi_batch_test"));

        let make = |id: &str, pv: f64| {
            TableEnvelope::new_with_metadata(
                vec![
                    TableColumn::new("id", TableColumnData::String(vec![id.to_string()]))
                        .with_role(TableColumnRole::Dimension),
                    TableColumn::new("pv", TableColumnData::Float64(vec![pv]))
                        .with_role(TableColumnRole::Measure)
                        .with_metadata(column_meta.clone()),
                ],
                table_meta.clone(),
            )
            .unwrap()
        };

        let b1 = to_record_batch(&make("a", 1.5)).unwrap();
        let b2 = to_record_batch(&make("b", 2.5)).unwrap();
        let schema = b1.schema();
        let mut writer =
            arrow::ipc::writer::StreamWriter::try_new(Vec::new(), schema.as_ref()).unwrap();
        writer.write(&b1).unwrap();
        writer.write(&b2).unwrap();
        writer.finish().unwrap();
        let bytes = writer.into_inner().unwrap();

        let merged = super::from_ipc_bytes(&bytes).unwrap();
        assert_eq!(merged.row_count, 2);
        assert_eq!(merged.metadata, table_meta, "table-level metadata");

        let id_col = merged.column("id").unwrap();
        assert_eq!(id_col.role, Some(TableColumnRole::Dimension));
        assert_eq!(
            id_col.as_strings(),
            Some(["a".to_string(), "b".to_string()].as_slice())
        );

        let pv_col = merged.column("pv").unwrap();
        assert_eq!(pv_col.role, Some(TableColumnRole::Measure));
        assert_eq!(pv_col.metadata, column_meta, "column-level metadata");
        assert_eq!(pv_col.as_f64(), Some([1.5, 2.5].as_slice()));
    }

    /// A writer that calls `.finish()` without ever calling `.write()` is a
    /// legal Arrow IPC stream: schema message present, zero record-batch
    /// messages. `concat_batches` must handle an empty batch list cleanly
    /// (producing a zero-row table) rather than panicking or erroring.
    #[test]
    fn zero_batch_ipc_stream_produces_empty_table() {
        let table = TableEnvelope::new(vec![TableColumn::new(
            "id",
            TableColumnData::String(vec!["placeholder".into()]),
        )])
        .unwrap();
        let batch = to_record_batch(&table).unwrap();
        let schema = batch.schema();

        let mut writer =
            arrow::ipc::writer::StreamWriter::try_new(Vec::new(), schema.as_ref()).unwrap();
        // Deliberately no `.write()` call before `.finish()`.
        writer.finish().unwrap();
        let bytes = writer.into_inner().unwrap();

        let restored = super::from_ipc_bytes(&bytes).unwrap();
        assert_eq!(restored.row_count, 0);
        assert_eq!(restored.columns.len(), 1);
        assert_eq!(restored.columns[0].name, "id");
        assert_eq!(restored.columns[0].data, TableColumnData::String(vec![]));
    }

    /// Closes a task-8 reviewer gap: the direct `from_record_batch` path
    /// already proved a nullable field with zero actual nulls stays the
    /// `NullableFloat64` envelope variant (schema-driven, not data-driven),
    /// but that discrimination was never exercised through the IPC
    /// serialize/deserialize path specifically.
    #[test]
    fn ipc_round_trip_preserves_nullable_variant_with_zero_nulls() {
        let table = TableEnvelope::new(vec![TableColumn::new(
            "x",
            TableColumnData::NullableFloat64(vec![Some(1.0), Some(2.0)]),
        )])
        .unwrap();
        let bytes = super::to_ipc_bytes(&table).unwrap();
        let restored = super::from_ipc_bytes(&bytes).unwrap();
        assert_eq!(restored, table);
        match &restored.columns[0].data {
            TableColumnData::NullableFloat64(values) => {
                assert_eq!(values, &vec![Some(1.0), Some(2.0)]);
            }
            other => panic!("expected NullableFloat64, got {other:?}"),
        }
    }

    /// Foreign Arrow producers often emit `LargeUtf8`; inbound decode must
    /// accept it as a string column without requiring a pre-cast.
    #[test]
    fn large_utf8_column_decodes_to_string() {
        use arrow::array::LargeStringArray;
        use arrow::datatypes::{Field, Schema};
        use arrow::record_batch::RecordBatch;
        use std::sync::Arc;

        let field = Field::new("id", DataType::LargeUtf8, false);
        let schema = Arc::new(Schema::new(vec![field]));
        let batch = RecordBatch::try_new(
            schema,
            vec![Arc::new(LargeStringArray::from(vec!["a", "b"])) as _],
        )
        .unwrap();

        let table = from_record_batch(&batch).unwrap();
        assert_eq!(
            table.column("id").unwrap().as_strings(),
            Some(["a".to_string(), "b".to_string()].as_slice())
        );
    }

    /// Nullable `LargeUtf8` must keep the nullable envelope variant.
    #[test]
    fn large_utf8_nullable_column_decodes() {
        use arrow::array::LargeStringArray;
        use arrow::datatypes::{Field, Schema};
        use arrow::record_batch::RecordBatch;
        use std::sync::Arc;

        let field = Field::new("label", DataType::LargeUtf8, true);
        let schema = Arc::new(Schema::new(vec![field]));
        let batch = RecordBatch::try_new(
            schema,
            vec![Arc::new(LargeStringArray::from(vec![Some("x"), None])) as _],
        )
        .unwrap();

        let table = from_record_batch(&batch).unwrap();
        assert_eq!(
            table.column("label").unwrap().as_nullable_strings(),
            Some([Some("x".to_string()), None].as_slice())
        );
    }

    /// `Utf8View` is increasingly common in recent Arrow; decode into strings.
    #[test]
    fn utf8_view_column_decodes_to_string() {
        use arrow::array::StringViewArray;
        use arrow::datatypes::{Field, Schema};
        use arrow::record_batch::RecordBatch;
        use std::sync::Arc;

        let field = Field::new("id", DataType::Utf8View, false);
        let schema = Arc::new(Schema::new(vec![field]));
        let batch = RecordBatch::try_new(
            schema,
            vec![Arc::new(StringViewArray::from(vec!["alpha", "beta"])) as _],
        )
        .unwrap();

        let table = from_record_batch(&batch).unwrap();
        assert_eq!(
            table.column("id").unwrap().as_strings(),
            Some(["alpha".to_string(), "beta".to_string()].as_slice())
        );
    }

    /// Dictionary-encoded string columns (common from Parquet / DuckDB) decode
    /// into plain envelope strings.
    #[test]
    fn dictionary_utf8_column_decodes_to_string() {
        use arrow::array::{DictionaryArray, Int32Array, StringArray};
        use arrow::datatypes::{DataType, Field, Schema};
        use arrow::record_batch::RecordBatch;
        use std::sync::Arc;

        let values = Arc::new(StringArray::from(vec!["usd", "eur"]));
        let keys = Int32Array::from(vec![0, 1, 0]);
        let dict = DictionaryArray::try_new(keys, values).unwrap();
        let field = Field::new(
            "ccy",
            DataType::Dictionary(Box::new(DataType::Int32), Box::new(DataType::Utf8)),
            false,
        );
        let schema = Arc::new(Schema::new(vec![field]));
        let batch = RecordBatch::try_new(schema, vec![Arc::new(dict) as _]).unwrap();

        let table = from_record_batch(&batch).unwrap();
        assert_eq!(
            table.column("ccy").unwrap().as_strings(),
            Some(["usd".to_string(), "eur".to_string(), "usd".to_string()].as_slice())
        );
    }
}
