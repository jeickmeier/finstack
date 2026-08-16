# Bond Future

Exchange bond future on a deliverable basket. Pricing marks the
**caller-supplied CTD**; it does not rank the basket.

## Conventions

- Resolve the CTD with `BondFuture::determine_ctd_by_implied_repo` (or the
  other `determine_ctd*` helpers) and write it back onto the instrument.
  Refresh that choice when the basket can switch. `Instrument::value` will
  not search.
- Invoice price is `(futures price × conversion factor) + accrued`.
  Variation-margin futures are not discounted further.

Import path:
`finstack_quant_valuations::instruments::fixed_income::bond_future`
(`BondFuture` is also re-exported at `finstack_quant_valuations::instruments`).
