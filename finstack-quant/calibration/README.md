# finstack-quant-calibration

Quote ingestion, market construction, calibration execution, explicit model
fitting, and cached quote-space recalibration for `finstack-quant`.

The dependency direction is one-way:

```text
calibration -> valuations -> core/models/cashflows
```

`finstack-quant-valuations` owns the object-safe recalibration contract and
pricing concerns. This crate implements that contract with
`CachedRecalibrationProvider`; valuations never depends on this crate.

Public modules:

- `api`: calibration envelopes, engine configuration, reports, and validation.
- `quotes`: raw market quote contracts.
- `recalibration`: cached implementations of the valuations replay port.
- `hull_white`: explicit Hull-White parameter calibration.
- `lmm`: explicit Bermudan LMM base-volatility calibration.

Quote-to-instrument construction lives in crate-private `build/`.

Host APIs live at `finstack_quant.calibration` in Python and `calibration` in
the WASM facade. There are no compatibility exports under valuations.
