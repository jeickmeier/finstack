# finstack-quant-arrow

Apache Arrow interchange for finstack-quant tabular outputs.

Directory: `finstack-quant/arrow-interchange`. Package / import name:
`finstack-quant-arrow` / `finstack_quant_arrow`.

Converts [`finstack_quant_core::table::TableEnvelope`](../core/src/table.rs) to
and from Arrow `RecordBatch` values and Arrow IPC **stream-format** bytes.
Column roles and metadata round-trip through Arrow field/schema metadata.

This is a supporting crate:

- not re-exported by the `finstack-quant` umbrella crate
- no WASM binding (`arrow-rs` is not built for `wasm32`)
- consumed by Python bindings (`finstack-quant-py`) to back
  `finstack_quant.core.table.ArrowTable`

## Dependency

```toml
[dependencies]
finstack-quant-arrow = { path = "../arrow-interchange" }
finstack-quant-core = { path = "../core" }
```

```rust
use finstack_quant_arrow::{from_record_batch, to_record_batch, to_ipc_bytes, from_ipc_bytes};
use finstack_quant_core::table::{TableColumn, TableColumnData, TableEnvelope};
```

## Public API

| Item | Role |
|------|------|
| `to_record_batch` | `TableEnvelope` → Arrow `RecordBatch` |
| `from_record_batch` | Arrow `RecordBatch` → `TableEnvelope` |
| `to_ipc_bytes` | `TableEnvelope` → Arrow IPC stream bytes (single batch) |
| `from_ipc_bytes` | Arrow IPC stream bytes → `TableEnvelope` (concatenates multi-batch streams) |
| `ROLE_METADATA_KEY` | Field metadata key for column role (`"finstack:role"`) |
| `METADATA_KEY` | Field/schema metadata key for JSON metadata (`"finstack:metadata"`) |

All fallible APIs return `finstack_quant_core::Result`, mapping Arrow failures to
`Error::Validation` with an `arrow …` context prefix.

## Column type map

| `TableColumnData` | Outbound Arrow type | Nullable |
|-------------------|---------------------|----------|
| `String` | `Utf8` | no |
| `NullableString` | `Utf8` | yes |
| `Float64` | `Float64` | no |
| `NullableFloat64` | `Float64` | yes |
| `UInt32` | `UInt32` | no |
| `NullableUInt32` | `UInt32` | yes |
| `Int64` | `Int64` | no |
| `NullableInt64` | `Int64` | yes |

**Outbound** (`to_record_batch` / `to_ipc_bytes`) always emits the plain types
above (`Utf8`, `Float64`, `UInt32`, `Int64`).

**Inbound** (`from_record_batch` / `from_ipc_bytes`) accepts that same set, plus
common foreign string encodings that decode into `String` / `NullableString`:

- `Utf8`, `LargeUtf8`, `Utf8View`
- `Dictionary(_, Utf8 | LargeUtf8 | Utf8View)` (decoded via cast to `Utf8`)

Other Arrow types (dates, timestamps, booleans, nested types, …) are rejected;
cast them to a supported type before calling inbound APIs.

Nullability is schema-driven: a nullable Arrow field with zero nulls restores
the nullable envelope variant, not the non-nullable one. Non-nullable fields
that contain nulls are rejected.

Non-finite floats (`NaN`, `±∞`) round-trip.

## Metadata contract

| Key | Constant | Location | Value |
|-----|----------|----------|-------|
| `finstack:role` | `ROLE_METADATA_KEY` | Arrow field metadata | Role wire name: `dimension`, `index`, `measure`, or `attribute` (same spelling as `TableColumnRole` serde) |
| `finstack:metadata` | `METADATA_KEY` | Arrow field metadata | JSON object of per-column metadata (`IndexMap<String, Value>`) |
| `finstack:metadata` | `METADATA_KEY` | Arrow schema metadata | JSON object of table-level metadata |

Unknown `finstack:role` values and malformed metadata JSON return
`Error::Validation`.

## Edge cases

- Empty envelope (0 columns, 0 rows) round-trips.
- Zero rows with columns round-trips (schema preserved).
- A zero-column Arrow batch with a nonzero row count is rejected: `TableEnvelope`
  derives `row_count` from its first column and cannot represent that shape.
- `from_ipc_bytes` concatenates every batch in the stream column-wise. A
  finished stream with zero written batches yields a zero-row table with the
  stream schema.
- IPC equality checks are semantic (envelope equality), not byte-golden: Arrow
  IPC layout is not guaranteed stable across `arrow-rs` versions.

## Quick start

### RecordBatch round-trip

```rust
use finstack_quant_arrow::{from_record_batch, to_record_batch};
use finstack_quant_core::table::{TableColumn, TableColumnData, TableColumnRole, TableEnvelope};

# fn main() -> finstack_quant_core::Result<()> {
let table = TableEnvelope::new(vec![
    TableColumn::new("id", TableColumnData::String(vec!["a".into(), "b".into()]))
        .with_role(TableColumnRole::Dimension),
    TableColumn::new("pv", TableColumnData::Float64(vec![101.5, 99.25]))
        .with_role(TableColumnRole::Measure),
])?;

let batch = to_record_batch(&table)?;
assert_eq!(batch.num_rows(), 2);
assert_eq!(from_record_batch(&batch)?, table);
# Ok(())
# }
```

### IPC stream bytes

```rust
use finstack_quant_arrow::{from_ipc_bytes, to_ipc_bytes};
use finstack_quant_core::table::{TableColumn, TableColumnData, TableEnvelope};

# fn main() -> finstack_quant_core::Result<()> {
let table = TableEnvelope::new(vec![TableColumn::new(
    "qty",
    TableColumnData::Int64(vec![1, 2, 3]),
)])?;

let bytes = to_ipc_bytes(&table)?;
assert_eq!(from_ipc_bytes(&bytes)?, table);
# Ok(())
# }
```

IPC stream bytes are suitable for `pyarrow.ipc.open_stream`, DuckDB, and other
Arrow IPC consumers.

## Python binding surface

Rust producers build a `TableEnvelope`; the Python binding converts it with
`to_record_batch` and wraps the batch as `finstack_quant.core.table.ArrowTable`,
which implements the Arrow PyCapsule C-stream protocol
(`__arrow_c_stream__`). Host tools consume it zero-copy:

```python
import pyarrow as pa
import polars as pl

# arrow_table comes from StatementResult.to_arrow_long/wide or
# PortfolioValuation.to_arrow_positions (no public ArrowTable constructor).
table = pa.table(arrow_table)
df = pl.DataFrame(arrow_table)
```

Parity note: `finstack-quant-py/parity_contract.toml` marks `core.table` as
Python-only (no WASM). See `finstack-quant-py/finstack_quant/core/table.pyi`.

## Related types

- `finstack_quant_core::table::TableEnvelope` — canonical tabular result model
- `finstack_quant_core::table::TableColumn` / `TableColumnData` / `TableColumnRole`
- Python: `finstack_quant.core.table.ArrowTable`

## Verification

```bash
cargo fmt -p finstack-quant-arrow
cargo clippy -p finstack-quant-arrow --all-features -- -D warnings
cargo test -p finstack-quant-arrow
cargo test -p finstack-quant-arrow --doc
RUSTDOCFLAGS='-D warnings' cargo doc -p finstack-quant-arrow --no-deps
```

Python host-interop (optional, after `mise run python-build`):

```bash
uv run pytest finstack-quant-py/tests/test_arrow_interchange.py finstack-quant-py/tests/test_to_arrow_producers.py
```
