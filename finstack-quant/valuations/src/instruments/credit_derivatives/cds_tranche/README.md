# CDS Tranche

Synthetic CDO tranche on a CDS index (CDX / iTraxx). Attachment and
detachment are percents, not fractions.

## Conventions

- `CDSTranche::new` honors `ScheduleParams` (`standard_imm_dates` is
  **false**). `standard()` and `example()` are the IMM constructors
  (20th of Mar/Jun/Sep/Dec).
- Single-name CDS valuation still defaults to Bloomberg CDSW clean; that
  is independent of the tranche IMM flag.

Import path:
`finstack_quant_valuations::instruments::credit_derivatives::cds_tranche`
(`CDSTranche` is also re-exported at `finstack_quant_valuations::instruments`).
