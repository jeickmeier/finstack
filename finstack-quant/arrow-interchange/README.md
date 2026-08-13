# finstack-quant-arrow

Apache Arrow export for finstack-quant tabular outputs.

Directory: `finstack-quant/arrow-interchange`. Package / import name:
`finstack-quant-arrow` / `finstack_quant_arrow`.

Converts [`finstack_quant_core::table::TableEnvelope`](../core/src/table.rs)
to an Arrow `RecordBatch`. Column roles and metadata are written into Arrow
field/schema metadata. This crate is export-only: it does not convert Arrow
data back into a `TableEnvelope` and does not read or write Arrow IPC bytes.

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
use finstack_quant_arrow::to_record_batch;
use finstack_quant_core::table::{TableColumn, TableColumnData, TableEnvelope};
```

## Public API

| Item | Role |
|------|------|
| `to_record_batch` | `TableEnvelope` → Arrow `RecordBatch` |
| `ROLE_METADATA_KEY` | Field metadata key for column role (`"finstack:role"`) |
| `METADATA_KEY` | Field/schema metadata key for JSON metadata (`"finstack:metadata"`) |

Fallible APIs return `finstack_quant_core::Result`, mapping Arrow failures to
`Error::Validation` with an `arrow …` context prefix.

## Column type map

| `TableColumnData` | Arrow type | Nullable |
|-------------------|------------|----------|
| `String` | `Utf8` | no |
| `NullableString` | `Utf8` | yes |
| `Float64` | `Float64` | no |
| `NullableFloat64` | `Float64` | yes |
| `UInt32` | `UInt32` | no |
| `NullableUInt32` | `UInt32` | yes |
| `Int64` | `Int64` | no |
| `NullableInt64` | `Int64` | yes |

`to_record_batch` always emits the plain types above (`Utf8`, `Float64`,
`UInt32`, `Int64`). Nullability is schema-driven: a nullable envelope column
becomes a nullable Arrow field even when the column contains no nulls.
Non-finite floats (`NaN`, `±∞`) are preserved.

## Metadata contract

| Key | Constant | Location | Value |
|-----|----------|----------|-------|
| `finstack:role` | `ROLE_METADATA_KEY` | Arrow field metadata | Role wire name: `dimension`, `index`, `measure`, or `attribute` (same spelling as `TableColumnRole` serde) |
| `finstack:metadata` | `METADATA_KEY` | Arrow field metadata | JSON object of per-column metadata (`IndexMap<String, Value>`) |
| `finstack:metadata` | `METADATA_KEY` | Arrow schema metadata | JSON object of table-level metadata |

Unknown `finstack:role` values are not produced by this crate; malformed
metadata JSON returns `Error::Validation`.

## Edge cases

- Empty envelope (0 columns, 0 rows) exports as a zero-row, zero-column batch.
- Zero rows with columns export with the schema preserved.
- A valid `TableEnvelope` already has equal column lengths, so the exported
  batch row count matches `TableEnvelope::row_count`.

## Quick start

```rust
use finstack_quant_arrow::to_record_batch;
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
assert_eq!(batch.num_columns(), 2);
# Ok(())
# }
```

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
