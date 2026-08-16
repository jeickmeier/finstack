# CDS Tranche

Synthetic CDO tranche on a CDS index (CDX / iTraxx). Attachment and
detachment are percents, not fractions.

## Conventions

- `CDSTranche::new` and `example()` default `standard_imm_dates` to **true**
  (20th of Mar/Jun/Sep/Dec). `standard()` is the named IMM constructor.
  Non-IMM schedules set `standard_imm_dates: false` after construction.
- Single-name CDS valuation still defaults to Bloomberg CDSW clean; that
  is independent of the tranche IMM flag.

Import path:
`finstack_quant_valuations::instruments::credit_derivatives::cds_tranche`
(`CDSTranche` is also re-exported at `finstack_quant_valuations::instruments`).
